//! 実serial deviceのbackend。
//!
//! [`serial2::SerialPort`]は[`std::io::Read`]と[`std::io::Write`]を実装するため、
//! `transport.rs`のblanket implによって**そのままでも既に[`Transport`]である。**
//! それでもnewtypeを1つ置くのは、**下層のerrorを2点だけ正規化するため**である。
//! この理由が無ければこの型は要らない。消す前に下の2節を読む。
//!
//! [`SerialDevice`]は[`std::io::Read`]／[`std::io::Write`]を**実装しない。**
//! 実装するとblanket implと重なってcompileが通らない（E0119）。
//! 同じ理由で[`std::ops::Deref`]も張らない。derefは衝突しないが、
//! 正規化していない`read`／`write`を経路として再び露出させてしまう。
//!
//! # 正規化1: 切断を表すerrno
//!
//! [`IoDisposition::classify`](crate::IoDisposition::classify)は[`std::io::ErrorKind`]だけで判定し、
//! **知らないkindは意図的に[`Fatal`](crate::IoDisposition::Fatal)へ落とす。**`Fatal`は再接続せずに
//! sessionを止める。
//!
//! ところが、USB serialのadapterをLinux上で抜くと、下層は`EIO`／`ENXIO`／`ENODEV`を返す。
//! standard libraryはこれらに対応する`ErrorKind`を持たず`Uncategorized`にするため、
//! **そのままでは切断が`Fatal`になり、再接続の経路へ入らない。**
//!
//! この3つを[`std::io::ErrorKind::BrokenPipe`]へ包み直し、`classify`が
//! [`Disconnected`](crate::IoDisposition::Disconnected)と読めるようにする。`read`／`write`／`flush`の
//! **3つすべてに掛ける**。hangup中の`tcdrain`も`EIO`を返すため、`flush`を外すと
//! 「flush中に抜かれた」だけが`Fatal`になる。
//!
//! `EBADF`と`ENOTTY`は**写さない。**前者はこちらのfd管理の誤り、後者はtty以外のpathを
//! 設定した誤りであり、いずれも再接続では直らない。`Fatal`が正しい。
//!
//! 正規化を[`classify`](crate::IoDisposition::classify)側へ入れない理由は2つある。errnoの意味はdeviceの
//! 種別に依存する（regular fileの`EIO`は切断ではない）こと、そして`classify`はtestの
//! fakeも通る共通の関数であり、そこへ`libc`を持ち込むと分類の中心が特定OSへ寄ることである。
//!
//! # 正規化2: 読みの`TimedOut`
//!
//! [`serial2`]のunix実装は`read`の先頭で`poll(POLLIN, read_timeout)`を行い、満了すると
//! [`std::io::ErrorKind::TimedOut`]を返す。一方[`Transport`]の契約は
//! **「今はdataが無い」を[`std::io::ErrorKind::WouldBlock`]で表すと定めている。**
//! そのままつなぐと2つ困る。
//!
//! - 契約に反する
//! - 待っているだけのlinkが[`SessionCounters::timeouts`]を延々と増やす。
//!   [`Pump::TimedOut`]は「linkが生きているとも死んでいるとも言えない」＝異常として
//!   定義されているのに、idleを数える時計になってしまう
//!
//! そこで`read`に限り`TimedOut`を`WouldBlock`へ移す。ttyの`read(2)`は`ETIMEDOUT`を
//! 返さないため、[`serial2`]の`read`から出る`TimedOut`はpoll満了以外にありえない。
//!
//! **`write`は移さない。**`POLLOUT`の満了は「送信bufferが窓のあいだ詰まったまま」であり、
//! これは実際に異常である。[`SessionCounters::timeouts`]の意味をこちら側で保つ。
//!
//! # 決めていないこと（`HUPCL`）
//!
//! 既定では、portをcloseするときに DTR が落ちる。**本projectの ESP32 board で
//! DTR ／ RTS が自動resetへ繋がっているかは確認していない。**繋がっていれば、
//! 再接続のたびに ESP32 が再起動することになり、`boot`／`hello`のhandshake
//! （[Issue #12]）に直接効く。
//!
//! **確認していない以上、ここでは決めない。**現物の確認と、再起動させるか否かの判断は
//! protocol側の話であり、backendが黙って選ぶことではない。**触らない**（＝既定のまま）。
//! `TBD`として残す。
//!
//! [Issue #12]: https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12
//! [`Pump::TimedOut`]: crate::Pump::TimedOut
//! [`SessionCounters::timeouts`]: crate::SessionCounters::timeouts

