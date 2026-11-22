//! 引数の解析。
//!
//! **必須の値は、すべて既定値を持たない。**device名は未確認であり（確定はIssue #11の後半）、baudの正本は
//! `PROTO-TBD-001`、duplicate履歴の正本は`PROTO-TBD-005`で、いずれも未確定である。起動の期限に対応するTBDの
//! 登録は無く、値の根拠も無い（`ESP32`の起動時間は未測定）。
//! **確認していない値を既定として固定しない。**足りなければ起動せず、終了1にする（[`CliError`]）。
//! 値の暫定の置き場所は、`deploy/systemd/deskcatd.service`の1か所だけである。
//!
//! 同じ引数を重ねたときは、後の値で上書きする（`deskcat-serial`の`serial_link`と同じ。検出しない）。
//! baudの0は、ここでは検査しない。`SerialConfig::new`が弾く（判断は`deskcat-serial`に置く）。

use std::fmt;
use std::num::NonZeroU64;
use std::time::Duration;

use deskcat_domain::{Config as CompanionConfig, Expression, LocalMinutes};
use deskcat_serial::DuplicatePolicy;

use crate::daemon::ExpressionProbe;
use crate::expression_sync::from_status_name;

/// 使い方。
pub const USAGE: &str = "usage: deskcatd --port <path> --baud <rate> \
--duplicate-capacity <n> --duplicate-retention-ms <ms> --startup-deadline-ms <ms> \
[--speech-min-interval-ms <ms> --speech-duration-ms <ms>] [--cloud-speech] [--no-companion] \
[--probe-expression <name> --probe-interval-ms <ms>] [--verbose]\n\
[Companionの試験用の上書き: --companion-rest-interval-ms <ms> --companion-sleep-at <HH:MM> \
--companion-wake-at <HH:MM> --companion-hot-on-c <v> --companion-hot-off-c <v> \
--companion-cold-on-c <v> --companion-cold-off-c <v> --companion-dry-on-pct <v> \
--companion-dry-off-pct <v> --companion-pressure-drop-on-hpa <v> \
--companion-pressure-drop-off-hpa <v>]\n\
\n\
--port、--baud、--duplicate-capacity、--duplicate-retention-ms、--startup-deadline-ms は必須である。\
既定値を持たせない。\n\
device名は未確認であり、baudの正本は PROTO-TBD-001、duplicate履歴の正本は PROTO-TBD-005 である。\
起動の期限は、値の根拠が無い（ESP32の起動時間は未測定）。\n\
\n\
--probe-expression と --probe-interval-ms は実機確認用で、両方と --no-companion が要る。\
session確立後、指定した表情と neutral の set_expression を、指定の間隔で交互に送る（play_motion は送らない）。\
neutral は指定できない。運用では付けない。\n\
\n\
--companion-* は実機確認用で、指定した項目だけ Config::MVP を上書きする。指定が無ければ Config::MVP と同じである。\
運用では付けない。--no-companion とは併用できない。\
入る側と戻る側のしきい値の関係（hot/pressure は off < on、cold/dry は off > on）、\
間隔が 0 でないこと、時刻が HH:MM（00:00..=23:59）であること、寝る時刻と起きる時刻が異なることを検査する。";

/// 解析した引数。
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    /// Serial deviceのpath。
    pub port: String,
    /// Baud rate。0は`SerialConfig::new`が弾く。
    pub baud: u32,
    /// `boot`のduplicate履歴の方針。
    pub duplicate_policy: DuplicatePolicy,
    /// 起動から、ESP32のsessionの確立と`hello`の結果がそろうまでの期限（ms）。0は受け付けない。
    pub startup_deadline_ms: NonZeroU64,
    /// 両方の引数がある場合だけ独り言を出す。運用値の既定は無い。
    pub speech: Option<crate::runner::SpeechConfig>,
    /// 明示した場合だけクラウド経路を試す。credentialが無ければローカルだけ。
    pub cloud_speech: bool,
    /// `Companion`をつながない。Piの時刻（寝る時間帯）に依存しない試験のためで、運用では付けない。
    pub no_companion: bool,
    /// `Companion`の値。上書きが無ければ`Config::MVP`と同じ。`--no-companion`のときは使わない。
    pub companion: CompanionConfig,
    /// 実機確認用の表情の送信（Issue #514）。両方の引数がある場合だけ。`--no-companion`が要る。
    pub probe: Option<ExpressionProbe>,
    /// `Debug`まで出す。
    pub verbose: bool,
}

