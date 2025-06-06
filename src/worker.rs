use lapin::{options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions}, types::FieldTable};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::Duration;
use tracing::{info, error};
use crate::worker_scheduler::{Scheduler, ScheduledTask};
use crate::worker_processing::{process_email_task, process_video_task, process_image_task};
use crate::database::setup_database;
use crate::messaging::create_rabbit_channel;
use std::env;
use futures::StreamExt;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  let database_url = env::var("DATABASE_URL").unwrap();
  let rabbitmq_url = env::var("RABBITMQ_URL").unwrap();
  let worker_id = env::var("WORKER_ID").unwrap();

  let db_pool: Pool<Postgres> = setup_database(&database_url).await;
  let rabbit_channel = create_rabbit_channel(&rabbitmq_url)
    .await
    .expect("Failed to create RabbitMQ channel");

  let _ = rabbit_channel
    .queue_declare("task_queue", Default::default(), FieldTable::default())
    .await
    .expect("Queue declaration failed");

  let mut consumer = rabbit_channel
    .basic_consume("task_queue", "worker", BasicConsumeOptions::default(), FieldTable::default())
    .await
    .expect("Failed to start consumer");

  let scheduler = Arc::new(Scheduler::new());
  let semaphore = Arc::new(Semaphore::new(4));

  let scheduler_consumer = scheduler.clone();
  tokio::spawn(async move {
    while let Some(delivery) = consumer.next().await {
      match delivery {
        Ok(delivery) => {
          match serde_json::from_slice::<serde_json::Value>(&delivery.data) {
            Ok(task_data) => {
              let priority = task_data.get("priority")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as u8;
              let scheduled_task = ScheduledTask {
                priority,
                delivery,
                task_data,
              };
              scheduler_consumer.add_task(scheduled_task).await;
            }
            Err(e) => {
              error!("Failed to parse task: {:?}", e);
              let _ = delivery.ack(BasicAckOptions::default()).await;
            }
          }
        }
        Err(e) => error!("Consumer error: {:?}", e),
      }
    }
  });

  loop {
    if let Some(scheduled_task) = scheduler.get_next().await {
      let permit = semaphore.clone().acquire_owned().await.unwrap();
      let db_pool_clone = db_pool.clone();
      let task_data = scheduled_task.task_data.clone();
      let delivery = scheduled_task.delivery;
      let worker_id_clone = worker_id.clone();
      tokio::spawn(async move {
        let task_type = task_data.get("task_type").and_then(|v| v.as_str()).unwrap_or("");
        let task_id = task_data.get("task_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let processing_result = match task_type {
          "email" => process_email_task(&task_data, &db_pool_clone, &worker_id_clone).await,
          "video" => process_video_task(&task_data, &db_pool_clone, &worker_id_clone).await,
          "image" => process_image_task(&task_data, &db_pool_clone, &worker_id_clone).await,
          other   => Err(anyhow::anyhow!("Unknown task type: {}", other)),
        };
        match processing_result {
          Ok(_) => {
            info!("Task {} processed successfully", task_id);
            let _ = sqlx::query!(
                            "UPDATE tasks SET status = 'completed', progress = 100, updated_at = NOW() WHERE id::text = $1",
                            task_id
                        )
              .execute(&db_pool_clone)
              .await;
            let _ = delivery.ack(BasicAckOptions::default()).await;
          }
          Err(e) => {
            error!("Processing failed for task {}: {:?}", task_id, e);
            match sqlx::query!("UPDATE tasks SET attempts = attempts + 1, updated_at = NOW() WHERE id::text = $1 RETURNING attempts", task_id)
              .fetch_one(&db_pool_clone)
              .await {
              Ok(record) => {
                let attempts: i32 = record.attempts;
                if attempts < 5 {
                  error!("Retrying task {} (attempt {})", task_id, attempts);
                  let _ = delivery.nack(BasicNackOptions::default()).await;
                } else {
                  error!("Max attempts reached for task {}. Marking as failed.", task_id);
                  let _ = sqlx::query!(
                                        "UPDATE tasks SET status = 'failed', updated_at = NOW() WHERE id::text = $1",
                                        task_id
                                    )
                    .execute(&db_pool_clone)
                    .await;
                  let _ = delivery.ack(BasicAckOptions::default()).await;
                }
              }
              Err(err) => {
                error!("Failed to update attempt count for task {}: {:?}", task_id, err);
                let _ = delivery.nack(BasicNackOptions::default()).await;
              }
            }
          }
        }
        drop(permit);
      });
    } else {
      tokio::time::sleep(Duration::from_millis(100)).await;
    }
  }
}
