//! 首の動きの意図（[`MotionIntent`]）を、wireの`play_motion`（Protocol §5.3）へ変える。
//!
//! # 値は未決である（`PROTO-TBD-008`）
//!
//! `speed`、`repeat`、および`speed`と`repeat`の上限は、サーボの校正（#18）の後に決まる。
//! **記憶や推測で置かない。**値は[`MOTION_CONFIG`]の1か所だけに置き、決まるまで`None`とする。
//! motion名は決めてある（[`SWAY_NAME`]）。名前はサーボの安全に効かない識別子であり、protocol §5.3の
//! 例とfixtureが使う名前に合わせた（#766）。**名前があることは、送ってよいことを意味しない。**
//! firmwareのcatalogが受理する名前でもまだ無い（catalogは空である）。
//! **`None`の間は`play_motion`を送らない**（`Daemon`が数えるだけである）。これは`mind.rs`の
//! 申し送り（PROTO-TBD-008が決まるまで送らない。Issue #491の段階2）の意味を変えない。
//!
//! # 範囲
//!
//! この変換は、firmwareの検査の**代わりではない**。受理する名前、上限、速度・角度・加速度、
//! 単位時間あたりの受理数は、ESP32が強制する（Protocol §5.3）。ここで見るのは、Pi側が
//! 意味の決まっていない値を送らないための前段だけである。
//!
//! 値を入れる前に、`Daemon`が`play_motion`のACKを扱う経路（拒否の分類、`stale_session`での再開）を
//! 足す。**今は`play_motion`を追跡しないため、ACKは相関せず、再送もしない。**

use deskcat_domain::MotionIntent;
use deskcat_protocol::{FiniteF32, PlayMotion};

/// [`MotionIntent::Sway`]に対応するmotion名。サーボの安全に効かない識別子である。
pub const SWAY_NAME: &str = "sway";

/// 1つの動きの内容（名前を除く）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSpec {
    /// 正規化されたspeed。
    pub speed: f32,
    /// 繰り返す回数。
    pub repeat: u32,
}

/// 変換に要る値の組。**組で`Some`か`None`のどちらかにする。**一部だけ決まった状態を作らない。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionConfig {
    /// [`MotionIntent::Sway`]の内容。
    pub sway: MotionSpec,
    /// `speed`の上限（この値を含む）。`PROTO-TBD-008`で決める値であり、firmwareの上限とは別に、
    /// Piが超える値を作らないための上限である。
    pub speed_max: f32,
    /// `repeat`の上限（この値を含む）。
    pub repeat_max: u32,
}

/// 変換に使う値。**`PROTO-TBD-008`が決まるまで`None`である。**
///
/// 決まったら、根拠（校正の記録とsafety policyの水準）とともに`Some`へ置き換える。
pub const MOTION_CONFIG: Option<MotionConfig> = None;

/// 変換できなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionError {
    /// `speed`が有限でない。
    NonFiniteSpeed,
    /// `speed`が正でない（意味が決まっていない値は送らない）。
    SpeedNotPositive,
    /// `speed`が上限を超える、または上限が比較できない（`NaN`）。
    SpeedOverLimit,
    /// `repeat`が0。
    RepeatZero,
    /// `repeat`が上限を超える。
    RepeatOverLimit,
}

impl core::fmt::Display for MotionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFiniteSpeed => "speedが有限でない",
            Self::SpeedNotPositive => "speedが正でない",
            Self::SpeedOverLimit => "speedが上限を超える",
            Self::RepeatZero => "repeatが0である",
            Self::RepeatOverLimit => "repeatが上限を超える",
        })
    }
}

