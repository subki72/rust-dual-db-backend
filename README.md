# Braincode Backend: Dual-Database Enterprise REST API & Alarm Pipeline

[![CI Pipeline](https://github.com/subki72/rust-dual-db-backend/actions/workflows/ci.yml/badge.svg)](https://github.com/subki72/rust-dual-db-backend/actions/workflows/ci.yml)
[![Rust 2021](https://img.shields.io/badge/Rust-Edition%202021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> **High-Performance Asynchronous Backend in Rust (Axum, Tokio) featuring Clean Architecture, Dual-Database Persistence (PostgreSQL OLTP + ClickHouse OLAP), High-Throughput Streaming CSV Ingestion, and Production Security Hardening.**

---

## Executive Summary

**Braincode Backend** (`braincode-be`) is a production-grade REST API built in Rust to serve transactional workloads (**PostgreSQL 16 OLTP**) alongside high-throughput analytical and telecom network alarm queries (**ClickHouse 24.8 OLAP**).

Structured around **Clean Architecture (Dependency Inversion)**, every layer communicates through async trait contracts (`UserService`, `AlarmService`, `UserRepository`, `ActivityRepository`, `AlarmRepository`). This guarantees fast offline unit and HTTP integration testing, predictable zero-GC latency, and clean isolation between HTTP handlers, business rules, and database drivers.

---

## Dual-Database Architecture

```text
[ HTTP Client / NOC Frontend / Automated Scripts ]
                        |
                        v
+-------------------------------------------------------------+
| 1. ROUTER & SECURITY BOUNDARY (src/lib.rs, src/handlers/)   |
|    - Configurable CORS allowlist (ALLOWED_ORIGINS)          |
|    - Centralized ErrorHandler JSON fallback (404/4xx/5xx)   |
|    - Health probe (/health) + REST endpoints                |
+-----------------------------+-------------------------------+
                              | (Service Trait)
                              v
+-------------------------------------------------------------+
| 2. BUSINESS LOGIC & VALIDATION (src/services/, src/utils/)  |
|    - UserServiceImpl & AlarmServiceImpl                     |
|    - Shared guard-clause validation (email, dates, fields)  |
|    - Structured leveled logging (tracing::info/warn/error)  |
+--------------+------------------------------+---------------+
               |                              |
       (Postgres Trait)               (ClickHouse Trait)
               |                              |
               v                              v
+------------------------------+ +----------------------------+
| 3. OLTP PERSISTENCE          | | 4. OLAP ANALYTICS          |
|    - PostgresUserRepository  | |    - ClickHouseAlarmRepo   |
|    - sqlx PgPool (10s TO)    | |    - ClickHouseActivityRepo|
|    - Parameterized ($1, $2)  | |    - ReplacingMergeTree    |
+--------------+---------------+ +------------+---------------+
               |                              |
               v                              v
       [( PostgreSQL 16 )]            [( ClickHouse 24.8 )]
```

### Repository Layout

```text
rust-dual-db-backend/
├── .github/
│   ├── workflows/ci.yml        # fmt, clippy, test, release build, Semgrep, Docker, Trivy
│   └── dependabot.yml          # Weekly cargo, docker, github-actions updates (no major bumps)
├── migrations/
│   ├── 001_init_postgres_users.sql
│   └── 002_init_clickhouse_tables.sql
├── src/
│   ├── bin/ingest_csv.rs       # Streaming CSV -> ClickHouse ingestion binary
│   ├── config/app_config.rs    # Env loading, DSN redaction, PgPool timeout, ClickHouse client
│   ├── dtos/                   # Request/Response payloads & envelope types
│   ├── handlers/               # Axum HTTP handlers
│   ├── models/                 # Domain entities (User, UserActivity)
│   ├── repositories/           # Trait contracts & Postgres/ClickHouse implementations
│   ├── services/               # Business orchestration & validation
│   ├── utils/
│   │   ├── errors.rs           # AppError -> IntoResponse with safe external responses
│   │   └── validation.rs       # redact_dsn, validate_email, validate_date_range, CORS parser
│   ├── lib.rs                  # Public crate exports, build_app_router, ErrorHandler
│   └── main.rs                 # Composition root & tracing initialization
├── tests/                      # 11 offline unit & Axum oneshot HTTP integration test suites
├── Dockerfile                  # Multi-stage rust:1.82-slim -> debian:bookworm-slim (UID 10001)
├── docker-compose.yml          # PostgreSQL + ClickHouse + braincode-be stack
├── Makefile                    # Standardized developer & ops commands
├── runbook.md                  # Operational runbook & incident playbooks
├── SECURITY.md                 # Vulnerability disclosure & security controls
├── CHANGELOG.md                # Release history
├── CONTRIBUTING.md             # Contribution & quality standards
└── LICENSE                     # MIT License
```

---

## Tech Stack

| Component | Technology | Engineering Rationale |
|---|---|---|
| **Language** | Rust (Edition 2021, MSRV 1.82) | Memory safety, zero-cost abstractions, predictable tail latency |
| **Web Framework** | Axum 0.7 + Tower HTTP 0.5 | Type-safe extractors, composable middleware, native `oneshot` testing |
| **Async Runtime** | Tokio 1.0 | Multi-threaded work-stealing async I/O runtime |
| **OLTP Database** | PostgreSQL 16 + SQLx 0.8 | ACID transactional storage with parameterized `$1, $2` bindings |
| **OLAP Database** | ClickHouse 24.8 + `clickhouse` 0.13 | Columnar analytical engine for high-volume activity logs & active alarms |
| **Observability** | `tracing` + `tracing-subscriber` | Structured leveled logs controlled via `RUST_LOG` |
| **Error Handling** | `thiserror` 2.0 + `anyhow` 1.0 | Typed domain errors mapped cleanly to HTTP status codes |

---

## Security Hardening Notes

- **Credential Redaction (`redact_dsn`)**: Startup logs strip PostgreSQL passwords before emitting DSNs via `tracing::info!`.
- **Explicit CORS Allowlist**: `ALLOWED_ORIGINS` configures `CorsLayer::allow_origin` with explicit origins rather than wildcard permissive CORS.
- **Database Pool Timeout**: PostgreSQL `PgPoolOptions` enforces a 10-second `acquire_timeout` to fail fast during database outages.
- **Strict Input Validation**: Shared guard clauses in `src/utils/validation.rs` reject empty fields, malformed emails, invalid ISO `YYYY-MM-DD` dates, and inverted ranges (`start_date > end_date`).
- **Centralized Error Boundary**: `ErrorHandler` maps unmatched routes to consistent JSON `404` payloads while `AppError` logs detailed database diagnostics internally and returns safe generic messages to clients.
- **Hardened Container**: Multi-stage `Dockerfile` runs compiled release binaries under non-root `appuser` (UID `10001`) with a `/health` probe.
- **Continuous Security Automation**: GitHub Actions executes `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, Semgrep Rust SAST, and Trivy container scanning; Dependabot tracks Cargo, Docker, and GitHub Actions advisories weekly.

---

## Quickstart with Makefile

### Prerequisites

- Rust `1.82+` (`cargo`, `rustfmt`, `clippy`)
- Docker & Docker Compose (for full dual-database local stack)

### 1. Configure Environment

```bash
cp .env.example .env
```

Key environment variables (see `.env.example` and `runbook.md` for full matrix):

```ini
PORT=3000
RUST_LOG=braincode_be=info,tower_http=info
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173
DATABASE_URL=postgres://postgres:password_placeholder@localhost:5432/braincode_db
CLICKHOUSE_URL=http://localhost:8123
CLICKHOUSE_USER=default
CLICKHOUSE_PASSWORD=
CLICKHOUSE_DB=default
```

### 2. Common Makefile Targets

```bash
make fmt-check     # Verify formatting with rustfmt
make lint          # Run cargo clippy --all-targets -- -D warnings
make test          # Run all unit tests + 11 offline integration test suites
make build         # Compile optimized release binaries (--locked)
make run           # Start API server locally on PORT (default 3000)
make ingest        # Run streaming CSV -> ClickHouse ingestion binary
make docker-up     # Launch PostgreSQL + ClickHouse + braincode-be via Docker Compose
make docker-down   # Stop Docker Compose services
```

---

## REST API Endpoints

### Health Check

- **Method & Path**: `GET /health`
- **Response (`200 OK`)**:
  ```json
  {
    "status": "ok",
    "service": "braincode-be",
    "message": "Server berjalan dengan normal."
  }
  ```

### User Management (PostgreSQL OLTP + ClickHouse OLAP Audit Trail)

- **List Users**: `GET /api/v1/users` (`200 OK`)
- **Create User**: `POST /api/v1/users` (`201 Created`)
  ```json
  {
    "name": "Budi Santoso",
    "email": "budi@example.com"
  }
  ```
- **Recent Activities**: `GET /api/v1/analytics/activities` (`200 OK`)

### Active Alarm Service (ClickHouse OLAP)

- **Method & Paths**: `POST /alarm_list_active` (alias: `POST /api/v1/alarms/active`)
- **Request Body**:
  ```json
  {
    "start_date": "2026-09-08",
    "end_date": "2026-09-14"
  }
  ```
- **Response (`200 OK`)**:
  ```json
  {
    "status_code": "200",
    "result": [
      {
        "identifier": "ALARM-001",
        "alarmname": "ONU is Disconnected",
        "severity": 4,
        "sitecode": "RAP",
        "node": "GPON00-D1-1",
        "firstoccurrence": 1789218152000,
        "cleartime": 0,
        "acknowledged": "0"
      }
    ],
    "total": 1
  }
  ```
- **Validation Error (`400 Bad Request`)**: Returned when dates do not match `YYYY-MM-DD` or `start_date > end_date`.

---

## Streaming CSV Ingestion (`ingest_csv`)

Load semicolon-delimited alarm exports into ClickHouse in 15,000-row `JSONEachRow` batches with zero attribute loss:

```bash
cargo run --release --bin ingest_csv -- path/to/alarm_export.csv
```

See [runbook.md](runbook.md) for migration execution, operational monitoring, troubleshooting playbooks, and rollback procedures.
