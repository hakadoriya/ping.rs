//! ping コマンドの CLI 実装（同期版・ctrlc使用）

use pingrs::{PingError, Pinger};
use std::env;
use std::io::{self, Write};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// コマンドライン引数
struct Args {
    /// ターゲットホスト
    host: String,
    /// 送信回数 (-c オプション)
    count: Option<usize>,
    /// パケット間隔（秒）(-i オプション)
    interval: f64,
    /// タイムアウト（秒）(-W オプション)
    timeout: f64,
    /// パケットサイズ (-s オプション)
    packet_size: usize,
    /// ヘルプ表示
    help: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        let mut host = None;
        let mut count = None;
        let mut interval = 1.0;
        let mut timeout = 5.0;
        let mut packet_size = 56;
        let mut help = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-c" => {
                    if i + 1 >= args.len() {
                        return Err("-c requires an argument".to_string());
                    }
                    count = Some(args[i + 1].parse().map_err(|_| "Invalid count")?);
                    i += 2;
                }
                "-i" => {
                    if i + 1 >= args.len() {
                        return Err("-i requires an argument".to_string());
                    }
                    interval = args[i + 1].parse().map_err(|_| "Invalid interval")?;
                    i += 2;
                }
                "-W" => {
                    if i + 1 >= args.len() {
                        return Err("-W requires an argument".to_string());
                    }
                    timeout = args[i + 1].parse().map_err(|_| "Invalid timeout")?;
                    i += 2;
                }
                "-s" => {
                    if i + 1 >= args.len() {
                        return Err("-s requires an argument".to_string());
                    }
                    packet_size = args[i + 1].parse().map_err(|_| "Invalid packet size")?;
                    i += 2;
                }
                "-h" | "--help" => {
                    help = true;
                    i += 1;
                }
                arg if !arg.starts_with('-') => {
                    if host.is_none() {
                        host = Some(arg.to_string());
                    }
                    i += 1;
                }
                _ => {
                    return Err(format!("Unknown option: {}", args[i]));
                }
            }
        }

        if help {
            return Ok(Args {
                host: String::new(),
                count,
                interval,
                timeout,
                packet_size,
                help,
            });
        }

        let host = host.ok_or("Host argument is required")?;

        Ok(Args {
            host,
            count,
            interval,
            timeout,
            packet_size,
            help,
        })
    }
}

fn print_usage() {
    eprintln!("Usage: ping [OPTIONS] HOST");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  -c COUNT      Stop after sending COUNT packets");
    eprintln!("  -i INTERVAL   Wait INTERVAL seconds between packets (default: 1.0)");
    eprintln!("  -W TIMEOUT    Time to wait for response in seconds (default: 5.0)");
    eprintln!("  -s SIZE       Size of packet payload in bytes (default: 56)");
    eprintln!("  -h, --help    Show this help message");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  ping google.com");
    eprintln!("  ping -c 4 8.8.8.8");
}

/// 統計情報
struct Statistics {
    transmitted: AtomicU64,
    received: AtomicU64,
    min_rtt: AtomicU64,
    max_rtt: AtomicU64,
    total_rtt: AtomicU64,
}

impl Statistics {
    fn new() -> Self {
        Statistics {
            transmitted: AtomicU64::new(0),
            received: AtomicU64::new(0),
            min_rtt: AtomicU64::new(u64::MAX),
            max_rtt: AtomicU64::new(0),
            total_rtt: AtomicU64::new(0),
        }
    }

