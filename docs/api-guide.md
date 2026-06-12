# API Guide — Konversi PDF ke Word

Panduan penggunaan endpoint konversi MVP 1. Base URL contoh: `http://127.0.0.1:8888`.

Dokumentasi interaktif tersedia di [`/scalar`](../README.md) saat server berjalan.

---

## Autentikasi

Semua endpoint konversi dan activity logs memerlukan **Bearer JWT**.

### Register

```bash
curl -X POST http://127.0.0.1:8888/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Nadia",
    "email": "nadia@example.com",
    "password": "password123"
  }'
```

### Login

```bash
curl -X POST http://127.0.0.1:8888/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "nadia@example.com",
    "password": "password123"
  }'
```

Response sukses:

```json
{
  "success": true,
  "message": "Login successful",
  "data": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 86400,
    "user": { "id": "...", "email": "...", "status": "approved", ... }
  }
}
```

Simpan `data.access_token` untuk request berikutnya:

```bash
export TOKEN="eyJ..."
```

> **Catatan:** User harus berstatus `approved` untuk login. Akun baru mungkin perlu disetujui admin terlebih dahulu.

---

## Format Response

### Sukses

```json
{
  "success": true,
  "message": "Deskripsi singkat",
  "data": { ... }
}
```

### Error (konsisten di semua handler)

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Pesan yang bisa dibaca manusia",
    "details": ["opsional, untuk validation error"]
  }
}
```

### Kode error umum

| HTTP | Code | Kapan |
|------|------|-------|
| 400 | `VALIDATION_ERROR` | Input JSON tidak valid |
| 400 | `INVALID_FILE` | Bukan PDF, terlalu besar, corrupt, password protected |
| 400 | `FILE_REQUIRED` | Multipart tanpa field `file` |
| 400 | `MULTIPART_ERROR` | Gagal parse multipart |
| 400 | `JOB_NOT_DRAFT` | Konfirmasi/hapus job bukan draft |
| 400 | `JOB_NOT_DONE` | Download sebelum selesai |
| 401 | `UNAUTHORIZED` | Token tidak ada / tidak valid |
| 401 | `INVALID_CREDENTIALS` | Email/password salah |
| 401 | `USER_NOT_ACTIVE` | Akun belum aktif |
| 403 | `FORBIDDEN` | Akses ditolak |
| 403 | `ACCOUNT_NOT_APPROVED` | Akun belum disetujui (middleware ApprovedUser) |
| 404 | `JOB_NOT_FOUND` | Job tidak ada atau bukan milik user |
| 404 | `USER_NOT_FOUND` | User tidak ditemukan |
| 409 | `EMAIL_ALREADY_REGISTERED` | Email sudah terdaftar |
| 429 | `TOO_MANY_ATTEMPTS` | Terlalu banyak percobaan login |
| 500 | `INTERNAL_SERVER_ERROR` | Error tidak terduga |
| 500 | `STORAGE_ERROR` | Gagal baca/tulis file |

> **Rate limiting:** Endpoint `/api/auth/login` dan `/api/auth/register` dibatasi oleh IP (tower_governor). Response rate limit mungkin berformat berbeda dari tabel di atas.

---

## Alur Konversi PDF → Word

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────┐
│ Upload PDF  │───▶│ Draft (202)  │───▶│ Confirm     │───▶│ Queued   │
│ multipart   │    │              │    │ (202)       │    │          │
└─────────────┘    └──────────────┘    └─────────────┘    └────┬─────┘
                                                                 │
                    ┌──────────────┐    ┌─────────────┐          ▼
                    │ Download     │◀───│ Done        │◀─── Processing
                    │ .docx        │    │ (polling)   │     (background)
                    └──────────────┘    └─────────────┘
```

1. **Upload** — file disimpan, job dibuat dengan status `draft`
2. **Confirm** — user memulai konversi; status → `queued` → `processing`
3. **Poll status** — cek `GET /jobs/:id` sampai `done` atau `failed`
4. **Download** — ambil file `.docx` saat status `done`

User bisa **hapus draft** (`DELETE /jobs/:id`) sebelum konfirmasi.

---

## Endpoint Konversi

### 1. Upload PDF → Draft

```http
POST /api/v1/convert/pdf-to-word
Authorization: Bearer <token>
Content-Type: multipart/form-data
```

```bash
curl -X POST http://127.0.0.1:8888/api/v1/convert/pdf-to-word \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/document.pdf"
```

**Response `202 Accepted`:**

```json
{
  "success": true,
  "message": "PDF uploaded successfully",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "job_type": "pdf_to_word",
    "status": "draft",
    "download_url": null,
    "input_file": "uploads/.../input.pdf",
    "output_file": null,
    "error_message": null,
    "duration_ms": null,
    "created_at": "2026-06-12T10:00:00Z",
    "updated_at": "2026-06-12T10:00:00Z"
  }
}
```

**Validasi file:**
- Ekstensi `.pdf`
- Magic bytes `%PDF`
- Ukuran ≤ `MAX_UPLOAD_SIZE_MB`
- Tidak password protected
- Bukan file corrupt

