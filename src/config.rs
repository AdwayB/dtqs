use std::env;

#[derive(Debug, Clone)]
pub struct Config {
  pub database_url: String,
  pub rabbitmq_url: String,
  pub server_port: u16,
}

impl Config {
  pub fn from_env() -> Self {
    Self {
      database_url: env::var("DATABASE_URL").unwrap(),
      rabbitmq_url: env::var("RABBITMQ_URL").unwrap(),
      server_port: env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080),
    }
  }
}