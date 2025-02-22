use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
  pub id: Uuid,
  pub task_type: String,
  pub payload: serde_json::Value,
  pub status: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerNode {
  pub id: Uuid,
  pub node_id: String,
  pub status: String,
  pub last_health_check: DateTime<Utc>,
  pub current_task_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logs {
  pub id: Uuid,
  pub created_at: DateTime<Utc>,
  pub worker_node_id: String,
  pub message: serde_json::Value,
}
