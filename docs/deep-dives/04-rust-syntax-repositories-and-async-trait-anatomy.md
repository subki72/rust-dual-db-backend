# Deep Dive: Anatomi Rust Syntax `#[async_trait]`, Database Persistence (SQLx & ClickHouse), Trait Bounds `Send + Sync`, & Cursor Streaming

Dokumentasi ini merangkum sesi *micro-learning deep dive* yang membedah sintaksis, sistem *asynchronous trait object*, jaminan keamanan *multithreading* (`Send + Sync`), eksekusi query PostgreSQL via `sqlx`, dan mekanisme *binary streaming* ClickHouse pada layer Repository ([`src/repositories/`](../../src/repositories/)).

---

## Daftar Topik yang Dibedah

1. **Macro `#[async_trait]` & Misteri Object Safety pada `dyn Trait`**
2. **Jaminan Keamanan Multithreading Tokio (`Send + Sync`)**
3. **Pola Konstruktor Struct Idiomatis (`pub fn new(...) -> Self`)**
4. **Bedah Eksekusi Query PostgreSQL via SQLx (`query_as`, Turbofish, Raw String, & `?`)**
5. **Filosofi Tipe Kembalian: `Vec<User>` vs Double Envelope `Option<User>`**
6. **Mekanisme Binary Streaming Insert ClickHouse (`record` & Unit Type `()`)**
7. **Streaming Cursor Reader ClickHouse (`get_recent`, `?fields`, & `while let`)**
8. **Quick Reference Card**
9. **Kaitan dengan Arsitektur Codebase `braincode-beV2`**

---

## 1. Macro `#[async_trait]` & Misteri Object Safety pada `dyn Trait`

### Masalah Besar di Rust Standar:
Ketika kita menulis fungsi asynchronous biasa di Rust:
```rust
async fn foo() -> i32 { 42 }
```
Compiler Rust di balik layar menyulap tanda tangan fungsi tersebut menjadi:
```rust
fn foo() -> impl Future<Output = i32>
```
Tipe `impl Future` ini adalah tipe anonim tanpa nama (*unnamable type*) yang dibuat oleh compiler secara dinamis, dan ukuran memorinya (*memory layout size*) **belum diketahui** sebelum kode selesai dikompilasi.

Dalam arsitektur *Clean Architecture* atau *Dependency Injection*, kita ingin membungkus repository ke dalam pointer dinamis (*Dynamic Trait Object*):
```rust
Arc<dyn UserRepository> // <-- Membutuhkan ukuran memori yang pasti (Object Safe)
```
Rust mewajibkan setiap trait yang ingin dijadikan `dyn Trait` harus berstatus **Object-Safe** (ukuran pointer dan layout tabel virtual method / *vtable* harus terdefinisi pasti). Karena ukuran `impl Future` tidak menentu, Rust compiler bawaan **akan menolak keras** pembuatan `dyn UserRepository`!

### Solusi: `#[async_trait]`
Macro `#[async_trait]` dari crate `async-trait` memodifikasi tanda tangan fungsi secara otomatis di balik layar:
```rust
// Bentuk yang dihasilkan #[async_trait] di balik layar:
fn create<'a>(&'a self, name: &'a str, email: &'a str) 
    -> Pin<Box<dyn Future<Output = Result<User, AppError>> + Send + 'a>>
```
* **`Box<...>`**: Memindahkan masa hidup (*lifecycle*) `Future` tersebut ke Heap Memory.
* **Ukuran Pointer Heap Pasti**: Di arsitektur komputer 64-bit, sebuah pointer `Box` ukurannya selalu tepat **8 byte**.
* **Efek Arsitektur**: Trait resmi menjadi **Object-Safe** dan 100% legal dibungkus ke dalam smart pointer `Arc<dyn UserRepository>`.

