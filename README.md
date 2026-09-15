# Braincode Backend: Dual-Database Enterprise REST API & Alarm Pipeline

> **High-Performance Asynchronous Backend in Rust (Axum, Tokio) featuring Clean Architecture, Dual-Database Persistence (PostgreSQL OLTP + ClickHouse OLAP), High-Throughput Streaming Ingestion, and Apache Kafka Event Streaming.**

---

## Executive Summary

**Braincode Backend** adalah arsitektur REST API tingkat enterprise yang dirancang untuk menangani beban operasional transaksional (*high-integrity OLTP*) sekaligus analitik data skala besar berkecepatan tinggi (*high-throughput OLAP*) dalam industri telekomunikasi (Telkomsel Fixed-Broadband).

Dibangun dengan prinsip **Clean Architecture (Dependency Inversion Principle)**, sistem ini memisahkan secara tegas antara kontrak antarmuka (*traits*), orkestrasi bisnis (*services*), dan eksekusi fisik database (*repositories*). Seluruh komponen berjalan di atas runtime multi-threaded **Tokio** dan framework web **Axum** untuk menjamin latensi minimal, efisiensi memori tanpa jeda *Garbage Collector*, serta konkurensi aman bebas *data race*.

---

## Architectural Blueprint (Clean Architecture)

Aplikasi mengadopsi 7 layer arsitektur yang terisolasi secara modular:

```text
[ HTTP Client / NOC Frontend / Automated Scripts ]
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. PRESENTATION LAYER (`src/handlers/`)                     │
│    • Axum Extractors: State<Arc<dyn Service>>, Json, Query  │
│    • health_check, user_handlers, analytics, alarm_handler  │
└──────────────────────────────┬──────────────────────────────┘
                               │ (Invokes Service Trait)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. BUSINESS LOGIC LAYER (`src/services/`)                   │
│    • Trait Interfaces: UserService, AlarmService            │
│    • Concrete: UserServiceImpl, AlarmServiceImpl            │
│    • Fail-Fast Business Validation & Guard Clauses          │
│    • 100% Unit Test Coverage with Mock Repositories         │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │
       (Postgres Trait)               (ClickHouse Trait)
               │                              │
               ▼                              ▼
┌──────────────────────────────┐┌──────────────────────────────┐
│ 3. PERSISTENCE LAYER (OLTP)  ││ 4. ANALYTICS LAYER (OLAP)    │
│    • PostgresUserRepository  ││    • ClickHouseAlarmRepo     │
│    • sqlx (PgPool)           ││    • ClickHouseActivityRepo  │
│    • Prepared Queries ($1,$2)││    • Binary Streaming Buffer │
└──────────────┬───────────────┘└──────────────┬───────────────┘
               │                               │
               ▼                               ▼
       [( PostgreSQL DB )]             [( ClickHouse OLAP )]
                                               │
                                               ▼
                              ┌────────────────────────────────┐
                              │ 5. EVENT STREAMING (KAFKA)     │
                              │    • Native ClickHouse Engine  │
                              │    • Topic: alarms-stream      │
                              │    • Kafdrop Web UI (:9002)    │
                              └────────────────────────────────┘
```

