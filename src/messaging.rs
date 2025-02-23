use lapin::{Connection, ConnectionProperties, Channel, options::BasicPublishOptions, BasicProperties};
use tokio_retry::Retry;
use tokio_retry::strategy::ExponentialBackoff;
use tracing::info;
use anyhow::Result;

static MAX_RETRIES: usize = 5;
static DELAY: u64 = 100;

pub async fn create_rabbit_channel(rabbitmq_url: &str) -> Result<Channel> {
  let conn = Retry::spawn(ExponentialBackoff::from_millis(DELAY).take(MAX_RETRIES), || {
    Connection::connect(rabbitmq_url, ConnectionProperties::default())
  })
    .await?;
  let channel = conn.create_channel().await?;
  info!("RabbitMQ channel created");
  Ok(channel)
}

pub async fn publish_message(channel: &Channel, queue: &str, payload: &[u8]) -> Result<()> {
  Retry::spawn(ExponentialBackoff::from_millis(DELAY).take(MAX_RETRIES), || async {
    channel.basic_publish("", queue, BasicPublishOptions::default(), payload, BasicProperties::default()).await
  })
    .await?;
  Ok(())
}
