# 🦀 Braincode Backend: Dual-Database Enterprise REST API

> **High-Performance Asynchronous Backend in Rust (Axum, Tokio) featuring Clean Architecture and Dual-Database Persistence (PostgreSQL OLTP + ClickHouse OLAP).**

---

## 📌 Executive Summary

**Braincode Backend** adalah arsitektur REST API tingkat enterprise yang dirancang untuk menangani beban operasional transaksional (*high-integrity OLTP*) sekaligus pencatatan analitik berkecepatan tinggi (*high-throughput OLAP*) secara paralel.

Dibangun dengan prinsip **Clean Architecture (Dependency Inversion Principle)**, sistem ini memisahkan secara tegas antara kontrak antarmuka (*traits*), orkestrasi bisnis (*services*), dan eksekusi fisik database (*repositories*). Seluruh komponen berjalan di atas runtime multi-threaded **Tokio** dan framework web **Axum** untuk menjamin latensi minimal, keandalan memori tanpa Garbage Collector, dan konkurensi aman bebas *data race*.

---

## 🏛️ Architectural Blueprint (Clean Architecture)

Aplikasi mengadopsi 7 layer arsitektur yang terisolasi secara modular:

```text
[ HTTP Client / Frontend / Ingestion Pipeline ]
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. PRESENTATION LAYER (`src/handlers/`)                     │
│    • Axum Extractors: State<Arc<dyn UserService>>, Json     │
│    • health_check, create_user, get_users, get_activities   │
└──────────────────────────────┬──────────────────────────────┘
                               │ (Calls Service Contract)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. BUSINESS LOGIC LAYER (`src/services/`)                   │
│    • Trait Interface: UserService (Send + Sync)             │
│    • Concrete: UserServiceImpl (Arc<dyn Repository>)        │
│    • Fail-Fast Business Validation                          │
│    • Resilient Side-Effect Logging to ClickHouse            │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │
       (Postgres Contract)            (ClickHouse Contract)
               │                              │
               ▼                              ▼
┌──────────────────────────────┐┌──────────────────────────────┐
│ 3. PERSISTENCE LAYER (OLTP)  ││ 4. ANALYTICS LAYER (OLAP)    │
│    • PostgresUserRepository  ││    • ClickHouseActivityRepo  │
│    • sqlx (PgPool)           ││    • clickhouse (Client)     │
│    • Prepared Queries ($1,$2)││    • Binary Streaming Buffer │
└──────────────┬───────────────┘└──────────────┬───────────────┘
               │                               │
               ▼                               ▼
       [( PostgreSQL DB )]             [( ClickHouse OLAP )]
```

### 📁 Struktur Direktori Modular
```text
src/
├── config/           # Parsing environment (.env) & inisialisasi connection pools
│   ├── app_config.rs
│   └── mod.rs
├── dtos/             # Data Transfer Objects (Request validation & Response envelopes)
│   ├── requests/     # Input JSON payload (Deserialize)
│   ├── responses/    # Output JSON payload (Serialize & From Trait)
│   └── mod.rs
├── handlers/         # HTTP Controller endpoints (Axum Extractors & Status codes)
│   ├── health_handler.rs
│   ├── user_handler.rs
│   ├── analytic_handlers.rs
│   └── mod.rs
├── models/           # Domain Entities (Database representations)
│   ├── user.rs       # PostgreSQL entity (sqlx::FromRow)
│   ├── activity_log.rs # ClickHouse entity (clickhouse::Row)
│   └── mod.rs
├── repositories/     # Data Access Objects (SQLx & ClickHouse clients)
│   ├── traits/       # Interface kontrak data (UserRepository, ActivityRepository)
│   ├── impls/        # Implementasi konkret query & stream
│   └── mod.rs
├── services/         # Domain Business Rules & Dual-Storage Orchestration
│   ├── traits/       # Interface kontrak bisnis (UserService)
│   ├── impls/        # Orkestrasi pendaftaran user + logging analitik
│   └── mod.rs
├── utils/            # Penanganan error global terpusat
│   ├── errors.rs     # Enum AppError & implementasi Axum IntoResponse
│   └── mod.rs
└── main.rs           # Composition Root (Runtime Tokio, Routing, State Injection, TCP Listener)
```

