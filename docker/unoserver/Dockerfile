# ─────────────────────────────────────────────────────────────────────────────
# unoserver — LibreOffice headless daemon for file conversion
# Base: Debian slim (lebih stabil untuk LibreOffice dibanding Alpine)
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

# Metadata
LABEL maintainer="task-tools"
LABEL description="LibreOffice + unoserver for PDF/DOCX conversion"

# Hindari prompt interaktif saat apt install
ENV DEBIAN_FRONTEND=noninteractive

# Install LibreOffice + dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libreoffice \
    libreoffice-writer \
    python3 \
    python3-pip \
    # Font support untuk hasil konversi yang baik
    fonts-liberation \
    fonts-dejavu \
    fonts-noto \
    # Tools tambahan
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install unoserver via pip (Python sistem Debian tidak di-lock seperti macOS)
RUN pip3 install --break-system-packages unoserver

# Expose port unoserver
EXPOSE 2003

# Health check — pastikan unoserver merespons
HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:2003/ 2>/dev/null || \
        unoconvert --host 127.0.0.1 --port 2003 /dev/null /dev/null 2>&1 | grep -q "Error" && exit 0 || exit 1

# Jalankan unoserver sebagai daemon
# --interface 0.0.0.0 agar bisa diakses dari container lain
CMD ["unoserver", "--interface", "0.0.0.0", "--port", "2003"]
