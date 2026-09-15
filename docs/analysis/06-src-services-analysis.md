# Analysis: `src/services/` (Business Logic Layer)

## File Purpose
Layer Service adalah **jantung logika bisnis (Domain & Application Rules)** dari aplikasi `braincode-beV2`. Layer ini bertindak sebagai orkestrator yang menjembatani antara Presentation/HTTP Handler layer dengan Persistence/Repository layer.

Prinsip Clean Architecture diterapkan secara ketat melalui pemisahan:
1. **`traits/`**: Kontrak antarmuka (*Interface*) bisnis abstrak tanpa dependensi implementasi database tertentu.
2. **`impls/`**: Implementasi konkret orkestrasi bisnis yang memanfaatkan teknik **Dependency Injection** via pointer `Arc<dyn Trait>`.

---

## 1. Interface Trait: `src/services/traits/`

### A. `user_service.rs`
```rust
use async_trait::async_trait;
use crate::dtos::{CreateUserRequest, UserResponse};
use crate::models::UserActivity;
use crate::utils::AppError;

#[async_trait]
pub trait UserService: Send + Sync {
    async fn register_user(&self, req: CreateUserRequest) -> Result<UserResponse, AppError>;
    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError>;
    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError>;
}
```

#### Fitur Kunci & Komparasi Python:
- **`#[async_trait]`**: Menjamin method asinkron di dalam trait ini berstatus *Object-Safe* sehingga dapat dibungkus ke dalam *dynamic dispatch pointer* `Arc<dyn UserService>`.
- **`Send + Sync`**: Jaminan *thread-safety* agar service aman diakses bersamaan oleh berbagai *worker threads* Tokio runtime saat ribuan request HTTP masuk serentak.
- **Konsumsi DTO Murni**:
  - Menerima `CreateUserRequest` (bukan data mentah SQL).
  - Mengembalikan `UserResponse` (bukan entitas `User` internal database). Hal ini menjamin privasi data internal (seperti password hash di masa depan) tidak bocor ke luar.
- *Di Python (FastAPI):* Setara dengan *abstract base class* / *interface*:
  ```python
  from abc import ABC, abstractmethod

  class UserService(ABC):
      @abstractmethod
      async def register_user(self, req: CreateUserRequest) -> UserResponse: ...
      @abstractmethod
      async def get_all_users(self) -> list[UserResponse]: ...
      @abstractmethod
      async def get_recent_activities(self) -> list[UserActivity]: ...
  ```

File pasangannya: `src/services/traits/mod.rs`:
```rust
pub mod user_service;

pub use user_service::UserService;
```

---

## 2. Implementasi Konkret: `src/services/impls/`

### A. `user_service_impl.rs`
```rust
use async_trait::async_trait;
use crate::dtos::{CreateUserRequest, UserResponse};
use crate::models::UserActivity;
use crate::repositories::traits::{ActivityRepository, UserRepository};
use crate::services::traits::UserService;
use crate::utils::AppError;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct UserServiceImpl {
    user_repo: Arc<dyn UserRepository>,
    activity_repo: Arc<dyn ActivityRepository>,
}

impl UserServiceImpl {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        activity_repo: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self {
            user_repo,
            activity_repo,
        }
    }
}
```

#### Fitur Kunci Dependency Injection:
- **`Arc<dyn UserRepository>` & `Arc<dyn ActivityRepository>`**:
  - `Arc` (*Atomically Reference Counted*): Smart pointer thread-safe yang mengizinkan kepemilikan bersama (*shared ownership*) instance repository di heap.
  - `dyn Trait` (*Dynamic Dispatch*): Service tidak terikat langsung pada database PostgreSQL atau ClickHouse konkret (`PostgresUserRepository`), melainkan pada abstraksi kontraknya. Ini memudahkan *unit testing* dengan *mock repository*.
- **Konstruktor `pub fn new(...) -> Self`**:
  - Pola konstruktor standar Rust untuk menyuntikkan (*inject*) dependensi repository dari luar (diinisialisasi di `src/main.rs`).

---

### B. Orkestrasi Pendaftaran Pengguna (`register_user`)

