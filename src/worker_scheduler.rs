use std::collections::BinaryHeap;
use tokio::sync::Mutex;
use std::cmp::Ordering;
use lapin::message::Delivery;
use serde_json::Value;

#[derive(Debug)]
pub struct ScheduledTask {
  pub priority: u8,
  pub delivery: Delivery,
  pub task_data: Value,
}

impl Eq for ScheduledTask {}

impl PartialEq for ScheduledTask {
  fn eq(&self, other: &Self) -> bool {
    self.priority == other.priority
  }
}

impl PartialOrd for ScheduledTask {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(other.priority.cmp(&self.priority))
  }
}

impl Ord for ScheduledTask {
  fn cmp(&self, other: &Self) -> Ordering {
    other.priority.cmp(&self.priority)
  }
}

pub struct Scheduler {
  queue: Mutex<BinaryHeap<ScheduledTask>>,
}

impl Scheduler {
  pub fn new() -> Self {
    Self {
      queue: Mutex::new(BinaryHeap::new()),
    }
  }

  pub async fn add_task(&self, task: ScheduledTask) {
    self.queue.lock().await.push(task);
  }

  pub async fn get_next(&self) -> Option<ScheduledTask> {
    self.queue.lock().await.pop()
  }
}