---

### 2. Konfirmasi Draft → Mulai Konversi

```http
POST /api/v1/convert/jobs/{id}/confirm
Authorization: Bearer <token>
```

```bash
curl -X POST "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID/confirm" \
  -H "Authorization: Bearer $TOKEN"
```

**Response `202 Accepted`:**

```json
{
  "success": true,
  "message": "Conversion job confirmed and queued",
  "data": {
    "id": "...",
    "status": "queued",
    ...
  }
}
```

Status berkembang di background: `queued` → `processing` → `done` / `failed`.

Jika unoserver tidak tersedia, job akan berstatus `failed` dengan `error_message` di field job.

---

### 3. Cek Status Job

```http
GET /api/v1/convert/jobs/{id}
Authorization: Bearer <token>
```

```bash
curl "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID" \
  -H "Authorization: Bearer $TOKEN"
```

**Saat selesai (`done`):**

```json
{
  "success": true,
  "message": "Conversion job retrieved successfully",
  "data": {
    "id": "...",
    "status": "done",
    "download_url": "/api/v1/convert/jobs/{id}/download",
    "output_file": "outputs/.../output.docx",
    "duration_ms": 4500,
    ...
  }
}
```

**Saat gagal (`failed`):**

```json
{
  "data": {
    "status": "failed",
    "error_message": "conversion timed out after 60s",
    ...
  }
}
```

---

### 4. Download Hasil

```http
GET /api/v1/convert/jobs/{id}/download
Authorization: Bearer <token>
```

```bash
curl -o converted.docx \
  "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID/download" \
  -H "Authorization: Bearer $TOKEN"
```

Hanya tersedia jika `status == done`. Response adalah file binary (bukan JSON).

---

### 5. List Semua Job

```http
GET /api/v1/convert/jobs?page=1&per_page=10&status=draft
Authorization: Bearer <token>
```

| Query param | Default | Keterangan |
|-------------|---------|------------|
| `page` | `1` | Halaman (min 1) |
| `per_page` | `10` | Item per halaman (1–100) |
| `status` | — | Filter: `draft`, `queued`, `processing`, `done`, `failed` |

```bash
curl "http://127.0.0.1:8888/api/v1/convert/jobs?status=done&page=1&per_page=20" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 6. Hapus Draft

```http
DELETE /api/v1/convert/jobs/{id}
Authorization: Bearer <token>
```

```bash
curl -X DELETE "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID" \
  -H "Authorization: Bearer $TOKEN"
```

Hanya job berstatus `draft` yang bisa dihapus. File upload ikut dihapus dari storage.

---

## Activity Logs

```http
GET /api/v1/me/activity-logs?page=1&per_page=10&action=upload_pdf
Authorization: Bearer <token>
```

| Query param | Default | Keterangan |
|-------------|---------|------------|
| `page` | `1` | Halaman |
| `per_page` | `10` | Item per halaman (1–100) |
| `action` | — | Filter aksi, mis. `upload_pdf`, `confirm_job`, `download_file` |

```bash
curl "http://127.0.0.1:8888/api/v1/me/activity-logs" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Contoh Workflow Lengkap

```bash
# 1. Login
TOKEN=$(curl -s -X POST http://127.0.0.1:8888/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"nadia@example.com","password":"password123"}' \
  | jq -r '.data.access_token')

# 2. Upload
JOB_ID=$(curl -s -X POST http://127.0.0.1:8888/api/v1/convert/pdf-to-word \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@sample.pdf" \
  | jq -r '.data.id')

echo "Job ID: $JOB_ID"

# 3. Konfirmasi
curl -s -X POST "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID/confirm" \
  -H "Authorization: Bearer $TOKEN" | jq .

# 4. Poll sampai done (manual loop)
curl -s "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID" \
  -H "Authorization: Bearer $TOKEN" | jq '.data.status'

# 5. Download
curl -o output.docx \
  "http://127.0.0.1:8888/api/v1/convert/jobs/$JOB_ID/download" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Testing API

Lihat [README.md](../README.md#testing) untuk cara menjalankan:

- `cargo test` — semua test
- `cargo test --test api_integration` — butuh `DATABASE_URL`
- `cargo test --test pdf_converter -- --ignored` — butuh unoserver untuk test konversi nyata

---

## Troubleshooting

| Masalah | Solusi |
|---------|--------|
| `401 UNAUTHORIZED` | Pastikan header `Authorization: Bearer <token>` benar |
| `INVALID_FILE` | Cek file benar-benar PDF, tidak corrupt/password |
| Job `failed` dengan timeout | Naikkan `UNOSERVER_TIMEOUT_SECS` atau perkecil file |
| Job `failed` spawn error | Pastikan `unoconvert` ada di PATH dan container unoserver jalan |
| Integration test skip | Set `DATABASE_URL` di `.env` |
| Upload ditolak ukuran | Sesuaikan `MAX_UPLOAD_SIZE_MB` di `.env` |
