//! `TOUCH-01`（XPT2046）の読み出し。3 つの型がある。bus はどれもこの main-loop task が使う。
//!
//! - [`StrokeDetector`]：製品 build の撫での判定。状態機械（`deskcat_touch::TouchTracker`）で撫でを判定し、呼び出し側が
//!   `head_touched` を Pi へ送る（#21）。
//! - [`TouchSampler`]：#14 の試験 build（`touch-sample-14`、`touch-lcd-14`）の raw 読み出し。下の段落と「出力と、確かめていないこと」。
//! - [`IdleTouch`]：LCD を描く試験 build が、touch を読まずに持つ device。
//!
//! 制御 byte と 3 byte 応答の復号は `deskcat_touch` が持つ（host test 済み。ここへ再掲しない）。
//! 試験 build（[`TouchSampler`]）は `POLL_INTERVAL_MS` ごとに `X`・`Y`・`Z1`・`Z2` を 1 回ずつ読み（4 回の転送で 1 sample）、`T_IRQ`（`TOUCH-IRQ`）の level とあわせて
//! UART0 の log へ出す。**状態機械（`deskcat_touch::TouchTracker`）はこの build へ配線しない。**しきい値の根拠になる raw sample を
//! 取る build であり、仮の値を入れない（`deskcat_touch` の module doc）。
//!
//! # 配線と bus の共有
//!
//! pin は [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md) の信号 inventory が正本で、
//! `LCD-SCLK`＝GPIO18、`LCD-MOSI`＝GPIO23、`LCD-MISO`＝GPIO19（`TOUCH-01` と共有）、`TOUCH-CS`＝GPIO21、`TOUCH-IRQ`＝GPIO34 を使う。
//! bus の持ち方は 2 通りで、[`crate::spi_port::SpiPort`] が切り替える。
//!
//! - **`touch-sample-14`（[`TouchSampler::new`]）は LCD を初期化せず、`crate::display` を使わない。**bus は `SpiBusDriver` で専有し、
//!   bus lock を握り続ける。**LCD が接続されていても選択されないよう、`LCD-CS`（GPIO22）を High に固定する**（`crate::display` と同じく CS は active low）。
//! - **`touch-lcd-14`（[`TouchSampler::with_shared_bus`]）は LCD と `SpiDriver` を共有する。**`SpiDeviceDriver` で転送ごとに bus を取る。
//!   `LCD-CS` は LCD の driver（`crate::display`）が持つ。使い方は `crate::touch_lcd`。**この経路は console（stdin）を読まず、`MARK` を受け付けない**
//!   （`T_IRQ` の変化の記録は同じ）。
//! - **製品 build（[`StrokeDetector`]）は LCD と `SpiDriver` を共有し、touch を読む。**`X`・`Y`・`Z1` の 3 回の転送で 1 sample で、
//!   `T_IRQ` は読まず、`TOUCH-IRQ` は取らない。LCD の帯と帯の間（main loop の周回）で読み、帯の途中では読まない（`touch-lcd-14` と同じ）。
//! - **LCD を描く試験 build（`bringup-display-13`、`face-21` を含む構成）は [`IdleTouch`] を持つ。**LCD と `SpiDriver` を共有し、touch の
//!   device を足すが、**touch を読まない**（転送しない）。`TOUCH-IRQ` は取らない。
//!
//! 製品 build と LCD を描く試験 build は、`TOUCH-CS` を High に保ち、LCD の転送中に XPT2046 を選ばない。GPIO21 には外部 10 kΩ の
//! pull-up があり（`gpio-assignment.md`）、High に保つのはその電気的な状態と同じ側である。
//!
//! どの経路も CS は `crate::display` と同じく、hardware CS なしと自前の GPIO で扱う: idle は High、転送ごとに Low にし、
//! 転送が失敗しても High へ戻す。
//!
//! # SPI の設定
//!
//! Mode 0、[`SPI_CLOCK_HZ`]（1 MHz）。Mode 0 は資料からの導出で、資料が明記した値ではない（`sensor-datasheet-notes.md` の
//! `Touch controller` 節）。**clock は資料の上限（導出で 2.5 MHz、電気特性表の条件は 2 MHz）より下に取った一般値**であり、
//! 触覚の入力は安全要件の 5 項目に効かない。実機で `DOUT` が読めなければ、下げるか、`LCD-MISO` の線の状態（EXP-016〜018）を疑う。
//!
//! # 出力と、確かめていないこと
//!
//! すべて UART0 の log。`touch_raw` は IRQ が low の sample と、idle の [`IDLE_LOG_EVERY`] 件に 1 件。
//! `touch_window` は [`WINDOW_SAMPLES`] 件ごとに、各値の窓内の最小・最大と、起動からの累計（IRQ が low だった件数、全値が 0 または 0x0FFF の sample の件数、SPI の失敗）を出す
//! （`DOUT` が読めていないとき（線が駆動されていない、つながっていない）に、期待どおりの idle と区別するため）。
//! **実機で動かしたのは、`touch-sample-14` の `EXP-038`（develop `79f29ea`）と `EXP-041`（develop `b9a04e8`）の版、`touch-lcd-14` の `EXP-041` の版である**
//! （どれも touch 用 5 本を足した配線で、SPI の失敗なしに `touch_raw` と `touch_window` が出た。`EXP-041` の版は `T_IRQ` の変化と `MARK` の log、
//! `crate::spi_port` の bus の持ち方を含む。`touch-lcd-14` は LCD を描きながら 157 秒読み、`touch_spi_errors_total=0`。`docs/hardware/experiment-log.md`）。
//! 24 clock の転送での bit 位置（`deskcat_touch::decode_12bit`）と、`T_CS` が High の間 `T_DO` が線を駆動しないことは未確認である。
//! `T_IRQ` の level と X/Y/Z の値の関係は、`EXP-038` の観測（`irq_low=1` の 201 sample と idle の sample）に限り、一般化していない。
//! `T_IRQ` は poll で読み、ISR は使わない（資料は、制御 byte を送る間は `PENIRQ` が無効になるため、割り込みを mask するよう勧める）。
//!
//! # 操作との突き合わせ（#14 条件 6）
//!
//! 上の raw の log に加えて、2 つを出す。どちらも操作と `T_IRQ` を時刻（起動からの ms）で突き合わせるためで、しきい値や判定には使わない。
//!
//! - `touch_irq_edge`: `T_IRQ` の level が前回の poll から変わった poll で出す（`level=low` が high→low、`level=high` が low→high）。
//!   最初の poll は基準として `touch_irq_initial` を出す。**検出は poll ごとなので、[`POLL_INTERVAL_MS`] より短い変化（low が 1 poll の間に
//!   戻る、など）は取りこぼしうる。**時刻は poll の時刻で、変化の瞬間ではない（変化から遅れうる。poll は main loop の周回の中で呼ばれ、同じ周回の他の処理（log、Pi link）の所要時間も足されるため、
//!   遅れの上限は [`POLL_INTERVAL_MS`] とは限らず、確かめていない）。
//!   `edge` は直前の `touch_mark` からの番号、`mark` は直前の `touch_mark` の番号（まだ無ければ 0）。
//!   **`level` は転送の前に読んだ値**（変化の判定に使う）で、`after` は同じ poll の転送（4 回の変換）の後に読んだ値（`xfer=err` の poll では、失敗した時点までの変換の後の値）、`xfer` はその転送の成否である。
//!   転送の間は `PENIRQ` が無効になり low を示す（資料 `PENIRQ Output`）ため、転送が起こした変化と指の操作の変化を後で分けるのに使う
//!   （`level=low after=high` なら、転送の前は low で後は high。この build は毎 poll 転送するので、転送をしない poll は無い）。
//! - `touch_mark`: UART0 の console（stdin）から [`MARK_TOKEN`] の行を受けたとき出す（SPI の失敗が続いて停止した後は `MARK` を取り込まず、`touch_mark` は出ない）。操作を始める前に人間が `MARK` と入力して改行すれば、
//!   次の `touch_mark` までの `touch_irq_edge` を、その操作のものとして時刻で対応づけられる（`touch_raw` の payload は時刻を持たない。
//!   行頭の log の時刻（`t_ms` と同じ clock かは未確認）か、行の並びで対応づける）。`t_ms` は改行を読んだ時刻（poll の先頭の時刻と `Instant` の対を基準に、
//!   thread が読んだ `Instant` との差を足し引きしたもの）で、完全一致の行だけを受ける。
//!   **console は別 thread（`touch_console`）で読む。**`stdin.read` が入力を待って blocking なのか `WouldBlock` を返すのかは、実機で確かめていない
//!   （`bench-servo-test-17` の確認語は入力を待つ loop で、どちらでも動くため、実績はどちらも示さない）。別 thread なら、どちらでも poll は止まらない。
//!   **この thread は `EXP-041` で実機で動いた**（`MARK` を打たずに `touch_raw` が続き、`MARK` を打った 31 回に `touch_mark` 31 件）。

