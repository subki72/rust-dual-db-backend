mod config;
mod dtos;
mod handlers;
mod models;
mod repositories;
mod services;
mod utils;

use axum::{routing::{get, post}, Router};
use config::AppConfig;
use repositories::{ClickHouseActivityRepository, ClickHouseAlarmRepository, PostgresUserRepository};
use services::{AlarmService, AlarmServiceImpl, UserService, UserServiceImpl};
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
    let activity_repo = Arc::new(ClickHouseActivityRepository::new(config.clickhouse_client.clone()));
    let alarm_repo = Arc::new(ClickHouseAlarmRepository::new(config.clickhouse_client));

    // 4. Inisialisasi Layer Service
    let user_service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(user_repo, activity_repo));
    let alarm_service: Arc<dyn AlarmService> = Arc::new(AlarmServiceImpl::new(alarm_repo));

    // 5. Inisialisasi Router Axum (Modular Route Grouping)
    let user_routes = Router::new()
        .route(
            "/api/v1/users",
            get(handlers::get_users).post(handlers::create_user),
        )
        .route(
            "/api/v1/analytics/activities",
            get(handlers::get_activities),
        )
        .with_state(user_service);

    let alarm_routes = Router::new()
        .route(
            "/alarm_list_active",
            post(handlers::get_alarm_list_active),
        )
        .route(
            "/api/v1/alarms/active",
            post(handlers::get_alarm_list_active),
        )
        .with_state(alarm_service);

    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .merge(user_routes)
        .merge(alarm_routes)
        .layer(CorsLayer::permissive());

    // 6. Jalankan HTTP Listener
    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    println!("\n Server Axum siap menerima request di: http://localhost:{}", config.port);
    println!("Endpoints yang tersedia:");
    println!("   - GET  http://localhost:{}/health", config.port);
    println!("   - GET  http://localhost:{}/api/v1/users (PostgreSQL)", config.port);
    println!("   - POST http://localhost:{}/api/v1/users (PostgreSQL + ClickHouse)", config.port);
    println!("   - GET  http://localhost:{}/api/v1/analytics/activities (ClickHouse)", config.port);
    println!("   - POST http://localhost:{}/alarm_list_active (ClickHouse Active Alarms - Issue #2)", config.port);
    println!("   - POST http://localhost:{}/api/v1/alarms/active (ClickHouse Active Alarms)", config.port);
    println!("============================================================\n");

    axum::serve(listener, app).await?;

    Ok(())
}
