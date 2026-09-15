# Deep Dive: Anatomi Rust Syntax & Logic pada `src/utils/errors.rs`

Dokumentasi ini merangkum sesi *micro-learning deep dive* yang membedah setiap elemen sintaksis, kata kunci, makro, dan logika penanganan kesalahan (*error handling*) pada file [`src/utils/errors.rs`](../../src/utils/errors.rs).

---

## Daftar Topik yang Dibedah

1. **Kluster 1 — Bentuk Data (Data Definition & Encapsulation):**
   - Kata kunci `pub` (Visibility)
   - `struct` vs `enum` (Product Type vs Sum Type)
   - Konsep *Wrapper Tipe Data* (Tuple Variants)
2. **Kluster 2 — Perilaku & Kontrak (Traits & Implementation):**
   - Apa itu `Trait` (`IntoResponse`)
   - Pasangan sintaks `impl ... for ...`
   - Manajemen Memori: `self` (Consume/Move) vs `&self` (Borrowing)
3. **Kluster 3 — Meta-Programming & Attributes (Macros):**
   - Macro Deklaratif (`!`) vs Macro Derive (`#[derive(...)]`)
   - `#[derive(Error, Debug)]`
   - `#[allow(dead_code)]`
   - Sihir library `thiserror`: `#[error("...")]` dan `#[from]` (Auto-conversion)
   - Macro `json!({ ... })`
4. **Bedah Logika Fungsi `into_response(self) -> Response`:**
   - Kata kunci `fn`
   - Perbedaan Panah Ramping `->` vs Panah Tebal `=>`
   - Sintaks `AppError::NotFound(msg)` & variabel pengikat (`msg`, `err`)
   - Pola Keamanan: Logging teknis internal vs Sanitasi pesan publik ke client

---

## Kluster 1: Bentuk Data (Data Definition & Encapsulation)

### 1. `pub` (Visibility Modifier)
* **Penjelasan Teoretis:** Secara default, semua elemen di Rust (fungsi, struct, enum, field) bersifat **private** (hanya bisa diakses di dalam modul/file tempat ia dideklarasikan). Kata kunci `pub` mengekspos elemen tersebut agar bisa diimpor dan digunakan oleh file lain (misalnya layer handler API atau service).
* **Analogi:** Seperti jendela loket pelayanan restoran. Dapur di belakang tertutup rapat (private), tetapi jendela bertuliskan *"Ambil Pesanan"* diberi label `pub` sehingga pelanggan luar bisa berinteraksi.
* **Komparasi Python:** Di Python, privatisasi hanya konvensi tanda underscore (`_fungsi_internal()`). Di Rust, compiler memblokir keras upaya akses jika tidak ada label `pub`.

### 2. `struct` vs `enum` (Product Type vs Sum Type)
* **Penjelasan Teoretis:**
  * **`struct` (Product Type / "DAN"):** Menggabungkan beberapa field sekaligus. Nilainya memiliki Field A **DAN** Field B.
  * **`enum` (Sum Type / Tagged Union / "ATAU"):** Menyatakan bahwa sebuah nilai hanya bisa berupa **salah satu** kemungkinan dari daftar varian yang didefinisikan. Nilainya adalah Varian A **ATAU** Varian B **ATAU** Varian C.
* **Analogi:**
  * `struct` = **KTP** (Wajib memiliki NIK, Nama, dan Alamat sekaligus dalam satu kartu).
  * `enum` = **Lampu Lalu Lintas** (Bisa Merah, Kuning, atau Hijau; tidak mungkin menyala Merah dan Hijau secara bersamaan).
* **Komparasi Python:** `Enum` di Python biasanya hanya label konstanta angka/string sederhana (`Status.PENDING = 1`). Di Rust, enum adalah *Algebraic Data Type* di mana tiap varian bisa membawa muatan data yang berbeda-beda.

### 3. Wrapper Tipe Data (`DatabaseError(sqlx::Error)`, `ValidationError(String)`)
* **Penjelasan Teoretis:** Bentuk varian bertanda kurung disebut **Tuple Struct Variant**. Varian ini membungkus tipe data eksternal ke dalam payung `AppError`. Rust mewajibkan sebuah fungsi hanya mengembalikan satu tipe error (`Result<T, AppError>`). Dengan membungkus error database, Clickhouse, dan validasi ke dalam satu enum `AppError`, semua error dari berbagai sumber bisa disatukan ke dalam satu tipe terpadu.
* **Analogi:** Kardus standar pengiriman paket kargo. Baik isinya laptop (`sqlx::Error`) maupun dokumen kertas (`String`), semuanya dimasukkan ke dalam kardus berlabel resmi `AppError` agar kurir pengantar hanya perlu membawa satu format kardus standar.