### Struktur Direktori Modular
```text
src/
├── bin/              # Standalone CLI tools & Ingestion Engines
│   └── ingest_csv.rs # Streaming CSV Ingestion (194k baris / 461 MB dalam ~18 detik)
├── config/           # Parsing environment (.env) & inisialisasi connection pools
│   ├── app_config.rs
│   └── mod.rs
├── dtos/             # Data Transfer Objects (Request validation & Response envelopes)
│   ├── requests/     # Input JSON payload (Deserialize)
│   │   ├── alarm_active_request.rs
│   │   ├── create_user_request.rs
│   │   └── mod.rs
│   ├── responses/    # Output JSON payload (Serialize & From Trait)
│   │   ├── alarm_active_response.rs
│   │   ├── api_response.rs
│   │   ├── user_response.rs
│   │   └── mod.rs
│   └── mod.rs
├── handlers/         # HTTP Controller endpoints (Axum Extractors & Status codes)
│   ├── alarm_handler.rs
│   ├── analytics_handler.rs
│   ├── health_handler.rs
│   ├── user_handler.rs
│   └── mod.rs
├── models/           # Domain Entities (Database representations)
│   ├── activity_log.rs
│   ├── user.rs
│   └── mod.rs
├── repositories/     # Data Access Objects (SQLx & ClickHouse clients)
│   ├── impls/        # Implementasi konkret query & streaming
│   │   ├── clickhouse_activity_repository.rs
│   │   ├── clickhouse_alarm_repository.rs
│   │   ├── postgres_user_repository.rs
│   │   └── mod.rs
│   ├── traits/       # Interface kontrak data (Decoupling)
│   │   ├── activity_repository.rs
│   │   ├── alarm_repository.rs
│   │   ├── user_repository.rs
│   │   └── mod.rs
│   └── mod.rs
├── services/         # Domain Business Rules & Orchestration
│   ├── impls/        # Logika bisnis, validasi, dan unit test mock
│   │   ├── alarm_service_impl.rs
│   │   ├── user_service_impl.rs
│   │   └── mod.rs
│   ├── traits/       # Interface kontrak bisnis
│   │   ├── alarm_service.rs
│   │   ├── user_service.rs
│   │   └── mod.rs
│   └── mod.rs
├── utils/            # Penanganan error global terpusat
│   ├── errors.rs     # Enum AppError & implementasi Axum IntoResponse
│   └── mod.rs
└── main.rs           # Composition Root (Dependency Injection, Routing, TCP Listener)
```

---

## Tech Stack & Engineering Highlights

| Komponen | Teknologi | Alasan Pemilihan & Manfaat Teknis |
|---|---|---|
| **Bahasa** | **Rust (Edition 2021)** | Menjamin *memory safety*, *fearless concurrency*, dan performa *bare-metal* tanpa jeda *Garbage Collector (GC)*. |
| **Web Framework** | **Axum 0.7** | Framework modern berbasis ekosistem Tower/Hyper dengan sistem *Type-safe Extractor* yang ergonomis. |
| **Async Runtime** | **Tokio 1.0** | Engine *multi-threaded work-stealing event loop* teruji untuk menangani puluhan ribu koneksi I/O simultan. |
| **Relational Database** | **PostgreSQL 16 + SQLx 0.8** | Async SQL toolkit dengan parameter binding (`$1, $2`) untuk integritas transaksional ACID. |
| **Analytical Engine** | **ClickHouse 24.8** | Database kolumnar OLAP tercepat untuk agregasi ratusan ribu alarm via *vectorized query execution*. |
| **Message Broker** | **Apache Kafka + Kafdrop** | Buffer pesan asinkronus penahan lonjakan (*shock absorber*) alarm jaringan real-time. |
| **Serialization** | **Serde & Serde JSON** | Standar industri serialisasi/deserialisasi format JSON *zero-copy*. |
| **Date & Time** | **Chrono 0.4** | Manipulasi dan validasi tanggal berbasis ISO-8601/RFC-3339. |
| **Error Handling** | **thiserror 2.0** | Generator error kustom idiomatis yang dipadukan dengan trait `IntoResponse` Axum. |
| **Telemetry / Log** | **Tracing & Tracing-Subscriber** | Structured logging dengan filter level dinamis berbasis environment (`RUST_LOG`). |

---

## Fitur Utama & Modul Sistem

