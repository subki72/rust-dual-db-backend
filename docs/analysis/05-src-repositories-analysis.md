# Analysis: `src/repositories/` (Persistence / Database Layer)

## File Purpose
Layer Repository adalah **pintu gerbang interaksi langsung dengan database fisik**. Bertanggung jawab mengeksekusi query SQL mentah ke PostgreSQL (via `sqlx`) dan ClickHouse (via `clickhouse`), lalu memetakan hasilnya ke dalam struct domain (`User` dan `UserActivity`).

Prinsip Clean Architecture diterapkan secara ketat di sini:
- **`traits/`**: Kontrak antarmuka (*Interface*) abstrak tanpa implementasi database.
- **`impls/`**: Implementasi konkret yang memegang koneksi pool (`PgPool` dan `Client`).

---

## 1. Interface Trait: `src/repositories/traits/`

### A. `user_repository.rs`
```rust
use async_trait::async_trait;
use crate::models::User;
use crate::utils::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError>;
    async fn find_all(&self) -> Result<Vec<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
}
```

#### Fitur Kunci:
- **`#[async_trait]`**: Di Rust standar, trait bawaan belum mendukung `async fn` secara native dalam bentuk *dynamic trait object* (`dyn Trait`). Macro `#[async_trait]` menyulap return type fungsi menjadi `Pin<Box<dyn Future>>` di balik layar sehingga aman disimpan dalam pointer `Arc<dyn UserRepository>`.
- **`Send + Sync`**: Menjamin implementasi trait ini aman dipindah (*Send*) dan diakses bersama (*Sync*) lintas thread sistem operasi oleh runtime Tokio.
- **`Option<User>`**: Pada method `find_by_email`, kembaliannya dibungkus `Option`. Jika email ada, mengembalikan `Some(User)`; jika tidak ada, mengembalikan `None`. (Di Python: `Optional[User]` yang bisa `None`).

### B. `activity_repository.rs`
```rust
use async_trait::async_trait;
use crate::models::UserActivity;
use crate::utils::AppError;

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError>;
    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError>;
}
```

File pasangannya: `src/repositories/traits/mod.rs`:
```rust
pub mod user_repository;
pub mod activity_repository;

pub use user_repository::UserRepository;
pub use activity_repository::ActivityRepository;
```

---

## 2. Implementasi Konkret: `src/repositories/impls/`

### A. PostgreSQL: `postgres_user_repository.rs`

```rust
use async_trait::async_trait;
use crate::models::User;
use crate::repositories::traits::UserRepository;
use crate::utils::AppError;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email)
            VALUES ($1, $2)
            RETURNING id, name, email;
            "#,
        )
        .bind(name)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users ORDER BY id DESC LIMIT 50;",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users WHERE email = $1;",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
}
```

#### Operasi SQLx Tingkat Lanjut:
- **`sqlx::query_as::<_, User>`**: Memetakan baris query SQL secara otomatis ke struct `User` (berkat `#[derive(FromRow)]`). Simbol `_` artinya tipe database driver diserahkan ke inferensi compiler (PostgreSQL).
- **`.bind($1).bind($2)`**: Parameterized queries untuk **mencegah SQL Injection** secara mutlak.
- **`.fetch_one(&self.pool)`**: Mengharapkan tepat 1 baris hasil. Jika tidak ada, melempar `sqlx::Error::RowNotFound`.
- **`.fetch_all(&self.pool)`**: Mengembalikan semua baris dalam bentuk `Vec<User>`.
- **`.fetch_optional(&self.pool)`**: Mengembalikan `Ok(None)` jika baris tidak ditemukan, dan `Ok(Some(User))` jika ditemukan (sangat ideal untuk cek keunikan email!).

---

### B. ClickHouse: `clickhouse_activity_repository.rs`

```rust
use async_trait::async_trait;
use crate::models::UserActivity;
use crate::repositories::traits::ActivityRepository;
use crate::utils::AppError;
use clickhouse::Client;

#[derive(Clone)]
pub struct ClickHouseActivityRepository {
    client: Client,
}

impl ClickHouseActivityRepository {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ActivityRepository for ClickHouseActivityRepository {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError> {
        let mut insert = self.client.insert("user_activities")?;
        insert.write(activity).await?;
        insert.end().await?;
        Ok(())
    }

    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError> {
        let mut cursor = self
            .client
            .query("SELECT ?fields FROM user_activities ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch::<UserActivity>()?;

        let mut list = Vec::new();
        while let Some(row) = cursor.next().await? {
            list.push(row);
        }
        Ok(list)
    }
}
```

#### Operasi ClickHouse Tingkat Lanjut:
- **Streaming Insert (`insert.write(...).await?`)**: ClickHouse adalah database analitik kolom (*columnar*). Data ditulis melalui stream HTTP terkompresi berkecepatan tinggi.
- **Cursor Stream Query (`while let Some(row) = cursor.next().await?`)**: Hasil query dibaca secara streaming baris per baris (*chunked streaming*) tanpa membebani memori RAM server.

---

## 3. Master Re-export: `src/repositories/mod.rs`

```rust
pub mod traits;
pub mod impls;

pub use impls::*;
```
Memajang `PostgresUserRepository` dan `ClickHouseActivityRepository` agar siap diinisialisasi oleh `main.rs`.