---

## Kluster 2: Perilaku & Kontrak (Traits & Implementation)

### 1. Apa itu `Trait`?
* **Penjelasan Teoretis:** Trait adalah kontrak perilaku (*behavioral contract*) atau antarmuka (*interface*). Trait mendefinisikan kumpulan method yang harus dimiliki oleh suatu tipe data. Pada framework Axum, trait `IntoResponse` mensyaratkan method `into_response(self) -> Response`.
* **Analogi:** **Sertifikat SIM Mengemudi**. Tidak peduli kamu mengendarai truk, sedan, atau motor (tipe data berbeda), selama kamu lulus uji mengemudi (mengimplementasikan Trait SIM), polisi jalan raya (Web Server Axum) mengizinkanmu melintas di jalan raya (di-return sebagai HTTP response).
* **Komparasi Python:** Mirip dengan `Protocol` pada modul `typing` atau `abc.ABC` dengan `@abstractmethod`. Bedanya, Rust memverifikasi pemenuhan kontrak ini secara ketat saat kompilasi (*compile-time check*).

### 2. Pasangan Sintaks `impl ... for ...`
* **Penjelasan Teoretis:** Rust memisahkan secara tegas antara **definisi data** (`enum AppError`) dengan **perilaku/metode** (`impl IntoResponse for AppError`). Sintaks `impl [Trait] for [Type]` berarti kita menempelkan kontrak perilaku `IntoResponse` secara spesifik pada tipe data `AppError`.
* **Analogi:** Membeli rangka mobil (`enum AppError`), lalu di bengkel terpisah mekanik memasang modul GPS (`impl GPS for Mobil`). Penambahan fitur dilakukan secara modular tanpa membongkar desain rangka mobil.
* **Komparasi Python:** Di Python, data dan method disatukan dalam satu blok: `class AppError(IntoResponse): def into_response(self): ...`. Di Rust, keduanya terpisah demi modularitas dan fleksibilitas.

### 3. `self` vs `&self` (Ownership vs Borrowing)
* **Penjelasan Teoretis:**
  * **`self` (Milik Penuh / Move / Consume):** Method mengambil alih kepemilikan objek. Ketika fungsi selesai, nilai `self` akan langsung dihancurkan (*dropped*) dari memori. Dipakai pada `fn into_response(self)` karena instance error dikonsumsi habis untuk dilebur menjadi objek baru bertipe `Response`.
  * **`&self` (Pinjam Baca / Read-Only Borrow):** Method hanya meminjam objek untuk membaca isinya tanpa memindahkan kepemilikan. Dipakai pada `match &self` agar kita bisa membaca varian error tanpa merusak data sebelum seluruh respon selesai dirakit.
* **Analogi:**
  * `&self` = **Memperlihatkan KTP ke resepsionis**. Resepsionis hanya membaca nama dan mencatatnya, KTP tetap ada di tanganmu.
  * `self` = **Menyerahkan tiket bioskop ke petugas pintu**. Tiket dirobek dan disita (dikonsumsi habis). Tiket tidak bisa kamu pakai lagi karena sudah ditukar dengan hak masuk studio.
* **Komparasi Python:** Di Python, semua method menerima argumen `self` yang selalu berupa referensi objek yang dikelola oleh *Garbage Collector*. Di Rust, karena tidak ada GC, pemrogram harus menentukan secara eksplisit apakah data dipinjam (`&self`) atau dikonsumsi habis (`self`).

---

## Kluster 3: Meta-Programming & Attributes (Macros)

### 1. Macro Deklaratif vs Macro Derive
* **Penjelasan Teoretis:** Macro adalah teknik *metaprogramming* (kode yang menulis kode lain) yang dijalankan oleh compiler saat tahap kompilasi (*zero runtime overhead*).
  * **Macro Deklaratif (`!`):** Seperti `json!(...)` atau `println!(...)`. Bekerja seperti template ekspansi pola kode.
  * **Macro Derive (`#[derive(...)]`):** Diletakkan di atas struct/enum untuk menginstruksikan compiler menuliskan implementasi trait standar secara otomatis di balik layar.