    fn update_success(&self, rtt_micros: u64) {
        self.received.fetch_add(1, Ordering::Relaxed);
        self.total_rtt.fetch_add(rtt_micros, Ordering::Relaxed);

        // 最小値を更新
        loop {
            let current_min = self.min_rtt.load(Ordering::Relaxed);
            if rtt_micros >= current_min {
                break;
            }
            if self
                .min_rtt
                .compare_exchange(
                    current_min,
                    rtt_micros,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        // 最大値を更新
        loop {
            let current_max = self.max_rtt.load(Ordering::Relaxed);
            if rtt_micros <= current_max {
                break;
            }
            if self
                .max_rtt
                .compare_exchange(
                    current_max,
                    rtt_micros,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }
    }

    fn print_summary(&self, host: &str) {
        let transmitted = self.transmitted.load(Ordering::Relaxed);
        let received = self.received.load(Ordering::Relaxed);

        println!("\n--- {} ping statistics ---", host);

        if transmitted > 0 {
            let loss_percent = ((transmitted - received) as f64 / transmitted as f64) * 100.0;
            println!(
                "{} packets transmitted, {} received, {:.0}% packet loss",
                transmitted, received, loss_percent
            );

            if received > 0 {
                let min = self.min_rtt.load(Ordering::Relaxed) as f64 / 1000.0;
                let max = self.max_rtt.load(Ordering::Relaxed) as f64 / 1000.0;
                let avg =
                    (self.total_rtt.load(Ordering::Relaxed) as f64 / received as f64) / 1000.0;

                println!("rtt min/avg/max = {:.3}/{:.3}/{:.3} ms", min, avg, max);
            }
        }
    }
}

fn main() {
    // コマンドライン引数をパース
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    };

    if args.help {
        print_usage();
        return;
    }

    // アドレスを解決
    let ip = match resolve_host(&args.host) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("ping: cannot resolve {}: {}", args.host, e);
            std::process::exit(1);
        }
    };

    // Ctrl+C ハンドラーを設定
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let stats = Arc::new(Statistics::new());
    let s = stats.clone();
    let host_for_handler = args.host.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
        s.print_summary(&host_for_handler);
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // ping を実行
    if let Err(e) = run_ping(ip, &args, &stats, &running) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // 統計情報を表示
    stats.print_summary(&args.host);
}

fn resolve_host(host: &str) -> Result<IpAddr, PingError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        Ok(ip)
    } else {
        pingrs::ping::resolve_addr(host)
    }
}

fn run_ping(
    ip: IpAddr,
    args: &Args,
    stats: &Arc<Statistics>,
    running: &Arc<AtomicBool>,
) -> Result<(), PingError> {
    // Pinger を作成
    let mut pinger = Pinger::new(&ip)?;
    pinger.set_timeout(Duration::from_secs_f64(args.timeout))?;

    let socket_addr = SocketAddr::new(ip, 0);

    // ペイロードを作成
    let payload = vec![0u8; args.packet_size];

    println!(
        "PING {} ({}) {} bytes of data.",
        args.host, ip, args.packet_size
    );

    let mut seq = 0;
    let start_time = Instant::now();

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        // カウント制限の確認
        if let Some(count) = args.count {
            if seq >= count {
                break;
            }
        }

        seq += 1;
        stats.transmitted.fetch_add(1, Ordering::Relaxed);

        // ping を送信
        match pinger.ping(&socket_addr, Some(payload.clone())) {
            Ok(result) => {
                let rtt_ms = result.rtt.as_secs_f64() * 1000.0;
                let rtt_micros = result.rtt.as_micros() as u64;

                stats.update_success(rtt_micros);

                println!(
                    "{} bytes from {}: icmp_seq={} ttl=64 time={:.1} ms",
                    result.size + 8, // ICMP ヘッダーを含む
                    result.addr,
                    result.sequence,
                    rtt_ms
                );

                // 出力をフラッシュ
                io::stdout().flush().unwrap();
            }
            Err(e) => match e {
                PingError::Timeout => {
                    println!("Request timeout for icmp_seq {}", seq);
                }
                _ => {
                    println!("Error for icmp_seq {}: {}", seq, e);
                }
            },
        }

        // 次のパケットまで待機（最後のパケットの後は待機しない）
        if args.count.is_none() || seq < args.count.unwrap() {
            let elapsed = start_time.elapsed();
            let next_time = Duration::from_secs_f64(seq as f64 * args.interval);
            if let Some(sleep_time) = next_time.checked_sub(elapsed) {
                thread::sleep(sleep_time);
            }
        }
    }

    Ok(())
}
