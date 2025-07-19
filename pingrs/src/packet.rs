//! ICMP パケット構造体の定義

use crate::error::{PingError, Result};

/// ICMP パケットタイプ
/// 参考: https://www.iana.org/assignments/icmp-parameters/icmp-parameters.xhtml
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpType {
    /// Echo Reply
    EchoReply = 0,
    /// Echo Request
    EchoRequest = 8,
}

impl IcmpType {
    /// u8 から IcmpType への変換
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpType::EchoReply),
            8 => Some(IcmpType::EchoRequest),
            _ => None,
        }
    }
}

/// ICMP パケットヘッダー
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpHeader {
    /// ICMP タイプ
    pub icmp_type: u8,
    /// ICMP コード
    pub code: u8,
    /// チェックサム
    pub checksum: u16,
    /// 識別子
    pub identifier: u16,
    /// シーケンス番号
    pub sequence: u16,
}

impl IcmpHeader {
    /// 新しい ICMP ヘッダーを作成
    pub fn new(icmp_type: IcmpType, identifier: u16, sequence: u16) -> Self {
        IcmpHeader {
            icmp_type: icmp_type as u8,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
        }
    }

    /// バイト配列にシリアライズ
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.icmp_type;
        bytes[1] = self.code;
        bytes[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.identifier.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.sequence.to_be_bytes());
        bytes
    }

    /// バイト配列からデシリアライズ
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(PingError::InvalidPacket(
                "ICMP header must be at least 8 bytes".to_string(),
            ));
        }

        Ok(IcmpHeader {
            icmp_type: bytes[0],
            code: bytes[1],
            checksum: u16::from_be_bytes([bytes[2], bytes[3]]),
            identifier: u16::from_be_bytes([bytes[4], bytes[5]]),
            sequence: u16::from_be_bytes([bytes[6], bytes[7]]),
        })
    }
}

/// ICMP パケット
#[derive(Debug, Clone)]
pub struct IcmpPacket {
    /// ICMP ヘッダー
    pub header: IcmpHeader,
    /// ペイロード
    pub payload: Vec<u8>,
}

impl IcmpPacket {
    /// 新しい Echo Request パケットを作成
    pub fn new_echo_request(identifier: u16, sequence: u16, payload: Vec<u8>) -> Self {
        IcmpPacket {
            header: IcmpHeader::new(IcmpType::EchoRequest, identifier, sequence),
            payload,
        }
    }

    /// バイト配列にシリアライズ（チェックサムは含まない）
    pub fn to_bytes_without_checksum(&self) -> Vec<u8> {
        let header_bytes = self.header.to_bytes();
        let mut bytes = Vec::with_capacity(header_bytes.len() + self.payload.len());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// バイト配列にシリアライズ（チェックサムを計算して含める）
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // チェックサムフィールドを 0 にリセット
        self.header.checksum = 0;
        let bytes = self.to_bytes_without_checksum();

        // チェックサムを計算
        self.header.checksum = crate::checksum::calculate(&bytes);

        // 最終的なバイト配列を生成
        self.to_bytes_without_checksum()
    }

    /// バイト配列からデシリアライズ
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(PingError::InvalidPacket(
                "ICMP packet must be at least 8 bytes".to_string(),
            ));
        }

        let header = IcmpHeader::from_bytes(&bytes[0..8])?;
        let payload = bytes[8..].to_vec();

        Ok(IcmpPacket { header, payload })
    }

    /// チェックサムを検証
    pub fn verify_checksum(&self) -> bool {
        let bytes = self.to_bytes_without_checksum();
        crate::checksum::verify(&bytes)
    }
}
