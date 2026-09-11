use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

pub const SALT_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

const ARGON2_TIME_COST: u32 = 3;
const ARGON2_MEMORY_COST: u32 = 65536; // 64 MB
const ARGON2_PARALLELISM: u32 = 4;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Argon2 error: {0}")]
    Argon2(String),
    #[error("OpenSSL error: {0}")]
    OpenSsl(String),
    #[error("Tampering detected: File contents corrupted or modified on disk")]
    Tampered,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid data size: {0}")]
    InvalidSize(String),
}

// C FFI bindings to OpenSSL EVP and Argon2
#[allow(non_camel_case_types)]
mod ffi {
    use libc::{c_int, c_uchar, c_void, size_t};

    pub const EVP_CTRL_GCM_SET_IVLEN: c_int = 0x9;
    pub const EVP_CTRL_GCM_GET_TAG: c_int = 0x10;
    pub const EVP_CTRL_GCM_SET_TAG: c_int = 0x11;

    pub enum EVP_CIPHER_CTX {}
    pub enum EVP_CIPHER {}

    extern "C" {
        pub fn RAND_bytes(buf: *mut c_uchar, num: c_int) -> c_int;

        pub fn EVP_CIPHER_CTX_new() -> *mut EVP_CIPHER_CTX;
        pub fn EVP_CIPHER_CTX_free(ctx: *mut EVP_CIPHER_CTX);
        pub fn EVP_aes_256_gcm() -> *const EVP_CIPHER;

        pub fn EVP_EncryptInit_ex(
            ctx: *mut EVP_CIPHER_CTX,
            cipher: *const EVP_CIPHER,
            impl_: *mut c_void,
            key: *const c_uchar,
            iv: *const c_uchar,
        ) -> c_int;

        pub fn EVP_EncryptUpdate(
            ctx: *mut EVP_CIPHER_CTX,
            out: *mut c_uchar,
            outl: *mut c_int,
            in_: *const c_uchar,
            inl: c_int,
        ) -> c_int;

        pub fn EVP_EncryptFinal_ex(
            ctx: *mut EVP_CIPHER_CTX,
            out: *mut c_uchar,
            outl: *mut c_int,
        ) -> c_int;

        pub fn EVP_DecryptInit_ex(
            ctx: *mut EVP_CIPHER_CTX,
            cipher: *const EVP_CIPHER,
            impl_: *mut c_void,
            key: *const c_uchar,
            iv: *const c_uchar,
        ) -> c_int;

        pub fn EVP_DecryptUpdate(
            ctx: *mut EVP_CIPHER_CTX,
            out: *mut c_uchar,
            outl: *mut c_int,
            in_: *const c_uchar,
            inl: c_int,
        ) -> c_int;

        pub fn EVP_DecryptFinal_ex(
            ctx: *mut EVP_CIPHER_CTX,
            out: *mut c_uchar,
            outl: *mut c_int,
        ) -> c_int;

        pub fn EVP_CIPHER_CTX_ctrl(
            ctx: *mut EVP_CIPHER_CTX,
            type_: c_int,
            arg: c_int,
            ptr: *mut c_void,
        ) -> c_int;

        // libargon2
        pub fn argon2id_hash_raw(
            t_cost: u32,
            m_cost: u32,
            parallelism: u32,
            pwd: *const c_void,
            pwdlen: size_t,
            salt: *const c_void,
            saltlen: size_t,
            hash: *mut c_void,
            hashlen: size_t,
        ) -> c_int;
    }
}

pub fn random_bytes(buf: &mut [u8]) {
    unsafe {
        ffi::RAND_bytes(buf.as_mut_ptr(), buf.len() as libc::c_int);
    }
}

pub fn generate_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0u8; SALT_SIZE];
    random_bytes(&mut salt);
    salt
}

pub fn generate_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    random_bytes(&mut key);
    key
}

pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    random_bytes(&mut nonce);
    nonce
}

