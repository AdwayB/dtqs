use sqlx::{Pool, Postgres};
use sqlx::migrate::Migrator;
use tracing::info;

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn setup_database(database_url: &str) -> Pool<Postgres> {
  let pool = Pool::<Postgres>::connect(database_url)
    .await
    .expect("Failed to connect to database.");

  MIGRATOR.run(&pool)
    .await
    .expect("Failed to run database migrations.");
  info!("Database migrations complete");
  pool
}