/// 解析の結果。
///
/// `--help`は**成功である。**`Err`へ畳むと終了コードが1になり、scriptから「使い方を聞いた」と
/// 「引数を間違えた」を区別できない。
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    /// 実行する。
    Run(Box<Args>),
    /// 使い方を出して正常終了する。
    Help,
}

/// 引数の誤り。いずれも終了1である。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CliError {
    /// 必須の引数が無い。
    Missing(&'static str),
    /// 引数に値が無い。
    NoValue(String),
    /// 値を解釈できない、または受け付けない値である。
    Invalid {
        /// 引数名。
        flag: &'static str,
        /// 理由。
        reason: String,
    },
    /// 知らない引数である。
    Unknown(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(flag) => write!(f, "{flag}が必要である"),
            Self::NoValue(flag) => write!(f, "{flag}に値が無い"),
            Self::Invalid { flag, reason } => write!(f, "{flag}が不正である: {reason}"),
            Self::Unknown(flag) => write!(f, "不明な引数: {flag}"),
        }
    }
}

impl std::error::Error for CliError {}

fn number<T: std::str::FromStr>(flag: &'static str, raw: &str) -> Result<T, CliError>
where
    T::Err: fmt::Display,
{
    raw.parse::<T>().map_err(|e| CliError::Invalid {
        flag,
        reason: format!("数値でない: {e}"),
    })
}

/// `Companion`の試験用の上書き（`--companion-*`）。指定した項目だけが`Some`である。
#[derive(Debug, Default)]
struct CompanionOverrides {
    rest_interval_ms: Option<u64>,
    sleep_at: Option<LocalMinutes>,
    wake_at: Option<LocalMinutes>,
    hot_on_c: Option<f32>,
    hot_off_c: Option<f32>,
    cold_on_c: Option<f32>,
    cold_off_c: Option<f32>,
    dry_on_pct: Option<f32>,
    dry_off_pct: Option<f32>,
    pressure_drop_on_hpa: Option<f32>,
    pressure_drop_off_hpa: Option<f32>,
}

impl CompanionOverrides {
    fn any(&self) -> bool {
        self.rest_interval_ms.is_some()
            || self.sleep_at.is_some()
            || self.wake_at.is_some()
            || self.hot_on_c.is_some()
            || self.hot_off_c.is_some()
            || self.cold_on_c.is_some()
            || self.cold_off_c.is_some()
            || self.dry_on_pct.is_some()
            || self.dry_off_pct.is_some()
            || self.pressure_drop_on_hpa.is_some()
            || self.pressure_drop_off_hpa.is_some()
    }
}

impl CompanionOverrides {
    /// `--companion-*`の1つを読む。知らない引数なら`Ok(false)`。値は読めるときだけ使う。
    fn set(&mut self, flag: &str, value: Result<String, CliError>) -> Result<bool, CliError> {
        let name: &'static str = match flag {
            "--companion-rest-interval-ms" => "--companion-rest-interval-ms",
            "--companion-sleep-at" => "--companion-sleep-at",
            "--companion-wake-at" => "--companion-wake-at",
            "--companion-hot-on-c" => "--companion-hot-on-c",
            "--companion-hot-off-c" => "--companion-hot-off-c",
            "--companion-cold-on-c" => "--companion-cold-on-c",
            "--companion-cold-off-c" => "--companion-cold-off-c",
            "--companion-dry-on-pct" => "--companion-dry-on-pct",
            "--companion-dry-off-pct" => "--companion-dry-off-pct",
            "--companion-pressure-drop-on-hpa" => "--companion-pressure-drop-on-hpa",
            "--companion-pressure-drop-off-hpa" => "--companion-pressure-drop-off-hpa",
            _ => return Ok(false),
        };
        let raw = value?;
        match name {
            "--companion-rest-interval-ms" => self.rest_interval_ms = Some(number(name, &raw)?),
            "--companion-sleep-at" => self.sleep_at = Some(clock(name, &raw)?),
            "--companion-wake-at" => self.wake_at = Some(clock(name, &raw)?),
            "--companion-hot-on-c" => self.hot_on_c = Some(finite(name, &raw)?),
            "--companion-hot-off-c" => self.hot_off_c = Some(finite(name, &raw)?),
            "--companion-cold-on-c" => self.cold_on_c = Some(finite(name, &raw)?),
            "--companion-cold-off-c" => self.cold_off_c = Some(finite(name, &raw)?),
            "--companion-dry-on-pct" => self.dry_on_pct = Some(finite(name, &raw)?),
            "--companion-dry-off-pct" => self.dry_off_pct = Some(finite(name, &raw)?),
            "--companion-pressure-drop-on-hpa" => {
                self.pressure_drop_on_hpa = Some(finite(name, &raw)?);
            }
            _ => self.pressure_drop_off_hpa = Some(finite(name, &raw)?),
        }
        Ok(true)
    }
}

