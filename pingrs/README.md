# pingrs

Pure Rust の ICMP ping ライブラリ

## 特徴

- 外部コマンドに依存しない
- 最小限の依存関係（libc, socket2 のみ）
- クロスプラットフォーム対応（Linux, macOS, Windows）

## インストール

```toml
[dependencies]
pingrs = "0.1"
```

## 使用例

```rust
use pingrs::ping;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let results = ping("google.com", 4)?;
    
    for result in results {
        match result {
            Ok(r) => println!("Reply: time={:.1}ms", r.rtt.as_secs_f64() * 1000.0),
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## 権限要件

Raw ソケットを使用するため、プラットフォームごとに以下の権限が必要です：

- **Linux**: root 権限または CAP_NET_RAW capability
- **macOS**: root 権限
- **Windows**: 管理者権限

## ライセンス

MIT OR Apache-2.0