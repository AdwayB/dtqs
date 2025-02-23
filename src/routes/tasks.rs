use warp::{Filter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Pool;
use sqlx::Postgres;
use lapin::Channel;
use tracing::{info, error};
use crate::messaging::publish_message;
use regex::Regex;
use std::format;

#[derive(Deserialize)]
pub struct NewTask {
  pub task_type: String,
  pub payload: serde_json::Value,
  pub priority: Option<u8>,
}

#[derive(Serialize)]
pub struct TaskResponse {
  pub task_id: Uuid,
  pub status: String,
  pub sse_url: String,
}

#[derive(Debug)]
struct CustomError {
  message: String
}
impl warp::reject::Reject for CustomError {}

fn sanitize_input(input: &str) -> bool {
  let re = Regex::new(r"^[\w\s.,@!?\-]+$").unwrap();
  re.is_match(input)
}

fn validate_payload(task_type: &str, payload: &serde_json::Value) -> Result<(), String> {
  match task_type {
    "email" => {
      for field in &["from", "to", "subject", "content"] {
        if let Some(val) = payload.get(*field) {
          if !val.is_string() || !sanitize_input(val.as_str().unwrap()) {
            return Err(format!("Invalid or unsafe value for field '{}'", field));
          }
        } else {
          return Err(format!("Missing field '{}'", field));
        }
      }
    },
    "image" => {
      if let Some(val) = payload.get("img_src") {
        if !val.is_string() || !sanitize_input(val.as_str().unwrap()) {
          return Err("Invalid or unsafe 'img_src'".into());
        }
      } else {
        return Err("Missing 'img_src' field".into());
      }
      if payload.get("resize_factor").is_none() {
        return Err("Missing 'resize_factor' field".into());
      }
    },
    "video" => {
      if let Some(val) = payload.get("vid_src") {
        if !val.is_string() || !sanitize_input(val.as_str().unwrap()) {
          return Err("Invalid or unsafe 'vid_src'".into());
        }
      } else {
        return Err("Missing 'vid_src' field".into());
      }
      if payload.get("resize_factor").is_none() {
        return Err("Missing 'resize_factor' field".into());
      }
    },
    _ => return Err("Unsupported task type".into()),
  }
  Ok(())
}

pub fn submit_route(db_pool: Pool<Postgres>, rabbit_channel: Channel) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path("submit")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_db(db_pool))
    .and(with_channel(rabbit_channel))
    .and_then(handle_submit_task)
}

fn with_db(db_pool: Pool<Postgres>) -> impl Filter<Extract = (Pool<Postgres>,), Error = std::convert::Infallible> + Clone {
  warp::any().map(move || db_pool.clone())
}

fn with_channel(channel: Channel) -> impl Filter<Extract = (Channel,), Error = std::convert::Infallible> + Clone {
  warp::any().map(move || channel.clone())
}

async fn handle_submit_task(new_task: NewTask, db_pool: Pool<Postgres>, channel: Channel) -> Result<impl warp::Reply, warp::Rejection> {
  if let Err(e) = validate_payload(&new_task.task_type, &new_task.payload) {
    error!("Payload validation failed: {}", e);
    return Err(warp::reject::custom(CustomError {message: e}));
  }

  let task_id = Uuid::new_v4();
  let now = Utc::now().naive_utc();
  let status = "pending";
  let priority = new_task.priority.unwrap_or(5) as i32;

  sqlx::query!(
        "INSERT INTO tasks (id, task_type, payload, status, priority, progress, attempts, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 0, 0, $6, $6)",
        task_id,
        new_task.task_type,
        new_task.payload,
        status,
        priority,
        now
    )
    .execute(&db_pool)
    .await
    .map_err(|e| {
      error!("DB insertion failed: {:?}", e);
      warp::reject::custom(e)
    })?;

  let task_message = serde_json::json!({
        "task_id": task_id.to_string(),
        "task_type": new_task.task_type,
        "payload": new_task.payload,
        "priority": priority,
    });

  let payload_bytes = serde_json::to_vec(&task_message).map_err(|e| {
    error!("Serialization failed: {:?}", e);
    warp::reject::custom(CustomError {message: "Serialization Failed.".to_string()})
  })?;

  publish_message(&channel, "task_queue", &payload_bytes)
    .await
    .map_err(|e| {
      error!("Failed to publish task {}: {:?}", task_id, e);
      warp::reject::custom(CustomError {message: "An error occurred when publishing task.".to_string()})
    })?;

  info!("Task {} submitted successfully", task_id);
  let response = TaskResponse {
    task_id,
    status: "submitted".into(),
    sse_url: format!("/sse?task_id={}", task_id),
  };

  Ok(warp::reply::json(&response))
}
