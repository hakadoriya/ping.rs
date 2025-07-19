//! Ping クライアントの実装

use crate::error::{PingError, Result};
use crate::packet::IcmpPacket;
use crate::socket::IcmpSocket;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Ping の結果
#[derive(Debug, Clone)]
pub struct PingResult {
    /// 送信先アドレス
    pub addr: IpAddr,
    /// シーケンス番号
    pub sequence: u16,
    /// ラウンドトリップタイム (RTT)
    pub rtt: Duration,
    /// 受信したパケットのサイズ
    pub size: usize,
}

/// Ping クライアント
pub struct Pinger {
    socket: IcmpSocket,
    identifier: u16,
    sequence: u16,
    timeout: Duration,
}

impl Pinger {
    /// 新しい Pinger を作成
    ///
    /// # Arguments
    /// * `addr` - ping の送信先アドレス
    ///
    /// # Example
    /// ```no_run
    /// use pingrs::Pinger;
    /// use std::net::IpAddr;
    ///
    /// let addr: IpAddr = "8.8.8.8".parse().unwrap();
    /// let pinger = Pinger::new(&addr).expect("Failed to create pinger");
    /// ```
    pub fn new(addr: &IpAddr) -> Result<Self> {
        let socket = IcmpSocket::new(addr)?;

        // プロセス ID を identifier として使用
        let identifier = std::process::id() as u16;

        Ok(Pinger {
            socket,
            identifier,
            sequence: 0,
            timeout: Duration::from_secs(5),
        })
    }

    /// タイムアウトを設定
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.timeout = timeout;
        self.socket.set_timeout(Some(timeout))?;
        Ok(())
    }

    /// 単一の ping を送信
    ///
    /// # Arguments
    /// * `addr` - 送信先アドレス
    /// * `payload` - ペイロードデータ（オプション）
    ///
    /// # Returns
    /// ping の結果
    pub fn ping(&mut self, addr: &SocketAddr, payload: Option<Vec<u8>>) -> Result<PingResult> {
        // シーケンス番号をインクリメント
        self.sequence = self.sequence.wrapping_add(1);

        // ペイロードの準備（デフォルトは "pingrs" + タイムスタンプ）
        let payload = payload.unwrap_or_else(|| {
            let mut data = b"pingrs".to_vec();
            let now = Instant::now();
            data.extend_from_slice(&now.elapsed().as_nanos().to_le_bytes());
            data
        });

        // Echo Request パケットを作成
        let mut packet =
            IcmpPacket::new_echo_request(self.identifier, self.sequence, payload.clone());

        // 送信時刻を記録
        let start = Instant::now();

        // パケットを送信
        self.socket.send_to(&mut packet, addr)?;

        // Echo Reply を待機
        let (reply_packet, _reply_addr) = self.socket.recv_echo_reply(
            self.identifier,
            self.sequence,
            65535, // 最大パケットサイズ
        )?;

        // RTT を計算
        let rtt = start.elapsed();

        Ok(PingResult {
            addr: addr.ip(),
            sequence: self.sequence,
            rtt,
            size: reply_packet.payload.len(),
        })
    }

    /// 複数回 ping を送信
    ///
    /// # Arguments
    /// * `addr` - 送信先アドレス
    /// * `count` - 送信回数
    /// * `interval` - 送信間隔
    ///
    /// # Returns
    /// 各 ping の結果のベクター
    pub fn ping_multiple(
        &mut self,
        addr: &SocketAddr,
        count: usize,
        interval: Duration,
    ) -> Vec<Result<PingResult>> {
        let mut results = Vec::with_capacity(count);

        for i in 0..count {
            // 最初以外は間隔を空ける
            if i > 0 {
                std::thread::sleep(interval);
            }

            results.push(self.ping(addr, None));
        }

        results
    }
}

/// アドレスを解決して最初の IP アドレスを取得
pub fn resolve_addr(host: &str) -> Result<IpAddr> {
    // ポート番号を付けてアドレス解決を試みる
    let host_with_port = format!("{host}:0");
    let addrs: Vec<_> = host_with_port
        .to_socket_addrs()
        .map_err(|e| PingError::InvalidArgument(format!("Failed to resolve host: {e}")))?
        .collect();

    if addrs.is_empty() {
        return Err(PingError::InvalidArgument(
            "No addresses found for host".to_string(),
        ));
    }

    Ok(addrs[0].ip())
}

/// 簡易的な ping 関数
///
/// # Arguments
/// * `host` - ホスト名または IP アドレス
/// * `count` - ping の回数
///
/// # Example
/// ```no_run
/// use pingrs::ping;
///
/// let results = ping("google.com", 4).expect("Failed to ping");
/// for result in results {
///     match result {
///         Ok(r) => println!("{:?}", r),
///         Err(e) => println!("Error: {}", e),
///     }
/// }
/// ```
pub fn ping(host: &str, count: usize) -> Result<Vec<Result<PingResult>>> {
    // アドレスを解決
    let ip = if let Ok(ip) = host.parse::<IpAddr>() {
        ip
    } else {
        resolve_addr(host)?
    };

    // ソケットアドレスを作成（ICMP はポート番号を使用しないが、形式上必要）
    let socket_addr = SocketAddr::new(ip, 0);

    // Pinger を作成
    let mut pinger = Pinger::new(&ip)?;

    // ping を実行
    Ok(pinger.ping_multiple(&socket_addr, count, Duration::from_secs(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_addr() {
        // localhost は常に解決できるはず
        let addr = resolve_addr("localhost");
        assert!(addr.is_ok());
    }

    #[test]
    fn test_resolve_invalid_addr() {
        let addr = resolve_addr("this-should-not-exist-12345.invalid");
        assert!(addr.is_err());
    }
}