---

## ⚡ Tech Stack & Engineering Highlights

| Komponen | Teknologi | Alasan Pemilihan & Manfaat Teknis |
|---|---|---|
| **Bahasa** | **Rust (Edition 2021)** | Menjamin *memory safety*, *fearless concurrency*, dan performa *bare-metal* tanpa jeda *Garbage Collector (GC)*. |
| **Web Framework** | **Axum 0.7** | Framework modern berbasis ekosistem Tower/Hyper dengan sistem *Type-safe Extractor* yang ergonomis. |
| **Async Runtime** | **Tokio 1.0** | Engine *multi-threaded work-stealing event loop* teruji untuk menangani puluhan ribu koneksi I/O simultan. |
| **Relational Database** | **PostgreSQL + SQLx 0.8** | Async SQL toolkit dengan parameter binding (`$1, $2`) untuk pencegahan mutlak serangan SQL Injection. |
| **Analytical Engine** | **ClickHouse 0.13** | Database kolumnar OLAP tercepat untuk agregasi data berskala miliaran baris via *binary streaming protocol*. |
| **Serialization** | **Serde & Serde JSON** | Standar industri serialisasi/deserialisasi format JSON *zero-copy*. |
| **Error Handling** | **thiserror 2.0** | Generator error kustom idiomatis yang dipadukan dengan trait `IntoResponse` Axum. |
| **Telemetry / Log** | **Tracing & Tracing-Subscriber** | Structured logging dengan filter level dinamis berbasis environment (`RUST_LOG`). |

---

## 🛠️ Desain Pola Rust Khusus (*Key Engineering Patterns*)

### 1. Resilient Dual-Database Orchestration
Pada proses pendaftaran pengguna (`register_user`):
* Data pengguna dicatat ke **PostgreSQL (OLTP)** secara transaksional dengan klausa `RETURNING id, name, email`.
* Rekam jejak audit di-stream ke **ClickHouse (OLAP)** sebagai *side effect*.
* **Resilience Pattern:** Kegagalan pada server ClickHouse ditangkap secara anggun menggunakan `if let Err(...)` dan dicatat ke log peringatan (`tracing::warn!`). Hal ini menjamin pendaftaran pengguna yang sudah sah di PostgreSQL **tidak akan pernah digagalkan** oleh gangguan analitik sekunder.

### 2. Dependency Injection via `Arc<dyn Trait>`
* Layer service dan handler tidak terikat pada implementasi database konkret, melainkan pada pointer *trait object* thread-safe:
  ```rust
  Arc<dyn UserRepository>
  Arc<dyn ActivityRepository>
  ```
* Didukung oleh procedural macro `#[async_trait]` dan bound `Send + Sync` agar aman diakses bersamaan oleh berbagai *worker threads* Tokio.

### 3. Idiomatic DTO Separation & Trait `From`
* Entitas tabel database internal tidak pernah diekspos langsung ke publik.
* Menggunakan trait standar Rust `From<User> for UserResponse` untuk menjamin data sensitif internal tidak akan pernah bocor ke jaringan luar. Pemanggilan konversi dilakukan secara elegan:
  ```rust
  let res: UserResponse = user.into();
  ```

---

## 🗄️ Database Schemas (DDL Requirements)

Pastikan tabel berikut telah dibuat sebelum menjalankan aplikasi:

### 1. PostgreSQL (OLTP)
```sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(150) UNIQUE NOT NULL
);
```

### 2. ClickHouse (OLAP)
```sql
CREATE TABLE IF NOT EXISTS default.user_activities (
    user_id UInt64,
    action String,
    details String,
    timestamp UInt64
) ENGINE = MergeTree()
ORDER BY (timestamp, user_id);
```

---

## 🚀 Panduan Menjalankan Aplikasi (*Getting Started*)

