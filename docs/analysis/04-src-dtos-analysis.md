# Analysis: `src/dtos/` (Data Transfer Objects Layer)

## File Purpose
Layer DTO (*Data Transfer Objects*) adalah **kontrak data jaringan** antara aplikasi kita dengan dunia luar. Tujuannya adalah memisahkan representasi internal tabel database (`models`) dari apa yang diterima atau dikirimkan melalui HTTP API.

Layer ini dibagi menjadi dua bagian:
1. **`requests/`**: Formulir input dari client (`Deserialize`).
2. **`responses/`**: Format amplop keluaran resmi ke client (`Serialize`).

---

## 1. Request DTO: `src/dtos/requests/create_user_request.rs`

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}
```

### Penjelasan & Komparasi Python:
- *Di Python (FastAPI):* Persis seperti `class CreateUserRequest(BaseModel): name: str; email: str`.
- **`#[derive(Deserialize)]`**: Memberi kemampuan struct ini untuk dibongkar (*unpacked*) otomatis oleh Axum dari HTTP Request Body JSON.
- **Tanpa field `id`**: Karena client yang mendaftar belum memiliki ID database (mencegah eksploitasi *over-posting / mass assignment*).

File pasangannya: `src/dtos/requests/mod.rs`:
```rust
pub mod create_user_request;
pub use create_user_request::CreateUserRequest;
```

---

## 2. Generic Envelope: `src/dtos/responses/api_response.rs`

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.into(),
            data,
        }
    }
}
```

### Penjelasan & Komparasi Python:
- **Generics `ApiResponse<T>`**: Tipe data `T` adalah tipe variabel generic (bisa diisi `UserResponse`, `Vec<UserResponse>`, atau tipe apapun).
  - *Di Python:* `class ApiResponse(BaseModel, Generic[T]): status: str; message: str; data: T`.
- **`message: impl Into<String>`**: Fitur fleksibilitas Rust. Parameter ini menerima baik string literal `&str` (`"Pengguna berhasil dibuat!"`) maupun `String` dinamis, dan method `.into()` akan otomatis mengonversinya menjadi `String` kepemilikan struct.
- **`#[derive(Serialize)]`**: Mengizinkan struct ini diubah (*serialized*) menjadi teks JSON oleh Axum saat dikirim kembali ke client browser.

---

## 3. Response DTO: `src/dtos/responses/user_response.rs`

```rust
use crate::models::User;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
        }
    }
}
```

### Penjelasan Trait `From<User>`:
- **`impl From<User> for UserResponse`**: Ini adalah pola standar idiomatis di Rust untuk konversi tipe data.
- Ketika kamu punya objek `User` dari database dan ingin mengubahnya menjadi `UserResponse`, kamu cukup mengetik:
  ```rust
  let res: UserResponse = user.into(); // atau UserResponse::from(user)
  ```
- *Manfaat Arsitektur:* Jika tabel database nanti memiliki kolom rahasia (seperti `password_hash` atau `salt`), kolom tersebut tidak dimasukkan ke `UserResponse`, sehingga data sensitif tidak akan pernah bocor ke publik!

File pasangannya: `src/dtos/responses/mod.rs`:
```rust
pub mod api_response;
pub mod user_response;

pub use api_response::ApiResponse;
pub use user_response::UserResponse;
```

---

## 4. Master Re-export: `src/dtos/mod.rs`

```rust
pub mod requests;
pub mod responses;

pub use requests::*;
pub use responses::*;
```
Sintaks wildcard `*` mengangkat seluruh DTO request dan response langsung ke `crate::dtos::*`.
Dengan begitu, siapapun yang butuh DTO cukup mengetik:
```rust
use crate::dtos::{ApiResponse, CreateUserRequest, UserResponse};
```
Sangat bersih dan rapi!
