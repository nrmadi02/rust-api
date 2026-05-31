# 📋 Task Tracker — Task Tools API

> **Project**: File Conversion & Processing API (PDF ↔ Word, Image ↔ PDF, Compress, Merge, dll.)
> **Stack**: Rust · Axum · PostgreSQL · SQLx · JWT
> **Last Updated**: 2026-05-31

---

## 🏗️ Project Roadmap Overview

```
✅ Auth Group      → MVP 1 (PDF→Word) → MVP 2 (More Conversions)
                  → MVP 3 (Advanced Tools) → MVP 4 (Polish & Scale)
```

---

## ✅ Auth Group — SELESAI

> Semua fitur autentikasi dasar sudah selesai diimplementasi.

- **Register** — Endpoint registrasi user baru dengan validasi input & hashing password (Argon2)
- **Login** — Endpoint login dengan JWT access token + refresh token
- **Logout** — Invalidasi refresh token dari database
- **Rate Limiting** — Proteksi endpoint auth dari brute-force (tower_governor)
- **Login Attempt Tracking** — Pencatatan percobaan login (berhasil/gagal) di DB
- **Middleware Auth Guard** — Middleware validasi JWT untuk route yang dilindungi
- **OpenAPI / Scalar Docs** — Dokumentasi API interaktif via utoipa + utoipa-scalar
- **Database Migrations** — Setup migrasi PostgreSQL via SQLx

---

## 🚀 MVP 1 — PDF to Word Conversion

> **Goal**: Implementasi fitur konversi PDF → Word (`.docx`) dengan performa terbaik,
> dilengkapi activity logs, draft system, dan test coverage yang solid.
>
> **Engine**: LibreOffice Headless via `unoserver` (daemon mode) + Async Job Queue

### 📦 1.1 — Setup & Dependencies

- **unoserver — Podman Container (local dev)** ✅ SELESAI
  - Image custom dibuild: `localhost/task-tools-unoserver:latest`
  - Dockerfile: `docker/unoserver/Dockerfile` (Debian bookworm + LibreOffice 7.4 + unoserver 3.6)
  - Jalankan container: `podman run -d --name task-tools-unoserver -p 2003:2003 task-tools-unoserver:latest`
  - Bisa dikelola via **Podman Desktop**
- **unoconvert — client di host** ✅ SELESAI
  - Install via pipx: `pipx install unoserver` → binary `unoconvert` tersedia di PATH
  - Berkomunikasi ke container via TCP port `2003`
  - Mode **stdin/stdout** (file tidak perlu di-mount ke container):
    ```bash
    cat input.pdf | unoconvert \
      --host 127.0.0.1 --port 2003 \
      --convert-to docx \
      --input-filter "writer_pdf_import" \
      - - > output.docx
    ```
- **Konfigurasi `.env` untuk unoserver** ✅ SELESAI
  - Tambah variabel:
    ```env
    UNOSERVER_HOST=127.0.0.1
    UNOSERVER_PORT=2003
    UNOSERVER_TIMEOUT_SECS=60
    ```
  - Strategi fallback: jika unoserver tidak tersedia → return error `503 Service Unavailable`
- **Tambah crate `lopdf`** ke `Cargo.toml` ✅ SELESAI
  - Digunakan untuk operasi non-konversi: deteksi kompleksitas PDF, validasi magic bytes
  - Bukan sebagai engine konversi utama
- **Setup storage lokal** ✅ SELESAI
  - Buat abstraksi `StorageRepository` trait
  - Implementasi `LocalStorageRepository` untuk dev
  - Struktur folder: `uploads/{user_id}/{job_id}/input.pdf` & `outputs/{job_id}/output.docx`
  - Tambah config di `.env`: `STORAGE_BASE_PATH`, `MAX_UPLOAD_SIZE_MB`, `FILE_TTL_HOURS`
- **Database migration — file jobs** ✅ SELESAI
  - Buat tabel `conversion_jobs`:
    ```sql
    id UUID PK, user_id UUID FK, job_type VARCHAR,
    status VARCHAR (draft|queued|processing|done|failed),
    input_path TEXT, output_path TEXT,
    file_size_bytes BIGINT, duration_ms INT,
    error_message TEXT, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ
    ```
  - Buat tabel `activity_logs`:
    ```sql
    id UUID PK, user_id UUID FK, action VARCHAR,
    resource_type VARCHAR, resource_id UUID,
    ip_address VARCHAR, user_agent TEXT,
    metadata JSONB, created_at TIMESTAMPTZ
    ```

