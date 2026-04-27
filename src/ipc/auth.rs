use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

use crate::constants::{AUTH_OPERATION_TIMEOUT_SECS, IPC_CONNECTION_TIMEOUT_SECS};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::debug;

use super::{IpcError, Result};

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

/// Perform challenge-response authentication (client side)
pub async fn client_perform_authentication(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    let mut challenge = [0u8; CHALLENGE_SIZE];

    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.read_exact(&mut challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let response = compute_response(auth_key, &challenge);

    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.write_all(&response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let mut result = [0u8; 1];
    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.read_exact(&mut result),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    Ok(result[0] == AUTH_RESULT_SUCCESS)
}

/// Perform challenge-response authentication (server side, with timeout)
pub async fn server_perform_authentication(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    let challenge = generate_challenge();

    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let mut response = [0u8; RESPONSE_SIZE];
    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.read_exact(&mut response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let auth_ok = verify_response(auth_key, &challenge, &response);

    let result_byte = if auth_ok {
        AUTH_RESULT_SUCCESS
    } else {
        AUTH_RESULT_FAILURE
    };
    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&[result_byte]),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    debug!("Server authentication completed, result: {}", auth_ok);
    Ok(auth_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        assert_ne!(challenge1, challenge2);
    }

    #[test]
    fn test_response_computation() {
        let auth_key = "my-secret-key";
        let challenge = generate_challenge();
        let response1 = compute_response(auth_key, &challenge);
        let response2 = compute_response(auth_key, &challenge);
        assert_eq!(response1, response2);
    }

    #[test]
    fn test_response_different_keys() {
        let challenge = generate_challenge();
        let response1 = compute_response("key1", &challenge);
        let response2 = compute_response("key2", &challenge);
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
}
