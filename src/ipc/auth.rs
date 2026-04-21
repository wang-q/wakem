use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

/// Challenge length (32 bytes)
pub const CHALLENGE_SIZE: usize = 32;

/// Response length (32 bytes, HMAC-SHA256 output)
pub const RESPONSE_SIZE: usize = 32;

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

/// Verify response
pub fn verify_response(auth_key: &str, challenge: &[u8], response: &[u8]) -> bool {
    if response.len() != RESPONSE_SIZE {
        return false;
    }

    let expected = compute_response(auth_key, challenge);
    constant_time_eq(&expected, response)
}

/// Constant-time comparison (prevent timing attacks)
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();

        // Two challenges should be different (very low probability of being same)
        assert_ne!(challenge1, challenge2);
    }

    #[test]
    fn test_response_computation() {
        let auth_key = "my-secret-key";
        let challenge = generate_challenge();

        let response1 = compute_response(auth_key, &challenge);
        let response2 = compute_response(auth_key, &challenge);

        // Same key and challenge should produce same response
        assert_eq!(response1, response2);
    }

    #[test]
    fn test_response_different_keys() {
        let challenge = generate_challenge();

        let response1 = compute_response("key1", &challenge);
        let response2 = compute_response("key2", &challenge);

        // Different keys should produce different responses
        assert_ne!(response1, response2);
    }

    #[test]
    fn test_verify_response_success() {
        let auth_key = "test-key";
        let challenge = generate_challenge();
        let response = compute_response(auth_key, &challenge);

        assert!(verify_response(auth_key, &challenge, &response));
    }

    #[test]
    fn test_verify_response_wrong_key() {
        let challenge = generate_challenge();
        let response = compute_response("correct-key", &challenge);

        assert!(!verify_response("wrong-key", &challenge, &response));
    }

    #[test]
    fn test_verify_response_wrong_challenge() {
        let auth_key = "test-key";
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        let response = compute_response(auth_key, &challenge1);

        assert!(!verify_response(auth_key, &challenge2, &response));
    }

    #[test]
    fn test_constant_time_eq() {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 3, 4];
        let c = [1u8, 2, 3, 5];
        let d = [1u8, 2, 3];

        assert!(constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, &c));
        assert!(!constant_time_eq(&a, &d));
    }
}
