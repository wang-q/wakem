// Security module extension tests - IP address validation and security boundary conditions

use std::net::{IpAddr, Ipv4Addr};
use wakem::ipc::security::{is_allowed_ip, is_private_ip};

// ==================== RFC 1918 private address range complete tests ====================

/// Test 10.0.0.0/8 range
#[test]
fn test_10_range() {
    // Within range
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 128, 64, 32))));

    // Outside boundary
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(9, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 0))));
}

/// Test 172.16.0.0/12 range (boundary values)
#[test]
fn test_172_range_boundary() {
    // Range start boundary (172.16.x.x)
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));

    // Range end boundary (172.31.x.x)
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));

    // Just outside range - 172.15.x.x
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 0, 0))));

    // Just outside range - 172.32.x.x
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 0))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));

    // Range middle values
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 20, 100, 50))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 24, 0, 1))));
}

/// Test 192.168.0.0/16 range
#[test]
fn test_192_168_range() {
    // Within range
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));

    // Outside boundary
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        192, 167, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 0))));
}

// ==================== Special private addresses ====================

/// Test localhost (127.0.0.0/8) range
#[test]
fn test_localhost_range() {
    // Standard localhost
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

    // Other addresses in 127.x.x.x range
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 1, 1, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));

    // Outside boundary
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        126, 255, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(128, 0, 0, 0))));
}

/// Test link-local address (169.254.0.0/16)
#[test]
fn test_link_local() {
    // Link-local within range
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 128, 128))));

    // Outside boundary
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        169, 253, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 255, 0, 0))));
}

// ==================== Public IP address tests ====================

/// Test common public DNS and service IPs
#[test]
fn test_public_ips() {
    // Google DNS
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4))));

    // Cloudflare DNS
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

    // Common public IPs
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)))); // Google
}

/// Test broadcast address and network addresses
#[test]
fn test_special_addresses() {
    // 0.0.0.0 - Usually used for listening on all interfaces, not a private address
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));

    // 255.255.255.255 - Broadcast Address
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        255, 255, 255, 255
    ))));
}

// ==================== is_allowed_ip function tests ====================

/// Test is_allowed_ip is an alias for is_private_ip
#[test]
fn test_is_allowed_ip_alias() {
    // Private addresses should be allowed
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

    // Public addresses should be rejected
    assert!(!is_allowed_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_allowed_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
}

// ==================== Boundary condition combination tests ====================

/// Test all private range boundaries combination
#[test]
fn test_all_private_ranges_boundaries() {
    // 10.x.x.x boundaries
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(9, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 0))));

    // 172.16.x.x - 172.31.x.x boundaries
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 0))));

    // 192.168.x.x boundaries
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        192, 167, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 0))));

    // 127.x.x.x (localhost) boundaries
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        126, 255, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(128, 0, 0, 0))));

    // 169.254.x.x (link-local) boundaries
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        169, 253, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 255, 0, 0))));
}