pub fn hash_password(password: &str, salt: &[u8; SALT_SIZE]) -> Result<[u8; KEY_SIZE], CryptoError> {
    let mut hash = [0u8; KEY_SIZE];
    let ret = unsafe {
        ffi::argon2id_hash_raw(
            ARGON2_TIME_COST,
            ARGON2_MEMORY_COST,
            ARGON2_PARALLELISM,
            password.as_ptr() as *const libc::c_void,
            password.len(),
            salt.as_ptr() as *const libc::c_void,
            salt.len(),
            hash.as_mut_ptr() as *mut libc::c_void,
            hash.len(),
        )
    };

    if ret != 0 {
        return Err(CryptoError::Argon2(format!("argon2 error code: {}", ret)));
    }

    Ok(hash)
}

pub fn verify_password(password: &str, hash: &[u8], salt: &[u8]) -> bool {
    if salt.len() != SALT_SIZE || hash.len() != KEY_SIZE {
        return false;
    }
    let mut salt_arr = [0u8; SALT_SIZE];
    salt_arr.copy_from_slice(salt);

    match hash_password(password, &salt_arr) {
        Ok(computed) => {
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
    if salt.len() < SALT_SIZE {
        return Err(CryptoError::Argon2("Salt too short".into()));
    }
    let mut salt_arr = [0u8; SALT_SIZE];
    salt_arr.copy_from_slice(&salt[..SALT_SIZE]);
    hash_password(password, &salt_arr)
}

pub fn encrypt(plaintext: &[u8], key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE]) -> Result<Vec<u8>, CryptoError> {
    unsafe {
        let ctx = ffi::EVP_CIPHER_CTX_new();
        if ctx.is_null() {
            return Err(CryptoError::OpenSsl("Failed to create EVP_CIPHER_CTX".into()));
        }

        struct CtxGuard(*mut ffi::EVP_CIPHER_CTX);
        impl Drop for CtxGuard {
            fn drop(&mut self) {
                unsafe { ffi::EVP_CIPHER_CTX_free(self.0) };
            }
        }
        let _guard = CtxGuard(ctx);

        if ffi::EVP_EncryptInit_ex(ctx, ffi::EVP_aes_256_gcm(), std::ptr::null_mut(), std::ptr::null(), std::ptr::null()) != 1 {
            return Err(CryptoError::OpenSsl("EVP_EncryptInit_ex cipher init failed".into()));
        }

        if ffi::EVP_CIPHER_CTX_ctrl(ctx, ffi::EVP_CTRL_GCM_SET_IVLEN, NONCE_SIZE as libc::c_int, std::ptr::null_mut()) != 1 {
            return Err(CryptoError::OpenSsl("Failed to set IV length".into()));
        }

        if ffi::EVP_EncryptInit_ex(ctx, std::ptr::null(), std::ptr::null_mut(), key.as_ptr(), nonce.as_ptr()) != 1 {
            return Err(CryptoError::OpenSsl("EVP_EncryptInit_ex key/iv failed".into()));
        }

        let mut output = vec![0u8; plaintext.len() + TAG_SIZE];
        let mut out_len: libc::c_int = 0;

        if !plaintext.is_empty() {
            if ffi::EVP_EncryptUpdate(
                ctx,
                output.as_mut_ptr(),
                &mut out_len,
                plaintext.as_ptr(),
                plaintext.len() as libc::c_int,
            ) != 1 {
                return Err(CryptoError::OpenSsl("EVP_EncryptUpdate failed".into()));
            }
        }

        let mut final_len: libc::c_int = 0;
        if ffi::EVP_EncryptFinal_ex(
            ctx,
            output.as_mut_ptr().add(out_len as usize),
            &mut final_len,
        ) != 1 {
            return Err(CryptoError::OpenSsl("EVP_EncryptFinal_ex failed".into()));
        }

        let total_ct_len = (out_len + final_len) as usize;

        // Retrieve authentication tag and append
        let mut tag = [0u8; TAG_SIZE];
        if ffi::EVP_CIPHER_CTX_ctrl(
            ctx,
            ffi::EVP_CTRL_GCM_GET_TAG,
            TAG_SIZE as libc::c_int,
            tag.as_mut_ptr() as *mut libc::c_void,
        ) != 1 {
            return Err(CryptoError::OpenSsl("EVP_CTRL_GCM_GET_TAG failed".into()));
        }

        output.truncate(total_ct_len);
        output.extend_from_slice(&tag);

        Ok(output)
    }
}

pub fn decrypt(ciphertext_with_tag: &[u8], key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext_with_tag.len() < TAG_SIZE {
        return Err(CryptoError::Tampered);
    }

    let ct_len = ciphertext_with_tag.len() - TAG_SIZE;
    let ciphertext = &ciphertext_with_tag[..ct_len];
    let tag = &ciphertext_with_tag[ct_len..];

    unsafe {
        let ctx = ffi::EVP_CIPHER_CTX_new();
        if ctx.is_null() {
            return Err(CryptoError::OpenSsl("Failed to create EVP_CIPHER_CTX".into()));
        }

        struct CtxGuard(*mut ffi::EVP_CIPHER_CTX);
        impl Drop for CtxGuard {
            fn drop(&mut self) {
                unsafe { ffi::EVP_CIPHER_CTX_free(self.0) };
            }
        }
        let _guard = CtxGuard(ctx);

        if ffi::EVP_DecryptInit_ex(ctx, ffi::EVP_aes_256_gcm(), std::ptr::null_mut(), std::ptr::null(), std::ptr::null()) != 1 {
            return Err(CryptoError::OpenSsl("EVP_DecryptInit_ex cipher init failed".into()));
        }

        if ffi::EVP_CIPHER_CTX_ctrl(ctx, ffi::EVP_CTRL_GCM_SET_IVLEN, NONCE_SIZE as libc::c_int, std::ptr::null_mut()) != 1 {
            return Err(CryptoError::OpenSsl("Failed to set IV length".into()));
        }

        if ffi::EVP_DecryptInit_ex(ctx, std::ptr::null(), std::ptr::null_mut(), key.as_ptr(), nonce.as_ptr()) != 1 {
            return Err(CryptoError::OpenSsl("EVP_DecryptInit_ex key/iv failed".into()));
        }

        let mut output = vec![0u8; ct_len + TAG_SIZE];
        let mut out_len: libc::c_int = 0;

        if ct_len > 0 {
            if ffi::EVP_DecryptUpdate(
                ctx,
                output.as_mut_ptr(),
                &mut out_len,
                ciphertext.as_ptr(),
                ct_len as libc::c_int,
            ) != 1 {
                return Err(CryptoError::Tampered);
            }
        }

        // Set expected authentication tag before final
        if ffi::EVP_CIPHER_CTX_ctrl(
            ctx,
            ffi::EVP_CTRL_GCM_SET_TAG,
            TAG_SIZE as libc::c_int,
            tag.as_ptr() as *mut libc::c_void,
        ) != 1 {
            return Err(CryptoError::Tampered);
        }

        let mut final_len: libc::c_int = 0;
        let ret = ffi::EVP_DecryptFinal_ex(
            ctx,
            output.as_mut_ptr().add(out_len as usize),
            &mut final_len,
        );

        if ret <= 0 {
            // Authentication tag mismatch = tampering!
            return Err(CryptoError::Tampered);
        }

        output.truncate((out_len + final_len) as usize);
        Ok(output)
    }
}

pub fn encrypt_key(data_key: &[u8; KEY_SIZE], password_key: &[u8; KEY_SIZE]) -> Result<Vec<u8>, CryptoError> {
    let nonce = generate_nonce();
    let ciphertext = encrypt(data_key, password_key, &nonce)?;
    let mut blob = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

pub fn decrypt_key(encrypted_blob: &[u8], password_key: &[u8; KEY_SIZE]) -> Result<[u8; KEY_SIZE], CryptoError> {
    if encrypted_blob.len() < NONCE_SIZE + TAG_SIZE {
        return Err(CryptoError::InvalidSize("Encrypted key blob too small".into()));
    }

    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&encrypted_blob[..NONCE_SIZE]);
    let ciphertext = &encrypted_blob[NONCE_SIZE..];

    let decrypted = decrypt(ciphertext, password_key, &nonce)?;
    if decrypted.len() != KEY_SIZE {
        return Err(CryptoError::InvalidSize("Decrypted key invalid length".into()));
    }

    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&decrypted);
    Ok(key)
}