use std::fmt;
use std::io;
use std::time::Duration;

use crate::config::SerialConfig;
use crate::transport::Transport;

/// 実serial portの上で[`Transport`]を満たす型。
///
/// 生成方法は[`SerialDevice::open`]である。詳細と、この型が存在する理由は
/// [module doc](self)にある。
pub struct SerialDevice {
    port: serial2::SerialPort,
}

impl SerialDevice {
    /// 読みのtimeoutの既定値。
    ///
    /// **暫定値であり、確定値ではない。**[`Transport::read`]がこの時間だけ待って
    /// dataが来なければ`WouldBlock`を返す（[module doc](self)の正規化2）。
    /// pumpのloopが空回りする間隔でもあるため、短くするとCPUを、長くすると
    /// 送信の待ち時間を食う。確定は実機での測定を待つ。
    ///
    /// [`serial2`]の既定値は読み書きとも3秒であり、そのままでは単線のloopが
    /// idle時に1周3秒止まる。**明示的に設定する必要がある。**
    pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(50);

    /// 書きのtimeoutの既定値。
    ///
    /// **暫定値であり、確定値ではない。**送信bufferがこの時間ずっと詰まったままなら
    /// `TimedOut`を返す。読みと違い、これは異常として数える（[module doc](self)）。
    pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_millis(500);