use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use std::rc::Rc;

use deskcat_touch::{
    decode_12bit, frame, Channel, Sample, TouchConfig, TouchEvent, TouchTracker, FRAME_LEN,
};
use esp_idf_svc::hal::gpio::{Input, InputPin, Output, OutputPin, PinDriver, Pull};
use esp_idf_svc::hal::spi::config::DriverConfig;
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiDriver};
use esp_idf_svc::sys::EspError;

use crate::spi_port::SpiPort;

/// 読む間隔（ms）。1 sample は 4 回の転送で、1 MHz なら 1 回 24 µs 強。触れ方を見るための一般値で、`crate::config` の周期とは独立。
pub const POLL_INTERVAL_MS: u64 = 20;
/// SPI の clock。module doc の「SPI の設定」。
pub const SPI_CLOCK_HZ: u32 = 1_000_000;
/// idle（`T_IRQ` が high）の sample を、この件数に 1 件 log に出す。
pub const IDLE_LOG_EVERY: u32 = 25;
/// 1 窓の件数（`POLL_INTERVAL_MS` が 20 ms なら約 5 秒）。
pub const WINDOW_SAMPLES: u32 = 250;
/// 続けて失敗したら止める回数。
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;
/// 操作の区切りとして受ける語（module doc の「操作との突き合わせ」）。完全一致だけを受ける。
pub const MARK_TOKEN: &str = "MARK";
/// これを超える長さの行は、改行まで捨てる（byte）。
const MARK_MAX_LINE: usize = 16;
/// console の読み取り thread が 1 回に読む上限（byte）。
const MARK_READ_BYTES: usize = 32;
/// console にデータが無いとき、読み取り thread が待つ時間（ms）。
const MARK_IDLE_SLEEP_MS: u64 = 20;
/// console の読み取り thread の stack（byte）。この stack の大きさで足りるかは、実機で確かめていない。
const MARK_THREAD_STACK_BYTES: usize = 6144;

