//! Authentication utilities for IPC.

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

/// Challenge length (32 bytes)
pub const CHALLENGE_SIZE: usize = 32;

/// Response length (32 bytes, HMAC-SHA256 output)
pub const RESPONSE_SIZE: usize = 32;

/// Authentication result byte: success
pub const AUTH_RESULT_SUCCESS: u8 = 0x01;
/// Authentication result byte: failure
pub const AUTH_RESULT_FAILURE: u8 = 0x00;

/// Generate random challenge
pub fn generate_challenge() -> [u8; CHALLENGE_SIZE] {
    let mut challenge = [0u8; CHALLENGE_SIZE];
    rand::thread_rng().fill_bytes(&mut challenge);
    challenge
}

/// Compute response (HMAC-SHA256)
pub fn compute_response(auth_key: &str, challenge: &[u8]) -> [u8; RESPONSE_SIZE] {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(auth_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(challenge);

    let result = mac.finalize();
    let bytes = result.into_bytes();

    let mut response = [0u8; RESPONSE_SIZE];
    response.copy_from_slice(&bytes[..RESPONSE_SIZE]);
    response
}

/// Verify response using constant-time comparison via hmac crate
pub fn verify_response(auth_key: &str, challenge: &[u8], response: &[u8]) -> bool {
    if response.len() != RESPONSE_SIZE {
        return false;
    }

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = match HmacSha256::new_from_slice(auth_key.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(challenge);
    mac.verify_slice(response).is_ok()
}

/// Zero out a String's memory contents using zeroize crate
///
/// This is used to clear sensitive data (e.g., authentication keys) from memory
/// after use, preventing key material from lingering in heap memory where it
/// could potentially be exposed through memory dumps or core dumps.
///
/// Uses the zeroize crate which provides secure memory clearing that is not
/// optimized away by the compiler.
pub fn zero_string(s: &mut String) {
    use zeroize::Zeroize;
    s.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        assert_ne!(challenge1, challenge2);
        assert_eq!(challenge1.len(), CHALLENGE_SIZE);
    }

    #[test]
    fn test_response_computation() {
        let auth_key = "test_key";
        let challenge = generate_challenge();
        let response = compute_response(auth_key, &challenge);
        assert_eq!(response.len(), RESPONSE_SIZE);
    }

    #[test]
    fn test_response_verification() {
        let auth_key = "test_key";
        let challenge = generate_challenge();
        let response = compute_response(auth_key, &challenge);
        assert!(verify_response(auth_key, &challenge, &response));
    }

    #[test]
    fn test_response_verification_wrong_key() {
        let auth_key = "test_key";
        let wrong_key = "wrong_key";
        let challenge = generate_challenge();
        let response = compute_response(auth_key, &challenge);
        assert!(!verify_response(wrong_key, &challenge, &response));
    }

    #[test]
    fn test_response_verification_wrong_challenge() {
        let auth_key = "test_key";
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        let response = compute_response(auth_key, &challenge1);
        assert!(!verify_response(auth_key, &challenge2, &response));
    }
}
