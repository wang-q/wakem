// Security 模块扩展测试 - IP 地址验证和安全性边界条件

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use wakem::ipc::security::{is_allowed_ip, is_private_ip};

// ==================== RFC 1918 私有地址范围完整测试 ====================

/// 测试 10.0.0.0/8 范围
#[test]
fn test_10_range() {
    // 范围内
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 128, 64, 32))));

    // 边界外
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(9, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 0))));
}

/// 测试 172.16.0.0/12 范围（边界值）
#[test]
fn test_172_range_boundary() {
    // 范围起始边界 (172.16.x.x)
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));

    // 范围结束边界 (172.31.x.x)
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));

    // 刚好在范围外 - 172.15.x.x
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 0, 0))));

    // 刚好在范围外 - 172.32.x.x
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 0))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));

    // 范围中间值
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 20, 100, 50))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 24, 0, 1))));
}

/// 测试 192.168.0.0/16 范围
#[test]
fn test_192_168_range() {
    // 范围内
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));

    // 边界外
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        192, 167, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 0))));
}

// ==================== 特殊私有地址 ====================

/// 测试 localhost (127.0.0.0/8) 范围
#[test]
fn test_localhost_range() {
    // 标准 localhost
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

    // 127.x.x.x 范围内的其他地址
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 1, 1, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));

    // 边界外
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        126, 255, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(128, 0, 0, 0))));
}

/// 测试 link-local 地址 (169.254.0.0/16)
#[test]
fn test_link_local() {
    // link-local 范围内
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 128, 128))));

    // 边界外
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        169, 253, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 255, 0, 0))));
}

// ==================== 公共 IP 地址测试 ====================

/// 测试常见公共 DNS 和服务 IP
#[test]
fn test_public_ips() {
    // Google DNS
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4))));

    // Cloudflare DNS
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

    // 常见公共 IP
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)))); // Google
}

/// 测试广播地址和网络地址
#[test]
fn test_special_addresses() {
    // 0.0.0.0 - 通常用于监听所有接口，不是私有地址
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));

    // 255.255.255.255 - 广播地址
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        255, 255, 255, 255
    ))));
}

// ==================== IPv6 测试 ====================

/// 测试 IPv6 地址（应该全部被拒绝）
#[test]
fn test_ipv6_not_private() {
    let ipv6 = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1); // ::1 (localhost)
    assert!(!is_private_ip(IpAddr::V6(ipv6)));

    let ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1); // fe80::1 (link-local)
    assert!(!is_private_ip(IpAddr::V6(ipv6)));

    let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1); // 2001:db8::1 (documentation)
    assert!(!is_private_ip(IpAddr::V6(ipv6)));

    let ipv6 = Ipv6Addr::new(0xfc00, 1, 0, 0, 0, 0, 0, 1); // fc00:1::1 (unique local)
    assert!(!is_private_ip(IpAddr::V6(ipv6)));
}

// ==================== is_allowed_ip 函数测试 ====================

/// 测试 is_allowed_ip 是 is_private_ip 的别名
#[test]
fn test_is_allowed_ip_alias() {
    // 私有地址应该被允许
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_allowed_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

    // 公共地址应该被拒绝
    assert!(!is_allowed_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_allowed_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

    // IPv6 应该被拒绝
    assert!(!is_allowed_ip(IpAddr::V6(Ipv6Addr::new(
        0, 0, 0, 0, 0, 0, 0, 1,
    ))));
}

// ==================== 边界情况组合测试 ====================

/// 测试所有私有范围的边界组合
#[test]
fn test_all_private_ranges_boundaries() {
    // 10.x.x.x 的边界
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(9, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 0))));

    // 172.16.x.x - 172.31.x.x 的边界
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 0))));

    // 192.168.x.x 的边界
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        192, 167, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 0))));

    // 127.x.x.x (localhost) 的边界
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        126, 255, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(128, 0, 0, 0))));

    // 169.254.x.x (link-local) 的边界
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 0))));
    assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(
        169, 253, 255, 255
    ))));
    assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 255, 0, 0))));
}
