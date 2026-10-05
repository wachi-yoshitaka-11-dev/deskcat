//! Pi側の`sid`の作り方（`PROTO-TBD-011`のPi側。[#491の決定]）。
//!
//! processの起動ごとにOSの乱数（`/dev/urandom`の4 byte）から選ぶ。`sid`は`u32`であり、
//! 仕様§3は禁じる値を定めていない（0も正当な値である）。
//!
//! **衝突は完全には排除できない。**衝突したときの回復（hello自身が`stale_session`で拒否された
//! 場合にprocessを再起動して選び直す）は[`crate::daemon`]が持つ。
//!
//! [#491の決定]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491#issuecomment-5965350355

use std::fs::File;
use std::io::{self, Read};

/// 乱数の源は`/dev/urandom`である。
const URANDOM: &str = "/dev/urandom";

/// `source`から4 byteを読んで`sid`にする。
///
/// 読めなければ（4 byteに満たない場合を含む）errorを返す。**固定値や時刻へ
/// 黙って落とさない。**同じ値を選ぶ確率が上がり、§3が塞ぐ「再起動しても同じ`sid`」を作るため。
///
/// # Errors
///
/// `source`の読み出しに失敗した場合。
pub fn sid_from(source: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0_u8; 4];
    source.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

/// `/dev/urandom`から`sid`を選ぶ。
///
/// # Errors
///
/// `/dev/urandom`を開けない、または読めない場合。
pub fn sid_from_os() -> io::Result<u32> {
    sid_from(&mut File::open(URANDOM)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_bytes_become_the_sid() {
        let mut source: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0xff];
        assert_eq!(sid_from(&mut source).expect("読める"), 0x0403_0201);
    }

    #[test]
    fn zero_is_a_valid_sid() {
        let mut source: &[u8] = &[0, 0, 0, 0];
        assert_eq!(sid_from(&mut source).expect("読める"), 0);
    }

    #[test]
    fn a_short_read_is_an_error_not_a_fallback() {
        let mut source: &[u8] = &[1, 2, 3];
        let err = sid_from(&mut source).expect_err("4 byteに満たない");
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn the_os_source_yields_a_sid() {
        // 値は乱数であり、検査できるのは読めたことだけである。
        sid_from_os().expect("/dev/urandomを読める");
    }
}
