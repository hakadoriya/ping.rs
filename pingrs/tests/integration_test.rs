//! pingrs の統合テスト

use pingrs::{IcmpPacket, IcmpType, PingError, Pinger};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[test]
fn test_icmp_packet_serialization() {
    let identifier = 12345;
    let sequence = 1;
    let payload = b"Hello, ICMP!".to_vec();

    // パケットを作成
    let mut packet = IcmpPacket::new_echo_request(identifier, sequence, payload.clone());

    // シリアライズ
    let bytes = packet.to_bytes();

    // デシリアライズ
    let deserialized = IcmpPacket::from_bytes(&bytes).expect("Failed to deserialize");

    // 検証
    assert_eq!(deserialized.header.icmp_type, IcmpType::EchoRequest as u8);
    assert_eq!(deserialized.header.code, 0);
    assert_eq!(deserialized.header.identifier, identifier);
    assert_eq!(deserialized.header.sequence, sequence);
    assert_eq!(deserialized.payload, payload);

    // チェックサムの検証
    assert!(deserialized.verify_checksum());
}

#[test]
fn test_ping_localhost() {
    // localhost に対する ping をテスト
    // 注意: このテストは root/管理者権限が必要
    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_addr = SocketAddr::new(localhost, 0);

    match Pinger::new(&localhost) {
        Ok(mut pinger) => {
            // タイムアウトを設定
            pinger.set_timeout(Duration::from_secs(1)).unwrap();

            // ping を実行
            match pinger.ping(&socket_addr, None) {
                Ok(result) => {
                    println!("Ping successful: {:?}", result);
                    assert_eq!(result.addr, localhost);
                    assert!(result.rtt.as_millis() < 1000); // 1秒以内
                }
                Err(e) => {
                    // OS によっては localhost への ping が失敗することがある
                    println!("Ping failed: {}", e);
                }
            }
        }
        Err(PingError::PermissionDenied) => {
            println!("Skipping test: Permission denied (needs root/admin)");
        }
        Err(e) => {
            panic!("Unexpected error creating pinger: {}", e);
        }
    }
}

#[test]
fn test_resolve_addr() {
    // 既知のホスト名を解決
    let addrs = [
        ("localhost", true),
        ("127.0.0.1", true),
        ("::1", true),
        ("invalid-host-name-12345.invalid", false),
    ];

    for (host, should_succeed) in &addrs {
        let result = pingrs::ping::resolve_addr(host);
        if *should_succeed {
            assert!(result.is_ok(), "Failed to resolve {}", host);
        } else {
            assert!(result.is_err(), "Should not resolve {}", host);
        }
    }
}

#[test]
fn test_ping_api() {
    // 高レベル API のテスト
    match pingrs::ping("127.0.0.1", 1) {
        Ok(results) => {
            assert_eq!(results.len(), 1);
            for result in results {
                match result {
                    Ok(r) => println!("Ping result: {:?}", r),
                    Err(e) => println!("Ping error: {}", e),
                }
            }
        }
        Err(PingError::PermissionDenied) => {
            println!("Skipping test: Permission denied (needs root/admin)");
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

#[test]
fn test_error_display() {
    // エラー型の Display trait をテスト
    let errors = vec![
        PingError::Timeout,
        PingError::PermissionDenied,
        PingError::InvalidPacket("Test error".to_string()),
        PingError::InvalidArgument("Test argument".to_string()),
        PingError::UnsupportedPlatform,
    ];

    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty());
    }
}
