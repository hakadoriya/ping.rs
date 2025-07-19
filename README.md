# ping.rs

このリポジトリは2つのクレートを含むワークスペースです：

- **pingrs** - Pure Rust の ICMP ping ライブラリ（依存関係最小）
- **pingrs-cli** - ping コマンドラインツール

ICMP を直接扱う pure Rust の ping ライブラリ

## 概要

このライブラリは、外部の ping コマンドに依存せず、ICMP プロトコルを直接実装して ping 機能を提供します。

## 設計方針

- Pure Rust での実装
- 外部依存を最小限に抑える
- 安全性と効率性を重視
- クロスプラットフォーム対応（Linux, macOS, Windows）

## アーキテクチャ

### レイヤー構造

1. **低レベル層**
   - Raw ソケット操作
   - システムコールのラッパー

2. **プロトコル層**
   - ICMP パケット構造体
   - チェックサム計算
   - パケットのシリアライズ/デシリアライズ

3. **アプリケーション層**
   - Ping クライアント API
   - 非同期/同期インターフェース
   - エラーハンドリング

## インストール

### ライブラリとして使用

```toml
[dependencies]
pingrs = "0.1"
```

非同期機能を使用する場合：

```toml
[dependencies]
pingrs = { version = "0.1", features = ["async"] }
```

### CLIツールとして使用

```bash
cargo install pingrs-cli
```

## 使用方法

### ライブラリ：高レベル API（同期版）

```rust
use pingrs::ping;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // google.com に 4 回 ping を送信
    let results = ping("google.com", 4)?;
    
    for result in results {
        match result {
            Ok(r) => println!("Reply from {}: time={:.1}ms", r.addr, r.rtt.as_secs_f64() * 1000.0),
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### ライブラリ：高レベル API（非同期版）

```rust
use pingrs::ping_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // google.com に 4 回 ping を送信
    let results = ping_async("google.com", 4).await?;
    
    for result in results {
        match result {
            Ok(r) => println!("Reply from {}: time={:.1}ms", r.addr, r.rtt.as_secs_f64() * 1000.0),
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### ライブラリ：低レベル API

```rust
use pingrs::{Pinger, PingError};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

fn main() -> Result<(), PingError> {
    let addr: IpAddr = "8.8.8.8".parse().unwrap();
    let mut pinger = Pinger::new(&addr)?;
    
    // タイムアウトを設定
    pinger.set_timeout(Duration::from_secs(2))?;
    
    // ping を送信
    let socket_addr = SocketAddr::new(addr, 0);
    let result = pinger.ping(&socket_addr, None)?;
    
    println!("RTT: {:.1}ms", result.rtt.as_secs_f64() * 1000.0);
    
    Ok(())
}
```

## 権限要件

Raw ソケットを使用するため、プラットフォームごとに以下の権限が必要です：

- **Linux**: root 権限または CAP_NET_RAW capability
- **macOS**: root 権限
- **Windows**: 管理者権限

### Linux での権限設定例

```bash
# バイナリに capability を付与
sudo setcap cap_net_raw+ep target/debug/examples/simple_ping

# または sudo で実行
sudo cargo run --example simple_ping google.com
```

## サンプルの実行

```bash
# サンプルをビルド
cargo build --example simple_ping

# 実行（要権限）
sudo ./target/debug/examples/simple_ping google.com
```

## CLIツール（pingrs-cli）の使用

### インストール

```bash
# ソースからビルド
cd pingrs-cli
cargo build --release

# または cargo install
cargo install pingrs-cli
```

### 使用例

```bash
# 基本的な使用
sudo ping google.com

# 4回だけ ping
sudo ping -c 4 google.com

# 0.5秒間隔で ping
sudo ping -i 0.5 8.8.8.8

# ヘルプ表示
ping -h
```

## ライセンス

MIT OR Apache-2.0
