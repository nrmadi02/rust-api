use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{Error, PasswordHasher, SaltString, rand_core::OsRng},
};

pub fn hash_password(password: &str) -> Result<String, Error> {
    let salt = SaltString::generate(OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, Error> {
    let parsed = PasswordHash::new(password_hash).ok();
    Ok(parsed
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false))
}
