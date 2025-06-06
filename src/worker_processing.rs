use sqlx::PgPool;
use serde_json::Value;
use std::time::Duration;
use tokio::time::{sleep};
use anyhow::{Result, anyhow};
use tracing::info;

pub async fn update_progress_in_db(task_id: &str, db_pool: &PgPool, progress: i32) -> Result<()> {
  sqlx::query!(
        "UPDATE tasks SET progress = $1, updated_at = NOW() WHERE id::text = $2",
        progress,
        task_id
    )
    .execute(db_pool)
    .await?;
  Ok(())
}

pub async fn log_message(db_pool: &PgPool, worker_node_id: &str, message: &str) -> Result<()> {
  sqlx::query!(
        "INSERT INTO logs (worker_node_id, message) VALUES ($1, $2)",
        worker_node_id,
        message
    )
    .execute(db_pool)
    .await?;
  Ok(())
}

pub async fn process_email_task(task_data: &Value, db_pool: &PgPool, worker_id: &str) -> Result<()> {
  let task_id = task_data.get("task_id").and_then(|v| v.as_str()).ok_or(anyhow!("Missing task_id in email task"))?;
  info!("Worker {}: Processing email task {}", worker_id, task_id);
  log_message(db_pool, worker_id, &format!("Started email task {}", task_id)).await?;

  for progress in &[20, 40, 60, 80] {
    sleep(Duration::from_secs(3)).await;
    update_progress_in_db(task_id, db_pool, *progress).await?;
    log_message(db_pool, worker_id, &format!("Email task {} progress {}%", task_id, progress)).await?;
  }
  
  update_progress_in_db(task_id, db_pool, 100).await?;
  log_message(db_pool, worker_id, &format!("Completed email task {}", task_id)).await?;
  Ok(())
}

pub async fn process_video_task(task_data: &Value, db_pool: &PgPool, worker_id: &str) -> Result<()> {
  let task_id = task_data.get("task_id").and_then(|v| v.as_str()).ok_or(anyhow!("Missing task_id in video task"))?;
  info!("Worker {}: Processing video task {}", worker_id, task_id);
  log_message(db_pool, worker_id, &format!("Started video task {}", task_id)).await?;

  for progress in &[25, 50, 75] {
    sleep(Duration::from_secs(3)).await;
    update_progress_in_db(task_id, db_pool, *progress).await?;
    log_message(db_pool, worker_id, &format!("Video task {} progress {}%", task_id, progress)).await?;
  }
  
  update_progress_in_db(task_id, db_pool, 100).await?;
  log_message(db_pool, worker_id, &format!("Completed video task {}", task_id)).await?;
  Ok(())
}

pub async fn process_image_task(task_data: &Value, db_pool: &PgPool, worker_id: &str) -> Result<()> {
  let task_id = task_data.get("task_id").and_then(|v| v.as_str()).ok_or(anyhow!("Missing task_id in image task"))?;
  info!("Worker {}: Processing image task {}", worker_id, task_id);
  log_message(db_pool, worker_id, &format!("Started image task {}", task_id)).await?;

  sleep(Duration::from_secs(3)).await;
  update_progress_in_db(task_id, db_pool, 50).await?;
  log_message(db_pool, worker_id, &format!("Image task {} progress 50%", task_id)).await?;
  sleep(Duration::from_secs(3)).await;
  update_progress_in_db(task_id, db_pool, 100).await?;
  log_message(db_pool, worker_id, &format!("Completed image task {}", task_id)).await?;
  Ok(())
}
