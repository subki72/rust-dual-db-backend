# 00 — Project Overview: `braincode-be` Architecture & Rebuild Blueprint

## Project Purpose
`braincode-be` adalah backend REST API enterprise berperforma tinggi yang dibangun di atas bahasa **Rust**. Aplikasi ini mengadopsi prinsip **Clean Architecture (Layered Architecture)** dengan fitur dual-database storage:
1. **PostgreSQL**: Untuk transactional persistence (CRUD data user).
2. **ClickHouse**: Untuk analytical persistence (logging aktivitas user berkecepatan tinggi).

Tujuan sesi ini adalah **menganalisis, memahami secara mendalam (dengan komparasi Python), dan membangun ulang kode secara bertahap ke dalam `braincode-beV2`**.

---

## Project Inventory

- **Total Files**: 30 file `.rs`
- **Dependencies (12 Crates Utama)**:
  - `axum`: Web framework modern berbasis Tokio, Hyper, dan Tower.
  - `tokio`: Asynchronous runtime event loop multi-threaded.
  - `sqlx`: Asynchronous, pure Rust SQL toolkit & connection pooler untuk PostgreSQL.
  - `clickhouse`: Driver HTTP client ClickHouse berkinerja tinggi.
  - `serde` & `serde_json`: Framework serialisasi & deserialisasi data JSON.
  - `dotenvy`: Pembaca file environment `.env`.
  - `thiserror`: Macro generator error type yang ergonomis.
  - `async-trait`: Macro pengaktifan `async fn` di dalam Trait object.
  - `tower-http`: Middleware HTTP (CORS, tracing, timeout).
  - `tracing` & `tracing-subscriber`: Structured logging & telemetry runtime.

---

## Layer Structure & Architectural Blueprint

```text
[HTTP Client (Frontend / Postman)]
                 │
                 ▼
 1. PRESENTATION LAYER (`src/handlers/`)
    - health_handler.rs, user_handler.rs, analytics_handler.rs
    - Mengurai request HTTP (Axum Extractors: State, Json), memanggil service, mengemas ApiResponse.
                 │
                 ▼
 2. BUSINESS LOGIC LAYER (`src/services/`)
    - traits/ (UserService), impls/ (UserServiceImpl)
    - Mengorkestrasi aturan bisnis, menggabungkan data dari Postgres dan ClickHouse.
                 │
                 ▼
 3. PERSISTENCE LAYER (`src/repositories/`)
    - traits/ (UserRepository, ActivityRepository)
    - impls/ (PostgresUserRepository, ClickHouseActivityRepository)
    - Melakukan query SQL langsung ke PostgreSQL (sqlx) dan ClickHouse.
                 │
                 ▼
 4. DOMAIN & TRANSIT DATA LAYER
    - `src/models/`: Struct tabel fisik database (User, UserActivity).
    - `src/dtos/`: Struct kontrak request/response (CreateUserRequest, ApiResponse, UserResponse).
    - `src/utils/`: Penanganan error terpusat (AppError).
    - `src/config/`: Bootstrapping environment & koneksi database pool (AppConfig).
```

---

## Roadmap Eksekusi Pembelajaran (Dependency-First)

Untuk memahami kode tanpa kebingungan, kita akan melangkah dengan urutan **Dependency-First**:
1. **Utils & Errors (`src/utils/errors.rs`)**: Fondasi error yang dipakai di seluruh layer.
2. **Configuration (`src/config/app_config.rs`)**: Cara membaca `.env` dan menyambungkan DB pool.
3. **Domain Models (`src/models/`)**: Entitas data dasar tabel database.
4. **DTOs (`src/dtos/`)**: Format data request dan response API.
5. **Repositories (`src/repositories/`)**: Kontrak trait & implementasi query database.
6. **Services (`src/services/`)**: Logika bisnis penggabungan Postgres + ClickHouse.
7. **Handlers (`src/handlers/`)**: HTTP Endpoint controllers.
8. **Main Orchestrator (`src/main.rs`)**: Titik awal perakitan seluruh komponen.
