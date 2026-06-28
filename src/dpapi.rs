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
}

pub fn encrypt_string(plaintext: &str) -> Result<Vec<u8>, DpapiError> {
    let bytes = plaintext.as_bytes();
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            0,
            &mut output,
        )?;
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut c_void));
        Ok(result)
    }
}

pub fn decrypt_string(blob: &[u8]) -> Result<String, DpapiError> {
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            0,
            &mut output,
        )?;
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = String::from_utf8(slice.to_vec()).map_err(|_| DpapiError::InvalidUtf8)?;
        let _ = LocalFree(HLOCAL(output.pbData as *mut c_void));
        Ok(result)
    }
}

pub fn encrypt_to_base64(plaintext: &str) -> Result<String, DpapiError> {
    Ok(STANDARD.encode(&encrypt_string(plaintext)?))
}

pub fn decrypt_from_base64(encoded: &str) -> Result<String, DpapiError> {
    let bytes = STANDARD.decode(encoded).map_err(|e| {
        DpapiError::Windows(windows::core::Error::new(
            windows::Win32::Foundation::E_UNEXPECTED,
            format!("base64 decode failed: {}", e),
        ))
    })?;
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
}
