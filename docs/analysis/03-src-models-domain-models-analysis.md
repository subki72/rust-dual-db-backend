# Analysis: `src/models/user.rs` & `src/models/activity_log.rs` (Core Domain Models)

## File Purpose
Modul `src/models/` adalah **jantung representasi data domain (Database Entities)**. Berisi cetak biru (*struct*) yang memetakan langsung struktur kolom pada tabel database fisik ke dalam memori aplikasi:
1. `user.rs`: Memetakan tabel transaksional PostgreSQL `users`.
2. `activity_log.rs`: Memetakan tabel analitik ClickHouse `user_activities`.
3. `mod.rs`: Pintu gerbang modul dan re-export facade.

---

## Overview `src/models/user.rs` (PostgreSQL Model)

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}
```

### Breakdown Komponen & Derive:
- **`sqlx::FromRow` (Paling Krusial)**: Trait dari library SQLx yang mengajarkan Rust cara memetakan sebaris baris data SQL dari PostgreSQL (kolom `id`, `name`, `email`) secara otomatis ke field struct `User` tanpa perlu kita ketik manual seperti `row.get("id")`.
  - *Python equivalent*: Deklarasi model ORM di SQLAlchemy (`class User(Base): id = Column(Integer, primary_key=True)`).
- **`pub id: i32`**: Integer 32-bit bertanda (`-2 miliar s/d +2 miliar`) yang cocok 1:1 dengan tipe kolom `SERIAL` atau `INTEGER` di PostgreSQL.
- **`pub name: String` & `pub email: String`**: Teks dinamis di heap memory yang cocok dengan kolom `VARCHAR(100)` di PostgreSQL.
- **`Serialize` & `Deserialize`**: Memungkinkan struct `User` diubah ke format JSON (saat dikirim sebagai response API) maupun dibaca dari JSON.
- **`Clone` & `Debug`**: Memungkinkan objek user diduplikasi dan dicetak saat logging debug.

---

## Overview `src/models/activity_log.rs` (ClickHouse Model)

```rust
use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct UserActivity {
    pub user_id: u64,
    pub action: String,
    pub details: String,
    pub timestamp: u64,
}
```

### Breakdown Komponen:
- **`clickhouse::Row`**: Trait khusus dari crate `clickhouse` yang menyusun serialisasi biner super cepat untuk format baris ClickHouse.
- **`pub user_id: u64`**: Angka bulat 64-bit tanpa tanda (*Unsigned 64-bit Integer*, `0 s/d 18 kuantiliun`). Di ClickHouse, tipe kolomnya adalah `UInt64`.
- **`pub action: String` & `pub details: String`**: Tipe `String` di ClickHouse tidak terbatas ukuran (arbitrary length string).
- **`pub timestamp: u64`**: Waktu event dalam format Unix Timestamp (milidetik atau detik sejak 1970) bertipe `UInt64`.

---

## Overview `src/models/mod.rs` (Re-export Facade)

```rust
pub mod user;
pub mod activity_log;

pub use user::User;
pub use activity_log::UserActivity;
```

Memajang struct `User` dan `UserActivity` setingkat lebih tinggi ke root namespace `models` agar layer repository dan service cukup memanggil:
```rust
use crate::models::{User, UserActivity};
```
Tanpa perlu memanggil path internal panjang seperti `use crate::models::user::User;`.

---

## Komparasi Penting: Kenapa Ada Struct `User` vs `CreateUserRequest`?

| Karakteristik | `src/models/user.rs` (Model) | `src/dtos/requests/create_user_request.rs` (DTO) |
|---|---|---|
| **Field `id`** | **Ada** (`pub id: i32`), dibuat otomatis oleh DB | **Tidak Ada** (Client tidak boleh tentukan ID) |
| **Derive DB** | Memakai `#[derive(FromRow)]` (SQLx) | Tidak terikat ke SQLx sama sekali |
| **Keamanan** | Representasi data internal server | Filter validasi input luar (mencegah *mass assignment*) |
