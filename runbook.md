# Operational Runbook: Braincode Dual-Database Backend

## 1. Service Overview

`braincode-be` is an asynchronous Rust REST API (Axum + Tokio) backed by two specialized data stores:

- **PostgreSQL 16 (OLTP)**: Transactional user records (`users` table).
- **ClickHouse 24.8 (OLAP)**: High-throughput user activity logs (`user_activities`) and active telecom alarm telemetry (`active_alarms`).

## 2. Environment Configuration

Copy `.env.example` to `.env` before running locally. Never commit `.env` to version control.

| Variable | Required | Default | Description |
|---|:---:|---|---|
| `PORT` | No | `3000` | HTTP listener port |
| `RUST_LOG` | No | `braincode_be=info,tower_http=info` | Structured `tracing` log filter |
| `DATABASE_URL` | Yes | `postgres://postgres:default@localhost:5432/postgres` | PostgreSQL DSN (password redacted in startup logs) |
| `CLICKHOUSE_URL` | Yes | `http://localhost:8123` | ClickHouse HTTP endpoint |
| `CLICKHOUSE_USER` | No | `default` | ClickHouse username |
| `CLICKHOUSE_PASSWORD` | No | *(empty)* | ClickHouse password |
| `CLICKHOUSE_DB` | No | `default` | Target ClickHouse database |
| `ALLOWED_ORIGINS` | No | `http://localhost:3000,http://localhost:5173` | Comma-separated CORS allowlist |

## 3. Startup & Health Verification

### Local Stack (Docker Compose)

```bash
cp .env.example .env
make docker-up
curl -fsS http://localhost:3000/health
```

Expected response (`200 OK`):

```json
{
  "status": "ok",
  "service": "braincode-be",
  "message": "Server berjalan dengan normal."
}
```

### Database Migrations

Versioned DDL scripts are stored under `migrations/`:

1. `migrations/001_init_postgres_users.sql` — creates the PostgreSQL `users` table and timestamp index.
2. `migrations/002_init_clickhouse_tables.sql` — creates `default.user_activities`, `telkomsel.active_alarms`, and `default.active_alarms`.

Docker Compose mounts both files into each database container's initialization entrypoint automatically.

## 4. Incident Playbooks

### 4.1 PostgreSQL Connection Timeout on Startup

- **Symptom**: Service exits during boot with `PoolTimedOut` or `Connection refused`.
- **Checks**:
  1. Inspect redacted startup log (`Menghubungkan ke PostgreSQL: postgres://user:***@host:5432/db`) to verify target host/port without exposing passwords.
  2. Verify PostgreSQL readiness: `docker compose exec postgres pg_isready`.
  3. Confirm `DATABASE_URL` credentials and network reachability; default pool acquire timeout is `10s`.

### 4.2 ClickHouse Analytics Write Failure

- **Symptom**: `POST /api/v1/users` succeeds (`201 Created`) while warning logs emit `Gagal mencatat analitik ClickHouse`.
- **Behavior**: User registration intentionally degrades gracefully so ClickHouse downtime never blocks transactional OLTP writes.
- **Remediation**:
  1. Check ClickHouse liveness: `curl -fsS http://localhost:8123/ping`.
  2. Confirm `user_activities` exists by running `migrations/002_init_clickhouse_tables.sql`.

### 4.3 Active Alarm Endpoint Returning 400 Bad Request

- **Symptom**: `POST /alarm_list_active` or `POST /api/v1/alarms/active` returns `400`.
- **Cause**: Client passed malformed `start_date`/`end_date` (must be ISO `YYYY-MM-DD`) or `start_date > end_date`.
- **Action**: Validate caller payload against the date range contract; server logs emit structured `INFO` diagnostics without exposing internal stack traces.

### 4.4 High-Volume CSV Ingestion Failure (`ingest_csv`)

- **Symptom**: `cargo run --release --bin ingest_csv -- <path.csv>` exits with ClickHouse HTTP error.
- **Remediation**:
  1. Confirm `CLICKHOUSE_URL` and `CLICKHOUSE_DB` point to a database where `active_alarms` has been migrated.
  2. Re-run ingestion safely; `ReplacingMergeTree(lastoccurrence)` deduplicates rows by `(location, severity, alarm_start, identifier)`.

## 5. Quality & Security Verification Commands

```bash
make fmt-check
make lint
make test
make docker-build
```

## 6. Rollback Procedure

1. Identify the last known-good Git tag (`git tag -l`) or container image digest.
2. Redeploy the previous container tag (`docker compose up -d --no-build`).
3. Verify `/health`, `GET /api/v1/users`, and `POST /api/v1/alarms/active` return expected status codes.
