# Analysis: `src/utils/errors.rs`

## File Purpose
File ini bertindak sebagai **pusat penanganan error aplikasi (*Centralized Error Handling*)**. Tujuannya adalah mengumpulkan semua kemungkinan kegagalan di backend (koneksi PostgreSQL putus, ClickHouse query error, data tidak valid, 404 Not Found) ke dalam satu tipe data resmi (`AppError`), lalu secara otomatis menerjemahkannya menjadi respon HTTP JSON seragam (`IntoResponse`) lengkap dengan status code yang tepat.

---

## Overview
Di framework Python seperti FastAPI, ketika ada error di database atau validasi, kamu biasanya menulis:
```python
raise HTTPException(status_code=400, detail="Data tidak valid")
```
Di Rust tidak ada konsep melempar exception liar (*no runtime exceptions*). Rust menganggap error sebagai data nilai (*value*) bertipe `Result<T, AppError>`. File ini menyediakan cetak biru `AppError` tersebut dan mengajarkan Axum cara mengubah `AppError` menjadi respon JSON saat terjadi kegagalan.

File ini memiliki 3 seksi utama:
1. **Imports (Baris 1–7)**: Dependensi HTTP dari Axum, JSON macro, dan macro derive `thiserror`.
2. **Definisi Enum Error (Baris 9–26)**: Enumerasi varian error dengan anotasi `#[error(...)]` dan `#[from]`.
3. **Implementasi Trait Axum (Baris 28–51)**: `impl IntoResponse for AppError` untuk menyusun payload JSON dan HTTP status code.

---

## Imports & Dependencies

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
```

**Penjelasan Komponen:**
- **`axum::http::StatusCode`**: Kumpulan konstanta kode status HTTP standar (misal `StatusCode::BAD_REQUEST = 400`, `StatusCode::INTERNAL_SERVER_ERROR = 500`).
  - *Python equivalent*: `from fastapi import status` (seperti `status.HTTP_400_BAD_REQUEST`).
- **`axum::response::{IntoResponse, Response}`**: Trait dan struct resmi Axum untuk membentuk respon HTTP jaringan.
- **`axum::Json`**: Wrapper tipe data untuk memastikan header `Content-Type: application/json` disematkan ke respon.
- **`serde_json::json`**: Macro deklaratif untuk membuat objek JSON dinamis tanpa harus membuat struct baru.
  - *Python equivalent*: Dictionary literal biasa `{"status": "error", "message": message}`.
- **`thiserror::Error`**: Macro derive cerdas dari crate `thiserror` yang secara otomatis menuliskan implementasi trait bawaan Rust `std::fmt::Display` dan `std::error::Error`.

---

## Data Structures

### Enum: `AppError`

```rust
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("ClickHouse error: {0}")]
    ClickHouseError(#[from] clickhouse::error::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}
```

### Breakdown Varian Error:

| Varian Enum | Tipe Data Payload | HTTP Status Code | Keterangan & Asal Error |
|---|---|---|---|
| `DatabaseError` | `sqlx::Error` | `500 Internal Server Error` | Berasal otomatis dari query PostgreSQL via `sqlx`. |
| `ClickHouseError`| `clickhouse::error::Error`| `500 Internal Server Error` | Berasal otomatis dari query ClickHouse. |
| `ValidationError`| `String` | `400 Bad Request` | Pesan validasi bisnis (misal: email kosong). |
| `NotFound` | `String` | `404 Not Found` | Data resource tidak ditemukan di database. |
| `InternalError` | `String` | `500 Internal Server Error` | Error server umum yang tidak terduga. |

### Perbandingan Konseptual dengan Python:

```python
# Di Python (FastAPI): Biasanya membuat hirarki Class Exception
class AppError(Exception):
    def __init__(self, message: str, status_code: int = 500):
        self.message = message
        self.status_code = status_code
        super().__init__(message)

class ValidationError(AppError):
    def __init__(self, message: str):
        super().__init__(message, status_code=400)

class NotFoundError(AppError):
    def __init__(self, message: str):
        super().__init__(message, status_code=404)
```

**Perbedaan Kunci:**
- Di Python, kamu membuat banyak class yang mewarisi `Exception` (*Inheritance*).
- Di Rust, kamu menggunakan satu `enum` bertipe aljabar (*Tagged Union / Algebraic Data Type*). Semua kemungkinan error terdaftar di satu tempat dan compiler menjamin kamu menangani seluruh cabangnya secara lengkap saat pattern matching.

---

## Fitur Rust Spesial di File Ini

### 1. Keajaiban Atribut `#[from]` (Otomatisasi Operator `?`)
Perhatikan baris ini:
```rust
DatabaseError(#[from] sqlx::Error)
```
Atribut `#[from]` yang disediakan oleh `thiserror` secara otomatis membangkitkan kode:
```rust
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}
```
**Dampaknya:** Di layer Repository, ketika kamu memanggil query database:
```rust
let user = sqlx::query_as(...).fetch_one(&self.pool).await?;
```
Jika query gagal menghasilkan `sqlx::Error`, operator tanda tanya `?` secara otomatis mengonversi error tersebut menjadi `AppError::DatabaseError` tanpa perlu kamu tulis `match` atau `map_err` manual!

### 2. Keamanan Info Internal (Information Disclosure Protection)
Perhatikan di `into_response`:
```rust
AppError::DatabaseError(err) => {
    tracing::error!("Database error: {:?}", err);
    (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada database.".to_string())
}
```
Pesan error SQL asli yang memuat struktur tabel atau password dicatat ke internal server log (`tracing::error!`), sedangkan pesan yang dikirim ke browser/client disamarkan menjadi kalimat umum `"Terjadi kesalahan pada database."`. Ini adalah best practice keamanan API enterprise.

---

## Trait Implementation: `IntoResponse`

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada database.".to_string())
            }
            AppError::ClickHouseError(err) => {
                tracing::error!("ClickHouse error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada analitik ClickHouse.".to_string())
            }
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "status": "error",
            "message": message
        }));

        (status, body).into_response()
    }
}
```

### Alur Eksekusi:
1. `match &self`: Mencocokkan varian error saat ini.
2. Mengekstrak pasangan tuple `(status: StatusCode, message: String)`.
3. Membungkus format JSON: `{"status": "error", "message": message}`.
4. Menghasilkan respon HTTP final via `(status, body).into_response()`.

---

## Keterkaitan File Ini dengan Layer Lain

- **Repository Layer**: Menggunakan `Result<T, AppError>` untuk method database. Error dari `sqlx` langsung di-bubble up menggunakan operator `?`.
- **Service Layer**: Mengembalikan `Result<T, AppError>`. Jika ada validasi gagal (misal email kembar), melempar `Err(AppError::ValidationError(...))`.
- **Handler Layer**: Mengembalikan `Result<impl IntoResponse, AppError>`. Jika ada service yang gagal, handler langsung melempar `Err(e)` dan Axum otomatis menjalankan method `into_response()` di file ini untuk merespons client.
