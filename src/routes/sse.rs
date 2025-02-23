use warp::{ Filter};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use sqlx::{Pool, Postgres};
use serde_json::json;

#[derive(Debug)]
struct CustomError {
  message: String
}
impl warp::reject::Reject for CustomError {}

fn with_db(db_pool: Pool<Postgres>) -> impl Filter<Extract = (Pool<Postgres>,), Error = Infallible> + Clone {
  warp::any().map(move || db_pool.clone())
}

pub fn sse_route(db_pool: Pool<Postgres>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path("sse")
    .and(warp::get())
    .and(warp::query::<std::collections::HashMap<String, String>>())
    .and(with_db(db_pool))
    .and_then(handle_sse)
}

async fn handle_sse(query: std::collections::HashMap<String, String>, db_pool: Pool<Postgres>) -> Result<impl warp::Reply, warp::Rejection> {
  let task_id = query.get("task_id").ok_or_else(|| warp::reject::custom(CustomError {message: "Missing task_id".to_string()}))?.clone();

  let interval = IntervalStream::new(tokio::time::interval(Duration::from_secs(2)));
  let stream = interval.then(move |_| {
    let db_pool = db_pool.clone();
    let task_id = task_id.clone();
    async move {
      let row = sqlx::query!("SELECT status, progress FROM tasks WHERE id = $1", Uuid::parse_str(&task_id).unwrap())
        .fetch_optional(&db_pool)
        .await;
      match row {
        Ok(Some(record)) => {
          if record.status != "pending" {
            let event = warp::sse::Event::default()
              .data(json!({"task_id": task_id, "status": record.status, "progress": record.progress}).to_string());
            return Some(Ok(event));
          }
        },
        Ok(None) => {
        },
        Err(e) => {
          println!("Error fetching task status: {:?}", e);
        }
      }
      None
    }
  })
    .filter_map(|x| { x });

  Ok(warp::sse::reply(warp::sse::keep_alive().stream(stream)))
}
