//! [Issue #514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)の追加LEDの
//! bring-up。`LED-COMM`（白、GPIO2）と`LED-REACT`（赤、GPIO5）を駆動する。どちらもHighで点灯する。
//!
//! **`bringup-led-514` feature付きbuildだけがcompileする。**既定buildはGPIO2／GPIO5に触れない。
//! 配線、抵抗、起動時の電位、試験の順序は
//! [led-514-demo.md](../../../docs/hardware/led-514-demo.md)が正本であり、ここへ再掲しない。
//!
//! # 表示
//!
//! 起動時のbring-upが終わった時刻（`main.rs`の`bringup_done_ms`）からの経過時間だけで決める
//! （[`outputs_at`]）。その後のPi linkのUART初期化と`boot`送信の分だけ、最初の区間が短くなる。
//! `bringup-display-13`付きbuildでは、LCDの描画とI2Cのbring-upがmain loopの中で進み、点灯試験は
//! それと並行して進む。
//!
//! 1. 点灯試験。赤だけ→白だけ→両方を、それぞれ[`crate::config::LED_LAMP_TEST_HOLD_MS`]ずつ点ける。
//!    人間が極性・明るさ・抵抗の両端電圧を確かめるための区間である。
//! 2. その後は、白を[`crate::config::LED_BLINK_PERIOD_MS`]周期で点滅させ、赤を消す。
//!
//! **白を点灯（通信成立）へ切り替える経路は無い。**継続通信の判定（`PROTO-TBD-010`／
//! `HW-TBD-017`）がまだ無いためである。`boot`のACKや`ping`の受信だけでは、その後の
//! 通信が続いていることを示せない。**赤を`happy`で点ける経路も無い。**`face-21`付きbuildは受け取った
//! `set_expression`をLCDへ描くが（`crate::face`）、LEDは駆動しない。
//! どちらも`HW-TBD-037`が追う。
//!
//! # 起動からの状態
//!
//! [`Leds::new`]が両pinを出力にし、消灯側（Low）へ設定する。
//! reset中とreset直後は、IO_MUXの記載どおり内部pull（GPIO2はpull-down、GPIO5はpull-up）が
//! 有効である。外部10 kΩはどちらもpull-downとして置き、GPIO5も消灯側へ定める
//! （`gpio-assignment.md`の`reset時のpin状態`と`選定した値と本数`）。
//! **その後、ROM／2nd-stage bootloaderが動く区間と`main()`の先頭の実際の電位は、一次資料でも
//! 実機でも確かめていない**（`gpio-assignment.md`の`LED-COMM`／`LED-REACT`行）。**向きの設定（`gpio_set_direction`）と消灯levelの設定の間に、出力registerの
//! 値が一瞬出る。**両pinとも、その値が点灯側（High）か消灯側（Low）かも、長さも確かめていない。
//! 電流は直列抵抗`R`で制限される（led-514-demo.mdの`電流の見積もりと残る観察`）。esp-idf-halの
//! `PinDriver`は向きを先に設定する。`unsafe`を使わずにこの区間を無くす方法は見つけていない（試していない）。
//! Hardware Safety Policy §4からの逸脱であり、led-514-demo.mdの残余riskに挙げてある。
//!
//! drive strengthはESP-IDFの既定のままにする。電流を決めるのは直列の抵抗`R`である
//! （値はled-514-demo.mdが持つ）。
//!
//! [`Leds::new`]が途中で失敗すると、作り終えた側のdriverはDropでpinを無効（内部pullなし）へ戻す。
//! そのpinの消灯は外部pullだけが保つ。

use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::sys::EspError;

use crate::config::{LED_BLINK_PERIOD_MS, LED_LAMP_TEST_HOLD_MS};

/// 表示の区間。logへ出す名前は[`Phase::name`]。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// 点灯試験。赤だけ点ける。
    LampRed,
    /// 点灯試験。白だけ点ける。
    LampWhite,
    /// 点灯試験。両方点ける。
    LampBoth,
    /// 通信成立を確認できていない。白を点滅させ、赤を消す。
    LinkUnconfirmed,
}

impl Phase {
    /// logへ出す名前。
    pub fn name(self) -> &'static str {
        match self {
            Phase::LampRed => "lamp_red",
            Phase::LampWhite => "lamp_white",
            Phase::LampBoth => "lamp_both",
            Phase::LinkUnconfirmed => "link_unconfirmed",
        }
    }
}

/// 2本のLEDを点けるかどうか。pinのlevel（極性）はここでは扱わない。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outputs {
    /// `LED-COMM`（白）を点けるか。
    pub white_on: bool,
    /// `LED-REACT`（赤）を点けるか。
    pub red_on: bool,
}

/// 点灯試験が終わる経過時間（milliseconds）。
const LAMP_TEST_END_MS: u64 = 3 * LED_LAMP_TEST_HOLD_MS as u64;