> **💡 Komparasi Python:**
> Di Python, semua method `async def` otomatis menghasilkan objek *coroutine* tanpa hambatan memori karena Python adalah bahasa interpreted dinamis dengan Garbage Collector:
> ```python
> class UserRepository(ABC):
>     @abstractmethod
>     async def create(self, name: str, email: str) -> User: ...
> ```
> Di Rust yang strictly-typed tanpa garbage collector, `#[async_trait]` adalah jembatan wajib agar interface async bisa disimpan dalam pointer polimorfik.

---

## 2. Jaminan Keamanan Multithreading Tokio (`Send + Sync`)

Di file [`src/repositories/traits/user_repository.rs`](../../src/repositories/traits/user_repository.rs):

```rust
#[async_trait]
pub trait UserRepository: Send + Sync { ... }
```

Simbol `: Send + Sync` adalah **Super-traits / Trait Bounds**:

1. **`Send`**: Menjamin bahwa data/objek repository ini **bisa dipindahkan (*moved*) kepemilikannya ke thread OS lain secara aman**.
2. **`Sync`**: Menjamin bahwa referensi repository (`&self`) **bisa diakses dan dibaca bersamaan (*shared borrow*) oleh banyak thread sekaligus tanpa risiko *data race***.

### Mengapa Wajib untuk Tokio Runtime?
Web framework Axum berjalan di atas runtime multi-threaded **Tokio (Work-Stealing Thread Pool)**.
Ketika ratusan request HTTP masuk serentak:
* Request A dikerjakan oleh Worker Thread 1.
* Request B dikerjakan oleh Worker Thread 2.
* Kedua thread tersebut meminjam instance `UserRepository` yang sama secara bersamaan.
Jika trait tidak memiliki bound `: Send + Sync`, Tokio akan menolak kode saat kompilasi demi mencegah *race condition* atau kerusakan memori (*undefined behavior*).

---

## 3. Pola Konstruktor Struct Idiomatis (`pub fn new`)

Di file [`src/repositories/impls/postgres_user_repository.rs`](../../src/repositories/impls/postgres_user_repository.rs):

```rust
#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
```

* **Tanpa Keyword `constructor` / `__init__`**: Rust tidak memiliki method konstruktor khusus bawaan bahasa. Pola standar komunitas Rust adalah membuat fungsi asosiasi publik bernama **`new`**.
* **`pool: PgPool`**: Parameter koneksi database PostgreSQL yang dioper masuk. Karena `PgPool` di dalamnya dibungkus pointer `Arc`, meng-copy atau memindahkan `pool` ini sangat murah (hanya menyalin pointer 8 byte).
* **`-> Self`**: Mengembalikan instance struct itu sendiri (`PostgresUserRepository`).
* **`Self { pool }`**: Instansiasi struct menggunakan fitur *field init shorthand* (`pool: pool`).

---

## 4. Bedah Eksekusi Query PostgreSQL via SQLx

```rust
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
```

### A. `sqlx::query_as::<_, User>(...)`
* **`query_as`**: Menjalankan query SQL sekaligus otomatis memetakan kolom hasil baris database ke dalam struct yang mengimplementasikan trait `sqlx::FromRow`.
* **Sintaks Turbofish `::<_, User>`**:
  * Parameter generic pertama `_` (Underscore): Memerintahkan compiler untuk menebak (*infer*) driver database dari parameter pool (`sqlx::Postgres`).
  * Parameter generic kedua `User`: Struct tujuan pemetaan data.

### B. `r#" ... "#` (Raw String Literal)
* Awalan `r#"` dan akhiran `"#` menandakan string multi-baris mentah di Rust. Tidak perlu khawatir dengan karakter petik dua `"` atau baris baru (`\n`).
* **Fitur PostgreSQL `RETURNING id, name, email;`**: PostgreSQL langsung me-return baris data yang baru dibuat beserta nomor `id` auto-increment tanpa perlu melakukan query `SELECT` susulan!
* **`$1, $2`**: Parameter query berindeks PostgreSQL yang menjamin 100% kebal terhadap serangan **SQL Injection**.

### C. `.bind(name).bind(email)`
* Mengikat nilai variabel ke placeholder:
  * `.bind(name)` mengisi `$1`.
  * `.bind(email)` mengisi `$2`.