### 1. Active Alarm Service (GitHub Issue #2)
Endpoint khusus untuk tim NOC (*Network Operations Center*) memantau gangguan jaringan Fixed-Broadband (FBB) Telkomsel:
* **Mandatory Active Filter:** Kondisi `cleartime = 0` dikunci paten pada repository layer dan tidak dapat di-override oleh parameter luar.
* **Date Range Filtering:** Filter rentang tanggal fleksibel pada kolom timestamp `firstoccurrence` menggunakan ekspresi ClickHouse `toDate(intDiv(firstoccurrence, 1000)) BETWEEN toDate(?) AND toDate(?)`.
* **Guard Clause Validation:** Validasi ketat format ISO `YYYY-MM-DD` dan aturan bisnis `start_date <= end_date`.
* **Standard Response Envelope:** Format JSON `{ status_code: "200", result: [...], total: N }`.

### 2. High-Throughput Streaming CSV Ingest Engine (`ingest_csv`)
Engine mandiri di `src/bin/ingest_csv.rs` untuk memuat dump CSV alarm sebesar **461 MB** (194.575 baris × 273 kolom):
* **Zero Data Loss:** 28 kolom metrik kritis disimpan dalam kolom terindeks, sementara seluruh 273 atribut mentah disimpan utuh dalam kolom `raw_attributes String` (format JSON).
* **Hemat Memori (RAM < 80 MB):** Menggunakan `BufReader` berkapasitas 256 KB untuk membaca stream baris demi baris tanpa menelan seluruh file ke RAM.
* **Batch Optimization:** Mengirimkan batch per 15.000 baris langsung ke HTTP interface ClickHouse (`FORMAT JSONEachRow`), selesai dalam ~15-20 detik tanpa memicu error *"Too many parts"*.

### 3. Dual-Database Kafka Streaming Pipeline
Mengalirkan event alarm real-time antar-database:
* **Metode A (Application Pipeline via Rust):** ClickHouse $\rightarrow$ Kafka $\rightarrow$ PostgreSQL.
* **Metode B (Native ClickHouse Kafka Engine):** ClickHouse $\rightarrow$ Kafka $\rightarrow$ ClickHouse (*Zero-Code Backend Pipeline* menggunakan tabel `ENGINE = Kafka` dan `MATERIALIZED VIEW`).
* **Kafdrop Monitoring:** Pantau status topik, partisi, consumer group, dan lag pesan secara visual di `http://localhost:9002`.

---

## Database Schemas (DDL Requirements)

### 1. PostgreSQL (OLTP)
```sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(150) UNIQUE NOT NULL
);
```

### 2. ClickHouse (OLAP)
Lihat skrip lengkap di [create_table_alarms.sql](create_table_alarms.sql):
```sql
-- Tabel Analitik User
CREATE TABLE IF NOT EXISTS default.user_activities (
    user_id UInt64,
    action String,
    details String,
    timestamp UInt64
) ENGINE = MergeTree()
ORDER BY (timestamp, user_id);

-- Tabel Master Active Alarms (273 Kolom / Zero Data Loss)
CREATE TABLE IF NOT EXISTS default.active_alarms
(
    identifier String,
    ticketid Nullable(String),
    alarmid String,
    alarmserialnumber String,
    cleartime Int64,
    is_active UInt8 MATERIALIZED (if(cleartime = 0, 1, 0)),
    severity UInt8,
    ticketstatus Int32,
    alarmname LowCardinality(String),
    alarmtext String,
    alarmtype LowCardinality(String),
    location LowCardinality(String),
    sitecode LowCardinality(String),
    node LowCardinality(String),
    firstoccurrence Int64,
    lastoccurrence Int64,
    alarm_start DateTime('Asia/Jakarta'),
    cleared_time_dt Nullable(DateTime('Asia/Jakarta')),
    duration_seconds UInt32 MATERIALIZED (if(cleartime = 0, toUInt32(now() - alarm_start), toUInt32(ifNull(cleared_time_dt, alarm_start) - alarm_start))),
    acknowledged LowCardinality(String) DEFAULT '0',
    totalcount UInt32 DEFAULT 1,
    raw_attributes String,
    ingested_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(alarm_start)
ORDER BY (cleartime, location, severity, firstoccurrence, identifier);
```

