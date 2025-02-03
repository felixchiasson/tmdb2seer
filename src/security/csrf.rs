use crate::error::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::HeaderMap;
use rand::RngCore;

pub fn generate_csrf_token() -> String {
    let mut buffer = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buffer);
    URL_SAFE_NO_PAD.encode(&buffer)
}

pub fn validate_csrf_token(headers: &HeaderMap) -> Result<()> {
    if let Some(token) = headers.get("X-CSRF-Token") {
        if token.is_empty() {
            return Err(Error::CSRF("Empty CSRF token received".into()));
        }
        Ok(())
    } else {
        Err(Error::CSRF("Missing CSRF token".into()))
    }
}
