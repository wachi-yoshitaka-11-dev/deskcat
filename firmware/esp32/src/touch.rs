//! #14 の試験 build 専用の `TOUCH-01`（XPT2046）の raw 読み出し。bus はこの main-loop task が所有する。
//!
//! 制御 byte と 3 byte 応答の復号は `deskcat_touch` が持つ（host test 済み。ここへ再掲しない）。
//! `POLL_INTERVAL_MS` ごとに `X`・`Y`・`Z1`・`Z2` を 1 回ずつ読み（4 回の転送で 1 sample）、`T_IRQ`（`TOUCH-IRQ`）の level とあわせて
//! UART0 の log へ出す。**状態機械（`deskcat_touch::TouchTracker`）はこの build へ配線しない。**しきい値の根拠になる raw sample を
//! 取る build であり、仮の値を入れない（`deskcat_touch` の module doc）。
//!
//! # 配線と bus の共有
//!
//! pin は [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md) の信号 inventory が正本で、
//! `LCD-SCLK`＝GPIO18、`LCD-MOSI`＝GPIO23、`LCD-MISO`＝GPIO19（`TOUCH-01` と共有）、`TOUCH-CS`＝GPIO21、`TOUCH-IRQ`＝GPIO34 を使う。
//! **この build は LCD を初期化せず、`crate::display` を使わない**（LCD の driver は `SpiBusDriver` で 6 MHz の bus lock を握り続けるため、
//! 同じ SPI host を 2 つの driver で共有できない。両者を同じ bus で動かすには `display.rs` と製品 build を変える必要があり、この build の範囲外である）。
//! **LCD が接続されていても選択されないよう、`LCD-CS`（GPIO22）を High に固定する**（`crate::display` と同じく CS は active low）。
//! CS は `crate::display` と同じく、`SpiBusDriver`（hardware CS なし）と自前の GPIO で扱う: idle は High、転送ごとに Low にし、
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
//! **この file の手順は実機で動かしていない。**ESP32 build と Clippy までである。24 clock の転送での bit 位置（`deskcat_touch::decode_12bit`）、
//! `T_CS` が High の間 `T_DO` が線を駆動しないこと、`T_IRQ` の level と X/Y/Z の値の関係はすべて未確認である。
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
//!   行頭の log の時刻（`t_ms` と同じ clock かは未確認）か、行の並びで対応づける）。`t_ms` は改行を読んだ時刻（poll の時刻から、
//!   thread が読んでからの経過を引いたもの）で、完全一致の行だけを受ける。
//!   **console は別 thread（`touch_console`）で読む。**`stdin.read` が入力を待って blocking なのか `WouldBlock` を返すのかは、実機で確かめていない
//!   （`bench-servo-test-17` の確認語は入力を待つ loop で、どちらでも動くため、実績はどちらも示さない）。別 thread なら、どちらでも poll は止まらない。
//!   **この thread が実機で動くこと（MARK を打たずに `touch_raw` が続くこと、MARK で `touch_mark` が出ること）は未確認である。**

use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use deskcat_touch::{decode_12bit, frame, Channel, FRAME_LEN};
use esp_idf_svc::hal::gpio::{Input, InputPin, Output, OutputPin, PinDriver, Pull};
use esp_idf_svc::hal::spi::config::{Config as SpiConfig, DriverConfig, MODE_0};
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiBusDriver, SpiDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;

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
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let console = match std::thread::Builder::new()
            .name("touch_console".into())
            .stack_size(MARK_THREAD_STACK_BYTES)
            .spawn(move || read_console(&tx))
        {
            Ok(_) => Some(rx),
            Err(err) => {
                log::error!("touch_mark_console_spawn_failed error={err}");
                None
            }
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

    /// thread が受けた事象を取り込み、`MARK` なら区切りを log へ出す。`now_ms` は poll の時刻で、`MARK` の時刻はそこから
    /// 改行を読んでからの経過を引いて求める。
    fn poll_console(&mut self, now_ms: u64) {
        while let Some(rx) = &self.console {
            match rx.try_recv() {
                Ok(ConsoleEvent::Mark { at }) => {
                    let age_ms = u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX);
                    self.mark = self.mark.saturating_add(1);
                    self.edge = 0;
                    log::info!(
                        "touch_mark t_ms={} mark={}",
                        now_ms.saturating_sub(age_ms),
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

pub struct TouchSampler {
    bus: SpiBusDriver<'static, SpiDriver<'static>>,
    cs: PinDriver<'static, Output>,
    irq: PinDriver<'static, Input>,
    /// `LCD-CS`。High に固定して LCD を選択させない。値は読まないが、drop で pin が戻らないよう保持する。
    _lcd_cs: PinDriver<'static, Output>,
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
        let bus = SpiBusDriver::new(
            driver,
            &SpiConfig::new()
                .baudrate(Hertz(SPI_CLOCK_HZ))
                .data_mode(MODE_0),
        )?;
        let mut cs = PinDriver::output(cs)?;
        let mut lcd_cs = PinDriver::output(lcd_cs)?;
        // 外部 pull-up が無い間の浮きを避けるため、driver を作ったらすぐ High にする。
        cs.set_high()?;
        lcd_cs.set_high()?;
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
            marks: Marks::new(),
        })
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
        // **IRQ は転送の前に読む。**転送の間は `PENIRQ` が無効になり low を示すため（資料 `PENIRQ Output`）、後で読むと touch と区別できない。
        let irq_low = self.irq.is_low();
        self.marks.poll_console(now_ms);
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

    /// 1 回の変換。CS を Low にして 3 byte を転送し、**転送の成否に関わらず** CS を High へ戻す（`crate::display::Ili9341::with_transaction` と同じ）。
    fn convert(&mut self, channel: Channel) -> Result<u16, EspError> {
        let tx = frame(channel);
        let mut rx = [0u8; FRAME_LEN];
        self.cs.set_low()?;
        let result = self.bus.transfer(&mut rx, &tx);
        let released = self.cs.set_high();
        match result {
            Ok(()) => released.map(|()| decode_12bit(rx)),
            Err(err) => Err(err),
        }
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
