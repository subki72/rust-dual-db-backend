# Deep Dive: Anatomi Rust Syntax Service Layer, Error Mapping (`.map_err()`), Timestamp Chaining, & Resilient Side-Effect Logging

Dokumentasi ini merangkum sesi *micro-learning deep dive* yang membedah sintaksis, manajemen kepemilikan (*ownership*), penanganan error adaptif (*error mapping & propagation*), kalkulasi waktu sistem (*system time duration*), dan ketahanan logika bisnis (*resilient side-effect execution*) pada layer Service ([`src/services/`](../../src/services/)).

---

## Daftar Topik yang Dibedah

1. **Anatomi Method Signature Service (`&self`, Ownership `req`, dan DTO Contract)**
2. **Fail-Fast Validation & Karakter Primitif (`.contains('@')`, `char`, dan `.into()`)**
3. **Misteri Kerentanan Waktu Sistem & Transformasi Error (`duration_since`, `.map_err()`, `?`, dan `.as_secs()`)**
4. **Resilient Side-Effect Logging (`if let Err` vs Operator `?`)**
5. **Pipeline Iterator Fungsional (`.into_iter().map(UserResponse::from).collect()`)**
6. **Quick Reference Card**
7. **Kaitan dengan Arsitektur Codebase `braincode-beV2`**

---

## 1. Anatomi Method Signature Service

Di file [`src/services/traits/user_service.rs`](../../src/services/traits/user_service.rs):

```rust
async fn register_user(&self, req: CreateUserRequest) -> Result<UserResponse, AppError>;
```

### Bedah Komponen:
* **`async`**:
  * Menandakan fungsi asinkron non-blocking yang mengembalikan `Future`. Bekerja sama dengan macro `#[async_trait]` agar berstatus *Object-Safe* untuk pointer `Arc<dyn UserService>`.
* **`&self` (Borrowing Immutable)**:
  * Meminjam instance struct service secara *read-only*.
  * **Mengapa bukan `&mut self`?** Karena layer service bersifat *stateless* (hanya memegang referensi pointer `Arc` ke repository). Meminjam secara immutable `&self` memungkinkan ratusan thread Tokio mengeksekusi method ini secara paralel tanpa saling mengunci (*lock-free*).
* **`req: CreateUserRequest` (Ownership Transfer / By-Value)**:
  * Perhatikan: **tidak ada tanda `&` di depan tipe `CreateUserRequest`**.
  * Artinya fungsi ini mengambil alih kepemilikan penuh (*takes ownership*) atas objek request. Fungsi berhak memecah dan memindahkan field string `req.name` dan `req.email` langsung ke entity database tanpa perlu menduplikasinya di memori (*zero-cost transfer*).
* **`-> Result<UserResponse, AppError>`**:
  * Mengembalikan data DTO aman `UserResponse` (hanya field publik: `id`, `name`, `email`) jika sukses.
  * Mengembalikan enum `AppError` jika gagal (yang otomatis dipetakan Axum menjadi status code HTTP).
* **Titik Koma (`;`) di Akhir**:
  * Menandakan method signature di dalam `trait` (kontrak antarmuka) tanpa blok body `{ ... }`.

### 💡 Komparasi Python:
```python
async def register_user(self, req: CreateUserRequest) -> UserResponse:
    pass
```

---

## 2. Fail-Fast Validation & Karakter Primitif (`.contains('@')`)

Di file [`src/services/impls/user_service_impl.rs`](../../src/services/impls/user_service_impl.rs):

```rust
if !req.email.contains('@') {
    return Err(AppError::ValidationError("Format email tidak valid.".into()));
}
```

### Bedah Logika & Sistem Tipe:
* **`req.email`**: Mengakses field `email` yang bertipe `String`.
* **`.contains('@')`**:
  * Method bawaan tipe string Rust untuk memeriksa keberadaan karakter atau substring.
  * **Perbedaan Petik Tunggal vs Petik Dua:**
    * `'@'` (petik tunggal) bertipe **`char`**: karakter Unicode bernilai 4-byte.
    * `"@"` (petik dua) bertipe **`&str`**: string slice.
    * Rust mengizinkan pencarian menggunakan karakter `char` tunggal yang sangat cepat secara komputasi.
