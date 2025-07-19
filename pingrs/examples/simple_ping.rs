//! pingrs の簡単な使用例

use pingrs::{ping, PingError, Pinger};
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

fn main() {
    // コマンドライン引数からホスト名を取得
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host>", args[0]);
        eprintln!("Example: {} google.com", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    println!("PING {} ...", host);

    // 高レベル API を使用した例
    match simple_ping_example(host) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n--- Advanced example ---\n");

    // 低レベル API を使用した例
    match advanced_ping_example(host) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// 高レベル API を使用した簡単な ping の例
fn simple_ping_example(host: &str) -> Result<(), PingError> {
    println!("Using high-level API:");

    // 4回 ping を送信
    let results = ping(host, 4)?;

    let mut success_count = 0;
    let mut total_rtt = Duration::from_secs(0);

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(r) => {
                println!(
                    "Reply from {}: seq={} time={:.1}ms",
                    r.addr,
                    r.sequence,
                    r.rtt.as_secs_f64() * 1000.0
                );
                success_count += 1;
                total_rtt += r.rtt;
            }
            Err(e) => {
                println!("Request {} failed: {}", i + 1, e);
            }
        }
    }

    // 統計情報を表示
    if success_count > 0 {
        let avg_rtt = total_rtt / success_count as u32;
        println!("\n--- {} ping statistics ---", host);
        println!(
            "{} packets transmitted, {} received, {:.0}% packet loss",
            results.len(),
            success_count,
            ((results.len() - success_count) as f64 / results.len() as f64) * 100.0
        );
        println!("round-trip avg = {:.1}ms", avg_rtt.as_secs_f64() * 1000.0);
    }

    Ok(())
}

/// 低レベル API を使用した詳細な ping の例
fn advanced_ping_example(host: &str) -> Result<(), PingError> {
    println!("Using low-level API:");

    // アドレスを解決
    let ip = if let Ok(ip) = host.parse::<IpAddr>() {
        ip
    } else {
        pingrs::ping::resolve_addr(host)?
    };

    println!("Resolved {} to {}", host, ip);

    // Pinger を作成
    let mut pinger = Pinger::new(&ip)?;
    pinger.set_timeout(Duration::from_secs(2))?;

    let socket_addr = SocketAddr::new(ip, 0);

    // カスタムペイロードで ping を送信
    let custom_payload = b"pingrs example payload".to_vec();

    for i in 0..4 {
        if i > 0 {
            std::thread::sleep(Duration::from_secs(1));
        }

        match pinger.ping(&socket_addr, Some(custom_payload.clone())) {
            Ok(result) => {
                println!(
                    "Reply from {}: seq={} time={:.1}ms size={} bytes",
                    result.addr,
                    result.sequence,
                    result.rtt.as_secs_f64() * 1000.0,
                    result.size
                );
            }
            Err(e) => {
                println!("Request {} failed: {}", i + 1, e);
            }
        }
    }

    Ok(())
}