/// 意図を`play_motion`のpayloadにする。値は`config`が持つ。
///
/// # Errors
///
/// `speed`が有限でない・正でない・上限を超える、`repeat`が0・上限を超えるとき。
pub fn play_motion(intent: MotionIntent, config: &MotionConfig) -> Result<PlayMotion, MotionError> {
    let (name, spec) = match intent {
        MotionIntent::Sway => (SWAY_NAME, &config.sway),
    };
    let speed = FiniteF32::new(spec.speed).ok_or(MotionError::NonFiniteSpeed)?;
    if spec.speed <= 0.0 {
        return Err(MotionError::SpeedNotPositive);
    }
    // 上限が`NaN`のときも通さない（`<=`の否定で比べる）。
    if !matches!(
        spec.speed.partial_cmp(&config.speed_max),
        Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
    ) {
        return Err(MotionError::SpeedOverLimit);
    }
    if spec.repeat == 0 {
        return Err(MotionError::RepeatZero);
    }
    if spec.repeat > config.repeat_max {
        return Err(MotionError::RepeatOverLimit);
    }
    Ok(PlayMotion {
        name: name.to_owned(),
        speed,
        repeat: spec.repeat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **test用の仮の値である。校正の値ではなく、`MOTION_CONFIG`へ写さない。**
    const TEST_CONFIG: MotionConfig = MotionConfig {
        sway: MotionSpec {
            speed: 0.5,
            repeat: 2,
        },
        speed_max: 0.5,
        repeat_max: 2,
    };

    #[test]
    fn the_shipped_config_is_still_undecided() {
        // 値が入ったら、この testと、`Daemon`の「送らない」testを、根拠とともに直す。
        assert_eq!(MOTION_CONFIG, None);
    }

    #[test]
    fn builds_the_wire_payload_with_the_given_values() {
        let pm = play_motion(MotionIntent::Sway, &TEST_CONFIG).expect("作れる");
        assert_eq!(pm.name, "sway");
        assert_eq!(pm.speed.get().to_bits(), 0.5_f32.to_bits());
        assert_eq!(pm.repeat, 2);
    }

    #[test]
    fn the_limits_are_inclusive() {
        // speed==speed_max、repeat==repeat_maxは通る（上のtest）。1つ超えると落ちる。
        let mut c = TEST_CONFIG;
        c.sway.repeat = 3;
        assert_eq!(
            play_motion(MotionIntent::Sway, &c),
            Err(MotionError::RepeatOverLimit)
        );
        let mut c = TEST_CONFIG;
        c.sway.speed = 0.51;
        assert_eq!(
            play_motion(MotionIntent::Sway, &c),
            Err(MotionError::SpeedOverLimit)
        );
    }

    #[test]
    fn rejects_values_that_have_no_decided_meaning() {
        let cases = [
            (
                MotionSpec {
                    speed: f32::NAN,
                    ..TEST_CONFIG.sway
                },
                MotionError::NonFiniteSpeed,
            ),
            (
                MotionSpec {
                    speed: f32::INFINITY,
                    ..TEST_CONFIG.sway
                },
                MotionError::NonFiniteSpeed,
            ),
            (
                MotionSpec {
                    speed: 0.0,
                    ..TEST_CONFIG.sway
                },
                MotionError::SpeedNotPositive,
            ),
            (
                MotionSpec {
                    speed: -0.1,
                    ..TEST_CONFIG.sway
                },
                MotionError::SpeedNotPositive,
            ),
            (
                MotionSpec {
                    repeat: 0,
                    ..TEST_CONFIG.sway
                },
                MotionError::RepeatZero,
            ),
        ];
        for (sway, expected) in cases {
            let c = MotionConfig {
                sway,
                ..TEST_CONFIG
            };
            assert_eq!(play_motion(MotionIntent::Sway, &c), Err(expected));
        }
    }

    #[test]
    fn a_nan_speed_limit_lets_nothing_through() {
        let c = MotionConfig {
            speed_max: f32::NAN,
            ..TEST_CONFIG
        };
        assert_eq!(
            play_motion(MotionIntent::Sway, &c),
            Err(MotionError::SpeedOverLimit)
        );
    }
}
