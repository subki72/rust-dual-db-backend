# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-23

### Added
- Library crate root (`src/lib.rs`) exporting modular architecture, configurable CORS router builder (`build_app_router`), and centralized `ErrorHandler` fallback for unmatched routes.
- Input validation and security utility module (`src/utils/validation.rs`) with `redact_dsn`, `validate_email`, `validate_non_empty`, `validate_date_range`, and `parse_allowed_origins`.
- Versioned database migrations under `migrations/` (`001_init_postgres_users.sql` and `002_init_clickhouse_tables.sql`).
- 11 offline unit and Axum `oneshot` HTTP integration test suites under `tests/`.
- Production containerization and orchestration (`Dockerfile`, `.dockerignore`, `docker-compose.yml`, `Makefile`).
- CI/CD, static analysis, and dependency governance (`.github/workflows/ci.yml`, `.github/dependabot.yml`, `clippy.toml`, `rustfmt.toml`, `.pre-commit-config.yaml`).
- Governance and operations documentation (`LICENSE`, `SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `runbook.md`).

### Changed
- Redacted PostgreSQL credentials from startup `tracing::info!` logs and added a 10-second pool acquire timeout in `AppConfig::init`.
- Replaced `println!` banners in `src/main.rs` and `src/bin/ingest_csv.rs` with structured `tracing` logging.
- Hardened CORS policy to honor `ALLOWED_ORIGINS` instead of unconditional permissive mode.
- Refactored `UserServiceImpl` and `AlarmServiceImpl` to reuse centralized validation helpers.

## [0.1.0] - 2026-09-15

### Added
- Initial Axum + Tokio REST API with PostgreSQL OLTP (`users`) and ClickHouse OLAP (`user_activities`, `active_alarms`).
- Active Alarm Service endpoints (`POST /alarm_list_active`, `POST /api/v1/alarms/active`).
- Streaming CSV to ClickHouse ingestion binary (`src/bin/ingest_csv.rs`).