/// 有限の小数を読む。`NaN`と無限大は、比較が常に偽になるため受け付けない。
fn finite(flag: &'static str, raw: &str) -> Result<f32, CliError> {
    let v = number::<f32>(flag, raw)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(CliError::Invalid {
            flag,
            reason: "有限の数でない".to_owned(),
        })
    }
}

/// `HH:MM`を読む。範囲外は拒否する。
fn clock(flag: &'static str, raw: &str) -> Result<LocalMinutes, CliError> {
    let invalid = |reason: &str| CliError::Invalid {
        flag,
        reason: format!("{reason}: {raw}"),
    };
    let (h, m) = raw.split_once(':').ok_or_else(|| invalid("HH:MMでない"))?;
    let h = h.parse::<u8>().map_err(|_| invalid("時が数でない"))?;
    let m = m.parse::<u8>().map_err(|_| invalid("分が数でない"))?;
    LocalMinutes::new(h, m).ok_or_else(|| invalid("範囲外（00:00..=23:59）"))
}

/// 上書きを`Config::MVP`へ適用し、値の関係を検査する。
fn build_companion(o: &CompanionOverrides) -> Result<CompanionConfig, CliError> {
    let base = CompanionConfig::MVP;
    let config = CompanionConfig {
        hot_on_c: o.hot_on_c.unwrap_or(base.hot_on_c),
        hot_off_c: o.hot_off_c.unwrap_or(base.hot_off_c),
        cold_on_c: o.cold_on_c.unwrap_or(base.cold_on_c),
        cold_off_c: o.cold_off_c.unwrap_or(base.cold_off_c),
        dry_on_pct: o.dry_on_pct.unwrap_or(base.dry_on_pct),
        dry_off_pct: o.dry_off_pct.unwrap_or(base.dry_off_pct),
        pressure_drop_on_hpa: o.pressure_drop_on_hpa.unwrap_or(base.pressure_drop_on_hpa),
        pressure_drop_off_hpa: o
            .pressure_drop_off_hpa
            .unwrap_or(base.pressure_drop_off_hpa),
        rest_interval_ms: o.rest_interval_ms.unwrap_or(base.rest_interval_ms),
        sleep_at: o.sleep_at.unwrap_or(base.sleep_at),
        wake_at: o.wake_at.unwrap_or(base.wake_at),
        ..base
    };
    let bad = |flag: &'static str, reason: &str| CliError::Invalid {
        flag,
        reason: reason.to_owned(),
    };
    if config.rest_interval_ms == 0 {
        return Err(bad("--companion-rest-interval-ms", "0は間隔にならない"));
    }
    if config.hot_off_c >= config.hot_on_c {
        return Err(bad(
            "--companion-hot-off-c",
            "戻る側は入る側（hot-on-c）より低くする",
        ));
    }
    if config.cold_off_c <= config.cold_on_c {
        return Err(bad(
            "--companion-cold-off-c",
            "戻る側は入る側（cold-on-c）より高くする",
        ));
    }
    if config.dry_off_pct <= config.dry_on_pct {
        return Err(bad(
            "--companion-dry-off-pct",
            "戻る側は入る側（dry-on-pct）より高くする",
        ));
    }
    if config.pressure_drop_off_hpa >= config.pressure_drop_on_hpa {
        return Err(bad(
            "--companion-pressure-drop-off-hpa",
            "戻る側は入る側（pressure-drop-on-hpa）より小さくする",
        ));
    }
    if config.sleep_at == config.wake_at {
        return Err(bad(
            "--companion-wake-at",
            "寝る時刻と起きる時刻が同じだと、夜の時間帯が無くなる",
        ));
    }
    Ok(config)
}

