//! 引数の解析。
//!
//! **必須の値は、すべて既定値を持たない。**device名は未確認であり（確定はIssue #11の後半）、baudの正本は
//! `PROTO-TBD-001`、duplicate履歴の正本は`PROTO-TBD-005`で、いずれも未確定である。起動の期限に対応するTBDの
//! 登録は無く、値の根拠も無い（`ESP32`の起動時間は未測定）。
//! **確認していない値を既定として固定しない。**足りなければ起動せず、終了1にする（[`CliError`]）。
//! 値の暫定の置き場所は、段階2b-iiiのunitの1か所だけである。
//!
//! 同じ引数を重ねたときは、後の値で上書きする（`deskcat-serial`の`serial_link`と同じ。検出しない）。
//! baudの0は、ここでは検査しない。`SerialConfig::new`が弾く（判断は`deskcat-serial`に置く）。

use std::fmt;
use std::num::NonZeroU64;
use std::time::Duration;

use deskcat_serial::DuplicatePolicy;

/// 使い方。
pub const USAGE: &str = "usage: deskcatd --port <path> --baud <rate> \
--duplicate-capacity <n> --duplicate-retention-ms <ms> --startup-deadline-ms <ms> [--verbose]\n\
\n\
--port、--baud、--duplicate-capacity、--duplicate-retention-ms、--startup-deadline-ms は必須である。\
既定値を持たせない。\n\
device名は未確認であり、baudの正本は PROTO-TBD-001、duplicate履歴の正本は PROTO-TBD-005 である。\
起動の期限は、値の根拠が無い（ESP32の起動時間は未測定）。";

/// 解析した引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Serial deviceのpath。
    pub port: String,
    /// Baud rate。0は`SerialConfig::new`が弾く。
    pub baud: u32,
    /// `boot`のduplicate履歴の方針。
    pub duplicate_policy: DuplicatePolicy,
    /// 起動から、ESP32のsessionの確立と`hello`の結果がそろうまでの期限（ms）。0は受け付けない。
    pub startup_deadline_ms: NonZeroU64,
    /// `Debug`まで出す。
    pub verbose: bool,
}

/// 解析の結果。
///
/// `--help`は**成功である。**`Err`へ畳むと終了コードが1になり、scriptから「使い方を聞いた」と
/// 「引数を間違えた」を区別できない。
#[derive(Debug, Clone, PartialEq, Eq)]
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
    let mut verbose = false;
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
            "--verbose" => verbose = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            other => return Err(CliError::Unknown(other.to_owned())),
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

    Ok(Parsed::Run(Box::new(Args {
        port: port.ok_or(CliError::Missing("--port"))?,
        baud: baud.ok_or(CliError::Missing("--baud"))?,
        duplicate_policy,
        startup_deadline_ms,
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
        assert!(!args.verbose);
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
}