/// 製品 build の撫での判定（[`StrokeDetector`]）に渡す値。**どれも暫定の一般値である。**触覚の感度は安全要件の 5 項目に効かない
/// （外れても誤検出か見逃しになるだけで、何も壊れない。`deskcat_touch` の module doc）。値の置き場はここで、`deskcat_touch` は値を持たない。
///
/// `pressure` には `Z1` の raw 値をそのまま使う。根拠は `EXP-038` と `EXP-041`（`docs/hardware/experiment-log.md`）の観測である：`EXP-038` の idle の `z1` は
/// 0〜69（A・C の窓）、`irq_low=1` の塊 9 個のうち 8 個の `z1` は 213 以上で、残る 1 個は 1 sample で `z1`=2（`x`=0、`y`=4095）だった。`EXP-041` の五点の `z1` の中央値は 342〜2251。
/// 移動量は隣り合う sample の `|Δx| + |Δy|` を足していくので、押したまま動かさなくても揺れが積もり、長押しが撫でと判定されうる
/// （`EXP-038` の塊 1 は、167 sample の間に `x` が 3367〜3653 の範囲で揺れた）。誤検出の側の誤りである。
/// **値は、製品 build を書き込む実機の窓で、撫でと長押しと軽く触れるの判定を数えてから決める。**
pub const STROKE_PRESS_THRESHOLD: u16 = 150;
/// 離れたと判定する `Z1` の下限（これ未満）。[`STROKE_PRESS_THRESHOLD`] の doc を参照する。
pub const STROKE_RELEASE_THRESHOLD: u16 = 100;
/// 触れたと判定するのに要る、続けて [`STROKE_PRESS_THRESHOLD`] 以上の sample の数。
pub const STROKE_PRESS_SAMPLES: u16 = 2;
/// 離れたと判定するのに要る、続けて [`STROKE_RELEASE_THRESHOLD`] 未満の sample の数。
pub const STROKE_RELEASE_SAMPLES: u16 = 3;
/// 撫でと判定する、1 回の接触の移動量（raw 値の単位）。`EXP-041` の五点の raw 値は、四隅の中央値の間で `x` も `y` も約 2900〜3000 動く。
pub const STROKE_TRAVEL: u32 = 600;

/// タッチが離れてから、tap（ADXL345の単tap）を送らない時間（ms）。画面を撫でる力が加速度センサーに伝わってtapと判定される（と推定している。`EXP-044`は原因を確かめていない）のを、
/// `tapped`として送らないための値で、接触の間（[`StrokeDetector::tap_blocked`]）に足す。**暫定の一般値である**（安全要件の5項目に効かない。
/// 外れても、撫でに`Surprised`が混ざるか、離れた直後の本物の叩きを落とすだけで、何も壊れない）。`deskcat_tap::MIN_TAP_INTERVAL_MS`の半分にした。
/// 根拠は `EXP-044` の観測：`stroke` 8（撫でる）と25（長押し）の2件で、`Happy` が ESP32 に受理された後、`stroke` の0.303秒後・0.505秒後の `tap_detected decision=Send` が
/// `Surprised` を送らせ、`Happy` が描かれなかった。**離れた後に tap が出る遅れは測っていない。**
/// **既知の穴：**触れたと判定する sample（[`STROKE_PRESS_SAMPLES`] 個目）より前の sample、つまり触れてから最大約40 msは接触中に数えないので、その間の tap は捨てない。
/// 実機の窓の結果を見てから、塞ぐか決める。
/// **値は、製品buildを書き込む実機の窓で、離れた後のtapの遅れを数えてから決める。**
pub const TAP_HOLDOFF_AFTER_RELEASE_MS: u64 = 250;

