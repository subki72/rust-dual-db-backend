use clickhouse::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub port: u16,
    pub postgres_pool: PgPool,
    pub clickhouse_client: Client,
}

impl AppConfig {
    pub async fn init() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:default@localhost:5432/postgres".to_string());

        let clickhouse_url = env::var("CLICKHOUSE_URL")
            .unwrap_or_else(|_| "http://localhost:8123".to_string());

        let clickhouse_user = env::var("CLICKHOUSE_USER")
            .unwrap_or_else(|_| "default".to_string());

        let clickhouse_password = env::var("CLICKHOUSE_PASSWORD")
            .unwrap_or_else(|_| "".to_string());

        let clickhouse_db = env::var("CLICKHOUSE_DB")
            .unwrap_or_else(|_| "default".to_string());

        // 1. Inisialisasi PostgreSQL Connection Pool
        tracing::info!("Menghubungkan ke PostgreSQL: {}", database_url);
        let postgres_pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        // Inisialisasi tabel users di PostgreSQL jika belum ada
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

        // 2. Inisialisasi ClickHouse Client
        tracing::info!("Menghubungkan ke ClickHouse: {}", clickhouse_url);
        let clickhouse_client = Client::default()
            .with_url(&clickhouse_url)
            .with_user(&clickhouse_user)
            .with_password(&clickhouse_password)
            .with_database(&clickhouse_db);

        // Inisialisasi tabel user_activities di ClickHouse jika belum ada
        clickhouse_client
            .query(
                "CREATE TABLE IF NOT EXISTS user_activities (
                    user_id UInt64,
                    action String,
                    details String,
                    timestamp UInt64
                ) ENGINE = MergeTree()
                ORDER BY (user_id, timestamp);"
            )
            .execute()
            .await?;
        tracing::info!("Tabel 'user_activities' di ClickHouse siap.");

        Ok(Self {
            port,
            postgres_pool,
            clickhouse_client,
        })
    }
}
