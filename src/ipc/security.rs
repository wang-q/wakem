use std::net::IpAddr;

/// Check if IP address is private (RFC 1918) or loopback
///
/// IPv6 addresses are intentionally rejected by design:
/// - The IPC server binds to 127.0.0.1 (IPv4 loopback only)
/// - IPv6 is not used in this project to keep the networking layer simple
/// - Any IPv6 connection attempt would not reach the server anyway
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 10
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                || (o[0] == 192 && o[1] == 168)
                || o[0] == 127
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(_) => false,
    }
}

/// Check if IP address is allowed to connect
///
/// Only private IPv4 addresses are allowed (RFC 1918, loopback, link-local).
/// IPv6 is intentionally not supported - see `is_private_ip` for rationale.
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    is_private_ip(ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_private_ip_10() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    }

    #[test]
    fn test_private_ip_172() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 0, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
    }

    #[test]
    fn test_private_ip_192() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 167, 0, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 1))));
    }

    #[test]
    fn test_localhost() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    }

    #[test]
    fn test_public_ip() {
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
    }
}