/// 実機確認用の表情の送信（`--probe-expression`と`--probe-interval-ms`）を解釈する。
fn parse_probe(
    name: Option<String>,
    interval: Option<u64>,
    no_companion: bool,
) -> Result<Option<ExpressionProbe>, CliError> {
    match (name, interval) {
        (None, None) => Ok(None),
        (Some(name), Some(interval)) => {
            let expression = from_status_name(&name).ok_or_else(|| CliError::Invalid {
                flag: "--probe-expression",
                reason: format!("知らない表情名: {name}"),
            })?;
            if expression == Expression::Neutral {
                return Err(CliError::Invalid {
                    flag: "--probe-expression",
                    reason: "neutralは交互の相手と同じで、何も確かめられない".to_owned(),
                });
            }
            let interval_ms = NonZeroU64::new(interval).ok_or(CliError::Invalid {
                flag: "--probe-interval-ms",
                reason: "0は間隔にならない".to_owned(),
            })?;
            // Companionも欲しい表情を書き換える（寝る時間帯は専有する）。干渉させず、併用を拒否する。
            if !no_companion {
                return Err(CliError::Invalid {
                    flag: "--probe-expression",
                    reason: "--no-companionが要る（Companionと併用しない）".to_owned(),
                });
            }
            Ok(Some(ExpressionProbe {
                expression,
                interval_ms,
            }))
        }
        (None, Some(_)) => Err(CliError::Missing("--probe-expression")),
        (Some(_), None) => Err(CliError::Missing("--probe-interval-ms")),
    }
}

/// 独り言の引数（`--speech-*`と`--cloud-speech`）を解釈する。
fn parse_speech(
    min_interval_ms: Option<u64>,
    duration_ms: Option<u32>,
    cloud_speech: bool,
) -> Result<Option<crate::runner::SpeechConfig>, CliError> {
    let speech = match (min_interval_ms, duration_ms) {
        (None, None) => None,
        (Some(interval), Some(duration)) => {
            let min_interval_ms = NonZeroU64::new(interval).ok_or(CliError::Invalid {
                flag: "--speech-min-interval-ms",
                reason: "0は最短間隔にならない".to_owned(),
            })?;
            Some(crate::runner::SpeechConfig {
                min_interval_ms,
                duration_ms: duration,
            })
        }
        (None, Some(_)) => return Err(CliError::Missing("--speech-min-interval-ms")),
        (Some(_), None) => return Err(CliError::Missing("--speech-duration-ms")),
    };
    if cloud_speech && speech.is_none() {
        return Err(CliError::Missing(
            "--speech-min-interval-ms/--speech-duration-ms",
        ));
    }
    if cloud_speech
        && let Some(config) = speech
        && (config.min_interval_ms.get() < crate::cloud_speech::MIN_INTERVAL_SECS * 1000
            || config.duration_ms == 0
            || config.duration_ms > deskcat_face::text::MAX_TEXT_DURATION_MS)
    {
        return Err(CliError::Invalid {
            flag: "--cloud-speech",
            reason: format!(
                "間隔は{}ms以上、表示時間は1..={}msが必要",
                crate::cloud_speech::MIN_INTERVAL_SECS * 1000,
                deskcat_face::text::MAX_TEXT_DURATION_MS
            ),
        });
    }
    Ok(speech)
}

