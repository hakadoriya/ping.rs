//! Raw ソケット操作の実装

use crate::error::{PingError, Result};
use crate::packet::{IcmpPacket, IcmpType};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

/// ICMP ソケットのラッパー
pub struct IcmpSocket {
    socket: Socket,
    is_ipv6: bool,
}

impl IcmpSocket {
    /// IPv4 用の ICMP ソケットを作成
    pub fn new_v4() -> Result<Self> {
        let socket = Socket::new(
            Domain::IPV4,
            Type::from(libc::SOCK_RAW),
            Some(Protocol::ICMPV4),
        )
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                PingError::PermissionDenied
            } else {
                PingError::Io(e)
            }
        })?;

        // ソケットオプションの設定
        socket.set_nonblocking(false)?;

        Ok(IcmpSocket {
            socket,
            is_ipv6: false,
        })
    }

    /// IPv6 用の ICMP ソケットを作成
    pub fn new_v6() -> Result<Self> {
        let socket = Socket::new(
            Domain::IPV6,
            Type::from(libc::SOCK_RAW),
            Some(Protocol::ICMPV6),
        )
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                PingError::PermissionDenied
            } else {
                PingError::Io(e)
            }
        })?;

        // ソケットオプションの設定
        socket.set_nonblocking(false)?;

        Ok(IcmpSocket {
            socket,
            is_ipv6: true,
        })
    }

    /// IP アドレスに基づいて適切な ICMP ソケットを作成
    pub fn new(addr: &IpAddr) -> Result<Self> {
        match addr {
            IpAddr::V4(_) => Self::new_v4(),
            IpAddr::V6(_) => Self::new_v6(),
        }
    }

    /// 受信タイムアウトを設定
    pub fn set_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        self.socket.set_read_timeout(timeout)?;
        Ok(())
    }

    /// ICMP パケットを送信
    pub fn send_to(&self, packet: &mut IcmpPacket, addr: &SocketAddr) -> Result<usize> {
        let bytes = packet.to_bytes();
        let sock_addr = SockAddr::from(*addr);
        let sent = self.socket.send_to(&bytes, &sock_addr)?;
        Ok(sent)
    }

    /// ICMP パケットを受信
    ///
    /// 戻り値: (受信したパケット, 送信元アドレス)
    pub fn recv_from(&self, buffer_size: usize) -> Result<(IcmpPacket, SocketAddr)> {
        let mut buffer = vec![MaybeUninit::uninit(); buffer_size];
        let (size, addr) = self.socket.recv_from(&mut buffer)?;

        if size == 0 {
            return Err(PingError::InvalidPacket(
                "Empty packet received".to_string(),
            ));
        }

        // MaybeUninit<u8> を u8 に変換
        let buffer: Vec<u8> = buffer
            .into_iter()
            .take(size)
            .map(|b| unsafe { b.assume_init() })
            .collect();

        // IPv4 の場合、IP ヘッダーをスキップする必要がある場合がある
        // (OS によって動作が異なる: Linux は IP ヘッダーを含む、macOS は含まない)
        let icmp_start = if !self.is_ipv6 && buffer.len() >= 20 {
            // IP ヘッダーの可能性をチェック
            let version = (buffer[0] >> 4) & 0xf;
            if version == 4 {
                // IPv4 ヘッダーが存在する
                let ihl = (buffer[0] & 0xf) as usize;
                ihl * 4 // IHL は 32 ビット単位
            } else {
                0
            }
        } else {
            0
        };

        if icmp_start >= buffer.len() {
            return Err(PingError::InvalidPacket(
                "Packet too small after IP header".to_string(),
            ));
        }

        let icmp_data = &buffer[icmp_start..];
        let packet = IcmpPacket::from_bytes(icmp_data)?;

        // チェックサムを検証
        if !packet.verify_checksum() {
            // デバッグ: 受信したICMPデータを確認
            eprintln!("\n=== Checksum Error Debug ===");
            eprintln!("ICMP Type: {} ({})", packet.header.icmp_type, 
                if packet.header.icmp_type == 0 { "Echo Reply" } 
                else if packet.header.icmp_type == 8 { "Echo Request" } 
                else { "Other" });
            eprintln!("ICMP Code: {}", packet.header.code);
            eprintln!("Identifier: 0x{:04x}", packet.header.identifier);
            eprintln!("Sequence: {}", packet.header.sequence);
            eprintln!("Checksum in packet: 0x{:04x}", packet.header.checksum);
            
            // チェックサムを0にしてから再計算
            let mut test_bytes = Vec::with_capacity(8 + packet.payload.len());
            let mut test_header = packet.header;
            test_header.checksum = 0;
            test_bytes.extend_from_slice(&test_header.to_bytes());
            test_bytes.extend_from_slice(&packet.payload);
            let calculated = crate::checksum::calculate(&test_bytes);
            eprintln!("Calculated checksum: 0x{:04x}", calculated);
            
            // 全体のチェックサム
            let mut full_bytes = Vec::with_capacity(8 + packet.payload.len());
            full_bytes.extend_from_slice(&packet.header.to_bytes());
            full_bytes.extend_from_slice(&packet.payload);
            let verify_result = crate::checksum::calculate(&full_bytes);
            eprintln!("Verify result (should be 0): 0x{:04x}", verify_result);
            
            eprintln!("Payload size: {} bytes", packet.payload.len());
            eprintln!("First 32 bytes of ICMP: {:02x?}", &icmp_data[..icmp_data.len().min(32)]);
            eprintln!("==========================\n");
            
            return Err(PingError::InvalidPacket("Invalid checksum".to_string()));
        }

        // SockAddr を SocketAddr に変換
        let socket_addr = addr
            .as_socket()
            .ok_or_else(|| PingError::InvalidPacket("Invalid socket address".to_string()))?;

        Ok((packet, socket_addr))
    }

    /// Echo Reply を待機
    ///
    /// 指定された identifier と sequence を持つ Echo Reply を受信するまで待機
    pub fn recv_echo_reply(
        &self,
        expected_identifier: u16,
        expected_sequence: u16,
        buffer_size: usize,
    ) -> Result<(IcmpPacket, SocketAddr)> {
        loop {
            // チェックサム検証をせずにパケットを受信
            let (packet, addr) = self.recv_from_without_verify(buffer_size)?;

            // Echo Reply かつ期待する ID/Seq であることを確認
            if packet.header.icmp_type == IcmpType::EchoReply as u8
                && packet.header.identifier == expected_identifier
                && packet.header.sequence == expected_sequence
            {
                // Echo Reply の場合のみチェックサムを検証
                if !packet.verify_checksum() {
                    // デバッグ: 受信したICMPデータを確認
                    eprintln!("\n=== Checksum Error Debug (Echo Reply) ===");
                    eprintln!("Identifier: 0x{:04x}", packet.header.identifier);
                    eprintln!("Sequence: {}", packet.header.sequence);
                    eprintln!("Checksum in packet: 0x{:04x}", packet.header.checksum);
                    
                    // チェックサムを0にしてから再計算
                    let mut test_bytes = Vec::with_capacity(8 + packet.payload.len());
                    let mut test_header = packet.header;
                    test_header.checksum = 0;
                    test_bytes.extend_from_slice(&test_header.to_bytes());
                    test_bytes.extend_from_slice(&packet.payload);
                    let calculated = crate::checksum::calculate(&test_bytes);
                    eprintln!("Calculated checksum: 0x{:04x}", calculated);
                    eprintln!("==========================\n");
                    
                    return Err(PingError::InvalidPacket("Invalid checksum".to_string()));
                }
                return Ok((packet, addr));
            }
            // それ以外のパケットは無視して次を待つ
        }
    }

    /// ICMP パケットを受信（チェックサム検証なし）
    fn recv_from_without_verify(&self, buffer_size: usize) -> Result<(IcmpPacket, SocketAddr)> {
        let mut buffer = vec![MaybeUninit::uninit(); buffer_size];
        let (size, addr) = self.socket.recv_from(&mut buffer)?;

        if size == 0 {
            return Err(PingError::InvalidPacket(
                "Empty packet received".to_string(),
            ));
        }

        // MaybeUninit<u8> を u8 に変換
        let buffer: Vec<u8> = buffer
            .into_iter()
            .take(size)
            .map(|b| unsafe { b.assume_init() })
            .collect();

        // IPv4 の場合、IP ヘッダーをスキップする必要がある場合がある
        // (OS によって動作が異なる: Linux は IP ヘッダーを含む、macOS は含まない)
        let icmp_start = if !self.is_ipv6 && buffer.len() >= 20 {
            // IP ヘッダーの可能性をチェック
            let version = (buffer[0] >> 4) & 0xf;
            if version == 4 {
                // IPv4 ヘッダーが存在する
                let ihl = (buffer[0] & 0xf) as usize;
                ihl * 4 // IHL は 32 ビット単位
            } else {
                0
            }
        } else {
            0
        };

        if icmp_start >= buffer.len() {
            return Err(PingError::InvalidPacket(
                "Packet too small after IP header".to_string(),
            ));
        }

        let icmp_data = &buffer[icmp_start..];
        let packet = IcmpPacket::from_bytes(icmp_data)?;

        // SockAddr を SocketAddr に変換
        let socket_addr = addr
            .as_socket()
            .ok_or_else(|| PingError::InvalidPacket("Invalid socket address".to_string()))?;

        Ok((packet, socket_addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_socket_creation() {
        // 注意: このテストは root/管理者権限が必要
        match IcmpSocket::new_v4() {
            Ok(_) => {
                // 権限がある場合はソケット作成成功
            }
            Err(PingError::PermissionDenied) => {
                // 権限がない場合は想定通りのエラー
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_socket_for_addr() {
        let ipv4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        match IcmpSocket::new(&ipv4) {
            Ok(socket) => {
                assert!(!socket.is_ipv6);
            }
            Err(PingError::PermissionDenied) => {
                // 権限がない場合は想定通りのエラー
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}
