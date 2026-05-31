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

- [x] **Register** — Endpoint registrasi user baru dengan validasi input & hashing password (Argon2)
- [x] **Login** — Endpoint login dengan JWT access token + refresh token
- [x] **Refresh Token** — Rotasi refresh token untuk memperpanjang sesi
- [x] **Logout** — Invalidasi refresh token dari database
- [x] **Rate Limiting** — Proteksi endpoint auth dari brute-force (tower_governor)
- [x] **Login Attempt Tracking** — Pencatatan percobaan login (berhasil/gagal) di DB
- [x] **Middleware Auth Guard** — Middleware validasi JWT untuk route yang dilindungi
- [x] **OpenAPI / Scalar Docs** — Dokumentasi API interaktif via utoipa + utoipa-scalar
- [x] **Database Migrations** — Setup migrasi PostgreSQL via SQLx

---

## 🚀 MVP 1 — PDF to Word Conversion

> **Goal**: Implementasi fitur konversi PDF → Word (`.docx`) dengan performa terbaik,
> dilengkapi activity logs, draft system, dan test coverage yang solid.

### 📦 1.1 — Setup & Dependencies

- [ ] **Tambah dependency konversi PDF**
  - Evaluasi & pilih library: `lopdf`, `pdf-extract`, atau integrasi via CLI tool (`LibreOffice headless` / `pandoc`)
  - Tambah crate ke `Cargo.toml`
  - Buat konfigurasi timeout & max file size di `config/`

- [ ] **Setup storage lokal / cloud**
  - Buat abstraksi `StorageRepository` trait
  - Implementasi local file storage (untuk dev)
  - Tambah config path upload & output di `.env`

- [ ] **Database migration — file jobs**
  - Buat tabel `conversion_jobs` (id, user_id, status, input_path, output_path, created_at, updated_at)
  - Buat tabel `activity_logs` (id, user_id, action, resource_type, resource_id, metadata, created_at)

---

### 📂 1.2 — Domain Layer

- [ ] **Entity `ConversionJob`**
  - Buat struct di `src/domain/conversion_job.rs`
  - Fields: `id`, `user_id`, `job_type` (enum: PdfToWord, WordToPdf, dll.), `status` (enum: Draft, Processing, Done, Failed), `input_file`, `output_file`, `created_at`, `updated_at`

- [ ] **Entity `ActivityLog`**
  - Buat struct di `src/domain/activity_log.rs`
  - Fields: `id`, `user_id`, `action` (string), `resource_type`, `resource_id`, `ip_address`, `user_agent`, `created_at`

- [ ] **Repository Trait `ConversionJobRepository`**
  - `create_job()` — simpan job baru sebagai Draft
  - `find_by_id()` — ambil job by ID
  - `find_by_user()` — list semua job milik user (dengan pagination)
  - `update_status()` — update status job (Processing → Done / Failed)
  - `delete_draft()` — hapus job yang masih Draft

- [ ] **Repository Trait `ActivityLogRepository`**
  - `log_activity()` — insert log baru
  - `find_by_user()` — ambil history activity user (dengan pagination & filter)

---

### ⚙️ 1.3 — Application Layer (Use Cases)

- [ ] **Use Case: `UploadAndConvertPdfToWord`**
  - Validasi file: hanya `.pdf`, ukuran max (misal 50MB)
  - Simpan file upload ke storage
  - Buat `ConversionJob` dengan status `Draft`
  - Trigger proses konversi (sync atau async)
  - Update status job ke `Processing` → `Done` / `Failed`
  - Catat ke `activity_logs`
  - Return URL download file `.docx`

- [ ] **Use Case: `GetConversionJobStatus`**
  - Ambil status job by ID
  - Validasi ownership (user hanya bisa lihat job milik sendiri)

- [ ] **Use Case: `ListMyConversionJobs`**
  - Ambil semua job milik user yang login
  - Support pagination (`page`, `per_page`)
  - Filter by status (Draft, Done, Failed)

- [ ] **Use Case: `DeleteDraftJob`**
  - Hapus job yang masih berstatus Draft
  - Hapus file upload dari storage
  - Validasi ownership

