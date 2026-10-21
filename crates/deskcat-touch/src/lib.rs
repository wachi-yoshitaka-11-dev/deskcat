//! `TOUCH-01`（XPT2046）の制御byteと応答の復号、raw sampleの列から「触れた・離れた」と撫でのeventを導く状態機械
//! （Issue #14）。
//!
//! このcrateはSPIもGPIOも知らない。SPIで送るbyte列と、受け取った3 byteの読み方と、sampleの列の判定だけを持つ
//! （hostでtestできるように分けた。SPIを動かすのは`firmware/esp32`の`crate::touch`）。
//!
//! # 出所
//!
//! 制御byteのbit配置、チャネルの割り当て、`PD`の意味、12 bit・straight binaryの出力は、**XPTEK `XPT2046 Data Sheet 2007.5`**
//! （`docs/hardware/sensor-datasheet-notes.md`の`Touch controller`節に、URL・sha256・取得日と、導出と明記した値の一覧がある）の
//! 表5・表6・表8、`Digital Interface`節、`Data Format`節による。**資料が16進で示した値ではなく、bit配置から組み立てた値であり、
//! 24 clockの転送でのbit位置（[`decode_12bit`]）は図12からの導出で、実機で確かめるまで確定しない。**
//!
//! # 値の扱い
//!
//! [`TouchConfig`]のしきい値・連続sample数・撫での移動量は、**このcrateは値を持たない。**呼び出し側が渡す。実機のraw sample
//! （idle時のnoiseと、触れたときの値）を取るまで根拠のある値が無く、仮の値をここへ書くと正本の値に見えるためである
//! （[AGENTS.md](../../../AGENTS.md)の推測禁止）。testの値は、境界の挙動を見るための入力であり、推奨値ではない。
//! **触覚の感度は安全要件の5項目に効かない**（外れても誤検出や検出漏れになるだけで、何も壊れない）。

#![cfg_attr(not(test), no_std)]

/// XPT2046が1回の変換に要する転送の長さ（byte）。制御byte、結果の上位、結果の下位の3回（各8 clock、計24 clock）。
pub const FRAME_LEN: usize = 3;

/// 差動modeで位置・圧力を測るチャネル（`A2 A1 A0`）。**表5**の割り当てである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// `Y`位置（`A2 A1 A0`＝`001`）。
    Y,
    /// `Z1`（`011`）。圧力の測定に使う。
    Z1,
    /// `Z2`（`100`）。圧力の測定に使う。
    Z2,
    /// `X`位置（`101`）。
    X,
}

impl Channel {
    /// `A2 A1 A0`の3 bit（表5）。
    const fn address(self) -> u8 {
        match self {
            Self::Y => 0b001,
            Self::Z1 => 0b011,
            Self::Z2 => 0b100,
            Self::X => 0b101,
        }
    }
}

/// 12 bit・差動（`SER/DFR`＝0）・`PD1 PD0`＝`00`の制御byte。
///
/// 並びは`S`（bit7、常に1）、`A2 A1 A0`（bit6〜4）、`MODE`（bit3、0＝12 bit）、`SER/DFR`（bit2、0＝差動）、`PD1 PD0`（bit1〜0）（表6）。
/// **`PD=00`は変換ごとにpower-downし`PENIRQ`を有効にする。最初の変換から有効であり、`PD1=0`なのでreferenceもoffにする**（表8と
/// `Internal Reference`節。電源投入後に`PD1=0`の書き込みを要求する）。
#[must_use]
pub const fn control_byte(channel: Channel) -> u8 {
    0x80 | (channel.address() << 4)
}

/// 1回の変換のために送る3 byte。2 byte目と3 byte目は、`DOUT`を読むためのclockを出すだけで、`DIN`の値は無視される。
#[must_use]
pub const fn frame(channel: Channel) -> [u8; FRAME_LEN] {
    [control_byte(channel), 0, 0]
}

/// 受け取った3 byteから12 bitの値を取り出す。
///
/// 1 byte目は制御byteの間に`DOUT`が出す値で、読まない。**2 byte目の最上位bitは0で（変換が始まる前の区間）、続く7 bitが`D11`〜`D5`、
/// 3 byte目の上位5 bitが`D4`〜`D0`である**（図12の`DOUT`と、`DOUT`が`DCLK`の立ち下がりで変わることからの導出。資料にこの式は無い）。
/// したがって`((rx[1] << 8) | rx[2]) >> 3`の下位12 bitである。**実機で確かめるまでこの位置を確定しない。**
/// 上位の余り（2 byte目の最上位bit）は`0x0FFF`で落とす。
#[must_use]
pub const fn decode_12bit(rx: [u8; FRAME_LEN]) -> u16 {
    ((((rx[1] as u16) << 8) | rx[2] as u16) >> 3) & 0x0FFF
}

