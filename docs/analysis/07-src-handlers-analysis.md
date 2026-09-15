# Analysis: `src/handlers/` (Presentation / HTTP Handlers Layer)

## File Purpose
Layer Handlers adalah **pintu masuk HTTP (Controller / Presentation Layer)** dari aplikasi `braincode-beV2`. Bertanggung jawab menerima request dari client jaringan, mengekstrak data dari HTTP body & query parameter, menyuntikkan dependensi aplikasi (*shared state*), mendelegasikan eksekusi ke layer service, dan mengembalikan response JSON beserta status code HTTP yang sesuai.

Modul ini terdiri dari 4 file:
1. `health_handler.rs`: Probe kesehatan sistem (Liveness / Readiness Probe).
2. `user_handler.rs`: Endpoint manajemen pengguna (POST `/api/users` & GET `/api/users`).
3. `analytic_handlers.rs`: Endpoint audit log analitik ClickHouse (GET `/api/analytics`).
4. `mod.rs`: Re-export facade yang mengekspos semua handler secara bersih.

---

## 1. Health Probe: `src/handlers/health_handler.rs`

```rust
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "braincode-be",
            "message": "Server Berjalan dengan normal"
        })),
    )
}
```

### Fitur Kunci:
- **`impl IntoResponse`**: Trait inti dari Axum yang menandakan bahwa tipe kembalian fungsi ini sah diubah menjadi response HTTP lengkap (status code, headers, dan payload body).
- **Kembalian Tuple `(StatusCode, Json)`**: Di Axum, pasangan tuple `(StatusCode, Body)` otomatis mengimplementasikan `IntoResponse`. Elemen pertama mengatur HTTP Status Code (`200 OK`), elemen kedua menulis body JSON.
- **Macro `json!({...})`**: Membuat objek JSON ad-hoc dari library `serde_json` tanpa perlu membuat struct perantara.
- **Wrapper `Json(...)`**: Otomatis menyuntikkan HTTP response header `Content-Type: application/json`.

### Komparasi Python (FastAPI):
```python
@app.get("/health", status_code=200)
async def health_check():
    return {
        "status": "ok",
        "service": "braincode-be",
        "message": "Server Berjalan dengan normal"
    }
```

---

## 2. User Controller: `src/handlers/user_handler.rs`

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use std::sync::Arc;
use crate::dtos::{ApiResponse, CreateUserRequest};
use crate::services::traits::UserService;
use crate::utils::AppError;

pub async fn create_user(
    State(user_service): State<Arc<dyn UserService>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service.register_user(payload).await?;
    let res = ApiResponse::success("Pengguna Berhasil dibuat", user);
    Ok((StatusCode::CREATED, Json(res)))
}

pub async fn get_users(
    State(user_service): State<Arc<dyn UserService>>,
) -> Result<impl IntoResponse, AppError> {
    let users = user_service.get_all_users().await?;
    let res = ApiResponse::success("Daftar Pengguna berhasil diambil", users);
    Ok((StatusCode::OK, Json(res)))
}
```

### Fitur Kunci:
- **Axum Extractor `State(user_service): State<Arc<dyn UserService>>`**:
  - Teknik Dependency Injection bawaan Axum.
  - Mengambil instance pointer `Arc` dari shared state aplikasi yang didaftarkan di router `main.rs`.
- **Axum Extractor `Json(payload): Json<CreateUserRequest>`**:
  - Membaca body HTTP stream, otomatis men-deserialize JSON menjadi struct `CreateUserRequest`.
  - Jika format data JSON yang dikirim client tidak sesuai, Axum otomatis membalas dengan status 422 Unprocessable Entity sebelum fungsi handler dieksekusi.
- **Return Type `Result<impl IntoResponse, AppError>`**:
  - Jika sukses: Mengembalikan `Ok((StatusCode::CREATED, Json(res)))` atau `Ok((StatusCode::OK, Json(res)))`.
  - Jika gagal: Tanda `?` mengalirkan `AppError`, dan Axum otomatis memanggil implementasi `IntoResponse` milik `AppError` untuk mengembalikan response error HTTP yang tepat (misal 400 Bad Request atau 500 Internal Error).

### Komparasi Python (FastAPI):
```python
@router.post("/api/users", status_code=status.HTTP_201_CREATED)
async def create_user(
    payload: CreateUserRequest,
    user_service: UserService = Depends(get_service)
):
    user = await user_service.register_user(payload)
    return ApiResponse.success("Pengguna Berhasil dibuat", user)

@router.get("/api/users", status_code=status.HTTP_200_OK)
async def get_users(
    user_service: UserService = Depends(get_service)
):
    users = await user_service.get_all_users()
    return ApiResponse.success("Daftar Pengguna berhasil diambil", users)
```

---

## 3. Analytics Controller: `src/handlers/analytic_handlers.rs`

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use std::sync::Arc;
use crate::dtos::ApiResponse;
use crate::services::traits::UserService;
use crate::utils::AppError;

pub async fn get_activities(
    State(user_service): State<Arc<dyn UserService>>,
) -> Result<impl IntoResponse, AppError> {
    let activities = user_service.get_recent_activities().await?;
    let res = ApiResponse::success(
        "Data analitik aktivitas terbaru dari ClickHouse Berhasil diambil",
        activities,
    );
    Ok((StatusCode::OK, Json(res)))
}
```

### 🔍 Fitur Kunci:
- Mengambil rekam log analitik terbaru dari ClickHouse melalui pemanggilan `user_service.get_recent_activities().await?`.
- Mengemas `Vec<UserActivity>` ke dalam amplop serbaguna `ApiResponse::success(...)`.
- Mengembalikan HTTP 200 OK dengan payload JSON terstruktur.

---

## 4. Master Re-export: `src/handlers/mod.rs`

```rust
pub mod health_handler;
pub mod user_handler;
pub mod analytic_handlers;

pub use health_handler::health_check;
pub use user_handler::{create_user, get_users};
pub use analytic_handlers::get_activities;
```

### 🔍 Fitur Kunci:
- Pola **Facade Pattern** yang menyatukan seluruh handler dari masing-masing submodul.
- Memungkinkan file `src/main.rs` mengimpor seluruh fungsi handler dalam satu baris:
  ```rust
  use crate::handlers::{create_user, get_activities, get_users, health_check};
  ```

### 💡 Komparasi Python (FastAPI):
```python
# handlers/__init__.py
from .health_handler import health_check
from .user_handler import create_user, get_users
from .analytic_handlers import get_activities
```