```rust
#[async_trait]
impl UserService for UserServiceImpl {
    async fn register_user(&self, req: CreateUserRequest) -> Result<UserResponse, AppError> {
        // 1. Validasi Input Bisnis
        if req.name.trim().is_empty() {
            return Err(AppError::ValidationError("Nama tidak boleh kosong.".into()));
        }
        if !req.email.contains('@') {
            return Err(AppError::ValidationError("Format email tidak valid.".into()));
        }

        // 2. Cek apakah email sudah terdaftar di PostgreSQL
        if (self.user_repo.find_by_email(&req.email).await?).is_some() {
            return Err(AppError::ValidationError("Email sudah terdaftar.".into()));
        }

        // 3. Simpan ke PostgreSQL (OLTP)
        let user = self.user_repo.create(&req.name, &req.email).await?;

        // 4. Catat Jejak Analitik ke ClickHouse (OLAP)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .as_secs();

        let activity = UserActivity {
            user_id: user.id as u64,
            action: "user_registered".to_string(),
            details: format!("Pendaftaran pengguna baru dengan email {}", user.email),
            timestamp: now,
        };

        // Simpan log analitik tanpa menggagalkan flow utama jika error
        if let Err(err) = self.activity_repo.record(&activity).await {
            tracing::warn!("Gagal mencatat analitik ClickHouse: {:?}", err);
        }

        Ok(UserResponse::from(user))
    }
```

#### Bedah Alur Kerja Langkah Demi Langkah:
1. **Validasi Aturan Bisnis (Fail-Fast)**:
   - `req.name.trim().is_empty()`: Menolak nama kosong atau hanya berisi spasi.
   - `!req.email.contains('@')`: Validasi sintaks dasar email sebelum membebani database.
   - Mengembalikan `Err(AppError::ValidationError(...))` yang langsung dipetakan menjadi HTTP 400 Bad Request di HTTP layer.
2. **Pengecekan Keunikan (Business Invariant)**:
   - `(self.user_repo.find_by_email(&req.email).await?).is_some()`:
     - Method `find_by_email` me-return `Result<Option<User>, AppError>`.
     - Tanda `?` membongkar `Result` menjadi `Option<User>`.
     - Method `.is_some()` mengecek apakah user ditemukan. Jika `true`, tolak registrasi dengan error *"Email sudah terdaftar."*.
3. **Persistensi Transaksional PostgreSQL (OLTP)**:
   - `self.user_repo.create(&req.name, &req.email).await?`: Menyimpan data permanen ke PostgreSQL dan mendapatkan entitas `user` yang memiliki `id`.
4. **Kalkulasi Waktu Unix Timestamp**:
   - `SystemTime::now().duration_since(UNIX_EPOCH)`: Mengambil durasi selisih waktu sekarang terhadap era Unix (1 Januari 1970).
   - `.as_secs()`: Mengambil angka detik integer bulat (`u64`) yang cocok dengan tipe `timestamp: u64` di ClickHouse.
5. **Resilient Side-Effect Logging (Pola Desain Kritis)**:
   - Perhatikan baris ini:
     ```rust
     if let Err(err) = self.activity_repo.record(&activity).await {
         tracing::warn!("Gagal mencatat analitik ClickHouse: {:?}", err);
     }
     ```
   - **Mengapa tidak menggunakan tanda `?` di sini?**
     Jika server ClickHouse sedang mengalami *down* atau *maintenance*, kita **TIDAK INGIN** pendaftaran pengguna ikut gagal! 
     Pendaftaran user di PostgreSQL sudah sah dan sukses. Log analitik adalah *side effect*, sehingga kegagalannya cukup dicatat sebagai *warning log* tanpa menggagalkan response ke client.
6. **Transformasi DTO Akhir**:
   - `Ok(UserResponse::from(user))`: Mengubah entitas database `User` menjadi DTO publik `UserResponse` menggunakan trait `From`.

---

### C. Pembacaan Data & Pipeline Iterator (`get_all_users`)

```rust
    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError> {
        let users = self.user_repo.find_all().await?;
        Ok(users.into_iter().map(UserResponse::from).collect())
    }

    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError> {
        self.activity_repo.get_recent(20).await
    }
}
```

#### Bedah Pipeline Iterator:
* `users.into_iter()`: Mengonsumsi `Vec<User>` dan mengubahnya menjadi iterator kepemilikan (*consuming iterator*).
* `.map(UserResponse::from)`: Menerapkan fungsi konversi trait `From` ke setiap elemen dengan gaya *point-free function pointer*.
* `.collect()`: Mengumpulkan kembali hasil transformasi menjadi `Vec<UserResponse>` baru.
* *Di Python:* Setara dengan *list comprehension*:
  ```python
  return [UserResponse.from_user(u) for u in users]
  ```

File pasangannya: `src/services/impls/mod.rs`:
```rust
pub mod user_service_impl;

pub use user_service_impl::UserServiceImpl;
```

---

## 3. Master Re-export: `src/services/mod.rs`

```rust
pub mod traits;
pub mod impls;

pub use traits::*;
pub use impls::*;
```

Sintaks wildcard `*` mengangkat seluruh trait dan implementasi service ke tingkat `crate::services::*` (Facade Pattern).
Dengan demikian, file entry point (`src/main.rs`) atau handler HTTP cukup mengimpor dengan bersih:

```rust
use crate::services::{UserService, UserServiceImpl};
```
