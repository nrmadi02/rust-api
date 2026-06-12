# Task Tools API

REST API untuk konversi dan pemrosesan file, dibangun dengan **Rust**, **Axum**, **PostgreSQL**, dan **SQLx**.

MVP 1 menyediakan konversi **PDF → Word (`.docx`)** dengan sistem draft, activity logs, dan background worker via unoserver/LibreOffice.

## Fitur (MVP 1)

- Autentikasi JWT (register, login)
- Upload PDF → job `draft`
- Konfirmasi draft → konversi background (`queued` → `processing` → `done` / `failed`)
- List, detail, download, dan hapus job
- Activity logs per user
- OpenAPI interaktif di `/scalar`

## Prerequisites

| Tool | Versi / Catatan |
|------|-----------------|
| Rust | 1.95+ (lihat `rust-toolchain.toml`) |
| PostgreSQL | 14+ |
| SQLx CLI | Untuk migrasi: `cargo install sqlx-cli --no-default-features --features native-tls,postgres` |
| unoserver | Container (Podman/Docker) — port `2003` |
| unoconvert | Client di host: `pipx install unoserver` |

## Quick Start

### 1. Clone & environment

```bash
cp .env.example .env
# Edit .env — set DATABASE_URL, JWT_SECRET (min 32 karakter), dll.
```

### 2. Database

```bash
createdb task_tools   # atau buat DB sesuai DATABASE_URL Anda

sqlx database create  # opsional, jika DB belum ada
sqlx migrate run
```

### 3. unoserver (konversi PDF → Word)

**Opsi A — Docker Compose:**

```bash
docker compose up -d unoserver
# atau: podman compose up -d unoserver
```

**Opsi B — Podman manual:**

```bash
podman build -t task-tools-unoserver:latest docker/unoserver
podman run -d --name task-tools-unoserver -p 2003:2003 task-tools-unoserver:latest
```

**Client di host:**

```bash
pipx install unoserver
unoconvert --version   # pastikan tersedia di PATH
```

### 4. Jalankan API

```bash
cargo run
```

Server default: `http://127.0.0.1:8888` (sesuai `PORT` di `.env`).

### 5. Cek kesehatan & docs

```bash
curl http://127.0.0.1:8888/
# Buka browser: http://127.0.0.1:8888/scalar
```

## Environment Variables

| Variable | Default | Deskripsi |
|----------|---------|-----------|
| `PORT` | `3000` | Port HTTP server |
| `DATABASE_URL` | — | Connection string PostgreSQL |
| `JWT_SECRET` | — | Secret JWT (min 32 karakter) |
| `JWT_EXPIRES_IN` | `86400` | Masa berlaku token (detik) |
| `UNOSERVER_HOST` | — | Host unoserver (mis. `127.0.0.1`) |
| `UNOSERVER_PORT` | — | Port unoserver (mis. `2003`) |
| `UNOSERVER_TIMEOUT_SECS` | — | Timeout konversi (detik) |
| `STORAGE_BASE_PATH` | — | Folder penyimpanan file lokal |
| `MAX_UPLOAD_SIZE_MB` | — | Batas ukuran upload |
| `FILE_TTL_HOURS` | — | TTL file (untuk cleanup masa depan) |

Lihat `.env.example` untuk template lengkap.

## Testing

### Semua test

```bash
cargo test
```

### Per kategori

```bash
# Unit test use case (mock, tanpa DB)
cargo test --test conversion_use_cases

# Unit test PDF validator & converter
cargo test --test pdf_converter

# OpenAPI — pastikan semua endpoint terdokumentasi
cargo test --test openapi_test
```

### Integration test (butuh PostgreSQL)

Set `DATABASE_URL` di `.env` atau environment, lalu:

```bash
cargo test --test api_integration
```

Test ini membuat user sementara, menjalankan migrasi, dan menguji endpoint HTTP secara end-to-end.

### Performance benchmark (butuh unoserver + unoconvert)

```bash
# Pastikan container unoserver sudah jalan
cargo test --test performance_benchmark -- --ignored --nocapture
```

Hasil benchmark dicatat di [`docs/benchmarks.md`](docs/benchmarks.md).

### Lint & format

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## Dokumentasi API

- **Panduan penggunaan**: [`docs/api-guide.md`](docs/api-guide.md)
- **OpenAPI interaktif**: `http://127.0.0.1:8888/scalar` (saat server berjalan)
- **Benchmark konversi**: [`docs/benchmarks.md`](docs/benchmarks.md)

## Struktur Proyek

```
src/
├── application/     # Use cases & business logic
├── domain/          # Entities, traits, enums
├── infrastructure/  # DB repos, storage, unoserver client
├── presentation/    # Handlers, DTO, router, middleware
└── config/          # Env & konfigurasi

migrations/          # SQLx migrations
tests/               # Integration & unit tests
docs/                # Dokumentasi tambahan
docker/unoserver/    # Dockerfile unoserver
```

## Alur Konversi (Ringkas)

```
Upload PDF → draft
     ↓
POST /jobs/:id/confirm → queued → processing
     ↓
GET /jobs/:id (polling) → done
     ↓
GET /jobs/:id/download → file .docx
```

Detail lengkap, contoh `curl`, dan format error: lihat [`docs/api-guide.md`](docs/api-guide.md).

## Referensi

- [TASKS.md](TASKS.md) — roadmap & progress
- [Axum](https://docs.rs/axum) · [SQLx](https://docs.rs/sqlx) · [utoipa](https://docs.rs/utoipa)
- [unoserver](https://github.com/unoconv/unoserver)