// [`outputs_at`]と[`next_change_ms`]は、点滅周期が2以上の偶数で、試験区間が0でないことを前提に
// 割り算する。configの値を変えてこの前提を崩したらcompileを止める。
const _: () = assert!(LED_BLINK_PERIOD_MS >= 2 && LED_BLINK_PERIOD_MS % 2 == 0);
const _: () = assert!(LED_LAMP_TEST_HOLD_MS > 0);

/// 経過時間`elapsed_ms`での区間。
pub fn phase_at(elapsed_ms: u64) -> Phase {
    let hold = u64::from(LED_LAMP_TEST_HOLD_MS);
    if elapsed_ms < hold {
        Phase::LampRed
    } else if elapsed_ms < 2 * hold {
        Phase::LampWhite
    } else if elapsed_ms < LAMP_TEST_END_MS {
        Phase::LampBoth
    } else {
        Phase::LinkUnconfirmed
    }
}

/// 経過時間`elapsed_ms`での出力。点滅は点灯試験の終わりを位相の起点とし、前半を点灯にする。
pub fn outputs_at(elapsed_ms: u64) -> Outputs {
    match phase_at(elapsed_ms) {
        Phase::LampRed => Outputs {
            white_on: false,
            red_on: true,
        },
        Phase::LampWhite => Outputs {
            white_on: true,
            red_on: false,
        },
        Phase::LampBoth => Outputs {
            white_on: true,
            red_on: true,
        },
        Phase::LinkUnconfirmed => {
            let period = u64::from(LED_BLINK_PERIOD_MS);
            let in_period = (elapsed_ms - LAMP_TEST_END_MS) % period;
            Outputs {
                white_on: in_period < period / 2,
                red_on: false,
            }
        }
    }
}

/// `elapsed_ms`より後で、出力が次に変わる経過時間。main loopの待ちの上限に使う。
pub fn next_change_ms(elapsed_ms: u64) -> u64 {
    let hold = u64::from(LED_LAMP_TEST_HOLD_MS);
    if elapsed_ms < LAMP_TEST_END_MS {
        return (elapsed_ms / hold + 1) * hold;
    }
    let half = u64::from(LED_BLINK_PERIOD_MS / 2);
    let since = elapsed_ms - LAMP_TEST_END_MS;
    LAMP_TEST_END_MS + (since / half + 1) * half
}

/// 2本のLEDのdriver。
pub struct Leds<'d> {
    comm: PinDriver<'d, Output>,
    react: PinDriver<'d, Output>,
    /// 最後にlogへ出した区間。区間が変わったときだけlogを出す（点滅の毎回は出さない）。
    logged_phase: Option<Phase>,
    /// pinへの書き込みに失敗した回数。**Protocol counterではない。**
    write_errors: u32,
}

impl<'d> Leds<'d> {
    /// 両pinを出力にし、消灯側へ設定する（module docの`起動からの状態`）。
    pub fn new(comm: impl OutputPin + 'd, react: impl OutputPin + 'd) -> Result<Self, EspError> {
        // esp-idf-halはpinをtype levelで選ぶため、configのGPIO番号はpin選択には使えない
        // （呼び出し側が手書きする）。一致はdebug buildでだけ確認する（release buildは確認しない）。
        debug_assert_eq!(
            comm.pin(),
            crate::config::LED_COMM_GPIO,
            "呼び出し側が渡したpinがconfig::LED_COMM_GPIOと一致しない"
        );
        debug_assert_eq!(
            react.pin(),
            crate::config::LED_REACT_GPIO,
            "呼び出し側が渡したpinがconfig::LED_REACT_GPIOと一致しない"
        );

        let mut comm = PinDriver::output(comm)?;
        comm.set_low()?;
        let mut react = PinDriver::output(react)?;
        react.set_low()?;

        Ok(Self {
            comm,
            react,
            logged_phase: None,
            write_errors: 0,
        })
    }

    /// 経過時間`elapsed_ms`の出力をpinへ書く。`now_ms`はlog用のuptime。
    ///
    /// 書き込みの失敗は数えてlogへ出し、次の呼び出しで書き直す。**表示のためにmain loopを止めない。**
    pub fn update(&mut self, elapsed_ms: u64, now_ms: u64) {
        let phase = phase_at(elapsed_ms);
        let out = outputs_at(elapsed_ms);

        // 白も赤もHighで点灯（led-514-demo.mdの`初回デモの配線案`）。
        let comm = if out.white_on {
            self.comm.set_high()
        } else {
            self.comm.set_low()
        };
        let react = if out.red_on {
            self.react.set_high()
        } else {
            self.react.set_low()
        };
        for (pin, result) in [("comm", comm), ("react", react)] {
            if let Err(err) = result {
                self.write_errors = self.write_errors.saturating_add(1);
                log::warn!(
                    "led_write_error pin={pin} errors={} uptime_ms={now_ms} err={err}",
                    self.write_errors
                );
            }
        }

        if self.logged_phase != Some(phase) {
            self.logged_phase = Some(phase);
            log::info!(
                "led phase={} white={} red={} uptime_ms={now_ms}",
                phase.name(),
                on_off(out.white_on),
                on_off(out.red_on)
            );
        }
    }
}

fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}