---

### 📂 1.2 — Domain Layer

- **Entity `ConversionJob`** ✅ SELESAI
  - Buat struct di `src/domain/conversion_job.rs`
  - Fields: `id`, `user_id`, `job_type` (enum: PdfToWord, WordToPdf, dll.), `status` (enum: Draft, Processing, Done, Failed), `input_file`, `output_file`, `created_at`, `updated_at`
- **Entity `ActivityLog`** ✅ SELESAI
  - Buat struct di `src/domain/activity_log.rs`
  - Fields: `id`, `user_id`, `action` (string), `resource_type`, `resource_id`, `ip_address`, `user_agent`, `created_at`
- **Repository Trait `ConversionJobRepository`**
  - `create_job()` — simpan job baru sebagai Draft
  - `find_by_id()` — ambil job by ID
  - `find_by_user()` — list semua job milik user (dengan pagination)
  - `update_status()` — update status job (Processing → Done / Failed)
  - `delete_draft()` — hapus job yang masih Draft
- **Repository Trait `ActivityLogRepository`**
  - `log_activity()` — insert log baru
  - `find_by_user()` — ambil history activity user (dengan pagination & filter)

---

### ⚙️ 1.3 — Application Layer (Use Cases)

- **Use Case: `UploadAndConvertPdfToWord`**
  - Validasi file: magic bytes check (`%PDF`), hanya `.pdf`, ukuran max 50MB
  - Simpan file ke storage dengan nama sanitized (`{job_id}_input.pdf`)
  - Buat `ConversionJob` dengan status `**draft**`
  - Catat ke `activity_logs` (action: `upload_pdf`)
  - Return `job_id` + status `draft` → **202 Accepted** (tidak tunggu konversi selesai)
- **Use Case: `EnqueueConversionJob`** *(dipanggil saat user confirm draft)*
  - Update status job: `draft` → `queued`
  - Spawn Tokio async task untuk proses konversi di background:
    ```
    queued → processing → done / failed
    ```
  - Di dalam task: panggil `UnoserverClient::convert()`, ukur durasi, update DB
  - Jika gagal: simpan `error_message`, update status ke `failed`
- **Use Case: `GetConversionJobStatus`**
  - Ambil status job by ID
  - Validasi ownership (user hanya bisa lihat job milik sendiri)
- **Use Case: `ListMyConversionJobs`**
  - Ambil semua job milik user yang login
  - Support pagination (`page`, `per_page`)
  - Filter by status (Draft, Done, Failed)
- **Use Case: `DeleteDraftJob`**
  - Hapus job yang masih berstatus Draft
  - Hapus file upload dari storage
  - Validasi ownership
- **Use Case: `DownloadConvertedFile`**
  - Validasi job sudah berstatus `Done`
  - Validasi ownership
  - Return file stream / presigned URL
  - Catat ke `activity_logs`

---

### 🗄️ 1.4 — Infrastructure Layer

- `**ConversionJobRepositoryImpl**` — implementasi SQLx query untuk semua method repository
- `**ActivityLogRepositoryImpl**` — implementasi SQLx query untuk activity log
- `**UnoserverClient**` — wrapper `tokio::process` untuk `unoconvert` binary
  - Method: `async fn convert(input_bytes: Bytes, format: ConvertFormat) -> Result<Bytes, ConvertError>`
  - Kirim file via **stdin**, terima hasil via **stdout** (tidak perlu tulis file ke disk dulu):
    ```rust
    // Contoh implementasi
    let mut child = Command::new("unoconvert")
        .args([
            "--host", &cfg.host,
            "--port", &cfg.port,
            "--convert-to", format.as_str(),
            "--input-filter", format.input_filter(),
            "-", "-",  // stdin → stdout
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Tulis input bytes ke stdin
    child.stdin.take().unwrap().write_all(&input_bytes).await?;

    // Baca output bytes dari stdout
    let output = child.wait_with_output().await?;
    ```
  - Timeout: `tokio::time::timeout(Duration::from_secs(cfg.timeout_secs), ...)`
  - Retry: 1x retry otomatis jika exit code non-zero
  - Log durasi ke `conversion_jobs.duration_ms`