const STROKE_CONFIG: TouchConfig = match TouchConfig::new(
    STROKE_PRESS_THRESHOLD,
    STROKE_RELEASE_THRESHOLD,
    STROKE_PRESS_SAMPLES,
    STROKE_RELEASE_SAMPLES,
    STROKE_TRAVEL,
) {
    Ok(config) => config,
    Err(_) => panic!("STROKE_* の組み合わせを TouchConfig::new が拒んだ"),
};

/// 1 回の変換。CS を Low にして 3 byte を転送し、**転送の成否に関わらず** CS を High へ戻す（`crate::display::Ili9341::with_transaction` と同じ）。
fn convert(
    bus: &mut SpiPort<'static>,
    cs: &mut PinDriver<'static, Output>,
    channel: Channel,
) -> Result<u16, EspError> {
    let tx = frame(channel);
    let mut rx = [0u8; FRAME_LEN];
    cs.set_low()?;
    let result = bus.transfer(&mut rx, &tx);
    let released = cs.set_high();
    match result {
        Ok(()) => released.map(|()| decode_12bit(rx)),
        Err(err) => Err(err),
    }
}

/// 1 回の変換の結果（12 bit、straight binary）。
#[derive(Clone, Copy)]
struct Raw {
    x: u16,
    y: u16,
    z1: u16,
    z2: u16,
}

/// 値ごとの最小・最大。
#[derive(Clone, Copy)]
struct MinMax {
    min: u16,
    max: u16,
}

impl MinMax {
    const fn new() -> Self {
        Self {
            min: u16::MAX,
            max: 0,
        }
    }

    fn push(&mut self, value: u16) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}

#[derive(Default)]
struct Counters {
    spi_errors: u32,
    samples: u32,
    irq_low: u32,
    all_zero: u32,
    all_ones: u32,
}

/// `T_IRQ` の 1 回の変化（log は転送の後に出す）。
struct IrqEdge {
    t_ms: u64,
    low: bool,
    edge: u32,
    mark: u32,
}

/// console の読み取り thread から poll へ渡す事象。
enum ConsoleEvent {
    /// `MARK` の行を読んだ。`at` は改行を読んだ時刻。
    Mark { at: Instant },
    /// 長すぎる行（[`MARK_MAX_LINE`] を超えた）を捨てた。長さは数えていない。
    TooLong,
    /// `MARK` ではない行を捨てた。`len` はその行の長さ（byte）。
    Mismatch { len: usize },
    /// 読み取りが失敗して thread を終えた。
    ReadFailed(String),
}