* **Analogi:**
  * Macro Deklaratif (`json!`) = **Cap Stempel Kilat**. Sekali tekan, seluruh format teks langsung tercetak rapi.
  * Macro Derive (`#[derive]`) = **Staf Asisten Magang Otomatis**. Alih-alih kamu mengetik puluhan baris kode boilerplate untuk memformat teks debug atau implementasi error, asisten magang yang mengetikkannya untukmu sebelum kode diserahkan ke compiler.
* **Komparasi Python:** Mirip dengan Decorator `@dataclass` di Python. Bedanya, decorator Python berjalan saat *runtime*, sedangkan macro Rust bekerja saat *compile-time*.

### 2. `#[derive(Error, Debug)]`
* **`Debug` (Bawaan Standar Rust):** Mengizinkan tipe data dicetak untuk keperluan logging/inspeksi menggunakan format `{:?}` (seperti pada baris `tracing::error!("Database Error: {:?}", err)`).
* **`Error` (Dari crate `thiserror`):** Secara otomatis mengimplementasikan trait standar Rust `std::error::Error`. Tanpa ini, enum kita hanyalah struktur data biasa, bukan objek error resmi.

### 3. `#[allow(dead_code)]`
* **Penjelasan Teoretis:** Compiler directive untuk mematikan peringatan (*warning*) linter jika ada varian enum yang dideklarasikan namun belum sempat dipanggil/digunakan di dalam codebase.
* **Analogi:** Memasang label *"Peralatan Siaga — Jangan Ditegur"* di gudang agar petugas inspeksi kebersihan pabrik (compiler) tidak menegurmu atas adanya barang cadangan yang belum dipakai di rak.
* **Komparasi Python:** Sama seperti komentar pengabaian linter di Python: `# noqa` atau `# pylint: disable=unused-variable`.

### 4. `#[error("...")]` dan `#[from]` (`thiserror`)
* **`#[error("Database error: {0}")]`:** Mengimplementasikan trait `Display` secara otomatis. Menentukan format pesan teks yang mudah dibaca manusia saat error dicetak. `{0}` merujuk pada isi data pertama di dalam varian tersebut.
* **`#[from] sqlx::Error`:** Fitur paling sakti dari `thiserror`. Atribut ini otomatis membuatkan implementasi `impl From<sqlx::Error> for AppError`.
  * **Dampak Praktis:** Setiap kali ada query SQL yang gagal mengembalikan `sqlx::Error`, kita cukup menambahkan operator tanda tanya **`?`** di ujung baris:
    ```rust
    let user = sqlx::query_as(...).fetch_one(&pool).await?;
    ```
    Rust akan **secara otomatis mengonversinya** menjadi `AppError::DatabaseError` tanpa perlu blok `try-catch` atau *error mapping* manual!
* **Analogi `#[from]`:** Steker adapter colokan universal otomatis. Colokan kaki tiga UK (`sqlx::Error`) langsung pas dicolokkan ke stopkontak kaki dua EU (`AppError`) begitu kabel dicolokkan (`?`).

### 5. Macro Deklaratif `json!({ ... })`
* **Penjelasan Teoretis:** Disediakan oleh crate `serde_json`. Memungkinkan programmer menulis struktur data JSON dengan gaya yang sangat mirip JavaScript Object Literal atau Python Dictionary langsung di tengah kode Rust, lalu mengubahnya menjadi pohon struktur `serde_json::Value`.
* **Analogi:** Cetakan silikon kue. Cukup tuang adonan, cetakan langsung mencetak bentuk JSON yang diinginkan tanpa perlu mengukir sudutnya secara manual dengan struct kaku.

---

## Bedah Logika Fungsi `into_response(self) -> Response`

