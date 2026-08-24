//! byte列を運ぶ層の境界。
//!
//! ここにあるのは境界だけである。testはfakeを注入し、実deviceは
//! [`crate::device::SerialDevice`]がこのtraitを実装する。**このmoduleは
//! deviceを開かないし、特定のOSを知らない。**
//!
//! [`IoDisposition::classify`]が[`io::ErrorKind`]だけで判定するのはそのためである。
//! errnoの意味はdeviceの種別で変わる（regular fileの`EIO`は切断ではない）。
//! **device固有の正規化はbackend側の責務であり、ここへ足さない。**

use std::fmt;
use std::io;

/// byte列を読み書きする対象。
///
/// `std::io::Read`と`std::io::Write`をそのまま使わずtraitを1つ置くのは、
/// **このcrateが要求する性質を明示するため**である。実装は次を満たす。
///
/// - `read`が`Ok(0)`を返すのは**EOFのときだけ**である。「今はdataが無い」は
///   `Ok(0)`ではなく[`io::ErrorKind::WouldBlock`]で表す。この区別を崩すと、
///   切断を「dataが無いだけ」と読み違える（受け入れ条件「Disconnectを観測できる」）。
///   `poll`で待つbackendは満了を[`io::ErrorKind::TimedOut`]で表しがちだが、
///   **それは「dataが無い」であって異常ではない。実装側で`WouldBlock`へ移す**
///   （[`crate::device::SerialDevice`]がそうしている）。
/// - 切断を[`IoDisposition::classify`]が`Disconnected`と読めるkindで返す。
///   下層のerrnoが`ErrorKind`へ写らない場合（Linuxの`EIO`等）、**実装側で移す。**
///   移さないと`classify`の既定に従って`Fatal`になり、再接続の経路へ入らない。
/// - `write`は要求より少ないbyte数を返してよい。呼び出し側が進捗を管理する。
pub trait Transport {
    /// `buf`へ読み込む。`Ok(0)`はEOF（切断）を表す。
    ///
    /// # Errors
    ///
    /// 下層のI/O errorをそのまま返す。分類は呼び出し側が行う。
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// `buf`から書き出す。要求より少ないbyte数を返してよい。
    ///
    /// # Errors
    ///
    /// 下層のI/O errorをそのまま返す。分類は呼び出し側が行う。
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;

    /// 書き出しをflushする。
    ///
    /// # Errors
    ///
    /// 下層のI/O errorをそのまま返す。
    fn flush(&mut self) -> io::Result<()>;
}

/// I/O errorの分類。
///
/// **errorを握りつぶさない。**分類した結果はcounterと接続stateへ落とす。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IoDisposition {
    /// 同じ操作をそのまま再試行してよい。進捗は無い。
    ///
    /// [`io::ErrorKind::WouldBlock`]と[`io::ErrorKind::Interrupted`]が該当する。
    /// `Interrupted`はsignalによる中断であり、linkの障害ではない。
    Retry,
    /// timeoutした。linkが生きているとも死んでいるとも言えない。
    ///
    /// 切断として扱わない。上位のACK timeout（`PROTO-TBD-004`）と混ぜない。
    TimedOut,
    /// linkが切れた。再接続の対象である。
    Disconnected,
    /// 設定または権限の誤りであり、再接続では直らない。
    Fatal,
}

impl IoDisposition {
    /// [`io::Error`]を分類する。
    #[must_use]
    pub fn classify(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => Self::Retry,
            io::ErrorKind::TimedOut => Self::TimedOut,
            io::ErrorKind::BrokenPipe
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected => Self::Disconnected,
            // 権限、不在、種別違いは再接続で直らない。再試行すると同じerrorを
            // 無限に踏む。**分類できないkindもここへ落とす。**未知のものを
            // 「一時的な失敗」と楽観すると、直らない状態で試行し続ける。
            _ => Self::Fatal,
        }
    }
}

/// どのI/O操作でerrorが起きたかを表す。**ログにこれが無いと、counterが増えた理由を
/// 後から辿れない。**read／write／flushは失敗の意味が違う。
///
/// **公開しない。**現状は`handle_io_error`（private）へ渡すだけで、公開署名に出ない。
/// 利用者のいない公開APIを増やさない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IoOp {
    /// transportからの読み出し。
    Read,
    /// transportへの書き出し。
    Write,
    /// bufferの押し出し。
    Flush,
}

impl fmt::Display for IoOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Flush => "flush",
        };
        f.write_str(name)
    }
}

/// `Read + Write`を実装する型を、そのまま[`Transport`]として使えるようにする。
///
/// **この impl があるため、`Read + Write`を実装する型に対して個別の
/// `impl Transport`を書くとcompileが通らない**（重複する。E0119）。
/// 上の契約に足りない型を載せるときは、`Read`／`Write`を実装しないnewtypeで包む。
/// [`crate::device::SerialDevice`]がその例である。
impl<T> Transport for T
where
    T: io::Read + io::Write,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(self, buf)
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_retryable_kinds_separately_from_disconnects() {
        let retry = io::Error::from(io::ErrorKind::WouldBlock);
        let interrupted = io::Error::from(io::ErrorKind::Interrupted);
        let timed_out = io::Error::from(io::ErrorKind::TimedOut);
        let broken = io::Error::from(io::ErrorKind::BrokenPipe);
        let denied = io::Error::from(io::ErrorKind::PermissionDenied);

        assert_eq!(IoDisposition::classify(&retry), IoDisposition::Retry);
        assert_eq!(IoDisposition::classify(&interrupted), IoDisposition::Retry);
        assert_eq!(IoDisposition::classify(&timed_out), IoDisposition::TimedOut);
        assert_eq!(
            IoDisposition::classify(&broken),
            IoDisposition::Disconnected
        );
        assert_eq!(IoDisposition::classify(&denied), IoDisposition::Fatal);
    }
}