/// console（stdin）を読み続ける。`stdin.read` が入力を待って blocking でも、`WouldBlock` を返しても、poll を止めないよう
/// 別 thread で読み、行の判定だけをここで行う。thread は poll 側の受け口が閉じたら終わる。
fn read_console(tx: &mpsc::Sender<ConsoleEvent>) {
    let mut stdin = std::io::stdin();
    let mut line: Vec<u8> = Vec::with_capacity(MARK_MAX_LINE);
    let mut overflowed = false;
    let mut buf = [0_u8; MARK_READ_BYTES];
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
            // データが無い（`WouldBlock`、または 0 byte）。task を譲る。
            std::thread::sleep(Duration::from_millis(MARK_IDLE_SLEEP_MS));
            continue;
        }
        for &byte in &buf[..n] {
            let event = match byte {
                b'\r' | b'\n' => {
                    let event = if overflowed {
                        Some(ConsoleEvent::TooLong)
                    } else if line == MARK_TOKEN.as_bytes() {
                        Some(ConsoleEvent::Mark { at: Instant::now() })
                    } else if !line.is_empty() {
                        Some(ConsoleEvent::Mismatch { len: line.len() })
                    } else {
                        None
                    };
                    line.clear();
                    overflowed = false;
                    event
                }
                other => {
                    if line.len() < MARK_MAX_LINE {
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

/// `MARK` の改行を読んだ時刻（uptime の ms）。`(now_ms, poll_at)` を同じ時点の対として、`at` との差を足し引きする。
fn mark_time_ms(now_ms: u64, poll_at: Instant, at: Instant) -> u64 {
    let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
    if at >= poll_at {
        now_ms.saturating_add(ms(at.saturating_duration_since(poll_at)))
    } else {
        now_ms.saturating_sub(ms(poll_at.saturating_duration_since(at)))
    }
}

/// `T_IRQ` の変化の記録と、操作の区切り（`MARK`）の受け付け。
struct Marks {
    /// console の読み取り thread からの事象。thread を起こせなかったときは `None`（touch の sample は続ける）。
    console: Option<mpsc::Receiver<ConsoleEvent>>,
    /// 直前の poll で読んだ `T_IRQ` の level（low なら `true`）。最初の poll の前は `None`。
    last_irq_low: Option<bool>,
    /// 受けた `MARK` の通し番号（まだ無ければ 0）。
    mark: u32,
    /// 直前の `MARK` からの `T_IRQ` の変化の番号。
    edge: u32,
}

impl Marks {
    /// `with_console` が偽なら、stdin を読む thread を起こさない（`MARK` を受け付けず、`T_IRQ` の変化の記録だけ行う）。
    fn new(with_console: bool) -> Self {
        let console = if with_console {
            let (tx, rx) = mpsc::channel();
            match std::thread::Builder::new()
                .name("touch_console".into())
                .stack_size(MARK_THREAD_STACK_BYTES)
                .spawn(move || read_console(&tx))
            {
                Ok(_) => Some(rx),
                Err(err) => {
                    log::error!("touch_mark_console_spawn_failed error={err}");
                    None
                }
            }
        } else {
            None
        };
        Self {
            console,
            last_irq_low: None,
            mark: 0,
            edge: 0,
        }
    }

    /// `T_IRQ` の level を前回と比べる。変わっていれば、その番号を返す（log は転送の後に [`Marks::log_edge`] で出す）。
    fn observe_irq(&mut self, now_ms: u64, irq_low: bool) -> Option<IrqEdge> {
        match self.last_irq_low.replace(irq_low) {
            None => {
                log::info!(
                    "touch_irq_initial t_ms={now_ms} level={}",
                    if irq_low { "low" } else { "high" }
                );
                None
            }
            Some(prev) if prev != irq_low => {
                self.edge = self.edge.saturating_add(1);
                Some(IrqEdge {
                    t_ms: now_ms,
                    low: irq_low,
                    edge: self.edge,
                    mark: self.mark,
                })
            }
            Some(_) => None,
        }
    }

    /// 変化を log へ出す。`level` は転送の前に読んだ level（変化の判定に使った値）、`after` はその poll の転送の後に読んだ level、
    /// `xfer` はその poll の転送の成否。**転送が `PENIRQ` を動かして出た変化と、指の操作で出た変化を、後で分けるための値である。**
    fn log_edge(edge: IrqEdge, after_low: bool, xfer_ok: bool) {
        log::info!(
            "touch_irq_edge t_ms={} level={} edge={} mark={} xfer={} after={}",
            edge.t_ms,
            if edge.low { "low" } else { "high" },
            edge.edge,
            edge.mark,
            if xfer_ok { "ok" } else { "err" },
            if after_low { "low" } else { "high" }
        );
    }

    /// thread が受けた事象を取り込み、`MARK` なら区切りを log へ出す。`now_ms` は poll の時刻、`poll_at` は同じ時点で取った `Instant`
    /// （対にして基準にする）で、`MARK` の時刻は改行を読んだ `Instant` との差を `now_ms` へ足し引きして求める。
    fn poll_console(&mut self, now_ms: u64, poll_at: Instant) {
        while let Some(rx) = &self.console {
            match rx.try_recv() {
                Ok(ConsoleEvent::Mark { at }) => {
                    self.mark = self.mark.saturating_add(1);
                    self.edge = 0;
                    log::info!(
                        "touch_mark t_ms={} mark={}",
                        mark_time_ms(now_ms, poll_at, at),
                        self.mark
                    );
                }
                Ok(ConsoleEvent::TooLong) => {
                    log::warn!("touch_mark_rejected reason=too_long");
                }
                Ok(ConsoleEvent::Mismatch { len }) => {
                    log::warn!("touch_mark_rejected reason=mismatch len={len}");
                }
                Ok(ConsoleEvent::ReadFailed(err)) => {
                    log::error!("touch_mark_console_read_failed error={err}");
                    self.console = None;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    log::error!("touch_mark_console_stopped");
                    self.console = None;
                }
            }
        }
    }
}

/// LCD と bus を共有し、touch の device を持つが読まない（module doc「配線と bus の共有」）。値は読まないが、drop で pin と device が
/// 戻らないよう保持する。
#[cfg_attr(
    any(feature = "touch-sample-14", feature = "touch-lcd-14"),
    allow(dead_code)
)]
pub struct IdleTouch {
    _device: SpiPort<'static>,
    _cs: PinDriver<'static, Output>,
}

#[cfg_attr(
    any(feature = "touch-sample-14", feature = "touch-lcd-14"),
    allow(dead_code)
)]
impl IdleTouch {
    /// **LCD と共有する `driver`** に [`SPI_CLOCK_HZ`] の device を足し、`TOUCH-CS` を High にする。**何も送らない。**
    pub fn with_shared_bus(
        driver: Rc<SpiDriver<'static>>,
        cs: impl OutputPin + 'static,
    ) -> Result<Self, EspError> {
        let mut cs = PinDriver::output(cs)?;
        cs.set_high()?;
        let device = SpiPort::shared(driver, SPI_CLOCK_HZ)?;
        Ok(Self {
            _device: device,
            _cs: cs,
        })
    }
}