* **Tanda Seru `!` (Operator NOT)**:
  * Membalik kondisi boolean (*jika tidak mengandung `@`*).
* **`return Err(...)`**:
  * Menghentikan fungsi seketika (*early exit / fail-fast*) sebelum menghabiskan resource server ke database.
* **`"Format email tidak valid.".into()`**:
  * String literal `"..."` bertipe `&str`.
  * Varian enum `ValidationError(String)` membutuhkan tipe `String` heap-allocated.
  * Method **`.into()`** (dari trait bawaan `Into<String>`) otomatis mengubah `&str` menjadi `String` secara rapi dan idiomatis menggantikan `.to_string()`.

### 💡 Komparasi Python:
```python
if "@" not in req.email:
    raise ValidationError("Format email tidak valid.")
```

---

## 3. Kerentanan Waktu Sistem & Transformasi Error (`.map_err()`)

Perhatikan baris komputasi timestamp Unix berikut:

```rust
let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|e| AppError::InternalError(e.to_string()))?
    .as_secs();
```

Ini adalah contoh klasik rantai method (*method chaining*) di Rust yang sarat perhitungan keamanan tingkat rendah:

### A. Mengapa `.duration_since(UNIX_EPOCH)` Mengembalikan `Result`?
* `SystemTime::now()` membaca jam sistem operasi (*OS Clock*).
* `UNIX_EPOCH` merepresentasikan waktu 1 Januari 1970 00:00:00 UTC.
* **Penyebab Kemungkinan Gagal (`SystemTimeError`)**:
  Secara teoretis, jam komputer/server bisa dimundurkan oleh sinkronisasi NTP yang salah atau user yang mengubah jam OS ke masa sebelum tahun 1970 (misal tahun 1965).
  Jika waktu saat ini lebih lampau dari 1970, selisih waktu akan bernilai negatif. Rust menolak menghasilkan durasi negatif dan mengembalikan `Result<Duration, SystemTimeError>`.

### B. Mengapa Wajib Memakai `.map_err()`?
* Tanda tangan fungsi `register_user` menuntut return type `Result<..., AppError>`.
* Error asli dari Rust bertipe `std::time::SystemTimeError`. Rust **sangat ketat pada sistem tipe**: Anda tidak bisa menggunakan operator `?` pada tipe error yang tidak dikenal oleh fungsi pemanggil.
* **`.map_err(...)`** bertugas mengubah tipe error internal tanpa mengubah data suksesnya:
  * **`|e|`**: Closure yang menerima objek `SystemTimeError`.
  * **`e.to_string()`**: Mengonversi pesan error teknis menjadi `String`.
  * **`AppError::InternalError(...)`**: Membungkus teks tersebut ke dalam varian error server aplikasi kita.
* Setelah baris `.map_err()`, tipe datanya berubah dari `Result<Duration, SystemTimeError>` menjadi **`Result<Duration, AppError>`**.

### C. Operator `?` & `.as_secs()`
* **Operator `?`**: Karena errornya sudah bertipe `AppError`, tanda `?` bisa membongkar `Duration` murni (atau langsung *early return* jika error).
* **`.as_secs()`**: Method struct `Duration` yang mengekstrak total waktu dalam satuan **detik integer bulat (`u64`)**. Angka ini cocok 1:1 dengan kolom `timestamp: UInt64` di ClickHouse.

```text
SystemTime::now().duration_since(UNIX_EPOCH)
    │  [Tipe: Result<Duration, SystemTimeError>]
    ▼
.map_err(|e| AppError::InternalError(e.to_string()))
    │  [Tipe: Result<Duration, AppError>]   <-- Tipe error serasi dengan fungsi
    ▼
?
    │  [Tipe: Duration]                     <-- Unpack nilai sukses
    ▼
.as_secs()
    │  [Tipe: u64]                          <-- Angka detik integer (misal: 1773321123)
```

---

## 4. Resilient Side-Effect Logging (`if let Err` vs Operator `?`)

Perhatikan blok penulisan log analitik ke ClickHouse:

