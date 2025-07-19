# pingrs-cli

pingrs ライブラリを使用した ping コマンドラインツール

## インストール

```bash
cargo install pingrs-cli
```

または、ソースからビルド：

```bash
git clone https://github.com/hakadoriya/ping.rs
cd ping.rs/pingrs-cli
cargo build --release
```

## 使用方法

```bash
# 基本的な使用
sudo ping google.com

# 送信回数を指定
sudo ping -c 4 google.com

# 送信間隔を指定（秒）
sudo ping -i 0.5 8.8.8.8

# タイムアウトを指定（秒）
sudo ping -W 2 example.com

# パケットサイズを指定（バイト）
sudo ping -s 128 google.com

# ヘルプ表示
ping -h
```

## オプション

- `-c COUNT`: 送信するパケット数を指定
- `-i INTERVAL`: パケット送信間隔を秒単位で指定（デフォルト: 1.0）
- `-W TIMEOUT`: 応答待機時間を秒単位で指定（デフォルト: 5.0）
- `-s SIZE`: ペイロードサイズをバイト単位で指定（デフォルト: 56）
- `-h, --help`: ヘルプメッセージを表示

## 機能

- 標準的な ping コマンドと同様のインターフェース
- Ctrl+C による中断時に統計情報を表示（ctrlc クレート使用）
- RTT の最小/平均/最大値を計算
- パケットロス率の表示
- 軽量なバイナリ（約 600KB）

## 権限要件

Raw ソケットを使用するため、root 権限または管理者権限が必要です。

### Linux での権限設定

```bash
# capability を付与して root 権限なしで実行可能にする
sudo setcap cap_net_raw+ep /path/to/ping
```

## ライセンス

MIT OR Apache-2.0