/// 製品 build の撫での判定。LCD と bus を共有し、[`POLL_INTERVAL_MS`] ごとに `X`・`Y`・`Z1` を読んで `deskcat_touch::TouchTracker` へ渡す
/// （module doc「配線と bus の共有」）。`T_IRQ` は読まない（状態機械の入力ではない）。
///
/// [`StrokeDetector::poll`] は、状態機械が `Stroke` を出した poll で、Pi link の session が確立していれば、接触の始まりからの時間（ms）を返す。
/// 呼び出し側はそれを `head_touched` の `duration_ms` として送る（`docs/protocol/esp32-pi-protocol.md` §4.2）。`Stroke` は 1 回の接触につき
/// 1 回しか出ないので、送るのも接触 1 回につき 1 回である（転送の失敗で接触の状態を捨てると、その後は別の接触として数えるので、
/// 指を置いたままの 1 回の操作で 2 回送りうる）。session が無いときの `Stroke` は送らずに捨て、件数を数える。
/// UART0 の log は event の 3 種類（`touch_event kind=pressed|stroke|released`）と失敗だけで、raw は出さない。
#[cfg_attr(
    any(feature = "touch-sample-14", feature = "touch-lcd-14"),
    allow(dead_code)
)]
pub struct StrokeDetector {
    bus: SpiPort<'static>,
    cs: PinDriver<'static, Output>,
    tracker: TouchTracker,
    next_poll_ms: u64,
    consecutive_errors: u32,
    enabled: bool,
    /// 触れたと判定した poll の時刻。接触していなければ `None`。
    pressed_at_ms: Option<u64>,
    /// 接触が終わった（離れた、またはSPIの失敗で状態を捨てた）poll の時刻。まだ一度も離れていなければ `None`。
    released_at_ms: Option<u64>,
    spi_errors: u32,
    strokes_sent: u32,
    strokes_dropped_no_session: u32,
}

#[cfg_attr(
    any(feature = "touch-sample-14", feature = "touch-lcd-14"),
    allow(dead_code)
)]
impl StrokeDetector {
    /// **LCD と共有する `driver`** に [`SPI_CLOCK_HZ`] の device を足し、`TOUCH-CS` を High にする。**この段では何も送らない**（最初の転送は `poll`）。
    pub fn with_shared_bus(
        driver: Rc<SpiDriver<'static>>,
        cs: impl OutputPin + 'static,
        now_ms: u64,
    ) -> Result<Self, EspError> {
        let mut cs = PinDriver::output(cs)?;
        cs.set_high()?;
        let bus = SpiPort::shared(driver, SPI_CLOCK_HZ)?;
        Ok(Self {
            bus,
            cs,
            tracker: TouchTracker::new(STROKE_CONFIG),
            next_poll_ms: now_ms,
            consecutive_errors: 0,
            enabled: true,
            pressed_at_ms: None,
            released_at_ms: None,
            spi_errors: 0,
            strokes_sent: 0,
            strokes_dropped_no_session: 0,
        })
    }

    /// 次に動く時刻（uptime の ms）。止まっていれば `None`。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.enabled.then_some(self.next_poll_ms)
    }

    /// 接触中か、離れてから [`TAP_HOLDOFF_AFTER_RELEASE_MS`] 未満か。`true` の間の tap は `tapped` として送らない（`deskcat_tap::touch_blocks_tap`）。
    pub fn tap_blocked(&self, now_ms: u64) -> bool {
        deskcat_tap::touch_blocks_tap(
            self.tracker.is_touching(),
            self.released_at_ms,
            now_ms,
            TAP_HOLDOFF_AFTER_RELEASE_MS,
        )
    }

    /// 時刻に達していれば 1 sample 読む。**`head_touched` を送るべきなら、接触の始まりからの時間（ms）を返す。**
    pub fn poll(&mut self, now_ms: u64, session_established: bool) -> Option<u32> {
        if !self.enabled || now_ms < self.next_poll_ms {
            return None;
        }
        self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
        let sample = match self.read_sample() {
            Ok(sample) => sample,
            Err(err) => {
                self.on_spi_error(&err, now_ms);
                return None;
            }
        };
        self.consecutive_errors = 0;
        match self.tracker.update(sample)? {
            TouchEvent::Pressed { x, y } => {
                self.pressed_at_ms = Some(now_ms);
                log::info!(
                    "touch_event kind=pressed x={x} y={y} z1={}",
                    sample.pressure
                );
                None
            }
            TouchEvent::Stroke { travel } => {
                let started = self.pressed_at_ms.unwrap_or(now_ms);
                let duration_ms = u32::try_from(now_ms.saturating_sub(started)).unwrap_or(u32::MAX);
                if session_established {
                    self.strokes_sent = self.strokes_sent.saturating_add(1);
                } else {
                    self.strokes_dropped_no_session =
                        self.strokes_dropped_no_session.saturating_add(1);
                }
                log::info!(
                    "touch_event kind=stroke travel={travel} duration_ms={duration_ms} send={session_established} sent={} dropped_no_session={}",
                    self.strokes_sent,
                    self.strokes_dropped_no_session
                );
                session_established.then_some(duration_ms)
            }
            TouchEvent::Released {
                samples,
                travel,
                stroked,
            } => {
                self.pressed_at_ms = None;
                self.released_at_ms = Some(now_ms);
                log::info!(
                    "touch_event kind=released samples={samples} travel={travel} stroked={stroked}"
                );
                None
            }
        }
    }

    fn read_sample(&mut self) -> Result<Sample, EspError> {
        let x = convert(&mut self.bus, &mut self.cs, Channel::X)?;
        let y = convert(&mut self.bus, &mut self.cs, Channel::Y)?;
        let z1 = convert(&mut self.bus, &mut self.cs, Channel::Z1)?;
        Ok(Sample { pressure: z1, x, y })
    }

    /// 転送の失敗。接触の途中の状態を捨て（失敗した sample の前後を 1 回の接触として扱わない）、続けて
    /// [`MAX_CONSECUTIVE_ERRORS`] 回失敗したら止める。
    fn on_spi_error(&mut self, err: &EspError, now_ms: u64) {
        self.spi_errors = self.spi_errors.saturating_add(1);
        self.consecutive_errors += 1;
        if self.tracker.is_touching() {
            self.released_at_ms = Some(now_ms);
        }
        self.tracker.reset();
        self.pressed_at_ms = None;
        log::error!(
            "touch_fault kind=spi error={err} count={} consecutive={}",
            self.spi_errors,
            self.consecutive_errors
        );
        if self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            self.enabled = false;
            log::error!("touch_stroke_stopped spi_errors={}", self.spi_errors);
        }
    }
}

