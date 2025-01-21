use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;

pub fn generate_csrf_token() -> String {
    let mut buffer = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buffer);
    URL_SAFE_NO_PAD.encode(&buffer)
}