    /// 設定されたportを開く。
    ///
    /// baudは[`SerialConfig::baud`]から採る。8 data bits、parityなし、stop bit 1、
    /// flow controlなしのraw modeで開く。timeoutは[`Self::DEFAULT_READ_TIMEOUT`]と
    /// [`Self::DEFAULT_WRITE_TIMEOUT`]である。
    ///
    /// # Errors
    ///
    /// 下層のopenと設定適用の失敗をそのまま返す。
    ///
    /// **この errorを[`IoDisposition::classify`](crate::IoDisposition::classify)へ渡さない。**同関数が分類するのは
    /// 確立済みのlinkの上で起きたI/O errorである。openの失敗のうち`ENOENT`（device nodeが
    /// まだ無い）、`EACCES`（`udev`のruleが当たる前）、`EBUSY`はいずれもUSBの再列挙中に
    /// 起きる一時的なものだが、`classify`はこれらを[`IoDisposition::Fatal`](crate::IoDisposition::Fatal)にする。
    /// **復帰しようとしているまさにその場面でsessionを永久に止めることになる。**
    /// 再接続のloopは、openの失敗を[`Session::begin_reconnect`](crate::Session::begin_reconnect)が`None`を返すまで
    /// 単純に再試行する。下の例がその形である。
    ///
    /// # 例
    ///
    /// 呼び出し側のloop。**このcrateはloopを持たない**（[`Session`](crate::Session)はtransportを
    /// 所有せず、pumpの引数で受け取る）。実portを開くため`no_run`である。
    ///
    /// ```no_run
    /// use std::thread::sleep;
    ///
    /// use deskcat_serial::{Pump, SerialConfig, SerialDevice, Session};
    ///
    /// // device名は呼び出し側が設定として渡す。**ここに実機の名前を書かない。**
    /// // `/dev/ttyUSB*`の実際の名前は未確認であり、確定はIssue #11の後半である。
    /// let config = SerialConfig::new("/dev/example", 115_200)?;
    /// let mut session = Session::new(config.clone(), 90_312);
    ///
    /// loop {
    ///     // openの失敗はclassifyへ渡さない。予算が尽きるまで再試行する。
    ///     let Ok(mut device) = SerialDevice::open(&config) else {
    ///         let Some(backoff) = session.begin_reconnect() else {
    ///             break; // 上限に達した。session側がStoppedになっている
    ///         };
    ///         sleep(backoff);
    ///         continue;
    ///     };
    ///
    ///     session.note_connected();
    ///
    ///     loop {
    ///         let read = session.pump_read(&mut device, |outcome| {
    ///             let _ = outcome; // 上位（Issue #12）が受け取る
    ///         });
    ///         let write = session.pump_write(&mut device);
    ///
    ///         if matches!(read, Pump::Disconnected | Pump::Fatal)
    ///             || matches!(write, Pump::Disconnected | Pump::Fatal)
    ///         {
    ///             break;
    ///         }
    ///     }
    ///
    ///     let Some(backoff) = session.begin_reconnect() else {
    ///         break;
    ///     };
    ///     sleep(backoff);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open(config: &SerialConfig) -> io::Result<Self> {
        Self::open_with_timeouts(
            config,
            Self::DEFAULT_READ_TIMEOUT,
            Self::DEFAULT_WRITE_TIMEOUT,
        )
    }

    /// timeoutを明示して開く。
    ///
    /// 既定値で足りる場合は[`Self::open`]を使う。
    ///
    /// # Errors
    ///
    /// [`Self::open`]と同じである。**openのerrorを
    /// [`IoDisposition::classify`](crate::IoDisposition::classify)へ渡さない。**
    pub fn open_with_timeouts(
        config: &SerialConfig,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> io::Result<Self> {
        let baud = config.baud();

        let mut port =
            serial2::SerialPort::open(config.port(), move |mut settings: serial2::Settings| {
                // **必ず最初に呼ぶ。**`set_raw`はICANONを落とし`VMIN = 1`／`VTIME = 0`を立てる。
                // これを飛ばすとcanonical modeのままになり、受信byte列に`VEOF`（`0x04`）が
                // 現れた時点で`read`が`Ok(0)`を返す。呼び出し側はそれをEOF＝切断と読むため、
                // **線のnoiseが偽の切断になる。**`VMIN = 0`も同じく`Ok(0)`を生む。
                settings.set_raw();
                settings.set_baud_rate(baud)?;

                // `CLOCAL`を立てて、DCDの状態でhangup扱いにされないようにする。
                // `cfmakeraw`はこのbitを触らない。3線のUSB-TTL adapterやPiの2線UARTでは
                // DCDが配線されておらず、立てないと**生きているportで`Ok(0)`が出うる。**
                //
                // `serial2`側にsetterが無いためtermiosを直接触るが、これはfieldへの代入で
                // あり`unsafe`を要さない（workspaceは`unsafe_code = "forbid"`である）。
                //
                // **VM上では確認できない。**driverが`CLOCAL`を受け付けなければ、
                // `serial2`の読み戻し検証がopenを失敗させる。実機での確認は#11の後半に残る。
                #[cfg(unix)]
                {
                    settings.as_termios_mut().c_cflag |= libc::CLOCAL;
                }

                Ok(settings)
            })?;

        // unixでは、この2つはuserspaceのfieldへ代入するだけでioctlを伴わない。
        port.set_read_timeout(read_timeout)?;
        port.set_write_timeout(write_timeout)?;

        Ok(Self { port })
    }

    /// 開いた[`serial2::SerialPort`]をそのまま包む。
    ///
    /// **testからしか使わない。**`SerialPort::pair`が返す擬似端末を、openの経路を
    /// 通さずに載せるためにある。公開すると「利用者のいない公開API」になるため
    /// crate内にも出さない。
    #[cfg(all(unix, test))]
    fn from_port(port: serial2::SerialPort) -> Self {
        Self { port }
    }
}

impl fmt::Debug for SerialDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // file descriptorも設定も出さない。呼び出し側のlogへ載る型であり、
        // 出して意味があるのは「実deviceである」ことだけである。
        f.debug_struct("SerialDevice").finish_non_exhaustive()
    }
}