pub fn encrypt_file(path: &Path, key: &[u8; KEY_SIZE]) -> Result<[u8; NONCE_SIZE], CryptoError> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut plaintext = Vec::new();
    file.read_to_end(&mut plaintext)?;

    let nonce = generate_nonce();
    let ciphertext = encrypt(&plaintext, key, &nonce)?;

    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(&ciphertext)?;
    file.sync_all()?;

    Ok(nonce)
}

pub fn decrypt_file(path: &Path, key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE]) -> Result<(), CryptoError> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)?;

    let plaintext = decrypt(&ciphertext, key, nonce)?;

    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(&plaintext)?;
    file.sync_all()?;

    Ok(())
}

pub fn shred_file(path: &Path) -> Result<(), CryptoError> {
    if !path.exists() {
        return Err(CryptoError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File does not exist",
        )));
    }

    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        std::fs::remove_file(path)?;
        return Ok(());
    }

    let file_size = metadata.len();
    if file_size > 0 {
        let mut file = OpenOptions::new().write(true).open(path)?;
        let chunk_size = 64 * 1024;
        let mut buf = vec![0u8; chunk_size];

        // 3 overwrite passes
        for _ in 0..3 {
            file.seek(SeekFrom::Start(0))?;
            let mut remaining = file_size;
            while remaining > 0 {
                let to_write = std::cmp::min(remaining, chunk_size as u64) as usize;
                random_bytes(&mut buf[..to_write]);
                file.write_all(&buf[..to_write])?;
                remaining -= to_write as u64;
            }
            file.sync_all()?;
        }

        file.set_len(0)?;
        file.sync_all()?;
    }

    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_verify() {
        let salt = generate_salt();
        let hash = hash_password("Secret123!", &salt).unwrap();
        assert!(verify_password("Secret123!", &hash, &salt));
        assert!(!verify_password("Wrong!", &hash, &salt));
    }

    #[test]
    fn test_key_wrap() {
        let data_key = generate_key();
        let salt = generate_salt();
        let pw_key = derive_key("master-password", &salt).unwrap();

        let wrapped = encrypt_key(&data_key, &pw_key).unwrap();
        let unwrapped = decrypt_key(&wrapped, &pw_key).unwrap();
        assert_eq!(data_key, unwrapped);

        let wrong_pw = derive_key("wrong-password", &salt).unwrap();
        assert!(decrypt_key(&wrapped, &wrong_pw).is_err());
    }

    #[test]
    fn test_file_encrypt_decrypt() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();
        let original_data = b"Confidential documents text to be secured in Bubble!";
        std::fs::write(path, original_data).unwrap();

        let key = generate_key();
        let nonce = encrypt_file(path, &key).unwrap();

        let on_disk = std::fs::read(path).unwrap();
        assert_ne!(on_disk, original_data);

        decrypt_file(path, &key, &nonce).unwrap();
        let restored = std::fs::read(path).unwrap();
        assert_eq!(restored, original_data);
    }

    #[test]
    fn test_tamper_detection() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();
        std::fs::write(path, b"Financial records").unwrap();

        let key = generate_key();
        let nonce = encrypt_file(path, &key).unwrap();

        // Corrupt a byte
        let mut corrupted = std::fs::read(path).unwrap();
        let last_idx = corrupted.len() - 1;
        corrupted[last_idx] ^= 0xFF;
        std::fs::write(path, &corrupted).unwrap();

        let result = decrypt_file(path, &key, &nonce);
        assert!(matches!(result, Err(CryptoError::Tampered)));
    }

    #[test]
    fn test_shred() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();
        std::fs::write(&path, b"Data to shred").unwrap();
        assert!(path.exists());

        shred_file(&path).unwrap();
        assert!(!path.exists());
    }
}
