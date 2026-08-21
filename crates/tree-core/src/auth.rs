//! Canonical password hashing for Tree.
//!
//! Every credential in every storage backend is hashed with Argon2id and a
//! per-password random salt.  Hashes are stored in PHC string format, which
//! embeds the algorithm parameters and salt.  This module is the single
//! source of truth; storage crates and the HTTP layer must not implement
//! their own schemes.

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

/// Hash a password with Argon2id and a random salt (PHC string format).
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Argon2id hashing must not fail")
        .to_string()
}

/// Verify a plaintext password against a stored PHC hash string.
///
/// Returns `false` for any stored value that cannot be parsed as a PHC
/// hash, including empty placeholders and legacy SHA-256 digests.
pub fn verify_password(password: &str, phc_hash: &str) -> bool {
    let parsed = match PasswordHash::new(phc_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}
