# Analysis: `src/main.rs` (Application Orchestrator / Entry Point)

## File Purpose
File `src/main.rs` adalah **pintu masuk utama dan orkestrator (Composition Root)** dari seluruh sistem `braincode-beV2`. Di file inilah semua modul, koneksi database, repositori, layanan bisnis, middleware, dan handler HTTP dirangkai menjadi satu aplikasi web server yang utuh dan siap melayani request.

---

## Overview Arsitektur & Alur Inisialisasi

```text
[#[tokio::main]] Multi-threaded Async Runtime
       │
       ├─► 1. Inisialisasi Structured Logging (tracing-subscriber & EnvFilter)
       │
       ├─► 2. Inisialisasi Config & Connection Pool (PostgreSQL & ClickHouse)
       │
       ├─► 3. Perakitan Dependency Injection (DIP):
       │      PostgresUserRepository + ClickHouseActivityRepository ──► UserServiceImpl (Arc<dyn UserService>)
       │
       ├─► 4. Pembuatan Axum Router:
       │      - Route Health Check (/health)
       │      - Route Users CRUD (/api/v1/users)
       │      - Route Analytics (/api/v1/analytics/activities)
       │      - Middleware CORS (CorsLayer::permissive())
       │      - State Injection (.with_state(user_service))
       │
       └─► 5. Binding TCP Listener & Server Event Loop (axum::serve)
```

---

## Modul & Imports

```rust
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
```

### Penjelasan:
1. **Deklarasi `mod ...;`**:
   * Mendaftarkan ketujuh layer arsitektur aplikasi ke dalam root crate. Tanpa baris ini, Rust compiler tidak akan mengenali folder-folder tersebut sebagai bagian dari aplikasi.
2. **`axum::{routing::get, Router}`**:
   * Komponen router HTTP modern berbasis Tokio dan Hyper.
3. **`tower_http::cors::CorsLayer`**:
   * Middleware pengatur izin akses lintas domain (*Cross-Origin Resource Sharing*).
4. **`tracing_subscriber`**:
   * Pustaka telemetri dan structured logging modern di ekosistem Rust.

---

## Bedah Fungsi `main()` Langkah demi Langkah

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
```

### 1. Macro `#[tokio::main]`
* Fungsi `main` standar di sistem operasi bersifat sinkron.
* Macro `#[tokio::main]` menyulap fungsi `main` menjadi inisialisasi **Tokio Multi-threaded Runtime (Work-Stealing Thread Pool)**.
* Runtime ini mengelola threads OS, scheduler asinkron, dan I/O event loop sehingga kita bisa memanggil `.await` di dalam fungsi `main`.
* **Return Type `Result<(), Box<dyn std::error::Error>>`**: Mengizinkan penggunaan operator `?` di sepanjang fungsi `main` untuk menangani error apapun yang terjadi saat startup.

---

### 2. Inisialisasi Tracing & Logger Terstruktur

```rust
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "braincode_be=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
```

* **`EnvFilter`**: Membaca variabel lingkungan `RUST_LOG`. Jika tidak disetel di `.env`, ia secara otomatis fallback ke `"braincode_be=debug,tower_http=debug"`.
* **`fmt::layer()`**: Memformat log output ke terminal dengan format warna yang jelas (timestamp, log level, thread ID).
* **`.init()`**: Menetapkan logger ini sebagai *global default collector*.

---

### 3. Inisialisasi Config & Connection Pool Database

```rust
    let config = AppConfig::init().await?;
```

* Membaca file `.env` via `dotenvy`.
* Mem-parsing port HTTP (default 8080).
* Membuat connection pool PostgreSQL (`PgPool`) via `PgPoolOptions`.
* Membuat koneksi client ClickHouse (`Client`).

---

### 4. Rantai Perakitan Dependency Injection (Composition Root)

```rust
    let user_repo = Arc::new(PostgresUserRepository::new(config.postgres_pool));
    let activity_repo = Arc::new(ClickHouseActivityRepository::new(config.clickhouse_client));

    let user_service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(user_repo, activity_repo));
```

* **Repositories**:
  * `PostgresUserRepository` memegang `postgres_pool`.
  * `ClickHouseActivityRepository` memegang `clickhouse_client`.
  * Keduanya dibungkus ke dalam pointer thread-safe **`Arc::new(...)`**.
* **Services**:
  * `UserServiceImpl` menerima kedua repository tersebut melalui konstruktor `new(user_repo, activity_repo)`.
  * Disimpan ke dalam variabel bertipe trait abstraksi: `Arc<dyn UserService>`.
  * **Manfaat Arsitektur**: Handler HTTP tidak tahu apa-apa tentang PostgreSQL maupun ClickHouse; handler hanya bergantung pada abstraksi `UserService`.

---

### 5. Inisialisasi Router Axum & Routing Mapping

```rust
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
```

* **Fluent Method Chaining**:
  * Endpoint `/api/v1/users` menangani dua metode HTTP sekaligus: `GET` diarahkan ke `get_users` dan `POST` diarahkan ke `create_user`.
* **`CorsLayer::permissive()`**:
  * Mengizinkan frontend (React/Next.js/Vue) dari origin mana pun untuk memanggil API tanpa terblokir browser.
* **`.with_state(user_service)`**:
  * Menyuntikkan instance service ke dalam router Axum. State ini nantinya diekstrak di handler menggunakan `State(user_service)`.

---

### 6. Socket Binding & Eksekusi Server Loop

```rust
    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    axum::serve(listener, app).await?;
    Ok(())
```

* **`TcpListener::bind(&address).await?`**: Mengikat socket TCP pada host `0.0.0.0` dan port yang dikonfigurasi.
* **`axum::serve(listener, app).await?`**: Menjalankan web server HTTP secara non-blocking dan melayani ribuan request konkuren secara paralel.

---

## Komparasi Penuh dengan Python (FastAPI + Uvicorn)

Di ekosistem Python, seluruh alur pada `src/main.rs` setara dengan konfigurasi *lifespan* FastAPI:

```python
from contextlib import asynccontextmanager
import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from config import AppConfig
from repositories import PostgresUserRepository, ClickHouseActivityRepository
from services import UserServiceImpl
from handlers import health_router, user_router, analytics_router

@asynccontextmanager
async def lifespan(app: FastAPI):
    # 1. Init Config & DB Pools
    config = await AppConfig.init()
    
    # 2. Dependency Injection
    user_repo = PostgresUserRepository(config.postgres_pool)
    activity_repo = ClickHouseActivityRepository(config.clickhouse_client)
    app.state.user_service = UserServiceImpl(user_repo, activity_repo)
    
    yield

# Inisialisasi App & Middleware
app = FastAPI(lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

# Register Routers
app.include_router(health_router)
app.include_router(user_router, prefix="/api/v1")
app.include_router(analytics_router, prefix="/api/v1")

if __name__ == "__main__":
    uvicorn.run("main:app", host="0.0.0.0", port=8080)
```