- `**PdfValidatorService**` — validasi file sebelum konversi
  - Cek magic bytes: pastikan header `%PDF` (bukan hanya ekstensi)
  - Cek ukuran ≤ `MAX_UPLOAD_SIZE_MB`
  - Estimasi jumlah halaman via `lopdf` untuk logging
- `**ConversionWorker**` — Tokio async background worker
  - Spawn task saat `EnqueueConversionJob` dipanggil
  - Flow: `queued → processing → done / failed`
  - Update status & `duration_ms` di DB setelah selesai
  - Handle panic dengan `tokio::task::spawn` + error catch

---

### 🌐 1.5 — Presentation Layer (Handlers & Routes)

- **DTO: `UploadFileRequest`** — multipart form (file)
- **DTO: `ConversionJobResponse`** — response job detail (id, status, download_url, created_at)
- **DTO: `ListJobsResponse`** — paginated list of jobs
- **Handler: `POST /api/v1/convert/pdf-to-word`**
  - Terima upload multipart
  - Panggil use case `UploadAndConvertPdfToWord`
  - Return `ConversionJobResponse`
- **Handler: `GET /api/v1/convert/jobs`**
  - List semua job milik user yang login
  - Query params: `page`, `per_page`, `status`
- **Handler: `GET /api/v1/convert/jobs/:id`**
  - Ambil detail & status satu job
- **Handler: `GET /api/v1/convert/jobs/:id/download`**
  - Download file hasil konversi
- **Handler: `DELETE /api/v1/convert/jobs/:id`**
  - Hapus job (hanya jika Draft)
- **Daftarkan routes ke router** dengan auth guard middleware

---

### 📊 1.6 — Activity Logs Endpoint

- **Handler: `GET /api/v1/me/activity-logs`**
  - Ambil history aktivitas user yang sedang login
  - Query params: `page`, `per_page`, `action` (filter)
  - Return paginated activity log

---

### 📝 1.7 — Draft System

- **Flow Draft**: Upload file → simpan sebagai `Draft` → user konfirmasi → mulai konversi
  - Ini memungkinkan user membatalkan sebelum proses dimulai
  - Draft yang tidak dikonfirmasi bisa di-cleanup otomatis (via scheduled task / cron)
- **Handler: `POST /api/v1/convert/jobs/:id/confirm`**
  - Ubah status Draft → Processing, mulai konversi
  - (Alternatif: langsung proses tanpa confirm — pilih sesuai kebutuhan)

---

### 🧪 1.8 — Testing

- **Unit Test: `PdfConverterService`**
  - Test konversi file PDF valid → DOCX
  - Test handling file rusak / bukan PDF
  - Test timeout handling
- **Unit Test: Use Cases**
  - Test `UploadAndConvertPdfToWord` dengan mock repository
  - Test `GetConversionJobStatus` — ownership check
  - Test `DeleteDraftJob` — hanya bisa hapus Draft
- **Integration Test: Endpoints**
  - `POST /convert/pdf-to-word` — upload valid → expect job created
  - `GET /convert/jobs` — list jobs milik user
  - `GET /convert/jobs/:id/download` — download file done
  - Test auth guard: tanpa token → 401
- **Performance Test**
  - Ukur waktu konversi untuk berbagai ukuran file (1MB, 10MB, 50MB)
  - Catat benchmark di `docs/benchmarks.md`
- **Update OpenAPI docs** untuk semua endpoint baru

---

### 📄 1.9 — Dokumentasi & Finalisasi MVP 1

- Update `README.md` dengan cara setup & run
- Buat `docs/api-guide.md` — panduan penggunaan API konversi
- Review error handling — pastikan semua error return format yang konsisten
- Code review & refactor jika perlu

---

## 🔜 MVP 2 — Tambah Format Konversi

> **Goal**: Perluas kemampuan konversi dengan format-format populer lainnya.

- **Word → PDF** (`POST /api/v1/convert/word-to-pdf`)
  - Upload `.doc` / `.docx`, hasilkan `.pdf`
- **Image → PDF** (`POST /api/v1/convert/image-to-pdf`)
  - Upload satu atau banyak gambar (`.jpg`, `.jpeg`, `.png`)
  - Hasilkan satu file `.pdf`
  - Support pengaturan urutan halaman
- **PDF → Image** (`POST /api/v1/convert/pdf-to-image`)
  - Ekstrak setiap halaman PDF menjadi file gambar (`.png` / `.jpg`)
  - Return sebagai ZIP atau individual files
