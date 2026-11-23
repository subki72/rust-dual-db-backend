//! Application configuration and connection pool setup.

pub mod app_config;

pub use app_config::{
    load_allowed_origins, AppConfig, DB_CONNECT_TIMEOUT_SECS, DB_MAX_CONNECTIONS,
    DEFAULT_ALLOWED_ORIGINS, DEFAULT_PORT,
};