/// 1回のsample。`pressure`は呼び出し側が決めた「押しの強さ」で、大きいほど強い。**このcrateは`pressure`の求め方を知らない**
/// （`Z1`をそのまま使うか、`Z1`と`Z2`と`X`から接触抵抗を求めた値を反転するかは、実機のraw sampleを取ってから決める）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// 押しの強さ（呼び出し側の定義）。
    pub pressure: u16,
    /// `X`のraw値。
    pub x: u16,
    /// `Y`のraw値。
    pub y: u16,
}

/// [`TouchConfig::new`]が拒む設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// `release_threshold`が`press_threshold`より大きい（hysteresisが逆。触れた直後に離れる判定が起きうる）。
    ReleaseAbovePress,
    /// `press_samples`または`release_samples`が0（debounceなしでは1回の揺れで判定が変わる。1以上を要求する）。
    ZeroSamples,
    /// `stroke_travel`が0（接触した瞬間に撫でになる）。
    ZeroStrokeTravel,
}

/// 状態機械の設定。**値は呼び出し側が渡す**（モジュールdoc）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchConfig {
    press_threshold: u16,
    release_threshold: u16,
    press_samples: u16,
    release_samples: u16,
    stroke_travel: u32,
}

impl TouchConfig {
    /// 設定を検証して作る。
    ///
    /// - `press_threshold`: `pressure`がこの値**以上**のsampleが`press_samples`回連続したら触れたと判定する。
    /// - `release_threshold`: 接触中に`pressure`がこの値**未満**のsampleが`release_samples`回連続したら離れたと判定する。
    ///   `press_threshold`以下でなければならない（`press_threshold`との間は、接触中の揺れを許すhysteresisの帯である）。
    /// - `stroke_travel`: 接触中の移動量（隣り合うsample間の`|Δx| + |Δy|`の累計、raw値の単位）がこの値**以上**になったら撫で。
    ///
    /// # Errors
    ///
    /// [`ConfigError`]を参照する。
    pub const fn new(
        press_threshold: u16,
        release_threshold: u16,
        press_samples: u16,
        release_samples: u16,
        stroke_travel: u32,
    ) -> Result<Self, ConfigError> {
        if release_threshold > press_threshold {
            return Err(ConfigError::ReleaseAbovePress);
        }
        if press_samples == 0 || release_samples == 0 {
            return Err(ConfigError::ZeroSamples);
        }
        if stroke_travel == 0 {
            return Err(ConfigError::ZeroStrokeTravel);
        }
        Ok(Self {
            press_threshold,
            release_threshold,
            press_samples,
            release_samples,
            stroke_travel,
        })
    }
}

/// 状態機械が出すevent。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchEvent {
    /// 触れた。位置は、触れたと判定したsampleの値。
    Pressed {
        /// `X`のraw値。
        x: u16,
        /// `Y`のraw値。
        y: u16,
    },
    /// 撫で。1回の接触につき、移動量が`stroke_travel`に初めて達したときに1回だけ出る。
    Stroke {
        /// 出した時点の累計の移動量。
        travel: u32,
    },
    /// 離れた。
    Released {
        /// 接触中に数えたsampleの数（触れたと判定したsampleを含み、離れた判定のsampleを含まない）。
        samples: u32,
        /// 接触中の移動量の累計。
        travel: u32,
        /// 接触中に[`TouchEvent::Stroke`]を出したか。
        stroked: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// 触れていない。`pressed_run`は、`press_threshold`以上のsampleが連続している数。
    Idle { pressed_run: u16 },
    Touching {
        /// `release_threshold`未満のsampleが連続している数。
        below_run: u16,
        last_x: u16,
        last_y: u16,
        samples: u32,
        travel: u32,
        stroked: bool,
    },
}

/// raw sampleの列から[`TouchEvent`]を導く状態機械。
///
/// **位置は`pressure`が`release_threshold`以上のsampleだけで更新する**（弱い押しのsampleの位置は信用できず、離れる直前の
/// 位置の乱れを移動量に入れない）。1つのsampleが出すeventは高々1つである。
#[derive(Debug, Clone, Copy)]
pub struct TouchTracker {
    config: TouchConfig,
    state: State,
}

