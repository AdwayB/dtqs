use warp::Filter;
use sqlx::Pool;
use sqlx::Postgres;
use lapin::Channel;
pub mod tasks;
pub mod sse;

pub fn routes(
  db_pool: Pool<Postgres>,
  rabbit_channel: Channel
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  tasks::submit_route(db_pool.clone(), rabbit_channel.clone())
    .or(sse::sse_route(db_pool))
}