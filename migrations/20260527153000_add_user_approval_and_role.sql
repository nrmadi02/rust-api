ALTER TABLE users
    ADD COLUMN is_approved BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN role VARCHAR(50) NOT NULL DEFAULT 'user',
    ADD CONSTRAINT users_role_check CHECK (role IN ('user', 'admin'));

CREATE INDEX idx_users_role ON users(role);