---

## Panduan Menjalankan Aplikasi (*Getting Started*)

### 1. Prasyarat
* [Rust Toolchain](https://www.rust-lang.org/) (versi 1.75 atau lebih baru)
* Instance [PostgreSQL](https://www.postgresql.org/) aktif (`localhost:5432`)
* Instance [ClickHouse](https://clickhouse.com/) aktif (`localhost:8123` & `9000`)
* Docker & Docker Compose (untuk Kafka cluster)

### 2. Menjalankan Kafka Cluster & Kafdrop UI
```bash
docker compose -f docker-compose-kafka.yml up -d
```
* Buka browser di `http://localhost:9002` untuk melihat antrean Kafka.

### 3. Konfigurasi Environment
Salin template konfigurasi dan sesuaikan kredensial koneksi:
```bash
cp .env.example .env
```

Contoh isi berkas `.env`:
```ini
PORT=3000
DATABASE_URL=postgres://postgres:default@localhost:5432/postgres
CLICKHOUSE_URL=http://localhost:8123
CLICKHOUSE_USER=default
CLICKHOUSE_PASSWORD=
CLICKHOUSE_DB=default
```

### 4. Eksekusi CSV Ingestion (194k Baris Alarm)
Jalankan binary pengimpor data:
```bash
cargo run --bin ingest_csv
```

### 5. Menjalankan Unit Tests
```bash
cargo test
```
*Seluruh 3 unit test (Happy Path, Format Validasi, dan Negative Test start > end) akan lulus otomatis.*

### 6. Menjalankan Server API
```bash
cargo run
```
Server akan aktif mendengarkan request di `http://localhost:3000`.

---

## Dokumentasi Endpoint REST API

### 1. Health Check Probe
* **Method:** `GET`
* **Path:** `/health`
* **Sample Response (200 OK):**
  ```json
  {
    "status": "ok",
    "service": "braincode-be",
    "message": "Server Berjalan dengan normal"
  }
  ```

---

### 2. Active Alarm List (Spesifikasi Issue #2)
* **Method:** `POST`
* **Path:** `/alarm_list_active` *(alias: `/api/v1/alarms/active`)*
* **Headers:** `Content-Type: application/json`
* **Sample Request Body:**
  ```json
  {
    "start_date": "2026-09-08",
    "end_date": "2026-09-14"
  }
  ```
* **Sample Response (200 OK):**
  ```json
  {
    "status_code": "200",
    "result": [
      {
        "identifier": "1789218152000_cdf52eaa5d5abe0b734f9b69c6708b3bcb95b7cc4ef854f8a9b77c923a9d6049",
        "alarmname": "ONU is Disconnected",
        "severity": 4,
        "sitecode": "RAP",
        "node": "GPON00-D1-1",
        "firstoccurrence": 1789218152000,
        "cleartime": 0,
        "acknowledged": "0"
      }
    ],
    "total": 192683
  }
  ```
* **Negative Test (400 Bad Request):**
  Jika `start_date` lebih besar dari `end_date`:
  ```json
  {
    "message": "start_date tidak boleh lebih besar dari end_date"
  }
  ```

---

### 3. Pendaftaran Pengguna Baru (PostgreSQL + ClickHouse)
* **Method:** `POST`
* **Path:** `/api/v1/users`
* **Sample Request Body:**
  ```json
  {
    "name": "Budi Santoso",
    "email": "budi@example.com"
  }
  ```

---

### 4. Pengujian Otomatis via PowerShell
Telah disediakan skrip pengujian otomatis:
```powershell
.\test_alarm_service.ps1
```
Skrip ini akan menguji secara sekuensial: Health check, Happy path date range, dan Negative test validasi bisnis.
