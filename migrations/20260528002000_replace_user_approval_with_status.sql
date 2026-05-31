DO $$
BEGIN
    CREATE TYPE user_status AS ENUM ('pending', 'approved', 'rejected', 'suspended');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

ALTER TABLE users
    ADD COLUMN status user_status NOT NULL DEFAULT 'pending',
    ADD COLUMN approved_by UUID REFERENCES users(id),
    ADD COLUMN rejected_at TIMESTAMP,
    ADD COLUMN rejected_by UUID REFERENCES users(id),
    ADD COLUMN rejection_reason TEXT;

UPDATE users
SET status = CASE
    WHEN is_approved THEN 'approved'::user_status
    ELSE 'pending'::user_status
END;

ALTER TABLE users
    DROP COLUMN is_approved;

CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_users_approved_by ON users(approved_by);
CREATE INDEX idx_users_rejected_by ON users(rejected_by);