### D. `.fetch_one(&self.pool)`
* Meminjam satu koneksi dari pool `&self.pool`, mengirim query ke PostgreSQL, dan mengharapkan tepat 1 baris hasil kembali.

### E. `.await?` (Async Pause & Error Propagation)
* **`.await`**: Menyerahkan eksekusi kembali ke runtime Tokio selama menunggu paket data jaringan dari database PostgreSQL, sehingga thread OS tidak terblokir.
* **Tanda Tanya `?`**:
  * Jika query gagal (`Err(sqlx::Error)`), operator `?` otomatis mengonversinya menjadi `AppError::DatabaseError` (berkat macro `#[from]` di file `src/utils/errors.rs`) dan langsung keluar (*early return*).
  * Jika sukses (`Ok(user)`), tanda `?` membongkar bungkusan `Ok` dan memberikan nilai murni `User` ke variabel `let user`.

### F. `Ok(user)`
* Mengembalikan objek `User` di dalam varian `Ok` tanpa titik koma (*implicit return*).

---

## 5. Filosofi Tipe Kembalian: `Vec<User>` vs `Option<User>`

```rust
async fn find_all(&self) -> Result<Vec<User>, AppError>;
async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
```

### A. `Result<Vec<User>, AppError>` (Banyak Data)
* Jika tabel memiliki 10 user: Mengembalikan `Ok(vec![... 10 user ...])`.
* **Bagaimana jika tabel kosong?** Mengembalikan **`Ok(vec![])`** (vektor kosong dengan panjang 0), **bukan error!**
* Varian `Err(AppError)` hanya terjadi jika terjadi kegagalan infrastruktur (koneksi putus, server mati).

### B. `Result<Option<User>, AppError>` (Pola Double Envelope)
Struktur ini memisahkan secara tegas antara **kegagalan sistem** vs **ketiadaan data bisnis**:
1. **`Ok(Some(User))`**: Database sehat, user dengan email tersebut **ditemukan**.
2. **`Ok(None)`**: Database sehat, query sukses berjalan, tetapi user **tidak ada di tabel**. (Di SQLx dipanggil via `.fetch_optional(&self.pool)`).
3. **`Err(AppError)`**: Koneksi database putus atau error SQL fatal.

---

## 6. Mekanisme Binary Streaming Insert ClickHouse (`record`)

Di file [`src/repositories/impls/clickhouse_activity_repository.rs`](../../src/repositories/impls/clickhouse_activity_repository.rs):

```rust
async fn record(&self, activity: &UserActivity) -> Result<(), AppError> {
    let mut insert = self.client.insert("user_activities")?;
    insert.write(activity).await?;
    insert.end().await?;
    Ok(())
}
```

### Mengapa Berbeda dengan PostgreSQL?
ClickHouse adalah database analitik **Columnar (OLAP)**. Melakukan query insert konvensional `INSERT INTO ... VALUES (...)` satu per satu sangat tidak efisien untuk OLAP. Crate `clickhouse` menggunakan **Binary Streaming Protocol**.

1. **`activity: &UserActivity`**: Meminjam data analitik secara *immutable* (tanpa alokasi memori baru).
2. **`-> Result<(), AppError>`**: Menggunakan **Unit Type `()`**. Simbol `()` berukuran **0 byte** di memori, berfungsi sama seperti `None` atau `void` (menandakan fungsi tidak me-return data apapun selain sinyal sukses).
3. **`let mut insert = self.client.insert("user_activities")?;`**: Membuka stream insert biner ke tabel. Bersifat mutable (`mut`) karena buffer internalnya terus bertambah.
4. **`insert.write(activity).await?;`**: Mengubah struct `UserActivity` menjadi format biner ClickHouse (berkat `#[derive(Row)]`) dan mengirimkannya ke stream buffer.
5. **`insert.end().await?;`**: **Langkah krusial!** Menutup stream dan memerintahkan ClickHouse untuk melakukan commit/flush batch data ke penyimpanan disk.
6. **`Ok(())`**: Mengembalikan nilai unit `()` yang dibungkus `Ok`.

