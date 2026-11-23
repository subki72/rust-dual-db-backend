//! Braincode Backend library crate exposing modular domain layers and router assembly.

pub mod config;
pub mod dtos;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod services;
pub mod utils;

use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method, Uri},
    routing::{get, post},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::services::{AlarmService, UserService};
use crate::utils::AppError;

/// Fallback localhost origin used when `ALLOWED_ORIGINS` has no valid entries.
const FALLBACK_LOCAL_ORIGIN: &str = "http://localhost:3000";

/// Centralized HTTP fallback error handler ensuring consistent JSON error envelopes.
pub struct ErrorHandler;

impl ErrorHandler {
    /// Handles unmatched HTTP routes by returning a structured JSON `404 Not Found` error.
    ///
    /// # Arguments
    /// * `uri` - Request URI that did not match any registered route.
    pub async fn handle_not_found(uri: Uri) -> AppError {
        tracing::info!(path = %uri.path(), "Unmatched route intercepted by ErrorHandler");
        AppError::NotFound(format!("Endpoint tidak ditemukan: {}", uri.path()))
    }
}

/// Constructs an explicit CORS layer from the configured origin allowlist.
fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let mut parsed_origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin.trim()) {
            Ok(val) => Some(val),
            Err(err) => {
                tracing::warn!(origin = %origin, error = %err, "Ignoring invalid CORS origin");
                None
            }
        })
        .collect();

    if parsed_origins.is_empty() {
        tracing::warn!(
            fallback = FALLBACK_LOCAL_ORIGIN,
            "No valid ALLOWED_ORIGINS configured; falling back to local origin"
        );
        parsed_origins.push(HeaderValue::from_static(FALLBACK_LOCAL_ORIGIN));
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
}

/// Assembles the complete Axum application [`Router`] with dependency-injected services,
/// explicit CORS policy, health probe, and centralized fallback [`ErrorHandler`].
///
/// # Arguments
/// * `user_service` - Injected implementation of [`UserService`].
/// * `alarm_service` - Injected implementation of [`AlarmService`].
/// * `allowed_origins` - Slice of allowed CORS origins parsed from `ALLOWED_ORIGINS`.
pub fn build_app_router(
    user_service: Arc<dyn UserService>,
    alarm_service: Arc<dyn AlarmService>,
    allowed_origins: &[String],
) -> Router {
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
        .route("/alarm_list_active", post(handlers::get_alarm_list_active))
        .route(
            "/api/v1/alarms/active",
            post(handlers::get_alarm_list_active),
        )
        .with_state(alarm_service);

    tracing::info!(
        allowed_origin_count = allowed_origins.len(),
        "Application router assembled"
    );

    Router::new()
        .route("/health", get(handlers::health_check))
        .merge(user_routes)
        .merge(alarm_routes)
        .fallback(ErrorHandler::handle_not_found)
        .layer(build_cors_layer(allowed_origins))
}