impl TouchTracker {
    /// 触れていない状態で始める。
    #[must_use]
    pub const fn new(config: TouchConfig) -> Self {
        Self {
            config,
            state: State::Idle { pressed_run: 0 },
        }
    }

    /// 接触中か。
    #[must_use]
    pub const fn is_touching(&self) -> bool {
        matches!(self.state, State::Touching { .. })
    }

    /// 触れていない状態へ戻す（接触の途中で入力が途切れたときなど）。eventは出さない。
    pub fn reset(&mut self) {
        self.state = State::Idle { pressed_run: 0 };
    }

    /// sampleを1つ入れる。
    pub fn update(&mut self, sample: Sample) -> Option<TouchEvent> {
        let config = self.config;
        match self.state {
            State::Idle { pressed_run } => {
                if sample.pressure < config.press_threshold {
                    self.state = State::Idle { pressed_run: 0 };
                    return None;
                }
                let run = pressed_run.saturating_add(1);
                if run < config.press_samples {
                    self.state = State::Idle { pressed_run: run };
                    return None;
                }
                self.state = State::Touching {
                    below_run: 0,
                    last_x: sample.x,
                    last_y: sample.y,
                    samples: 1,
                    travel: 0,
                    stroked: false,
                };
                Some(TouchEvent::Pressed {
                    x: sample.x,
                    y: sample.y,
                })
            }
            State::Touching {
                below_run,
                last_x,
                last_y,
                samples,
                travel,
                stroked,
            } => {
                if sample.pressure < config.release_threshold {
                    let run = below_run.saturating_add(1);
                    if run >= config.release_samples {
                        self.state = State::Idle { pressed_run: 0 };
                        return Some(TouchEvent::Released {
                            samples,
                            travel,
                            stroked,
                        });
                    }
                    self.state = State::Touching {
                        below_run: run,
                        last_x,
                        last_y,
                        samples,
                        travel,
                        stroked,
                    };
                    return None;
                }
                let step =
                    u32::from(last_x.abs_diff(sample.x)) + u32::from(last_y.abs_diff(sample.y));
                let travel = travel.saturating_add(step);
                let samples = samples.saturating_add(1);
                let fire = !stroked && travel >= config.stroke_travel;
                self.state = State::Touching {
                    below_run: 0,
                    last_x: sample.x,
                    last_y: sample.y,
                    samples,
                    travel,
                    stroked: stroked || fire,
                };
                fire.then_some(TouchEvent::Stroke { travel })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 値は境界の挙動を見るための入力であり、推奨値ではない（モジュールdoc）。
    fn config() -> TouchConfig {
        TouchConfig::new(100, 60, 3, 2, 50).unwrap()
    }

    fn s(pressure: u16, x: u16, y: u16) -> Sample {
        Sample { pressure, x, y }
    }

    fn run(t: &mut TouchTracker, samples: &[Sample]) -> Vec<Option<TouchEvent>> {
        samples.iter().map(|&x| t.update(x)).collect()
    }

    #[test]
    fn control_bytes_follow_table_5_and_6() {
        // 0b1_001_0_0_00 など。表5のチャネル、表6のbit配置から組み立てた値。
        assert_eq!(control_byte(Channel::Y), 0x90);
        assert_eq!(control_byte(Channel::Z1), 0xB0);
        assert_eq!(control_byte(Channel::Z2), 0xC0);
        assert_eq!(control_byte(Channel::X), 0xD0);
        assert_eq!(frame(Channel::X), [0xD0, 0x00, 0x00]);
    }

    #[test]
    fn control_byte_keeps_start_bit_and_zeroes_mode_ser_pd() {
        for ch in [Channel::Y, Channel::Z1, Channel::Z2, Channel::X] {
            let b = control_byte(ch);
            assert_eq!(b & 0x80, 0x80, "S bit");
            assert_eq!(b & 0x0F, 0, "MODE, SER/DFR, PD1, PD0 are all 0");
        }
    }

    #[test]
    fn decode_takes_bits_14_to_3_of_bytes_1_and_2() {
        assert_eq!(decode_12bit([0, 0x00, 0x00]), 0);
        assert_eq!(decode_12bit([0, 0x7F, 0xF8]), 0x0FFF);
        assert_eq!(decode_12bit([0, 0x40, 0x00]), 0x0800);
        assert_eq!(decode_12bit([0, 0x00, 0x08]), 0x0001);
    }

    #[test]
    fn decode_ignores_first_byte_leading_bit_and_trailing_zero_bits() {
        // 1 byte目は読まない。
        assert_eq!(decode_12bit([0xFF, 0x00, 0x00]), 0);
        // 2 byte目の最上位bit（変換が始まる前の区間）と、3 byte目の下位3 bitは値に入らない。
        assert_eq!(decode_12bit([0, 0x80, 0x00]), 0);
        assert_eq!(decode_12bit([0, 0x00, 0x07]), 0);
        assert_eq!(decode_12bit([0, 0xFF, 0xFF]), 0x0FFF);
    }

    #[test]
    fn config_rejects_inverted_hysteresis() {
        assert_eq!(
            TouchConfig::new(50, 51, 1, 1, 1),
            Err(ConfigError::ReleaseAbovePress)
        );
        assert!(TouchConfig::new(50, 50, 1, 1, 1).is_ok());
    }

    #[test]
    fn config_rejects_zero_counts_and_travel() {
        assert_eq!(
            TouchConfig::new(50, 40, 0, 1, 1),
            Err(ConfigError::ZeroSamples)
        );
        assert_eq!(
            TouchConfig::new(50, 40, 1, 0, 1),
            Err(ConfigError::ZeroSamples)
        );
        assert_eq!(
            TouchConfig::new(50, 40, 1, 1, 0),
            Err(ConfigError::ZeroStrokeTravel)
        );
    }

    #[test]
    fn press_needs_consecutive_samples_at_or_above_threshold() {
        let mut t = TouchTracker::new(config());
        // しきい値ちょうど（100）は「以上」で数える。
        let out = run(&mut t, &[s(100, 1, 2), s(100, 1, 2)]);
        assert_eq!(out, [None, None]);
        assert!(!t.is_touching());
        assert_eq!(
            t.update(s(100, 7, 8)),
            Some(TouchEvent::Pressed { x: 7, y: 8 })
        );
        assert!(t.is_touching());
    }

    #[test]
    fn press_run_resets_on_a_sample_below_threshold() {
        let mut t = TouchTracker::new(config());
        // 99は未満。2回連続の後に途切れると最初から数え直す。
        let out = run(
            &mut t,
            &[
                s(100, 0, 0),
                s(100, 0, 0),
                s(99, 0, 0),
                s(100, 0, 0),
                s(100, 0, 0),
            ],
        );
        assert_eq!(out, [None; 5]);
        assert_eq!(
            t.update(s(100, 3, 4)),
            Some(TouchEvent::Pressed { x: 3, y: 4 })
        );
    }

    #[test]
    fn single_sample_spike_does_not_press() {
        let mut t = TouchTracker::new(config());
        let out = run(&mut t, &[s(0, 0, 0), s(4095, 9, 9), s(0, 0, 0), s(0, 0, 0)]);
        assert_eq!(out, [None; 4]);
        assert!(!t.is_touching());
    }

    #[test]
    fn press_samples_of_one_presses_immediately() {
        let cfg = TouchConfig::new(100, 60, 1, 1, 50).unwrap();
        let mut t = TouchTracker::new(cfg);
        assert_eq!(
            t.update(s(100, 5, 6)),
            Some(TouchEvent::Pressed { x: 5, y: 6 })
        );
    }

    fn pressed(t: &mut TouchTracker) {
        for _ in 0..2 {
            assert_eq!(t.update(s(100, 10, 10)), None);
        }
        assert_eq!(
            t.update(s(100, 10, 10)),
            Some(TouchEvent::Pressed { x: 10, y: 10 })
        );
    }

    #[test]
    fn release_needs_consecutive_samples_below_release_threshold() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        // 60はrelease_threshold以上なので離れていない。59は未満。
        assert_eq!(t.update(s(60, 10, 10)), None);
        assert_eq!(t.update(s(59, 10, 10)), None);
        assert!(t.is_touching());
        assert_eq!(
            t.update(s(59, 10, 10)),
            Some(TouchEvent::Released {
                samples: 2,
                travel: 0,
                stroked: false
            })
        );
        assert!(!t.is_touching());
    }

    #[test]
    fn a_single_dropout_does_not_release_and_run_resets() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        assert_eq!(t.update(s(0, 10, 10)), None);
        // 復帰すると数え直す。次の1回の落ち込みでは離れない。
        assert_eq!(t.update(s(100, 10, 10)), None);
        assert_eq!(t.update(s(0, 10, 10)), None);
        assert!(t.is_touching());
    }

    #[test]
    fn hysteresis_band_keeps_touching_but_does_not_start_one() {
        let mut t = TouchTracker::new(config());
        // 80は帯の中（60以上100未満）。触れていない間は触れたと数えない。
        let out = run(
            &mut t,
            &[s(80, 0, 0), s(80, 0, 0), s(80, 0, 0), s(80, 0, 0)],
        );
        assert_eq!(out, [None; 4]);
        assert!(!t.is_touching());
        // 触れた後は帯の中でも接触が続く。
        pressed(&mut t);
        let out = run(&mut t, &[s(80, 10, 10), s(80, 10, 10), s(80, 10, 10)]);
        assert_eq!(out, [None; 3]);
        assert!(t.is_touching());
    }

    #[test]
    fn stroke_fires_once_when_travel_reaches_threshold() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t); // (10, 10)
        // 49: まだ。|dx|+|dy|の累計を数える。
        assert_eq!(t.update(s(100, 40, 10)), None); // 30
        assert_eq!(t.update(s(100, 40, 29)), None); // 30 + 19 = 49
        // ちょうど50で撫で。
        assert_eq!(
            t.update(s(100, 41, 29)),
            Some(TouchEvent::Stroke { travel: 50 })
        );
        // 続けて動いても、1回の接触では2回目を出さない。
        assert_eq!(t.update(s(100, 200, 200)), None);
        assert_eq!(
            run(&mut t, &[s(0, 0, 0), s(0, 0, 0)])[1],
            Some(TouchEvent::Released {
                samples: 5,
                travel: 50 + 159 + 171,
                stroked: true
            })
        );
    }

    #[test]
    fn stroke_counts_both_directions() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        // 行って戻る往復も移動量になる（撫での往復）。
        assert_eq!(t.update(s(100, 40, 10)), None);
        assert_eq!(
            t.update(s(100, 10, 10)),
            Some(TouchEvent::Stroke { travel: 60 })
        );
    }

    #[test]
    fn stationary_touch_never_strokes_and_reports_zero_travel() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        for _ in 0..100 {
            assert_eq!(t.update(s(100, 10, 10)), None);
        }
        let out = run(&mut t, &[s(0, 0, 0), s(0, 0, 0)]);
        assert_eq!(
            out[1],
            Some(TouchEvent::Released {
                samples: 101,
                travel: 0,
                stroked: false
            })
        );
    }

    #[test]
    fn weak_samples_before_release_do_not_move_position_or_travel() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        // 離れる途中の弱いsample（pressure 0）は、位置が乱れていても移動量に入れない。
        assert_eq!(t.update(s(0, 4000, 4000)), None);
        // 復帰したsampleは、最後に信用した位置（10, 10）からの差で数える。
        assert_eq!(t.update(s(100, 12, 10)), None);
        let out = run(&mut t, &[s(0, 0, 0), s(0, 0, 0)]);
        assert_eq!(
            out[1],
            Some(TouchEvent::Released {
                samples: 2,
                travel: 2,
                stroked: false
            })
        );
    }

    #[test]
    fn a_new_touch_after_release_starts_from_zero() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        assert_eq!(
            t.update(s(100, 100, 10)),
            Some(TouchEvent::Stroke { travel: 90 })
        );
        run(&mut t, &[s(0, 0, 0), s(0, 0, 0)]);
        assert!(!t.is_touching());
        // 前の接触の距離・strokedを引き継がない。
        let out = run(&mut t, &[s(100, 5, 5), s(100, 5, 5)]);
        assert_eq!(out, [None, None]);
        assert_eq!(
            t.update(s(100, 5, 5)),
            Some(TouchEvent::Pressed { x: 5, y: 5 })
        );
        assert_eq!(
            t.update(s(100, 5, 60)),
            Some(TouchEvent::Stroke { travel: 55 })
        );
    }

    #[test]
    fn reset_returns_to_idle_without_an_event() {
        let mut t = TouchTracker::new(config());
        pressed(&mut t);
        t.reset();
        assert!(!t.is_touching());
        assert_eq!(t.update(s(100, 1, 1)), None);
    }

    #[test]
    fn extreme_values_do_not_overflow() {
        let cfg = TouchConfig::new(0, 0, 1, 1, u32::MAX).unwrap();
        let mut t = TouchTracker::new(cfg);
        assert!(matches!(
            t.update(s(0, 0, 0)),
            Some(TouchEvent::Pressed { .. })
        ));
        for i in 0..10u16 {
            let (x, y) = if i % 2 == 0 {
                (u16::MAX, u16::MAX)
            } else {
                (0, 0)
            };
            assert_eq!(t.update(s(u16::MAX, x, y)), None);
        }
        // pressureが常にrelease_threshold(0)以上なので離れない。
        assert!(t.is_touching());
    }
}
