use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
  pub id: Uuid,
  pub task_type: String,
  pub payload: serde_json::Value,
  pub status: String,
  pub priority: i32,
  pub progress: i32,
  pub attempts: i32,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerNode {
  pub node_id: String,
  pub status: String,
  pub last_health_check: DateTime<Utc>,
  pub current_task_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
  pub timestamp: DateTime<Utc>,
  pub worker_node_id: Option<String>,
  pub message: String,
}