- **Extend `ConversionJob.job_type`** — tambah varian enum baru
- **Unit & integration test** untuk setiap format baru
- **Update OpenAPI docs**

---

## 🔜 MVP 3 — PDF Tools

> **Goal**: Fitur manipulasi PDF yang sering dibutuhkan.

- **Compress PDF** (`POST /api/v1/tools/compress-pdf`)
  - Kurangi ukuran file PDF
  - Pilihan level kompresi (low, medium, high)
  - Return info: ukuran sebelum vs sesudah
- **Merge PDF** (`POST /api/v1/tools/merge-pdf`)
  - Upload beberapa file PDF
  - Tentukan urutan halaman
  - Hasilkan satu PDF gabungan
- **Split PDF** (`POST /api/v1/tools/split-pdf`)
  - Pisah PDF berdasarkan range halaman
  - Return ZIP berisi file-file hasil split
- **PDF → Text Extract** (`POST /api/v1/tools/pdf-extract-text`)
  - Ekstrak teks mentah dari PDF
  - Berguna untuk indexing / OCR pipeline
- **Unit & integration test**
- **Update OpenAPI docs**

---

## 🔜 MVP 4 — Polish, Optimasi & Scale

> **Goal**: Siapkan untuk production — performa, keamanan, dan UX developer.

- **Async Job Queue**
  - Pindahkan proses konversi ke background task (tokio task / message queue)
  - WebSocket atau polling endpoint untuk real-time status update
- **File Cleanup**
  - Scheduled job hapus file lama (> 24 jam setelah download)
  - Hapus Draft yang belum dikonfirmasi > 1 jam
- **Storage Cloud**
  - Integrasi S3-compatible storage (AWS S3 / MinIO)
  - Presigned URL untuk download
- **Usage Quota per User**
  - Batasi jumlah konversi per hari / bulan
  - Endpoint `GET /api/v1/me/quota` untuk cek sisa kuota
- **Webhook Notification**
  - Kirim notifikasi ke URL user saat job selesai
  - `POST /api/v1/webhooks` — daftar webhook
- **Admin Panel API**
  - List semua user, jobs, activity logs (admin only)
  - Manual trigger cleanup
- **Security Audit**
  - Pastikan tidak ada path traversal pada file upload
  - Sanitasi nama file
  - Validasi magic bytes (bukan hanya ekstensi)
- **Load Testing** — pastikan API bisa handle concurrent requests
- **Docker & Docker Compose** setup untuk production
- **CI/CD Pipeline** — GitHub Actions untuk test & build

---

## 📌 Catatan & Keputusan Arsitektur


| Topik               | Keputusan                                                      |
| ------------------- | -------------------------------------------------------------- |
| **Konversi Engine** | ✅ `unoserver` (Podman container) + `unoconvert` (pipx, client) |
| **Mode Konversi**   | ✅ stdin → stdout pipe (tidak perlu file mount ke container)    |
| **Dev Setup**       | ✅ `podman run task-tools-unoserver:latest` port 2003           |
| **Prod Setup**      | `unoserver` di container Linux (sama, behavior identik)        |
| **Manipulasi PDF**  | `lopdf` — pure Rust (merge, split, compress, validasi)         |
| **Async Model**     | Upload → Draft → Enqueue → Tokio background task → Done        |
| **Storage**         | Local FS (dev) → S3-compatible / RustFS (prod, MVP 4)          |
| **Job Queue**       | PostgreSQL `SKIP LOCKED` (MVP 1) → dedicated queue (MVP 4)     |
| **Auth**            | JWT (access + refresh token) ✅                                 |
| **API Style**       | REST + OpenAPI via utoipa ✅                                    |
| **DB**              | PostgreSQL via SQLx ✅                                          |


---

## 🔗 Referensi

- [Axum docs](https://docs.rs/axum)
- [SQLx docs](https://docs.rs/sqlx)
- [utoipa docs](https://docs.rs/utoipa)
- [lopdf](https://docs.rs/lopdf) — PDF manipulation in Rust
- [unoserver GitHub](https://github.com/unoconv/unoserver) — LibreOffice daemon via Python
- [unoconvert CLI docs](https://github.com/unoconv/unoserver#usage) — CLI binary untuk konversi
- [LibreOffice headless](https://help.libreoffice.org/latest/en-US/text/shared/guide/converting_files.html) — referensi format yang didukung
- [tokio::process docs](https://docs.rs/tokio/latest/tokio/process/index.html) — async subprocess di Rust

