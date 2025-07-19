//! エラー型の定義

use std::error::Error;
use std::fmt;
use std::io;

/// pingrs ライブラリのエラー型
#[derive(Debug)]
pub enum PingError {
    /// I/O エラー
    Io(io::Error),
    /// パケット解析エラー
    InvalidPacket(String),
    /// タイムアウト
    Timeout,
    /// 権限不足
    PermissionDenied,
    /// 不正な引数
    InvalidArgument(String),
    /// プラットフォームがサポートされていない
    UnsupportedPlatform,
}

impl fmt::Display for PingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PingError::Io(err) => write!(f, "I/O error: {err}"),
            PingError::InvalidPacket(msg) => write!(f, "Invalid packet: {msg}"),
            PingError::Timeout => write!(f, "Operation timed out"),
            PingError::PermissionDenied => write!(
                f,
                "Permission denied (raw socket requires root/admin privileges)"
            ),
            PingError::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            PingError::UnsupportedPlatform => write!(f, "Unsupported platform"),
        }
    }
}

impl Error for PingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PingError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for PingError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::PermissionDenied => PingError::PermissionDenied,
            io::ErrorKind::TimedOut => PingError::Timeout,
            _ => PingError::Io(err),
        }
    }
}

/// 結果型のエイリアス
pub type Result<T> = std::result::Result<T, PingError>;
