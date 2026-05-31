CREATE TABLE conversion_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    job_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL CHECK (status IN ('draft', 'queued', 'processing', 'done', 'failed')),
    input_path TEXT, output_path TEXT,
    file_size_bytes BIGINT, duration_ms INT,
    error_message TEXT, 
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, 
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);