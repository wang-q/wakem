//! IPC (Inter-Process Communication) module.
//!
//! Provides client/server communication for wakem daemon control.
//!
//! Features:
//! - TCP-based message protocol with JSON serialization
//! - Challenge-response authentication (HMAC-SHA256)
//! - IP whitelist (only private IPv4 addresses)
//! - Connection rate limiting (prevent brute force)
//! - Instance discovery (scan for running instances)

pub mod auth;
pub mod client;
pub mod discovery;
pub mod io;
pub mod messages;
pub mod rate_limiter;
pub mod security;
pub mod server;

// Re-export commonly used types
pub use client::IpcClient;
pub use discovery::discover_instances;
pub use io::{get_instance_address, get_instance_port};
pub use messages::{IpcError, Message};
pub use server::IpcServer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::auth::{
        compute_response, generate_challenge, verify_response, CHALLENGE_SIZE,
        RESPONSE_SIZE,
    };
    use crate::ipc::security::is_private_ip;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_private_ip() {
        assert!(is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            10, 0, 0, 1
        ))));
        assert!(is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            172, 16, 0, 1
        ))));
        assert!(is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            192, 168, 1, 1
        ))));
        assert!(is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            127, 0, 0, 1
        ))));
        assert!(is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            169, 254, 0, 1
        ))));
    }

    #[test]
    fn test_public_ip() {
        assert!(!is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            8, 8, 8, 8
        ))));
        assert!(!is_private_ip(std::net::IpAddr::V4(Ipv4Addr::new(
            1, 1, 1, 1
        ))));
    }

    #[test]
    fn test_ipv6_rejected() {
        assert!(!is_private_ip(std::net::IpAddr::V6(Ipv6Addr::new(
            0, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(!is_private_ip(std::net::IpAddr::V6(Ipv6Addr::new(
            0xfe80, 0, 0, 0, 0, 0, 0, 1
        ))));
    }

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

    #[test]
    fn test_get_instance_port() {
        use crate::constants::IPC_BASE_PORT;
        assert_eq!(get_instance_port(0), IPC_BASE_PORT);
        assert_eq!(get_instance_port(1), IPC_BASE_PORT + 1);
        assert_eq!(get_instance_port(255), IPC_BASE_PORT + 255);
    }

    #[test]
    fn test_get_instance_address() {
        use crate::constants::IPC_BASE_PORT;
        assert_eq!(
            get_instance_address(0),
            format!("127.0.0.1:{}", IPC_BASE_PORT)
        );
        assert_eq!(
            get_instance_address(1),
            format!("127.0.0.1:{}", IPC_BASE_PORT + 1)
        );
    }
}