impl Transport for SerialDevice {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // 空bufferを渡すと`read(2)`が`Ok(0)`を返し、呼び出し側がEOFと読む。
        // `Session`は固定長のbufferを丸ごと渡すため到達しないが、契約として残す。
        debug_assert!(
            !buf.is_empty(),
            "空のbufferを渡すとOk(0)がEOFと区別できない"
        );
        self.port
            .read(buf)
            .map_err(|error| normalize_read_idle(normalize_disconnect(error)))
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.port.write(buf).map_err(normalize_disconnect)
    }

    fn flush(&mut self) -> io::Result<()> {
        // `serial2`の`flush`は`tcdrain`であり、byteが物理的に出るまで待つ。
        // `set_raw`がRTS/CTSとXON/XOFFの両方を切っているため、待ち時間は
        // 行長（`MAX_LINE_BYTES` = 1024 byte）÷ baudで抑えられる。
        // **flow controlを有効にすると、この上限が無くなる。**
        self.port.flush().map_err(normalize_disconnect)
    }
}

/// 切断を表すerrnoを、[`IoDisposition`](crate::IoDisposition)が切断と読める
/// `ErrorKind`へ移す。
///
/// 元のerrorは包んで残す（[`std::io::Error::get_ref`]で辿れる）。ただし包んだ側の
/// [`std::io::Error::raw_os_error`]は`None`になる。log行には元の`Display`が出る。
fn normalize_disconnect(error: io::Error) -> io::Error {
    #[cfg(unix)]
    if matches!(
        error.raw_os_error(),
        Some(libc::EIO | libc::ENXIO | libc::ENODEV)
    ) {
        return io::Error::new(io::ErrorKind::BrokenPipe, error);
    }
    error
}

/// 読みのpoll満了（`TimedOut`）を、契約どおりの`WouldBlock`へ移す。
///
/// **読みにだけ掛ける。**理由は[module doc](self)の正規化2にある。
fn normalize_read_idle(error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::TimedOut {
        return io::Error::new(io::ErrorKind::WouldBlock, error);
    }
    error
}

#[cfg(all(unix, test))]
mod tests {
    use std::time::Duration;

    use deskcat_protocol::{Envelope, Frame, Message, Outcome, encode_line, limits};
    use serial2::SerialPort;

    use super::SerialDevice;
    use crate::config::SerialConfig;
    use crate::session::{ConnectionState, Pump, Session};
    use crate::transport::Transport;

    /// 擬似端末（`/dev/ptmx`）の対を開き、こちら側にtimeoutを設定して返す。
    ///
    /// **`SerialDevice::open`は通さない。**openこそがhardware無しに検証できない部分で
    /// あり、通したことにしない。`SerialPort::pair`は`set_raw`を掛けるだけでbaudを
    /// 触らないため、`serial2`の設定読み戻し検証に掛からない。**ここで再設定しない**
    /// ことが、その回避策そのものである。`set_read_timeout`はunixでは
    /// userspaceのfieldへ代入するだけでioctlを伴わない。
    fn pty_pair() -> (SerialDevice, SerialPort) {
        let (mut near, far) = SerialPort::pair().expect("擬似端末の対を開ける");
        near.set_read_timeout(Duration::from_millis(50))
            .expect("timeoutの設定はioctlを伴わない");
        near.set_write_timeout(Duration::from_millis(200))
            .expect("timeoutの設定はioctlを伴わない");
        (SerialDevice::from_port(near), far)
    }

    /// このsessionのsid。
    const SID: u32 = 90_312;

    fn session() -> Session {
        // device名は台本の中だけの値である。この対は擬似端末であり、
        // ここに書いた名前でopenするわけではない。
        let config = SerialConfig::new("/dev/simulated", 115_200).expect("設定は妥当である");
        let mut session = Session::new(config, SID);
        session.note_connected();
        session
    }