### 1. Prasyarat
* [Rust Toolchain](https://www.rust-lang.org/) (versi 1.75 atau lebih baru)
* Instance [PostgreSQL](https://www.postgresql.org/) aktif
* Instance [ClickHouse](https://clickhouse.com/) aktif

### 2. Konfigurasi Environment
Salin template konfigurasi dan sesuaikan kredensial koneksi lokal Anda:
```bash
cp .env.example .env
```

Contoh konfigurasi `.env`:
```ini
PORT=3000
DATABASE_URL=postgres://postgres:password@localhost:5432/braincode_db
CLICKHOUSE_URL=http://localhost:8123
CLICKHOUSE_USER=default
CLICKHOUSE_PASSWORD=
CLICKHOUSE_DB=default
```

### 3. Verifikasi & Kompilasi
Uji integritas sistem type dan syntax:
```bash
cargo check
```

### 4. Menjalankan Server
```bash
cargo run
```
Server akan aktif mendengarkan request di `http://localhost:3000`.

---

## 📡 Dokumentasi Endpoint REST API

### 1. Health Check Probe
* **Method:** `GET`
* **Path:** `/health`
* **Deskripsi:** Digunakan untuk memvalidasi ketersediaan server (Liveness/Readiness probe).
* **Sample Response (200 OK):**
  ```json
  {
    "status": "ok",
    "service": "braincode-be",
    "message": "Server berjalan dengan normal."
  }
  ```

---

### 2. Pendaftaran Pengguna Baru (*Dual-DB Trigger*)
* **Method:** `POST`
* **Path:** `/api/v1/users`
* **Headers:** `Content-Type: application/json`
* **Sample Request Body:**
  ```json
  {
    "name": "Budi Santoso",
    "email": "budi@example.com"
  }
  ```
* **Sample Response (201 Created):**
  ```json
  {
    "status": "success",
    "message": "Pengguna berhasil dibuat!",
    "data": {
      "id": 1,
      "name": "Budi Santoso",
      "email": "budi@example.com"
    }
  }
  ```

---

### 3. Mengambil Seluruh Pengguna (PostgreSQL)
* **Method:** `GET`
* **Path:** `/api/v1/users`
* **Sample Response (200 OK):**
  ```json
  {
    "status": "success",
    "message": "Daftar pengguna berhasil diambil.",
    "data": [
      {
        "id": 1,
        "name": "Budi Santoso",
        "email": "budi@example.com"
      }
    ]
  }
  ```

---

### 4. Mengambil Rekam Jejak Analitik (ClickHouse)
* **Method:** `GET`
* **Path:** `/api/v1/analytics/activities`
* **Sample Response (200 OK):**
  ```json
  {
    "status": "success",
    "message": "Data analitik aktivitas terbaru dari ClickHouse berhasil diambil.",
    "data": [
      {
        "user_id": 1,
        "action": "user_registered",
        "details": "Pendaftaran pengguna baru dengan email budi@example.com",
        "timestamp": 1773321123
      }
    ]
  }
  ```

---

## 👨‍💻 Engineering Notes (Internship Report Context)

Proyek ini dibangun dan dianalisis secara mendalam sebagai bagian dari program **Data Engineering Internship**. Fokus pembelajaran ditekankan pada pemahaman sistem programming tingkat rendah (*systems data engineering*) untuk mengatasi keterbatasan performa I/O dan konsumsi memori yang biasa dihadapi pada runtime berbasis Python atau JVM.

Key engineering takeaways yang berhasil dikuasai:
1. **Zero-Cost Abstractions & Ownership Model:** Mengelola siklus hidup memori tanpa alokasi tak terduga (*memory leaks*) dan bebas jeda *Garbage Collection*.
2. **Dual-Store Paradigm (OLTP vs OLAP):** Memahami kapan data harus disimpan dalam format baris relasional (*B-Tree/Postgres*) vs format kolom terindeks (*MergeTree/ClickHouse*).
3. **Resilient Failure Handling:** Mengadopsi prinsip *Fault Isolation* agar dependensi analitik sekunder tidak menyebabkan *Single Point of Failure* pada proses inti bisnis.
