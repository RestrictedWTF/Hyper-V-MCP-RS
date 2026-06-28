use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::ffi::c_void;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
};

#[derive(Debug, thiserror::Error)]
pub enum DpapiError {
    #[error("DPAPI operation failed: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("invalid UTF-8 in decrypted data")]
    InvalidUtf8,
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
}

struct CryptBlob(CRYPT_INTEGER_BLOB);

impl CryptBlob {
    fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.pbData, self.0.cbData as usize) }
    }
}

impl Drop for CryptBlob {
    fn drop(&mut self) {
        if !self.0.pbData.is_null() {
            unsafe {
                let _ = LocalFree(HLOCAL(self.0.pbData as *mut c_void));
            }
        }
    }
}

pub fn encrypt_string(plaintext: &str) -> Result<Vec<u8>, DpapiError> {
    let bytes = plaintext.as_bytes();
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CryptBlob(CRYPT_INTEGER_BLOB::default());
    unsafe {
        CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output.0)?;
        Ok(output.as_slice().to_vec())
    }
}

pub fn decrypt_string(blob: &[u8]) -> Result<String, DpapiError> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut output = CryptBlob(CRYPT_INTEGER_BLOB::default());
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output.0)?;
        let result =
            String::from_utf8(output.as_slice().to_vec()).map_err(|_| DpapiError::InvalidUtf8)?;
        Ok(result)
    }
}

pub fn encrypt_to_base64(plaintext: &str) -> Result<String, DpapiError> {
    Ok(STANDARD.encode(&encrypt_string(plaintext)?))
}

pub fn decrypt_from_base64(encoded: &str) -> Result<String, DpapiError> {
    let bytes = STANDARD.decode(encoded)?;
    decrypt_string(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let original = "MySecretP@ssw0rd!";
        let encrypted = encrypt_string(original).expect("encrypt");
        assert_ne!(encrypted, original.as_bytes());
        let decrypted = decrypt_string(&encrypted).expect("decrypt");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn round_trip_base64() {
        let original = "Another secret";
        let encoded = encrypt_to_base64(original).expect("encrypt");
        let decrypted = decrypt_from_base64(&encoded).expect("decrypt");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn round_trip_empty_string() {
        let original = "";
        let encrypted = encrypt_string(original).expect("encrypt empty string");
        assert!(!encrypted.is_empty());
        let decrypted = decrypt_string(&encrypted).expect("decrypt empty string");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn decrypt_from_base64_invalid_returns_base64_error() {
        let err = decrypt_from_base64("not valid base64 !!!").unwrap_err();
        assert!(matches!(err, DpapiError::Base64(_)));
    }
}
