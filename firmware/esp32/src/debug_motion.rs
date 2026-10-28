//! debug consoleから、motion指示を1行ずつ受ける（[Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)の
//! 受け入れ条件3。`debug-motion-19` feature付きのbuildだけが持つ）。
//!
//! UART0のconsole（stdin）を別threadで読み、`MOTION <name> <target>`の形の行を main loop へ渡す。
//! 行の読み方は`deskcat_servo::parse_debug_line`、判定は`crate::pi_link::PiLink::on_debug_line`を通る
//! `deskcat_servo::MotionDispatch::handle_debug`である。**この module は読んで渡すだけで、
//! `boot`の確立もcatalogも判定しない。**debug経路だけが飛ばせる検査は、Piのsession（`sid`）とduplicate（`id`）だけである
//! （`handle_debug`のdoc）。
//!
//! **このbuildもservoを駆動しない。**`LimitedServo`も`Limiter`も作らず、`Sg90`・LEDC・`SERVO-PWM`（GPIO27）にも
//! 触れない（`crate::pi_link`のmodule doc「`play_motion`」）。catalogは空のままなので、**debugの指示も全て
//! `UnknownMotion`で拒否され、logに残る。**debugの指示がPi由来と同じ`Limiter`（同じcounter）へ届くことは、
//! `deskcat_servo`のhost testが示す範囲であり、firmwareでは`LimitedServo`を作る後続の変更まで届かない。
//!
//! **別threadで読む。**`stdin.read`が入力を待つのか`WouldBlock`を返すのかは、`crate::touch`と同じく
//! どちらでもmain loopが止まらないようにするためである（`touch_console`が実機で動いた前例は
//! `crate::touch`のmodule docにある。この読み取りthreadそのものは実機で動かしていない）。

use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::time::Duration;

use deskcat_servo::MAX_DEBUG_LINE_BYTES;

/// 読み取りthreadが1回に読む上限（byte）。
const READ_BYTES: usize = 32;
/// consoleにデータが無いとき、読み取りthreadが待つ時間（ms）。
const IDLE_SLEEP_MS: u64 = 20;
/// 読み取りthreadのstack（byte）。`crate::touch`のconsole threadと同じ大きさで、足りるかは実機で確かめていない。
const THREAD_STACK_BYTES: usize = 6144;

/// consoleから受けた事象。
pub enum ConsoleEvent {
    /// 改行までの1行（改行を含まない）。
    Line(String),
    /// [`MAX_DEBUG_LINE_BYTES`]を超えた行。改行まで捨てた。
    TooLong,
    /// 読み取りに失敗した（threadは止まる）。
    ReadFailed(String),
}

/// debug consoleの読み取り。threadを起こせなかった場合は何も受けない。
pub struct DebugConsole {
    rx: Option<mpsc::Receiver<ConsoleEvent>>,
}

impl DebugConsole {
    /// 読み取りthreadを起こす。
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let rx = match std::thread::Builder::new()
            .name("debug_motion_console".into())
            .stack_size(THREAD_STACK_BYTES)
            .spawn(move || read_console(&tx))
        {
            Ok(_) => Some(rx),
            Err(err) => {
                log::error!("debug_motion_console_spawn_failed error={err}");
                None
            }
        };
        Self { rx }
    }

    /// 受けた事象を1件取り出す。無ければ`None`。
    pub fn try_recv(&self) -> Option<ConsoleEvent> {
        self.rx.as_ref()?.try_recv().ok()
    }
}

fn read_console(tx: &mpsc::Sender<ConsoleEvent>) {
    let mut stdin = std::io::stdin();
    let mut line: Vec<u8> = Vec::with_capacity(MAX_DEBUG_LINE_BYTES);
    let mut overflowed = false;
    let mut buf = [0_u8; READ_BYTES];
    loop {
        let n = match stdin.read(&mut buf) {
            Ok(n) => n,
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => 0,
            Err(err) => {
                let _ = tx.send(ConsoleEvent::ReadFailed(err.to_string()));
                return;
            }
        };
        if n == 0 {
            std::thread::sleep(Duration::from_millis(IDLE_SLEEP_MS));
            continue;
        }
        for &byte in &buf[..n] {
            let event = match byte {
                b'\r' | b'\n' => {
                    let event = if overflowed {
                        Some(ConsoleEvent::TooLong)
                    } else if line.is_empty() {
                        None
                    } else {
                        Some(ConsoleEvent::Line(
                            String::from_utf8_lossy(&line).into_owned(),
                        ))
                    };
                    line.clear();
                    overflowed = false;
                    event
                }
                other => {
                    if line.len() < MAX_DEBUG_LINE_BYTES {
                        line.push(other);
                    } else {
                        overflowed = true;
                    }
                    None
                }
            };
            if let Some(event) = event {
                if tx.send(event).is_err() {
                    return;
                }
            }
        }
    }
}