---

## 7. Streaming Cursor Reader ClickHouse (`get_recent`)

```rust
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
```

### A. Template Macro `?fields`
* Simbol **`?fields`** bukan sintaks SQL standar, melainkan fitur crate `clickhouse`.
* Crate memeriksa struct `UserActivity`, membaca field-field-nya (`user_id`, `action`, `details`, `timestamp`), lalu secara otomatis melebarkan `?fields` menjadi daftar kolom tersebut saat runtime.
* Jika di masa depan Anda menambah kolom pada struct, query SQL ini otomatis sinkron tanpa perlu diubah!

### B. Turbofish `.fetch::<UserActivity>()?`
* Menghasilkan objek **`Cursor`** (stream reader asinkron yang membaca baris demi baris dari socket jaringan).

### C. Anatomi `while let Some(row) = cursor.next().await?`
Menggabungkan 4 operasi dalam satu baris:
1. `cursor.next()`: Mengambil baris data berikutnya (mengembalikan Future).
2. `.await`: Menunggu paket data baris tersebut dari server ClickHouse secara non-blocking.
3. Operator `?`: Memastikan jika ada error jaringan di tengah pembacaan, eksekusi langsung berhenti dan me-return error.
4. Pattern Matching `while let Some(row)`: 
   * Jika baris ada (`Some(row)`), baris dimasukkan ke list: `list.push(row);`.
   * Jika stream sudah habis terbaca seluruhnya, cursor mengembalikan `None`. Kondisi `while let` selesai dan loop berhenti secara otomatis.

> **💡 Komparasi Python:**
> Setara dengan async iteration di Python:
> ```python
> async for row in cursor:
>     list_data.append(row)
> ```

---

## 8. Quick Reference Card

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                      REPOSITORY & DATABASE CHEAT SHEET                           │
├────────────────────────────┬─────────────────────────────────────────────────────┤
│ Sintaks                    │ Arti & Fungsi                                       │
├────────────────────────────┼─────────────────────────────────────────────────────┤
│ #[async_trait]             │ Menyulap async trait jadi Pin<Box<dyn Future>> (safe│
│ Trait: Send + Sync         │ Jaminan aman dipindah & diakses bareng antar-thread │
│ pub fn new(...) -> Self    │ Konvensi konstruktor pabrik objek struct            │
│ query_as::<_, Struct>      │ Eksekusi SQLx & auto mapping ke struct via FromRow  │
│ r#" ... "#                 │ Raw multiline string literal tanpa escape \n        │
│ $1, $2 (PG) / ? (CH)       │ Parameter binding aman dari SQL Injection           │
│ .fetch_one()               │ Query RDBMS ekspektasi tepat 1 baris data           │
│ Result<Option<T>, E>       │ Double envelope (bedakan DB error vs data kosong)   │
│ ()                         │ Unit type (0 byte, pengganti void/None)             │
│ ?fields                    │ Auto-expand seluruh kolom struct di ClickHouse      │
│ while let Some(x) = ...    │ Loop konsumsi data stream asinkron hingga None      │
└────────────────────────────┴─────────────────────────────────────────────────────┘
```

---

## 9. Kaitan dengan Arsitektur Codebase `braincode-beV2`

1. **Dependency Inversion Principle (DIP)**:
   * Layer Service nantinya di [`src/services/`](../../src/services/) tidak akan memegang `PostgresUserRepository` secara langsung, melainkan bergantung pada pointer abstraksi `Arc<dyn UserRepository>`. Ini dimungkinkan berkat macro `#[async_trait]` dan bound `Send + Sync`.
2. **Dual-Database Strategy**:
   * **PostgreSQL (OLTP)**: Menangani integritas relasional data pengguna via transaksi dan query SQLx ber-placeholder `$1, $2`.
   * **ClickHouse (OLAP)**: Menangani jutaan event log analitik berkecepatan tinggi via binary insert stream dan cursor stream reader.
