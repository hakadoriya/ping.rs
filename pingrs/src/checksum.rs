//! ICMP チェックサム計算
//!
//! RFC 792 に基づく ICMP チェックサムの実装
//! 参考: https://tools.ietf.org/html/rfc792

/// ICMP チェックサムを計算
///
/// # アルゴリズム
/// 1. チェックサムフィールドを 0 にする
/// 2. データを 16 ビット単位で合計する
/// 3. オーバーフローがあれば、キャリーを下位 16 ビットに加算
/// 4. 1 の補数を取る
pub fn calculate(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    // 16 ビット単位で処理
    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += word as u32;
        i += 2;
    }

    // 奇数バイトの場合の処理
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }

    // キャリーを加算
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    // 1 の補数を取る
    !sum as u16
}

/// チェックサムを検証
///
/// データのチェックサムが正しい場合は true を返す
pub fn verify(data: &[u8]) -> bool {
    calculate(data) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_even_bytes() {
        // テストデータ: ICMP Echo Request の例
        let data = vec![
            0x08, 0x00, // Type: 8, Code: 0
            0x00, 0x00, // Checksum: 0 (計算前)
            0x00, 0x01, // Identifier: 1
            0x00, 0x01, // Sequence: 1
        ];

        let checksum = calculate(&data);
        assert_ne!(checksum, 0);

        // チェックサムを設定してverifyをテスト
        let mut data_with_checksum = data.clone();
        data_with_checksum[2] = (checksum >> 8) as u8;
        data_with_checksum[3] = (checksum & 0xff) as u8;

        assert!(verify(&data_with_checksum));
    }

    #[test]
    fn test_checksum_odd_bytes() {
        let data = vec![0x45, 0x00, 0x00, 0x1c, 0x00];
        let checksum = calculate(&data);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_verify_invalid_checksum() {
        let data = vec![
            0x08, 0x00, // Type: 8, Code: 0
            0xFF, 0xFF, // 不正なチェックサム
            0x00, 0x01, // Identifier: 1
            0x00, 0x01, // Sequence: 1
        ];

        assert!(!verify(&data));
    }
}