    fn ping_line(id: u32) -> String {
        let frame = Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: 41_207,
                id,
                ts_ms: 100,
            },
            Message::Ping,
        );
        encode_line(&frame).expect("encodeできる")
    }

    /// 実のfile descriptor越しに行が復元される。
    ///
    /// fakeではなく擬似端末を通すため、termiosの設定、kernelのbuffer、
    /// 実際の`read(2)`が経路に入る。
    #[test]
    fn a_line_written_to_a_real_fd_is_reassembled() {
        let (mut device, far) = pty_pair();
        let mut session = session();
        let line = ping_line(7);

        far.write_all(line.as_bytes()).expect("対の片側へ書ける");
        far.flush().expect("flushできる");

        let mut frames = Vec::new();
        // 1回のreadで全部届くとは限らない。届くまで回すが、上限を置く。
        for _ in 0..line.len() {
            let _ = session.pump_read(&mut device, |outcome| {
                if let Outcome::Frame(frame) = outcome {
                    frames.push(frame);
                }
            });
            if !frames.is_empty() {
                break;
            }
        }

        assert_eq!(frames.len(), 1, "実fd越しでも1件だけ復元される");
        assert_eq!(frames[0].envelope.identity(), (41_207, 7));
        assert_eq!(session.state(), ConnectionState::Connected);
    }

    /// 送った行が、そのまま実のfile descriptorの向こう側へ出る。
    #[test]
    fn a_queued_message_reaches_the_other_end_of_a_real_fd() {
        let (mut device, far) = pty_pair();
        let mut session = session();

        // sessionが実際に採番するのは`id = 1`である（このsessionの最初の送出）。
        let expected = encode_line(&Frame::new(
            Envelope {
                v: limits::PROTOCOL_VERSION,
                sid: SID,
                id: 1,
                ts_ms: 100,
            },
            Message::Ping,
        ))
        .expect("encodeできる");

        assert_eq!(session.send(Message::Ping, 100).expect("queueへ入る"), 1);

        while session.pending_out() > 0 {
            match session.pump_write(&mut device) {
                Pump::Progress(_) | Pump::Idle => {}
                other => panic!("書き出しが進まない: {other:?}"),
            }
        }

        // 1回のreadで全部届くとは限らない。出したbyte数だけ集める。
        let mut received = Vec::new();
        let mut chunk = [0_u8; 256];
        while (received.len() as u64) < session.counters().bytes_out {
            let read = far.read(&mut chunk).expect("対の片側から読める");
            assert!(read > 0, "書き出したbyteが届く");
            received.extend_from_slice(&chunk[..read]);
        }

        assert_eq!(
            String::from_utf8(received).expect("行はUTF-8である"),
            expected,
            "encodeした行がそのまま実fdへ出る"
        );
        assert_eq!(session.counters().bytes_out, expected.len() as u64);
    }

    /// **この test が正規化1の回帰guardである。**
    ///
    /// 対の片側を落とすと、Linuxのtty層は`EIO`を返す。standard libraryはこれに
    /// 対応する`ErrorKind`を持たないため、正規化しなければ
    /// [`crate::IoDisposition::classify`]の既定に従って`Fatal`になり、
    /// **切断が再接続の経路へ入らない。**
    ///
    /// errnoではなく[`Pump`]で判定する。BSDはこの場面で`Ok(0)`を返すため、
    /// errnoで書くと移植性を失う。どちらでも「切断として観測できる」ことが要件である。
    #[test]
    fn dropping_the_peer_is_observed_as_a_disconnect_not_a_fatal_error() {
        let (mut device, far) = pty_pair();
        let mut session = session();

        drop(far);

        // 残留dataは無い（切る前に書いていない）ので、最初のreadで切断が出る。
        let pump = session.pump_read(&mut device, |_| {});

        assert_eq!(
            pump,
            Pump::Disconnected,
            "EIOを正規化しないとFatalになり、再接続しない"
        );
        assert_eq!(session.state(), ConnectionState::Disconnected);
        assert_eq!(session.counters().disconnects, 1);
    }

    /// **この test が正規化2の回帰guardである。**
    ///
    /// dataが来ないだけの状態は「異常」ではない。正規化しなければ`serial2`の
    /// poll満了が`TimedOut`のまま通り、[`Pump::TimedOut`]と
    /// `counters.timeouts`がidleのあいだ増え続ける。
    #[test]
    fn an_idle_port_is_idle_not_timed_out() {
        let (mut device, _far) = pty_pair();
        let mut session = session();

        for _ in 0..3 {
            assert_eq!(
                session.pump_read(&mut device, |_| {}),
                Pump::Idle,
                "dataが無いだけの状態はIdleである"
            );
        }

        assert_eq!(
            session.counters().timeouts,
            0,
            "idleをtimeoutとして数えない"
        );
        assert_eq!(session.counters().retries, 3);
        assert_eq!(session.state(), ConnectionState::Connected);
    }

    /// 空bufferを渡す経路が無いことを、契約として固定する。
    ///
    /// `debug_assert!`はrelease buildで消えるため、このtestもdebugのときだけ意味を持つ。
    /// gateを付けないと`cargo test --release`が「panicしなかった」で落ちる。
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "空のbuffer")]
    fn reading_into_an_empty_buffer_is_a_contract_violation() {
        let (mut device, _far) = pty_pair();
        let _ = device.read(&mut []);
    }
}

