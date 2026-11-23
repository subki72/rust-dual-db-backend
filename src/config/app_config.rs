//! Application configuration, connection pool initialization, and environment parsing.

use std::env;
use std::time::Duration;

use clickhouse::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::utils::validation::{parse_allowed_origins, redact_dsn};

/// Default HTTP server port when `PORT` is unset or invalid.
pub const DEFAULT_PORT: u16 = 3000;

/// Maximum concurrent connections maintained by the PostgreSQL pool.
pub const DB_MAX_CONNECTIONS: u32 = 10;

/// Maximum time in seconds allowed to acquire a PostgreSQL connection.
pub const DB_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Default local frontend origins permitted by CORS when `ALLOWED_ORIGINS` is unset.
pub const DEFAULT_ALLOWED_ORIGINS: &str = "http://localhost:3000,http://localhost:5173";

/// Runtime application settings and initialized dual-database clients.
#[derive(Clone)]
pub struct AppConfig {
    /// Listening TCP port for the Axum HTTP server.
    pub port: u16,
    /// Normalized CORS allowlist loaded from `ALLOWED_ORIGINS`.
    pub allowed_origins: Vec<String>,
    /// Shared PostgreSQL connection pool (OLTP).
    pub postgres_pool: PgPool,
    /// Shared ClickHouse HTTP client (OLAP).
    pub clickhouse_client: Client,
}

impl AppConfig {
    /// Loads environment variables and initializes PostgreSQL and ClickHouse connections.
    ///
    /// # Errors
    /// Returns an error if database connectivity fails or baseline DDL setup cannot execute.
    pub async fn init() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let port = env::var("PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let allowed_origins = load_allowed_origins();
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:default@localhost:5432/postgres".to_string());
        let clickhouse_url =
            env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".to_string());
        let clickhouse_user = env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
        let clickhouse_password = env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();
        let clickhouse_db = env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "default".to_string());

        let postgres_pool = init_postgres_pool(&database_url).await?;
        let clickhouse_client = init_clickhouse_client(
            &clickhouse_url,
            &clickhouse_user,
            &clickhouse_password,
            &clickhouse_db,
        )
        .await?;

        Ok(Self {
            port,
            allowed_origins,
            postgres_pool,
            clickhouse_client,
        })
    }
}

/// Reads and normalizes `ALLOWED_ORIGINS` from the environment with a safe local default.
pub fn load_allowed_origins() -> Vec<String> {
    let raw = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.to_string());
    let parsed = parse_allowed_origins(&raw);
    if parsed.is_empty() {
        return parse_allowed_origins(DEFAULT_ALLOWED_ORIGINS);
    }
    parsed
}

/// Initializes the PostgreSQL connection pool with redacted logging and explicit acquire timeout.
async fn init_postgres_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let redacted_url = redact_dsn(database_url);
    tracing::info!(dsn = %redacted_url, "Menghubungkan ke PostgreSQL");

    let connect_timeout = Duration::from_secs(DB_CONNECT_TIMEOUT_SECS);
    let postgres_pool = PgPoolOptions::new()
        .max_connections(DB_MAX_CONNECTIONS)
        .acquire_timeout(connect_timeout)
        .connect(database_url)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            email VARCHAR(100) UNIQUE NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&postgres_pool)
    .await?;

    tracing::info!("Tabel 'users' di PostgreSQL siap.");
    Ok(postgres_pool)
}

/// Initializes the ClickHouse OLAP client and verifies the `user_activities` table exists.
async fn init_clickhouse_client(
    clickhouse_url: &str,
    clickhouse_user: &str,
    clickhouse_password: &str,
    clickhouse_db: &str,
) -> Result<Client, Box<dyn std::error::Error>> {
    let redacted_url = redact_dsn(clickhouse_url);
    tracing::info!(url = %redacted_url, database = %clickhouse_db, "Menghubungkan ke ClickHouse");

    let clickhouse_client = Client::default()
        .with_url(clickhouse_url)
        .with_user(clickhouse_user)
        .with_password(clickhouse_password)
        .with_database(clickhouse_db);

    clickhouse_client
        .query(
            "CREATE TABLE IF NOT EXISTS user_activities (
                user_id UInt64,
                action String,
                details String,
                timestamp UInt64
            ) ENGINE = MergeTree()
            ORDER BY (user_id, timestamp);",
        )
        .execute()
        .await?;

    tracing::info!("Tabel 'user_activities' di ClickHouse siap.");
    Ok(clickhouse_client)
}