/// 引数（program名を除く）を解析する。
///
/// # Errors
///
/// 必須の引数が無い、値が無い、値を解釈できない、または知らない引数のとき。
pub fn parse<I>(raw: I) -> Result<Parsed, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut port = None;
    let mut baud = None;
    let mut duplicate_capacity = None;
    let mut duplicate_retention_ms = None;
    let mut startup_deadline_ms = None;
    let mut speech_min_interval_ms = None;
    let mut speech_duration_ms = None;
    let mut probe_expression = None;
    let mut probe_interval_ms = None;
    let mut companion = CompanionOverrides::default();
    let (mut verbose, mut cloud_speech, mut no_companion) = (false, false, false);
    let mut argv = raw.into_iter();

    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or_else(|| CliError::NoValue(flag.clone()));
        match flag.as_str() {
            "--port" => port = Some(value()?),
            "--baud" => baud = Some(number::<u32>("--baud", &value()?)?),
            "--duplicate-capacity" => {
                duplicate_capacity = Some(number::<usize>("--duplicate-capacity", &value()?)?);
            }
            "--duplicate-retention-ms" => {
                duplicate_retention_ms =
                    Some(number::<u64>("--duplicate-retention-ms", &value()?)?);
            }
            "--startup-deadline-ms" => {
                startup_deadline_ms = Some(number::<u64>("--startup-deadline-ms", &value()?)?);
            }
            "--speech-min-interval-ms" => {
                speech_min_interval_ms =
                    Some(number::<u64>("--speech-min-interval-ms", &value()?)?);
            }
            "--speech-duration-ms" => {
                speech_duration_ms = Some(number::<u32>("--speech-duration-ms", &value()?)?);
            }
            "--probe-expression" => probe_expression = Some(value()?),
            "--probe-interval-ms" => {
                probe_interval_ms = Some(number::<u64>("--probe-interval-ms", &value()?)?);
            }
            "--verbose" => verbose = true,
            "--cloud-speech" => cloud_speech = true,
            "--no-companion" => no_companion = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => {
                let target = value();
                if !companion.set(other, target)? {
                    return Err(CliError::Unknown(other.to_owned()));
                }
            }
        }
    }

    let duplicate_policy = DuplicatePolicy::new(
        duplicate_capacity.ok_or(CliError::Missing("--duplicate-capacity"))?,
        Duration::from_millis(
            duplicate_retention_ms.ok_or(CliError::Missing("--duplicate-retention-ms"))?,
        ),
    )
    .map_err(|e| CliError::Invalid {
        flag: "--duplicate-capacity/--duplicate-retention-ms",
        reason: e.to_string(),
    })?;
    let startup_deadline_ms =
        startup_deadline_ms.ok_or(CliError::Missing("--startup-deadline-ms"))?;
    let startup_deadline_ms = NonZeroU64::new(startup_deadline_ms).ok_or(CliError::Invalid {
        flag: "--startup-deadline-ms",
        reason: "0は起動直後に必ず超過する".to_owned(),
    })?;
    let speech = parse_speech(speech_min_interval_ms, speech_duration_ms, cloud_speech)?;
    let probe = parse_probe(probe_expression, probe_interval_ms, no_companion)?;
    if no_companion && companion.any() {
        return Err(CliError::Invalid {
            flag: "--no-companion",
            reason: "--companion-*と併用しない（上書きする相手が無い）".to_owned(),
        });
    }
    let companion = build_companion(&companion)?;

    Ok(Parsed::Run(Box::new(Args {
        port: port.ok_or(CliError::Missing("--port"))?,
        baud: baud.ok_or(CliError::Missing("--baud"))?,
        duplicate_policy,
        startup_deadline_ms,
        speech,
        cloud_speech,
        no_companion,
        companion,
        probe,
        verbose,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Parsed, CliError> {
        parse(args.iter().map(|s| (*s).to_owned()))
    }

    const ALL: [&str; 10] = [
        "--port",
        "/dev/x",
        "--baud",
        "115200",
        "--duplicate-capacity",
        "8",
        "--duplicate-retention-ms",
        "1000",
        "--startup-deadline-ms",
        "5000",
    ];

    fn without(flag: &str) -> Vec<&'static str> {
        let mut out = Vec::new();
        let mut skip = false;
        for a in ALL {
            if skip {
                skip = false;
            } else if a == flag {
                skip = true;
            } else {
                out.push(a);
            }
        }
        out
    }

    #[test]
    fn all_required_flags_run() {
        let Ok(Parsed::Run(args)) = run(&ALL) else {
            panic!("実行になる");
        };
        assert_eq!(args.port, "/dev/x");
        assert_eq!(args.baud, 115_200);
        assert_eq!(args.startup_deadline_ms.get(), 5000);
        assert_eq!(args.speech, None);
        assert!(!args.cloud_speech);
        assert!(!args.no_companion);
        assert!(!args.verbose);
    }

    #[test]
    fn no_companion_is_optional() {
        let mut v = ALL.to_vec();
        v.push("--no-companion");
        let Ok(Parsed::Run(args)) = run(&v) else {
            panic!("実行になる");
        };
        assert!(args.no_companion);
    }

    fn with_probe(name: &str, interval: &str, no_companion: bool) -> Vec<String> {
        let mut v: Vec<String> = ALL.iter().map(|s| (*s).to_owned()).collect();
        v.extend(["--probe-expression", name, "--probe-interval-ms", interval].map(str::to_owned));
        if no_companion {
            v.push("--no-companion".to_owned());
        }
        v
    }

    fn run_owned(v: Vec<String>) -> Result<Parsed, CliError> {
        parse(v)
    }

    #[test]
    fn probe_is_off_by_default_and_parses_happy_with_no_companion() {
        let Ok(Parsed::Run(args)) = run(&ALL) else {
            panic!("実行になる");
        };
        assert_eq!(args.probe, None);
        let Ok(Parsed::Run(args)) = run_owned(with_probe("happy", "3000", true)) else {
            panic!("実行になる");
        };
        let probe = args.probe.expect("指定した");
        assert_eq!(probe.expression, Expression::Happy);
        assert_eq!(probe.interval_ms.get(), 3000);
        assert!(args.no_companion);
    }

    #[test]
    fn probe_requires_both_flags() {
        let mut only_name: Vec<String> = ALL.iter().map(|s| (*s).to_owned()).collect();
        only_name.extend(["--probe-expression", "happy", "--no-companion"].map(str::to_owned));
        assert_eq!(
            run_owned(only_name),
            Err(CliError::Missing("--probe-interval-ms"))
        );
        let mut only_interval: Vec<String> = ALL.iter().map(|s| (*s).to_owned()).collect();
        only_interval.extend(["--probe-interval-ms", "3000", "--no-companion"].map(str::to_owned));
        assert_eq!(
            run_owned(only_interval),
            Err(CliError::Missing("--probe-expression"))
        );
    }

    #[test]
    fn probe_rejects_bad_values() {
        for (name, interval) in [
            ("grumpy", "3000"),
            ("neutral", "3000"),
            ("happy", "0"),
            ("happy", "soon"),
        ] {
            assert!(
                matches!(
                    run_owned(with_probe(name, interval, true)),
                    Err(CliError::Invalid { .. })
                ),
                "{name} {interval}"
            );
        }
    }

    #[test]
    fn probe_is_rejected_with_the_companion() {
        assert!(matches!(
            run_owned(with_probe("happy", "3000", false)),
            Err(CliError::Invalid {
                flag: "--probe-expression",
                ..
            })
        ));
    }

    #[test]
    fn verbose_is_optional() {
        let mut v = ALL.to_vec();
        v.push("--verbose");
        let Ok(Parsed::Run(args)) = run(&v) else {
            panic!("実行になる");
        };
        assert!(args.verbose);
    }

    #[test]
    fn cloud_requires_explicit_local_speech_config() {
        let mut v = ALL.to_vec();
        v.push("--cloud-speech");
        assert_eq!(
            run(&v),
            Err(CliError::Missing(
                "--speech-min-interval-ms/--speech-duration-ms"
            ))
        );
        v.extend([
            "--speech-min-interval-ms",
            "60000",
            "--speech-duration-ms",
            "3000",
        ]);
        let Ok(Parsed::Run(args)) = run(&v) else {
            panic!("有効な設定");
        };
        assert!(args.cloud_speech);
        let last = v.len() - 3;
        v[last] = "1000";
        assert!(matches!(
            run(&v),
            Err(CliError::Invalid {
                flag: "--cloud-speech",
                ..
            })
        ));
        v[last] = "60000";
        *v.last_mut().expect("duration") = "0";
        assert!(matches!(
            run(&v),
            Err(CliError::Invalid {
                flag: "--cloud-speech",
                ..
            })
        ));
    }

    #[test]
    fn each_required_flag_is_required() {
        const REQUIRED: [&str; 5] = [
            "--port",
            "--baud",
            "--duplicate-capacity",
            "--duplicate-retention-ms",
            "--startup-deadline-ms",
        ];
        for flag in REQUIRED {
            assert_eq!(run(&without(flag)), Err(CliError::Missing(flag)), "{flag}");
        }
    }

    #[test]
    fn no_arguments_is_an_error_not_a_default() {
        assert!(matches!(run(&[]), Err(CliError::Missing(_))));
    }

    #[test]
    fn a_flag_without_a_value_is_an_error() {
        assert_eq!(
            run(&["--port"]),
            Err(CliError::NoValue("--port".to_owned()))
        );
    }

    #[test]
    fn a_non_number_is_an_error() {
        let mut v = ALL.to_vec();
        v[3] = "fast";
        assert!(matches!(
            run(&v),
            Err(CliError::Invalid { flag: "--baud", .. })
        ));
    }

    #[test]
    fn a_zero_startup_deadline_is_rejected() {
        let mut v = ALL.to_vec();
        v[9] = "0";
        assert!(matches!(
            run(&v),
            Err(CliError::Invalid {
                flag: "--startup-deadline-ms",
                ..
            })
        ));
    }

    #[test]
    fn a_zero_baud_is_left_to_serial_config() {
        let mut v = ALL.to_vec();
        v[3] = "0";
        assert!(matches!(run(&v), Ok(Parsed::Run(_))));
    }

    #[test]
    fn an_unknown_flag_is_an_error() {
        assert_eq!(
            run(&["--nope"]),
            Err(CliError::Unknown("--nope".to_owned()))
        );
    }

    #[test]
    fn help_succeeds_even_without_the_required_flags() {
        assert_eq!(run(&["--help"]), Ok(Parsed::Help));
        assert_eq!(run(&["-h"]), Ok(Parsed::Help));
    }

    #[test]
    fn a_repeated_flag_takes_the_last_value() {
        let mut v = ALL.to_vec();
        v.extend(["--port", "/dev/y"]);
        let Ok(Parsed::Run(args)) = run(&v) else {
            panic!("実行になる");
        };
        assert_eq!(args.port, "/dev/y");
    }

    #[test]
    fn speech_requires_both_nonzero_values_and_is_off_by_default() {
        let mut only_interval = ALL.to_vec();
        only_interval.extend(["--speech-min-interval-ms", "100"]);
        assert_eq!(
            run(&only_interval),
            Err(CliError::Missing("--speech-duration-ms"))
        );

        let mut only_duration = ALL.to_vec();
        only_duration.extend(["--speech-duration-ms", "200"]);
        assert_eq!(
            run(&only_duration),
            Err(CliError::Missing("--speech-min-interval-ms"))
        );

        let mut zero_interval = ALL.to_vec();
        zero_interval.extend([
            "--speech-min-interval-ms",
            "0",
            "--speech-duration-ms",
            "200",
        ]);
        assert!(matches!(
            run(&zero_interval),
            Err(CliError::Invalid {
                flag: "--speech-min-interval-ms",
                ..
            })
        ));

        let mut values = ALL.to_vec();
        values.extend([
            "--speech-min-interval-ms",
            "100",
            "--speech-duration-ms",
            "200",
        ]);
        let Ok(Parsed::Run(args)) = run(&values) else {
            panic!("独り言を有効にする")
        };
        let speech = args.speech.expect("両方を指定した");
        assert_eq!(speech.min_interval_ms.get(), 100);
        assert_eq!(speech.duration_ms, 200);
    }

    fn with_extra(extra: &[&str]) -> Result<Parsed, CliError> {
        let mut v = ALL.to_vec();
        v.extend(extra);
        run(&v)
    }

    #[test]
    fn companion_is_the_mvp_config_without_overrides() {
        let Ok(Parsed::Run(args)) = run(&ALL) else {
            panic!("実行になる");
        };
        assert_eq!(args.companion, CompanionConfig::MVP);
    }

    #[test]
    fn companion_overrides_only_the_given_fields() {
        let Ok(Parsed::Run(args)) = with_extra(&[
            "--companion-rest-interval-ms",
            "60000",
            "--companion-sleep-at",
            "13:05",
            "--companion-wake-at",
            "13:30",
            "--companion-hot-on-c",
            "26.5",
            "--companion-hot-off-c",
            "25",
            "--companion-pressure-drop-on-hpa",
            "1",
            "--companion-pressure-drop-off-hpa",
            "0.5",
        ]) else {
            panic!("実行になる");
        };
        let mvp = CompanionConfig::MVP;
        assert_eq!(
            args.companion,
            CompanionConfig {
                rest_interval_ms: 60_000,
                sleep_at: LocalMinutes::new(13, 5).expect("範囲内"),
                wake_at: LocalMinutes::new(13, 30).expect("範囲内"),
                hot_on_c: 26.5,
                hot_off_c: 25.0,
                pressure_drop_on_hpa: 1.0,
                pressure_drop_off_hpa: 0.5,
                ..mvp
            }
        );
    }

    #[test]
    fn companion_overrides_reject_bad_values() {
        let cases: [&[&str]; 14] = [
            &["--companion-rest-interval-ms", "0"],
            &["--companion-sleep-at", "24:00"],
            &["--companion-sleep-at", "22:60"],
            &["--companion-wake-at", "7"],
            &["--companion-wake-at", "ab:cd"],
            &["--companion-sleep-at", "07:00"],
            &["--companion-hot-on-c", "nan"],
            &["--companion-hot-on-c", "inf"],
            &["--companion-hot-on-c", "20"],
            &["--companion-cold-off-c", "10"],
            &["--companion-dry-off-pct", "30"],
            &["--companion-pressure-drop-off-hpa", "4"],
            &["--companion-rest-interval-ms", "-1"],
            &["--companion-hot-off-c", "hot"],
        ];
        for extra in cases {
            assert!(
                matches!(with_extra(extra), Err(CliError::Invalid { .. })),
                "{extra:?}"
            );
        }
    }

    #[test]
    fn companion_overrides_are_rejected_with_no_companion() {
        assert!(matches!(
            with_extra(&["--no-companion", "--companion-rest-interval-ms", "1000"]),
            Err(CliError::Invalid {
                flag: "--no-companion",
                ..
            })
        ));
    }

    #[test]
    fn a_companion_flag_without_a_value_is_an_error() {
        assert_eq!(
            with_extra(&["--companion-sleep-at"]),
            Err(CliError::NoValue("--companion-sleep-at".to_owned()))
        );
    }

    #[test]
    fn overridden_config_drives_a_companion() {
        use deskcat_domain::{Bubble, Companion, Face, Millis};
        let Ok(Parsed::Run(args)) = with_extra(&[
            "--companion-rest-interval-ms",
            "1000",
            "--companion-sleep-at",
            "12:00",
            "--companion-wake-at",
            "12:10",
        ]) else {
            panic!("実行になる");
        };
        let noon = LocalMinutes::new(11, 0).expect("範囲内");
        let mut c = Companion::new(args.companion, Millis(0));
        assert_eq!(c.tick(Millis(500), noon).bubble, None);
        assert_eq!(c.tick(Millis(1000), noon).bubble, Some(Bubble::TakeBreak));
        // 上書きした夜の時間帯に入ると寝顔になり、MVPの夜（22時）には依存しない。
        let night = LocalMinutes::new(12, 5).expect("範囲内");
        assert_eq!(c.tick(Millis(1500), night).face, Some(Face::Sleeping));
        assert!(c.is_asleep());
    }
}
