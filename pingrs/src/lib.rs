//! # pingrs
//!
//! ICMP を直接扱う pure Rust の ping ライブラリ
//!
//! このライブラリは、外部の ping コマンドに依存せず、
//! ICMP プロトコルを直接実装して ping 機能を提供します。

#![warn(missing_docs)]

pub mod checksum;
pub mod error;
pub mod packet;
pub mod ping;
pub mod socket;

pub use error::{PingError, Result};
pub use packet::{IcmpHeader, IcmpPacket, IcmpType};
pub use ping::{ping, PingResult, Pinger};
pub use socket::IcmpSocket;

/// ライブラリのバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
