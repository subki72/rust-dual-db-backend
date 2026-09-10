mod config;
mod dtos;
mod handlers;
mod models;
mod repositories;
mod services;
mod utils;

use axum::{routing::get, Router};
use config::AppConfig;
use repositories::{ClickHouseActivityRepository, PostgresUserRepository};
use services::{UserService, UserServiceImpl};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inisialisasi Tracing & Logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "braincode_be=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("============================================================");
    println!("     MEMULAI SERVER BACKEND ENTERPRISE: BRAINCODE-BE     ");
    println!("============================================================");

    // 2. Inisialisasi Konfigurasi & Koneksi Database
    let config = AppConfig::init().await?;

    // 3. Inisialisasi Layer Repositori
    let user_repo = Arc::new(PostgresUserRepository::new(config.postgres_pool));
    let activity_repo = Arc::new(ClickHouseActivityRepository::new(config.clickhouse_client));

    // 4. Inisialisasi Layer Service (Menggabungkan Business Logic, Postgres & ClickHouse)
    let user_service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(user_repo, activity_repo));

    // 5. Inisialisasi Router Axum
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route(
            "/api/v1/users",
            get(handlers::get_users).post(handlers::create_user),
        )
        .route(
            "/api/v1/analytics/activities",
            get(handlers::get_activities),
        )
        .layer(CorsLayer::permissive())
        .with_state(user_service);

    // 6. Jalankan HTTP Listener
    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    println!("\n Server Axum siap menerima request di: http://localhost:{}", config.port);
    println!("Endpoints yang tersedia:");
    println!("   - GET  http://localhost:{}/health", config.port);
    println!("   - GET  http://localhost:{}/api/v1/users (PostgreSQL)", config.port);
    println!("   - POST http://localhost:{}/api/v1/users (PostgreSQL + ClickHouse)", config.port);
    println!("   - GET  http://localhost:{}/api/v1/analytics/activities (ClickHouse)", config.port);
    println!("============================================================\n");

    axum::serve(listener, app).await?;

    Ok(())
}
