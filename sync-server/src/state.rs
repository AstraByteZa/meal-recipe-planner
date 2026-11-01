use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::hub::Hub;
use crate::throttle::JoinThrottle;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<[u8]>,
    pub registration_open: bool,
    pub hub: Hub,
    pub join_throttle: JoinThrottle,
}

impl AppState {
    pub fn new(pool: PgPool, config: &Config) -> Self {
        Self {
            pool,
            jwt_secret: Arc::from(config.jwt_secret.as_bytes()),
            registration_open: config.registration_open,
            hub: Hub::default(),
            join_throttle: JoinThrottle::default(),
        }
    }
}
