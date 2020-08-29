use crate::settings::Settings;
use diesel::prelude::*;
use diesel::r2d2;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use std::sync::{Arc};

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type Connection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct AppState {
	pub settings: Arc<Settings>,
	pool: Pool,
}

impl AppState {

	pub fn new(settings: Settings, pool: Pool) -> Self {
		AppState {
			settings: Arc::new(settings),
			pool,
		}
	}

	pub fn new_connection(&self) -> Connection {
		self.pool.get().unwrap()
	}

}