-- Align conversion_jobs with Rust ConversionJob struct
ALTER TABLE conversion_jobs
    RENAME COLUMN input_path TO input_file;

ALTER TABLE conversion_jobs
    RENAME COLUMN output_path TO output_file;

ALTER TABLE conversion_jobs
    DROP COLUMN file_size_bytes,
    DROP COLUMN duration_ms;

-- Align activity_logs with Rust ActivityLog struct (optional fields)
ALTER TABLE activity_logs
    ALTER COLUMN resource_type DROP NOT NULL,
    ALTER COLUMN resource_id DROP NOT NULL,
    ALTER COLUMN ip_address DROP NOT NULL,
    ALTER COLUMN user_agent DROP NOT NULL,
    ALTER COLUMN metadata DROP NOT NULL;