#[cfg(all(unix, test))]
mod normalization_tests {
    use std::io;

    use super::{normalize_disconnect, normalize_read_idle};

    /// 切断を表すerrnoは、`classify`が切断と読めるkindへ移る。
    #[test]
    fn disconnect_errnos_become_broken_pipe() {
        for errno in [libc::EIO, libc::ENXIO, libc::ENODEV] {
            let normalized = normalize_disconnect(io::Error::from_raw_os_error(errno));
            assert_eq!(
                normalized.kind(),
                io::ErrorKind::BrokenPipe,
                "errno {errno} は切断として扱う"
            );
            assert_eq!(
                crate::IoDisposition::classify(&normalized),
                crate::IoDisposition::Disconnected
            );
        }
    }

    /// **再接続で直らないerrnoは移さない。**
    ///
    /// `EBADF`はこちらのfd管理の誤り、`ENOTTY`はtty以外のpathを設定した誤りである。
    /// 切断として扱うと、直らない状態のまま上限まで再接続を繰り返す。
    #[test]
    fn errnos_that_reconnecting_cannot_fix_are_left_alone() {
        for errno in [libc::EBADF, libc::ENOTTY] {
            let normalized = normalize_disconnect(io::Error::from_raw_os_error(errno));
            assert_eq!(
                crate::IoDisposition::classify(&normalized),
                crate::IoDisposition::Fatal,
                "errno {errno} は再接続では直らない"
            );
        }
    }

    /// 元のerrorは包んで残る。log行に出るのは元の`Display`である。
    #[test]
    fn the_original_error_is_wrapped_not_discarded() {
        let normalized = normalize_disconnect(io::Error::from_raw_os_error(libc::EIO));

        assert!(normalized.get_ref().is_some(), "元のerrorはget_refで辿れる");
        assert_eq!(
            normalized.raw_os_error(),
            None,
            "包んだ側はraw_os_errorを持たない。この点は呼び出し側の前提になる"
        );
        assert!(
            normalized.to_string().contains("os error 5"),
            "log行には元のDisplayが出る"
        );
    }

    /// 読みのpoll満了は「dataが無い」であり、契約どおり`WouldBlock`で表す。
    #[test]
    fn a_read_timeout_becomes_would_block() {
        let normalized = normalize_read_idle(io::Error::from(io::ErrorKind::TimedOut));

        assert_eq!(normalized.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(
            crate::IoDisposition::classify(&normalized),
            crate::IoDisposition::Retry
        );
    }

    /// 読み以外のkindは触らない。
    #[test]
    fn other_kinds_pass_through_the_idle_normalization() {
        for kind in [
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::Interrupted,
        ] {
            let normalized = normalize_read_idle(io::Error::from(kind));
            assert_eq!(normalized.kind(), kind);
        }
    }
}