Kode lengkap dari baris 28–50 file `src/utils/errors.rs`:

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::DatabaseError(err) => {
                tracing::error!("Database Error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada database.".to_string())
            }
            AppError::ClickhouseError(err) => {
                tracing::error!("Clickhouse error {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada analitik Clickhouse.".to_string())
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

### 1. Makna Simbol Panah (`->` vs `=>`)
* **`fn into_response(self) -> Response`**:
  * `fn`: Kata kunci deklarasi fungsi (setara `def` di Python).
  * `->` (*Thin Arrow*): Menunjukkan **Return Type** (fungsi ini menghasilkan nilai bertipe `Response`).
* **`AppError::NotFound(msg) => ...`**:
  * `=>` (*Fat Arrow*): Dipakai di dalam blok `match` untuk memisahkan antara **Pola Kondisi (Kiri)** dengan **Hasil/Aksi yang Dijalankan (Kanan)**. Format: `[Pola] => [Aksi],`.

### 2. Sintaks `AppError::NotFound(msg)` & Penamaan Variabel
* **Akses Namespace (`::`):** Mengakses varian `NotFound` di dalam enum `AppError`.
* **Destructuring `(msg)`:** Membuka bungkus varian dan menangkap data string di dalamnya ke dalam variabel lokal baru bernama `msg`.
* **Apakah harus `msg` dan `err`?**
  * **TIDAK HARUS.** Itu bukan kata kunci bawaan Rust, melainkan nama variabel bebas yang dipilih programmer (`msg` untuk pesan string, `err` untuk objek error teknis).
  * Kamu bebas menamainya `(pesan)`, `(e)`, atau mengabaikannya dengan tanda garis bawah `(_)` jika datanya tidak ingin digunakan.

### 3. Pola Keamanan: Sensor Data Sensitif Database
Perhatikan perbedaan perlakuan pada cabang error:
* **Error Klien (`ValidationError`, `NotFound`):**
  Pesan langsung dikembalikan ke user via `msg.clone()` dengan status 400 atau 404 karena user memang perlu tahu apa yang salah dengan input mereka.
* **Error Infrastruktur (`DatabaseError`, `ClickhouseError`):**
  1. Detail teknis (nama tabel, sintaks query, kredensial koneksi) **dicatat ke log rahasia internal**: `tracing::error!("Database Error: {:?}", err);`.
  2. Kepada klien publik di luar, pesan teknis tersebut **disensor** dan diganti dengan pesan umum yang aman: `"Terjadi kesalahan pada database."` dengan status 500. Ini mencegah kebocoran informasi arsitektur internal ke pihak luar/penyerang.

### 4. Merakit Body & Respon Akhir
* `let body = Json(...)`: Membungkus payload JSON dan otomatis menyematkan header HTTP `Content-Type: application/json`.
* `(status, body).into_response()`: Axum secara bawaan mendukung konversi tuple `(StatusCode, T)` menjadi HTTP Response lengkap, sehingga kombinasi status dan body JSON langsung diubah menjadi objek `Response`.

---

## Quick Reference Card

| Sintaks / Simbol | Peran & Kategori | Padanan / Catatan di Python |
|---|---|---|
| `pub` | Visibility Modifier | Tidak ada (Python memakai konvensi `_`) |
| `enum` | Algebraic Data Type (Sum Type) | Jauh lebih kuat dari `enum.Enum` Python |
| `impl Trait for Type` | Implementasi Interface Modular | Mirip mewarisi class `ABC`, tapi terpisah dari data |
| `self` | Mengambil kepemilikan (Consume/Move) | Python `self` selalu referensi heap dengan GC |
| `&self` | Meminjam data (Read-Only Borrow) | Membaca atribut tanpa merusak objek |
| `fn` | Keyword deklarasi fungsi | Setara `def` di Python |
| `->` | Menentukan tipe data kembalian (Return type) | Setara Type Hint `def foo() -> int:` |
| `=>` | Pemisah pola dan aksi di blok `match` | Setara `case Pattern:` di Python 3.10+ |
| `#[derive(...)]` | Macro Prosedural (Generate boilerplate) | Mirip decorator `@dataclass` |
| `#[from]` | Auto-conversion error untuk operator `?` | Menghilangkan kebutuhan blok `try-except ... raise from` |
| `#[allow(dead_code)]` | Compiler directive pengabaian warning | Setara `# noqa` atau `# pylint: disable` |
| `json!({ ... })` | Macro Deklaratif pembuatan JSON dinamis | Menulis dictionary literal `{"key": val}` di Python |

---

## Kaitan dengan Arsitektur Codebase `braincode-beV2`

* File: [`src/utils/errors.rs`](../../src/utils/errors.rs)
* Konsumen:
  * [`src/repositories/`](../../src/repositories/) mengembalikan `Result<T, AppError>` memanfaatkan auto-convert `#[from] sqlx::Error`.
  * [`src/services/`](../../src/services/) mengembalikan `Result<T, AppError>` untuk validasi bisnis.
  * [`src/handlers/`](../../src/handlers/) mengembalikan `Result<Json<T>, AppError>`. Ketika handler menghasilkan `Err(AppError)`, Axum otomatis memanggil `into_response(self)` untuk mengirimkan JSON error standar ke browser/aplikasi klien.
