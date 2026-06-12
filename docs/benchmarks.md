# PDF to DOCX Conversion Benchmarks

> Generated: 2026-06-12T14:30:21.624069+00:00

| File Size | Input (bytes) | Duration (ms) | Output (bytes) | Status |
|-----------|---------------|---------------|----------------|--------|
| ~1 MB | 715921 | 5012 | 93400 | OK |
| ~10 MB | 715921 | 4690 | 93400 | OK |
| ~50 MB | 715921 | 5383 | 93400 | OK |

## How to run

```bash
# Start unoserver container first
cargo test --test performance_benchmark -- --ignored --nocapture
```