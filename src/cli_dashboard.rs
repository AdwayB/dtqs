//! Overview (worker nodes and active tasks)
//! Queue (next 5 pending tasks)
//! Logs (latest log entries)

use std::{
  error::Error,
  io,
  sync::{Arc},
  thread,
  time::{Duration, Instant},
};

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
  backend::{Backend, CrosstermBackend},
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Span, Spans},
  widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
  Terminal,
};
use sqlx::{Pool, Postgres};
use crate::{config::Config, database::setup_database};
use crate::messaging::create_rabbit_channel;
use lapin::Channel;
use tokio::runtime::Runtime;
use futures_lite::StreamExt;

struct TaskInfo {
  id: String,
  task_type: String,
  status: String,
  progress: u8,
}

struct WorkerNodeInfo {
  node_id: String,
  status: String,
  last_health_check: String,
  current_task: Option<TaskInfo>,
}

struct LogEntry {
  timestamp: String,
  message: String,
}

#[derive(Clone, Copy)]
enum DashboardTab {
  Overview,
  Queue,
  Logs,
}

struct App {
  current_tab: DashboardTab,
  workers: Vec<WorkerNodeInfo>,
  queued_tasks: Vec<TaskInfo>,
  logs: Vec<LogEntry>,
  pending_count: u32,
}

impl App {
  fn new() -> Self {
    Self {
      current_tab: DashboardTab::Overview,
      workers: vec![],
      queued_tasks: vec![],
      logs: vec![],
      pending_count: 0,
    }
  }

  fn next_tab(&mut self) {
    self.current_tab = match self.current_tab {
      DashboardTab::Overview => DashboardTab::Queue,
      DashboardTab::Queue => DashboardTab::Logs,
      DashboardTab::Logs => DashboardTab::Overview,
    }
  }

  fn previous_tab(&mut self) {
    self.current_tab = match self.current_tab {
      DashboardTab::Overview => DashboardTab::Logs,
      DashboardTab::Queue => DashboardTab::Overview,
      DashboardTab::Logs => DashboardTab::Queue,
    }
  }
}

async fn fetch_db_state(pool: &Pool<Postgres>) -> Result<App, sqlx::Error> {
  let mut app = App::new();
  let worker_rows = sqlx::query!(
        r#"
        SELECT
            wn.node_id,
            wn.status,
            to_char(wn.last_health_check, 'YYYY-MM-DD HH24:MI:SS') as last_health_check,
            t.id as task_id,
            t.task_type,
            t.status as task_status,
            t.progress
        FROM worker_nodes wn
        LEFT JOIN tasks t ON wn.current_task_id = t.id
        ORDER BY wn.last_health_check DESC
        "#
    )
    .fetch_all(pool)
    .await?;
  app.workers = worker_rows
    .into_iter()
    .map(|row| WorkerNodeInfo {
      node_id: row.node_id,
      status: row.status,
      last_health_check: row.last_health_check.unwrap_or_else(|| "N/A".into()),
      current_task: row.task_id.map(|id| TaskInfo {
        id: id.to_string(),
        task_type: row.task_type.unwrap_or_else(|| "N/A".into()),
        status: row.task_status.unwrap_or_else(|| "N/A".into()),
        progress: row.progress.unwrap_or(0) as u8,
      }),
    })
    .collect();

  let task_rows = sqlx::query!(
        r#"
        SELECT id, task_type, status, progress
        FROM tasks
        WHERE status = 'pending'
        ORDER BY created_at
        LIMIT 5
        "#
    )
    .fetch_all(pool)
    .await?;
  app.queued_tasks = task_rows
    .into_iter()
    .map(|row| TaskInfo {
      id: row.id.to_string(),
      task_type: row.task_type,
      status: row.status,
      progress: row.progress.unwrap_or(0) as u8,
    })
    .collect();

  let log_rows = sqlx::query!(
        r#"
        SELECT to_char(timestamp, 'YYYY-MM-DD HH24:MI:SS') as timestamp, message
        FROM logs
        ORDER BY timestamp DESC
        LIMIT 20
        "#
    )
    .fetch_all(pool)
    .await?;
  app.logs = log_rows
    .into_iter()
    .map(|row| LogEntry {
      timestamp: row.timestamp.unwrap_or_else(|| "N/A".into()),
      message: row.message,
    })
    .collect();

  Ok(app)
}

async fn fetch_rabbitmq_state(channel: &Channel) -> Result<u32, lapin::Error> {
  let queue = channel
    .queue_declare("task_queue", lapin::options::QueueDeclareOptions { passive: true, ..Default::default() }, lapin::types::FieldTable::default())
    .await?;
  Ok(queue.message_count())
}