```rust
// Simpan log analitik tanpa menggagalkan flow utama jika error
if let Err(err) = self.activity_repo.record(&activity).await {
    tracing::warn!("Gagal mencatat analitik ClickHouse: {:?}", err);
}
```

### Mengapa TIDAK menggunakan tanda `?` di sini?
* **PostgreSQL (OLTP)** telah sukses menyimpan data pengguna baru. User sudah sah terdaftar di sistem.
* **ClickHouse (OLAP)** adalah sistem analitik sekunder (*side-effect*).
* Jika server ClickHouse sedang mengalami gangguan jaringan, *restart*, atau *timeout*, **kita TIDAK BOLEH membatalkan pendaftaran user**.
* Jika kita menulis:
  ```rust
  self.activity_repo.record(&activity).await?; // BAHAYA!
  ```
  Maka setiap kali ClickHouse bermasalah, user akan melihat pesan *"Registration Failed"* padahal email mereka sudah tersimpan di PostgreSQL (memicu inkonsistensi data bisnis).
* Dengan konstruksi **`if let Err(err)`**, kegagalan ClickHouse ditangkap secara anggun, dicatat ke server log sebagai peringatan (`tracing::warn!`), dan eksekusi tetap berlanjut mengembalikan `Ok(UserResponse)`.

---

## 5. Pipeline Iterator Fungsional (`.into_iter().map().collect()`)

Di method `get_all_users`:

```rust
async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError> {
    let users = self.user_repo.find_all().await?;
    Ok(users.into_iter().map(UserResponse::from).collect())
}
```

### Bedah Pipeline:
1. **`users.into_iter()`**:
   * Mengonsumsi `Vec<User>` dan mengubahnya menjadi *consuming iterator* (mengambil alih kepemilikan seluruh elemen).
2. **`.map(UserResponse::from)` (Point-Free Function Pointer)**:
   * Alih-alih menulis closure manual `.map(|u| UserResponse::from(u))`, Rust mengizinkan penulisan ringkas langsung nama fungsinya: `UserResponse::from`.
   * Fungsi ini otomatis dieksekusi untuk setiap elemen data.
3. **`.collect()`**:
   * Mengumpulkan kembali stream iterator menjadi koleksi baru bertipe `Vec<UserResponse>`.
4. **`Ok(...)`**:
   * Membungkus vektor baru tersebut ke dalam varian sukses `Result`.

### 💡 Komparasi Python (List Comprehension):
```python
users = await self.user_repo.find_all()
return [UserResponse.from_user(u) for u in users]
```

---

## 6. Quick Reference Card

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                         SERVICE LAYER CHEAT SHEET                                │
├────────────────────────────┬─────────────────────────────────────────────────────┤
│ Sintaks                    │ Arti & Kegunaan                                     │
├────────────────────────────┼─────────────────────────────────────────────────────┤
│ &self                      │ Immutable borrow, thread-safe untuk multi-threading │
│ req: CreateUserRequest     │ By-value ownership transfer (zero-copy DTO intake)  │
│ .contains('@')             │ Pencarian karakter tipe char tunggal (')            │
│ "teks".into()              │ Konversi idiomatis &str ke String (via Into trait)  │
│ .map_err(|e| ...)          │ Mengubah tipe error internal Result ke AppError     │
│ ?                          │ Error propagation operator (early exit on Err)      │
│ .as_secs()                 │ Konversi Duration ke integer detik bulat (u64)      │
│ if let Err(e) = ...        │ Tangkap error side-effect tanpa menggagalkan flow   │
│ .into_iter().map().collect │ Pipeline transformasi data fungsional               │
└────────────────────────────┴─────────────────────────────────────────────────────┘
```

---

## 7. Kaitan dengan Arsitektur Codebase `braincode-beV2`

* **Clean Separation of Concerns**:
  * Handler di [`src/handlers/user_handler.rs`](../../src/handlers/user_handler.rs) hanya bertugas memanggil `user_service.register_user(payload)`.
  * Seluruh aturan validasi bisnis, sinkronisasi dua database, pengamanan password/data sensitif, dan kalkulasi timestamp terisolasi sepenuhnya di dalam [`src/services/impls/user_service_impl.rs`](../../src/services/impls/user_service_impl.rs).