pub struct TouchSampler {
    bus: SpiPort<'static>,
    cs: PinDriver<'static, Output>,
    irq: PinDriver<'static, Input>,
    /// `LCD-CS`。High に固定して LCD を選択させない。値は読まないが、drop で pin が戻らないよう保持する。
    /// LCD と bus を共有する build（`touch-lcd-14`）では LCD の driver が持つので `None`。
    _lcd_cs: Option<PinDriver<'static, Output>>,
    next_poll_ms: u64,
    consecutive_errors: u32,
    stopped: bool,
    window: u32,
    window_samples: u32,
    x: MinMax,
    y: MinMax,
    z1: MinMax,
    z2: MinMax,
    counters: Counters,
    marks: Marks,
}

impl TouchSampler {
    /// SPI host と GPIO を結線する。**まだ何も送らない。**CS は idle の High、`LCD-CS` も High にする。
    /// bus は専有し、bus lock を握り続ける（`touch-sample-14` の型。[`SpiPort::Held`]）。
    #[cfg_attr(feature = "touch-lcd-14", allow(dead_code))]
    #[allow(clippy::too_many_arguments)]
    pub fn new<SPI: SpiAnyPins + 'static>(
        spi: SPI,
        sclk: impl OutputPin + 'static,
        mosi: impl OutputPin + 'static,
        miso: impl InputPin + 'static,
        cs: impl OutputPin + 'static,
        irq: impl InputPin + 'static,
        lcd_cs: impl OutputPin + 'static,
        now_ms: u64,
    ) -> Result<Self, EspError> {
        let driver = SpiDriver::new(spi, sclk, mosi, Some(miso), &DriverConfig::new())?;
        let bus = SpiPort::held(driver, SPI_CLOCK_HZ)?;
        let mut lcd_cs = PinDriver::output(lcd_cs)?;
        lcd_cs.set_high()?;
        Self::with_port(bus, cs, irq, Some(lcd_cs), true, now_ms)
    }

    /// `touch-lcd-14` 用。**LCD と共有する `driver`** に、[`SPI_CLOCK_HZ`] の device を足して作る。bus は転送ごとに取る
    /// （[`SpiPort::Shared`]）。`LCD-CS` は LCD の driver が持つ。
    #[cfg_attr(not(feature = "touch-lcd-14"), allow(dead_code))]
    pub fn with_shared_bus(
        driver: Rc<SpiDriver<'static>>,
        cs: impl OutputPin + 'static,
        irq: impl InputPin + 'static,
        now_ms: u64,
    ) -> Result<Self, EspError> {
        let bus = SpiPort::shared(driver, SPI_CLOCK_HZ)?;
        Self::with_port(bus, cs, irq, None, false, now_ms)
    }

    fn with_port(
        bus: SpiPort<'static>,
        cs: impl OutputPin + 'static,
        irq: impl InputPin + 'static,
        lcd_cs: Option<PinDriver<'static, Output>>,
        with_console: bool,
        now_ms: u64,
    ) -> Result<Self, EspError> {
        let mut cs = PinDriver::output(cs)?;
        // 外部 pull-up が無い間の浮きを避けるため、driver を作ったらすぐ High にする。
        cs.set_high()?;
        // `TOUCH-IRQ` は外部 pull を付けない（`gpio-assignment.md`）。XPT2046 の `PENIRQ` が内部 pull-up 付きの出力なので、ESP32 側も pull しない。
        let irq = PinDriver::input(irq, Pull::Floating)?;
        Ok(Self {
            bus,
            cs,
            irq,
            _lcd_cs: lcd_cs,
            next_poll_ms: now_ms,
            consecutive_errors: 0,
            stopped: false,
            window: 0,
            window_samples: 0,
            x: MinMax::new(),
            y: MinMax::new(),
            z1: MinMax::new(),
            z2: MinMax::new(),
            counters: Counters::default(),
            marks: Marks::new(with_console),
        })
    }

    /// 起動からの SPI 転送の失敗の件数。
    #[cfg_attr(not(feature = "touch-lcd-14"), allow(dead_code))]
    pub fn spi_errors(&self) -> u32 {
        self.counters.spi_errors
    }

    /// 次に動く時刻（uptime の ms）。止まっていれば `None`。
    pub fn next_deadline_ms(&self) -> Option<u64> {
        (!self.stopped).then_some(self.next_poll_ms)
    }

    /// 1 sample 読む。時刻が来ていなければ何もしない。
    pub fn poll(&mut self, now_ms: u64) {
        if self.stopped || now_ms < self.next_poll_ms {
            return;
        }
        self.next_poll_ms = now_ms + POLL_INTERVAL_MS;
        // `MARK` は IRQ を読む前に取り込む。この poll の IRQ の変化は、取り込んだ `MARK` の番号に数える（読んだ後に届いた `MARK` は次の poll）。
        self.marks.poll_console(now_ms, Instant::now());
        // **IRQ は転送の前に読む。**転送の間は `PENIRQ` が無効になり low を示すため（資料 `PENIRQ Output`）、後で読むと touch と区別できない。
        let irq_low = self.irq.is_low();
        let edge = self.marks.observe_irq(now_ms, irq_low);
        let result = self.read_raw();
        if let Some(edge) = edge {
            // 転送の後の level。転送が `PENIRQ` を動かしたかを、変化の log に添える。
            Marks::log_edge(edge, self.irq.is_low(), result.is_ok());
        }
        match result {
            Ok(raw) => {
                self.consecutive_errors = 0;
                self.accept(raw, irq_low);
            }
            Err(err) => {
                self.counters.spi_errors = self.counters.spi_errors.saturating_add(1);
                self.consecutive_errors += 1;
                log::error!(
                    "touch_fault kind=spi error={err} count={}",
                    self.counters.spi_errors
                );
                if self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    log::error!("touch_sample_stopped");
                    self.stopped = true;
                    // 停止の後は `MARK` を取り込まない。受け口を捨てて、読み取り thread の event が溜まらないようにする
                    // （thread は次の送信の失敗で終わる）。
                    self.marks.console = None;
                }
            }
        }
    }

    fn read_raw(&mut self) -> Result<Raw, EspError> {
        Ok(Raw {
            x: self.convert(Channel::X)?,
            y: self.convert(Channel::Y)?,
            z1: self.convert(Channel::Z1)?,
            z2: self.convert(Channel::Z2)?,
        })
    }

    fn convert(&mut self, channel: Channel) -> Result<u16, EspError> {
        convert(&mut self.bus, &mut self.cs, channel)
    }

    fn accept(&mut self, raw: Raw, irq_low: bool) {
        self.counters.samples = self.counters.samples.saturating_add(1);
        if irq_low {
            self.counters.irq_low = self.counters.irq_low.saturating_add(1);
        }
        let all = [raw.x, raw.y, raw.z1, raw.z2];
        if all.iter().all(|&v| v == 0) {
            self.counters.all_zero = self.counters.all_zero.saturating_add(1);
        }
        if all.iter().all(|&v| v == 0x0FFF) {
            self.counters.all_ones = self.counters.all_ones.saturating_add(1);
        }
        if irq_low || self.counters.samples % IDLE_LOG_EVERY == 1 {
            log::info!(
                "touch_raw n={} irq_low={} x={} y={} z1={} z2={}",
                self.counters.samples,
                u8::from(irq_low),
                raw.x,
                raw.y,
                raw.z1,
                raw.z2
            );
        }
        self.x.push(raw.x);
        self.y.push(raw.y);
        self.z1.push(raw.z1);
        self.z2.push(raw.z2);
        self.window_samples += 1;
        if self.window_samples >= WINDOW_SAMPLES {
            self.finish_window();
        }
    }

    fn finish_window(&mut self) {
        self.window += 1;
        log::info!(
            "touch_window w={} n={} x=[{},{}] y=[{},{}] z1=[{},{}] z2=[{},{}] irq_low_total={} all_zero_total={} all_ones_total={} spi_errors_total={}",
            self.window,
            self.window_samples,
            self.x.min,
            self.x.max,
            self.y.min,
            self.y.max,
            self.z1.min,
            self.z1.max,
            self.z2.min,
            self.z2.max,
            self.counters.irq_low,
            self.counters.all_zero,
            self.counters.all_ones,
            self.counters.spi_errors
        );
        self.window_samples = 0;
        self.x = MinMax::new();
        self.y = MinMax::new();
        self.z1 = MinMax::new();
        self.z2 = MinMax::new();
    }
}
