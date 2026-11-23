//! Composition root for the Braincode Dual-Database Backend HTTP service.

use std::sync::Arc;

use braincode_be::build_app_router;
use braincode_be::config::AppConfig;
use braincode_be::repositories::{
    ClickHouseActivityRepository, ClickHouseAlarmRepository, PostgresUserRepository,
};
use braincode_be::services::{AlarmService, AlarmServiceImpl, UserService, UserServiceImpl};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "braincode_be=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        service = "braincode-be",
        "Starting dual-database backend server"
    );

    let config = AppConfig::init().await?;

    let user_repo = Arc::new(PostgresUserRepository::new(config.postgres_pool));
    let activity_repo = Arc::new(ClickHouseActivityRepository::new(
        config.clickhouse_client.clone(),
    ));
    let alarm_repo = Arc::new(ClickHouseAlarmRepository::new(config.clickhouse_client));

    let user_service: Arc<dyn UserService> =
        Arc::new(UserServiceImpl::new(user_repo, activity_repo));
    let alarm_service: Arc<dyn AlarmService> = Arc::new(AlarmServiceImpl::new(alarm_repo));

    let app = build_app_router(user_service, alarm_service, &config.allowed_origins);

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    tracing::info!(
        listen_addr = %address,
        endpoints = "/health, /api/v1/users, /api/v1/analytics/activities, /alarm_list_active, /api/v1/alarms/active",
        "Axum server listening for HTTP traffic"
    );

    axum::serve(listener, app).await?;
    Ok(())
}
