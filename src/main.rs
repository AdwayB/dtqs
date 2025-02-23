use warp::Filter;
use tracing_subscriber;
use dtqs::{config::Config, database::setup_database, messaging::create_rabbit_channel, routes::routes};

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  let config = Config::from_env();
  let db_pool = setup_database(&config.database_url).await;
  let rabbit_channel = create_rabbit_channel(&config.rabbitmq_url)
    .await
    .expect("Failed to create RabbitMQ channel");

  let api = routes(db_pool, rabbit_channel)
    .or(warp::path("metrics").map(|| "prometheus_metrics_placeholder"));

  warp::serve(api)
    .run(([0, 0, 0, 0], config.server_port))
    .await;
}