- [ ] **Use Case: `DownloadConvertedFile`**
  - Validasi job sudah berstatus `Done`
  - Validasi ownership
  - Return file stream / presigned URL
  - Catat ke `activity_logs`

---

### 🗄️ 1.4 — Infrastructure Layer

- [ ] **`ConversionJobRepositoryImpl`** — implementasi SQLx query untuk semua method repository
- [ ] **`ActivityLogRepositoryImpl`** — implementasi SQLx query untuk activity log
- [ ] **`PdfConverterService`** — wrapper logic konversi PDF → DOCX
  - Pilih strategi: CLI subprocess (LibreOffice/pandoc) atau pure Rust library
  - Handle error & timeout
  - Log durasi konversi untuk monitoring performa

---

### 🌐 1.5 — Presentation Layer (Handlers & Routes)

- [ ] **DTO: `UploadFileRequest`** — multipart form (file)
- [ ] **DTO: `ConversionJobResponse`** — response job detail (id, status, download_url, created_at)
- [ ] **DTO: `ListJobsResponse`** — paginated list of jobs

- [ ] **Handler: `POST /api/v1/convert/pdf-to-word`**
  - Terima upload multipart
  - Panggil use case `UploadAndConvertPdfToWord`
  - Return `ConversionJobResponse`

- [ ] **Handler: `GET /api/v1/convert/jobs`**
  - List semua job milik user yang login
  - Query params: `page`, `per_page`, `status`

- [ ] **Handler: `GET /api/v1/convert/jobs/:id`**
  - Ambil detail & status satu job

- [ ] **Handler: `GET /api/v1/convert/jobs/:id/download`**
  - Download file hasil konversi

- [ ] **Handler: `DELETE /api/v1/convert/jobs/:id`**
  - Hapus job (hanya jika Draft)

- [ ] **Daftarkan routes ke router** dengan auth guard middleware

---

### 📊 1.6 — Activity Logs Endpoint

- [ ] **Handler: `GET /api/v1/me/activity-logs`**
  - Ambil history aktivitas user yang sedang login
  - Query params: `page`, `per_page`, `action` (filter)
  - Return paginated activity log

---

### 📝 1.7 — Draft System

- [ ] **Flow Draft**: Upload file → simpan sebagai `Draft` → user konfirmasi → mulai konversi
  - Ini memungkinkan user membatalkan sebelum proses dimulai
  - Draft yang tidak dikonfirmasi bisa di-cleanup otomatis (via scheduled task / cron)

- [ ] **Handler: `POST /api/v1/convert/jobs/:id/confirm`**
  - Ubah status Draft → Processing, mulai konversi
  - (Alternatif: langsung proses tanpa confirm — pilih sesuai kebutuhan)

---

### 🧪 1.8 — Testing

- [ ] **Unit Test: `PdfConverterService`**
  - Test konversi file PDF valid → DOCX
  - Test handling file rusak / bukan PDF
  - Test timeout handling

- [ ] **Unit Test: Use Cases**
  - Test `UploadAndConvertPdfToWord` dengan mock repository
  - Test `GetConversionJobStatus` — ownership check
  - Test `DeleteDraftJob` — hanya bisa hapus Draft

- [ ] **Integration Test: Endpoints**
  - `POST /convert/pdf-to-word` — upload valid → expect job created
  - `GET /convert/jobs` — list jobs milik user
  - `GET /convert/jobs/:id/download` — download file done
  - Test auth guard: tanpa token → 401

- [ ] **Performance Test**
  - Ukur waktu konversi untuk berbagai ukuran file (1MB, 10MB, 50MB)
  - Catat benchmark di `docs/benchmarks.md`

- [ ] **Update OpenAPI docs** untuk semua endpoint baru

---

### 📄 1.9 — Dokumentasi & Finalisasi MVP 1

- [ ] Update `README.md` dengan cara setup & run
- [ ] Buat `docs/api-guide.md` — panduan penggunaan API konversi
- [ ] Review error handling — pastikan semua error return format yang konsisten
- [ ] Code review & refactor jika perlu

---

## 🔜 MVP 2 — Tambah Format Konversi

> **Goal**: Perluas kemampuan konversi dengan format-format populer lainnya.

- [ ] **Word → PDF** (`POST /api/v1/convert/word-to-pdf`)
  - Upload `.doc` / `.docx`, hasilkan `.pdf`