fn main() -> Result<(), Box<dyn Error>> {
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let config = Config::from_env();
  let rt = Runtime::new()?;
  let db_pool = rt.block_on(setup_database(&config.database_url));
  let rabbit_channel = rt.block_on(create_rabbit_channel(&config.rabbitmq_url))
    .expect("Failed to create RabbitMQ channel");

  let db_pool_arc = Arc::new(db_pool);
  let rabbit_channel_arc = Arc::new(rabbit_channel);

  let (tx, rx) = std::sync::mpsc::channel::<App>();

  {
    let db_pool_clone = db_pool_arc.clone();
    let rabbit_channel_clone = rabbit_channel_arc.clone();
    thread::spawn(move || {
      let rt_bg = Runtime::new().unwrap();
      loop {
        let mut app_state = rt_bg.block_on(fetch_db_state(&db_pool_clone)).unwrap_or_else(|_| App::new());
        let pending = rt_bg.block_on(fetch_rabbitmq_state(&rabbit_channel_clone)).unwrap_or(0);
        app_state.pending_count = pending;
        let _ = tx.send(app_state);
        thread::sleep(Duration::from_secs(2));
      }
    });
  }

  let mut app = App::new();
  if let Ok(state) = rx.try_recv() {
    app = state;
  }

  let tick_rate = Duration::from_millis(500);
  let mut last_tick = Instant::now();

  loop {
    if let Ok(new_state) = rx.try_recv() {
      app = new_state;
    }
    terminal.draw(|f| ui(f, &app))?;

    let timeout = tick_rate
      .checked_sub(last_tick.elapsed())
      .unwrap_or_else(|| Duration::from_secs(0));
    if event::poll(timeout)? {
      if let CEvent::Key(key) = event::read()? {
        match key.code {
          KeyCode::Char('q') => break,
          KeyCode::Right => app.next_tab(),
          KeyCode::Left => app.previous_tab(),
          _ => {}
        }
      }
    }
    if last_tick.elapsed() >= tick_rate {
      last_tick = Instant::now();
    }
  }

  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
  terminal.show_cursor()?;
  Ok(())
}

fn ui<B: Backend>(f: &mut tui::Frame<B>, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints([
      Constraint::Length(3),
      Constraint::Min(0),
      Constraint::Length(3),
    ].as_ref())
    .split(f.size());

  let tab_titles = vec!["Overview", "Queue", "Logs"];
  let tabs = Tabs::new(
    tab_titles
      .iter()
      .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Yellow)))
      )
      .collect(),
  )
    .block(Block::default().borders(Borders::ALL).title("Dashboard Tabs"))
    .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    .select(match app.current_tab {
      DashboardTab::Overview => 0,
      DashboardTab::Queue => 1,
      DashboardTab::Logs => 2,
    });
  f.render_widget(tabs, chunks[0]);

  match app.current_tab {
    DashboardTab::Overview => render_overview(f, app, chunks[1]),
    DashboardTab::Queue => render_queue(f, app, chunks[1]),
    DashboardTab::Logs => render_logs(f, app, chunks[1]),
  }

  let footer = Paragraph::new("←/→: Switch Tabs | q: Quit")
    .style(Style::default().fg(Color::White))
    .block(Block::default().borders(Borders::ALL));
  f.render_widget(footer, chunks[2]);
}

fn render_overview<B: Backend>(f: &mut tui::Frame<B>, app: &App, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    .split(area);

  let worker_items: Vec<ListItem> = app.workers.iter().map(|w| {
    let task_info = if let Some(task) = &w.current_task {
      format!("Task: {} ({}%, {})", task.id, task.progress, task.status)
    } else {
      "No current task".into()
    };
    let lines = vec![
      Spans::from(Span::styled(format!("ID: {}", w.node_id), Style::default().add_modifier(Modifier::BOLD))),
      Spans::from(Span::raw(format!("Status: {}", w.status))),
      Spans::from(Span::raw(task_info)),
      Spans::from(Span::raw(format!("Last HC: {}", w.last_health_check))),
    ];
    ListItem::new(lines)
  }).collect();

  let workers_list = List::new(worker_items)
    .block(Block::default().borders(Borders::ALL).title("Worker Nodes"))
    .highlight_style(Style::default().bg(Color::Blue));
  f.render_widget(workers_list, chunks[0]);

  let active_tasks: Vec<ListItem> = app.workers.iter().filter_map(|w| {
    w.current_task.as_ref().map(|t| {
      ListItem::new(Spans::from(vec![
        Span::raw(format!("{}: {} ({}%)", w.node_id, t.task_type, t.progress))
      ]))
    })
  }).collect();
  let tasks_list = List::new(if active_tasks.is_empty() { vec![ListItem::new(Spans::from(Span::raw("No active tasks")))] } else { active_tasks })
    .block(Block::default().borders(Borders::ALL).title("Active Tasks"));
  f.render_widget(tasks_list, chunks[1]);
}

fn render_queue<B: Backend>(f: &mut tui::Frame<B>, app: &App, area: Rect) {
  let task_items: Vec<ListItem> = app.queued_tasks.iter().map(|t| {
    ListItem::new(Spans::from(vec![
      Span::styled(format!("{} ", t.id), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
      Span::raw(format!("Type: {} | Status: {} | Progress: {}%", t.task_type, t.status, t.progress))
    ]))
  }).collect();
  let header = format!("Next 5 Tasks in Queue (Pending in RabbitMQ: {})", app.pending_count);
  let tasks_list = List::new(task_items)
    .block(Block::default().borders(Borders::ALL).title(header));
  f.render_widget(tasks_list, area);
}

fn render_logs<B: Backend>(f: &mut tui::Frame<B>, app: &App, area: Rect) {
  let log_items: Vec<ListItem> = app.logs.iter().map(|l| {
    ListItem::new(Spans::from(vec![
      Span::styled(&l.timestamp, Style::default().fg(Color::Green)),
      Span::raw(" - "),
      Span::raw(&l.message),
    ]))
  }).collect();
  let logs_list = List::new(log_items)
    .block(Block::default().borders(Borders::ALL).title("Worker Node Logs / Health Checks"));
  f.render_widget(logs_list, area);
}
