use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use thiserror::Error;

pub const SALT_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const IV_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;
pub const HASH_SIZE: usize = 32;

const ARGON2_TIME_COST: u32 = 3;
const ARGON2_MEMORY_COST: u32 = 65536; // 64 MB
const ARGON2_PARALLELISM: u32 = 4;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key size, expected {KEY_SIZE}")]
    InvalidKeySize,
    #[error("Invalid salt size, expected {SALT_SIZE}")]
    InvalidSaltSize,
    #[error("Invalid IV size, expected {IV_SIZE}")]
    InvalidIvSize,
    #[error("Ciphertext too short")]
    CiphertextTooShort,
    #[error("Decryption failed: authentication failure or corrupt data")]
    DecryptionFailed,
    #[error("Argon2 error: {0}")]
    Argon2Error(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn generate_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0u8; SALT_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub fn generate_random_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn generate_iv() -> [u8; IV_SIZE] {
    let mut iv = [0u8; IV_SIZE];
    rand::thread_rng().fill_bytes(&mut iv);
    iv
}

fn create_argon2() -> Result<Argon2<'static>, CryptoError> {
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(HASH_SIZE),
    )
    .map_err(|e| CryptoError::Argon2Error(e.to_string()))?;

    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn hash_password(password: &str, salt: &[u8]) -> Result<[u8; HASH_SIZE], CryptoError> {
    if salt.len() != SALT_SIZE {
        return Err(CryptoError::InvalidSaltSize);
    }
    let argon2 = create_argon2()?;
    let mut out = [0u8; HASH_SIZE];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| CryptoError::Argon2Error(e.to_string()))?;
    Ok(out)
}

pub fn verify_password(password: &str, hash: &[u8], salt: &[u8]) -> bool {
    if hash.len() != HASH_SIZE || salt.len() != SALT_SIZE {
        return false;
    }
    match hash_password(password, salt) {
        Ok(computed) => {
            // Constant-time comparison
            let mut diff = 0u8;
            for (a, b) in computed.iter().zip(hash.iter()) {
                diff |= a ^ b;
            }
            diff == 0
        }
        Err(_) => false,
    }
}

pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_SIZE], CryptoError> {
    if salt.len() != SALT_SIZE {
        return Err(CryptoError::InvalidSaltSize);
    }
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(KEY_SIZE),
    )
    .map_err(|e| CryptoError::Argon2Error(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_SIZE];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| CryptoError::Argon2Error(e.to_string()))?;
    Ok(key)
}

pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, [u8; IV_SIZE]), CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKeySize);
    }
    let iv = generate_iv();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    Ok((ciphertext, iv))
}

pub fn decrypt(ciphertext_with_tag: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKeySize);
    }
    if iv.len() != IV_SIZE {
        return Err(CryptoError::InvalidIvSize);
    }
    if ciphertext_with_tag.len() < TAG_SIZE {
        return Err(CryptoError::CiphertextTooShort);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(iv);
    cipher
        .decrypt(nonce, ciphertext_with_tag)
        .map_err(|_| CryptoError::DecryptionFailed)
}

pub fn encrypt_key(data_key: &[u8], password_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let (ciphertext, iv) = encrypt(data_key, password_key)?;
    let mut envelope = Vec::with_capacity(iv.len() + ciphertext.len());
    envelope.extend_from_slice(&iv);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

pub fn decrypt_key(encrypted_blob: &[u8], password_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if encrypted_blob.len() < IV_SIZE + TAG_SIZE {
        return Err(CryptoError::CiphertextTooShort);
    }
    let iv = &encrypted_blob[..IV_SIZE];
    let ciphertext = &encrypted_blob[IV_SIZE..];
    decrypt(ciphertext, password_key, iv)
}

pub fn encrypt_file(file_path: &Path, key: &[u8]) -> Result<[u8; IV_SIZE], CryptoError> {
    let mut file = OpenOptions::new().read(true).open(file_path)?;
    let mut plaintext = Vec::new();
    file.read_to_end(&mut plaintext)?;
    drop(file);

    let (ciphertext, iv) = encrypt(&plaintext, key)?;

    let mut out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;
    out_file.write_all(&ciphertext)?;
    out_file.flush()?;

    Ok(iv)
}

pub fn decrypt_file(file_path: &Path, key: &[u8], iv: &[u8]) -> Result<(), CryptoError> {
    let mut file = OpenOptions::new().read(true).open(file_path)?;
    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)?;
    drop(file);

    let plaintext = decrypt(&ciphertext, key, iv)?;

    let mut out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;
    out_file.write_all(&plaintext)?;
    out_file.flush()?;

    Ok(())
}

pub fn shred_file(file_path: &Path) -> Result<(), CryptoError> {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_path) {
        Ok(f) => f,
        Err(e) => return Err(CryptoError::Io(e)),
    };

    let size = file.metadata()?.len();
    if size == 0 {
        drop(file);
        std::fs::remove_file(file_path)?;
        return Ok(());
    }

    let passes = 3;
    let buffer_size: usize = 1024 * 1024; // 1 MB buffer
    let mut random_data = vec![0u8; buffer_size];
    let mut rng = rand::thread_rng();

    for _ in 0..passes {
        file.seek(SeekFrom::Start(0))?;
        let mut remaining = size;
        while remaining > 0 {
            let to_write = (remaining as usize).min(buffer_size);
            rng.fill_bytes(&mut random_data[..to_write]);
            file.write_all(&random_data[..to_write])?;
            remaining -= to_write as u64;
        }
        file.flush()?;
    }

    drop(file);
    std::fs::remove_file(file_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let salt = generate_salt();
        let password = "TestVaultPassword123!";
        let hash = hash_password(password, &salt).expect("hash password failed");
        assert!(verify_password(password, &hash, &salt));
        assert!(!verify_password("wrong password", &hash, &salt));
    }

    #[test]
    fn test_key_derivation() {
        let salt = generate_salt();
        let key1 = derive_key("mypassword", &salt).unwrap();
        let key2 = derive_key("mypassword", &salt).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), KEY_SIZE);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_random_key();
        let plaintext = b"Hello, Bubble file manager secure vault!";
        let (ciphertext, iv) = encrypt(plaintext, &key).unwrap();
        assert_ne!(&ciphertext[..plaintext.len()], plaintext);

        let decrypted = decrypt(&ciphertext, &key, &iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_envelope() {
        let master_key = generate_random_key();
        let data_key = generate_random_key();

        let envelope = encrypt_key(&data_key, &master_key).unwrap();
        let decrypted_key = decrypt_key(&envelope, &master_key).unwrap();
        assert_eq!(decrypted_key, data_key);
    }

    #[test]
    fn test_file_encrypt_decrypt_and_shred() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("secret.txt");
        let content = b"Highly sensitive files to lock in vault";
        std::fs::write(&file_path, content).unwrap();

        let key = generate_random_key();
        let iv = encrypt_file(&file_path, &key).unwrap();

        let encrypted = std::fs::read(&file_path).unwrap();
        assert_ne!(encrypted, content);

        decrypt_file(&file_path, &key, &iv).unwrap();
        let decrypted = std::fs::read(&file_path).unwrap();
        assert_eq!(decrypted, content);

        shred_file(&file_path).unwrap();
        assert!(!file_path.exists());
    }
}
