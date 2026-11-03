-- 20261103135349_create_auth_user_credentials.sql
--
-- Per-user password hash + token version used by the auth crate. The
-- auth crate does NOT own the user lifecycle; `code` is the join key
-- against the user crate's `users.code`, and there is no foreign key
-- (the auth schema must deploy independently of the user crate).
--
-- `password_hash` stores an Argon2id PHC string produced by
-- `argon2::Argon2::default()`. `token_version` starts at 1 and is
-- monotonically incremented by `bump_token_version` on logout; every
-- outstanding JWT for that user carries the pre-bump version and is
-- rejected by `verify`.
--
-- `auth_user_credentials_set_updated_at` mirrors the trigger from the
-- user crate so an out-of-band `UPDATE` (e.g. via psql) still bumps
-- `updated_at` without the application having to remember.

CREATE TABLE auth_user_credentials (
    code TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    token_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_user_credentials_password_hash_check CHECK (length(password_hash) > 0)
);

CREATE OR REPLACE FUNCTION auth_user_credentials_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_user_credentials_set_updated_at
    BEFORE UPDATE ON auth_user_credentials
    FOR EACH ROW
    EXECUTE FUNCTION auth_user_credentials_set_updated_at();