- [ ] **Image → PDF** (`POST /api/v1/convert/image-to-pdf`)
  - Upload satu atau banyak gambar (`.jpg`, `.jpeg`, `.png`)
  - Hasilkan satu file `.pdf`
  - Support pengaturan urutan halaman

- [ ] **PDF → Image** (`POST /api/v1/convert/pdf-to-image`)
  - Ekstrak setiap halaman PDF menjadi file gambar (`.png` / `.jpg`)
  - Return sebagai ZIP atau individual files

- [ ] **Extend `ConversionJob.job_type`** — tambah varian enum baru
- [ ] **Unit & integration test** untuk setiap format baru
- [ ] **Update OpenAPI docs**

---

## 🔜 MVP 3 — PDF Tools

> **Goal**: Fitur manipulasi PDF yang sering dibutuhkan.

- [ ] **Compress PDF** (`POST /api/v1/tools/compress-pdf`)
  - Kurangi ukuran file PDF
  - Pilihan level kompresi (low, medium, high)
  - Return info: ukuran sebelum vs sesudah

- [ ] **Merge PDF** (`POST /api/v1/tools/merge-pdf`)
  - Upload beberapa file PDF
  - Tentukan urutan halaman
  - Hasilkan satu PDF gabungan

- [ ] **Split PDF** (`POST /api/v1/tools/split-pdf`)
  - Pisah PDF berdasarkan range halaman
  - Return ZIP berisi file-file hasil split

- [ ] **PDF → Text Extract** (`POST /api/v1/tools/pdf-extract-text`)
  - Ekstrak teks mentah dari PDF
  - Berguna untuk indexing / OCR pipeline

- [ ] **Unit & integration test**
- [ ] **Update OpenAPI docs**

---

## 🔜 MVP 4 — Polish, Optimasi & Scale

> **Goal**: Siapkan untuk production — performa, keamanan, dan UX developer.

- [ ] **Async Job Queue**
  - Pindahkan proses konversi ke background task (tokio task / message queue)
  - WebSocket atau polling endpoint untuk real-time status update

- [ ] **File Cleanup**
  - Scheduled job hapus file lama (> 24 jam setelah download)
  - Hapus Draft yang belum dikonfirmasi > 1 jam

- [ ] **Storage Cloud**
  - Integrasi S3-compatible storage (AWS S3 / MinIO)
  - Presigned URL untuk download

- [ ] **Usage Quota per User**
  - Batasi jumlah konversi per hari / bulan
  - Endpoint `GET /api/v1/me/quota` untuk cek sisa kuota

- [ ] **Webhook Notification**
  - Kirim notifikasi ke URL user saat job selesai
  - `POST /api/v1/webhooks` — daftar webhook

- [ ] **Admin Panel API**
  - List semua user, jobs, activity logs (admin only)
  - Manual trigger cleanup

- [ ] **Security Audit**
  - Pastikan tidak ada path traversal pada file upload
  - Sanitasi nama file
  - Validasi magic bytes (bukan hanya ekstensi)

- [ ] **Load Testing** — pastikan API bisa handle concurrent requests
- [ ] **Docker & Docker Compose** setup untuk production
- [ ] **CI/CD Pipeline** — GitHub Actions untuk test & build

---

## 📌 Catatan & Keputusan Arsitektur

| Topik | Keputusan |
|-------|-----------|
| **Konversi Engine** | TBD — evaluasi LibreOffice headless vs pure Rust |
| **Storage** | Local FS (dev) → S3-compatible (prod) |
| **Job Queue** | Tokio tasks (MVP 1) → dedicated queue (MVP 4) |
| **Auth** | JWT (access + refresh token) ✅ |
| **API Style** | REST + OpenAPI via utoipa ✅ |
| **DB** | PostgreSQL via SQLx ✅ |

---

## 🔗 Referensi

- [Axum docs](https://docs.rs/axum)
- [SQLx docs](https://docs.rs/sqlx)
- [utoipa docs](https://docs.rs/utoipa)
- [lopdf](https://docs.rs/lopdf) — PDF manipulation in Rust
- [LibreOffice headless conversion](https://help.libreoffice.org/latest/en-US/text/shared/guide/converting_files.html)
