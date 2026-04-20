use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

/// 挑战长度（32 字节）
pub const CHALLENGE_SIZE: usize = 32;

/// 响应长度（32 字节，HMAC-SHA256 输出）
pub const RESPONSE_SIZE: usize = 32;

/// 生成随机挑战
pub fn generate_challenge() -> [u8; CHALLENGE_SIZE] {
    let mut challenge = [0u8; CHALLENGE_SIZE];
    rand::thread_rng().fill_bytes(&mut challenge);
    challenge
}

/// 计算响应（HMAC-SHA256）
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

/// 验证响应
pub fn verify_response(auth_key: &str, challenge: &[u8], response: &[u8]) -> bool {
    if response.len() != RESPONSE_SIZE {
        return false;
    }

    let expected = compute_response(auth_key, challenge);
    constant_time_eq(&expected, response)
}

/// 常量时间比较（防止时序攻击）
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

        // 两个挑战应该不同（概率极低会相同）
        assert_ne!(challenge1, challenge2);
    }

    #[test]
    fn test_response_computation() {
        let auth_key = "my-secret-key";
        let challenge = generate_challenge();

        let response1 = compute_response(auth_key, &challenge);
        let response2 = compute_response(auth_key, &challenge);

        // 相同的密钥和挑战应该产生相同的响应
        assert_eq!(response1, response2);
    }

    #[test]
    fn test_response_different_keys() {
        let challenge = generate_challenge();

        let response1 = compute_response("key1", &challenge);
        let response2 = compute_response("key2", &challenge);

        // 不同的密钥应该产生不同的响应
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
