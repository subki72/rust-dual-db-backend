# Analysis: `src/config/app_config.rs`

## File Purpose
File ini adalah **manajer konfigurasi dan bootstrapper koneksi database aplikasi**. Bertanggung jawab untuk membaca variabel lingkungan (*environment variables*) dari file `.env`, membuat pool koneksi multi-thread ke PostgreSQL (`PgPool`), mengonfigurasi klien ClickHouse, serta secara otomatis memastikan tabel awal (`users` di Postgres dan `user_activities` di ClickHouse) sudah tercipta sebelum server mulai menerima request.

---

## Overview
Di ekosistem Python (FastAPI/Django), kamu biasanya menggunakan `pydantic-settings` (`BaseSettings`) untuk membaca `.env`, lalu membuat engine SQLAlchemy (`create_async_engine`) di file terpisah `database.py`.
Di Rust, file `app_config.rs` menyatukan konfigurasi dan inisialisasi koneksi database ke dalam satu struct `AppConfig` dengan satu method konstruktor asinkron `AppConfig::init()`.

File ini memiliki 4 bagian utama:
1. **Imports & Struct Definition (Baris 1–11)**: Mendefinisikan struct `AppConfig` penampung pool koneksi.
2. **Environment Variable Loading (Baris 14–36)**: Membaca port dan credential database dengan nilai default (*fallback*).
3. **PostgreSQL Setup & Migration (Baris 38–58)**: Inisialisasi pool koneksi Postgres dan eksekusi `CREATE TABLE IF NOT EXISTS users`.
4. **ClickHouse Setup & Migration (Baris 60–87)**: Inisialisasi klien ClickHouse dan eksekusi `CREATE TABLE IF NOT EXISTS user_activities`.

---

## Imports & Data Structure

```rust
use clickhouse::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub port: u16,
    pub postgres_pool: PgPool,
    pub clickhouse_client: Client,
}
```

### Penjelasan Komponen:
- **`sqlx::PgPool`**: Connection pool PostgreSQL. Di Rust, pool ini mengelola sekelompok koneksi database terbuka yang bisa dipakai bergantian oleh banyak request worker thread tanpa harus buka-tutup koneksi TCP terus-menerus.
  - *Python equivalent*: `AsyncEngine` atau `sessionmaker` di SQLAlchemy / Tortoise ORM.
- **`clickhouse::Client`**: Client HTTP untuk database analitik ClickHouse berkecepatan tinggi.
- **`#[derive(Clone)]`**: Memungkinkan struct `AppConfig` diduplikasi dengan murah. Mengapa murah? Karena di dalam `PgPool` dan `Client`, mereka sebenarnya sudah membungkus pointer `Arc` secara internal! Meng-clone `PgPool` hanya menduplikasi penunjuk memorinya, bukan membuat koneksi baru ke database.

---

## Breakdown Method `AppConfig::init()`

### 1. Pembacaan `.env` & Parsing Port (Baris 15–20)
```rust
dotenvy::dotenv().ok();

let port = env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .unwrap_or(3000);
```

**Penjelasan Baris:**
- `dotenvy::dotenv().ok()`: Mencari file `.env` di direktori proyek dan memuatnya ke environment process. Method `.ok()` mengabaikan error jika file `.env` tidak ditemukan (misalnya saat di-deploy di Docker/Kubernetes yang variabelnya di-inject langsung oleh OS).
- `env::var("PORT")`: Mengembalikan `Result<String, VarError>`.
- `.unwrap_or_else(|_| "3000".to_string())`: Jika key `"PORT"` tidak ada, gunakan string `"3000"`.
- `.parse::<u16>()`: Mengubah teks string menjadi angka integer 16-bit (`0 - 65535`) khusus port jaringan.
- `.unwrap_or(3000)`: Jika isi PORT bukan angka valid (misal ada yang ngetik `"abc"`), fallback ke angka `3000`.

*Perbandingan Python:*
```python
# Di Python:
import os
port = int(os.getenv("PORT", 3000))
```

---

### 2. Inisialisasi PostgreSQL Connection Pool (Baris 38–58)
```rust
tracing::info!("Menghubungkan ke PostgreSQL: {}", database_url);
let postgres_pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
```

**Penjelasan Baris:**
- `PgPoolOptions::new().max_connections(10)`: Membatasi maksimal 10 koneksi bersamaan ke Postgres agar server database tidak kehabisan RAM.
- `.connect(&database_url).await?`: Membuka koneksi TCP secara asinkron. Tanda tanya `?` di ujungnya memastikan: jika password salah atau Postgres mati, program langsung berhenti (*fail-fast*) saat startup.

**Auto-Migration / Table Bootstrapping:**
```rust
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(100) NOT NULL,
        email VARCHAR(100) UNIQUE NOT NULL,
        created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    );
    "#,
)
.execute(&postgres_pool)
.await?;
```
- `r#" ... "#`: Fitur **Raw String Literal** di Rust (mirip triple quote `""" ... """` di Python). Semua karakter kutip dan enter di dalamnya tidak perlu di-escape pakai backslash.

---

### 3. Inisialisasi ClickHouse Client (Baris 60–80)
```rust
let clickhouse_client = Client::default()
    .with_url(&clickhouse_url)
    .with_user(&clickhouse_user)
    .with_password(&clickhouse_password)
    .with_database(&clickhouse_db);
```
- Menggunakan **Builder Pattern** khas Rust: merangkai konfigurasi klien satu per satu secara berantai (*method chaining*).

**Tabel user_activities di ClickHouse:**
```sql
CREATE TABLE IF NOT EXISTS user_activities (
    user_id UInt64,
    action String,
    details String,
    timestamp UInt64
) ENGINE = MergeTree()
ORDER BY (user_id, timestamp);
```
- ClickHouse menggunakan tipe data kolom seperti `UInt64` (unsigned 64-bit integer) dan mesin tabel `MergeTree()` yang dioptimalkan untuk query analitik super cepat.

---

## Keterkaitan File Ini dengan Layer Lain

- **`src/main.rs`**: Memanggil `AppConfig::init().await?` di baris awal fungsi `main`.
- Mengoper `config.postgres_pool` ke `PostgresUserRepository::new(...)`.
- Mengoper `config.clickhouse_client` ke `ClickHouseActivityRepository::new(...)`.
- Menggunakan `config.port` untuk binding TCP listener Axum: `"0.0.0.0:{}"`.
