use std::net::IpAddr;

/// Check if IP address is private (RFC 1918) or loopback
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            // 10.0.0.0/8
            o[0] == 10
                // 172.16.0.0/12
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                // 192.168.0.0/16
                || (o[0] == 192 && o[1] == 168)
                // 127.0.0.0/8 (localhost)
                || o[0] == 127
                // 169.254.0.0/16 (link-local)
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(v6) => {
            // Allow IPv6 loopback (::1) and link-local addresses (fe80::/10)
            let segments = v6.segments();
            // ::1 (loopback)
            segments == [0, 0, 0, 0, 0, 0, 0, 1]
                // fe80::/10 (link-local)
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Check if IP address is allowed to connect
/// Only private IP addresses are allowed
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    is_private_ip(ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

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

    #[test]
    fn test_ipv6_loopback() {
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn test_ipv6_link_local() {
        // fe80::/10
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::new(0xfebf, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff))));
    }

    #[test]
    fn test_ipv6_public() {
        // 2001::/16 (Global unicast)
        assert!(!is_private_ip(IpAddr::V6(Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 1))));
        // 2606::/32 (Cloudflare)
        assert!(!is_private_ip(IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 1))));
    }
}
