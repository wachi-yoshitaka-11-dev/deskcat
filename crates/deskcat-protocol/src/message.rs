//! Message typeとtype固有payload。
//!
//! ここで型を起こすのは、Issue #4がreviewで承認した最小集合だけである。
//! `set_expression`、`play_motion`、`show_text`、`show_choices`、sensor eventは、
//! 上限値が`PROTO-TBD-007`／`008`／`009`／`014`で未確定であり、値を推測しないため含めない。

use serde::{Deserialize, Serialize};

use crate::error::{DecodeError, ErrorCode};
use crate::limits;

/// 受理するmessageと、そのtype固有payload。
///
/// 新しいtypeの追加で下流の`match`が壊れないよう`#[non_exhaustive]`とする。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Message {
    /// ESP32→Pi。protocol taskの準備完了を伝え、新しいESP32 sessionを開始する（§4.1）。
    Boot(Boot),
    /// Pi→ESP32。新しいPi sessionを開始する、またはlink再開後の同期を求める（§5.1）。
    Hello(Hello),
    /// Pi→ESP32。payloadを持たないheartbeat（§5.7）。
    Ping,
    /// Pi→ESP32。`status` snapshotを要求する（§5.6）。
    GetStatus,
    /// ESP32→Pi。実stateのsnapshot（§4.6）。
    ///
    /// 他のvariantより大きいため`Box`で包み、enum全体のsizeを抑える。
    Status(Box<Status>),
    /// 要求messageの受理または拒否を、`(reply_sid, reply_to)`で相関させて返す（§6）。
    Ack(Ack),
}

impl Message {
    /// envelopeの`type`に載せる文字列を返す。
    #[must_use]
    pub const fn type_str(&self) -> &'static str {
        match self {
            Self::Boot(_) => "boot",
            Self::Hello(_) => "hello",
            Self::Ping => "ping",
            Self::GetStatus => "get_status",
            Self::Status(_) => "status",
            Self::Ack(_) => "ack",
        }
    }

    /// wire上の`type`が既知の名前なら、その`'static`な綴りを返す。
    ///
    /// [`Self::type_str`]の逆写像である。上限付きprefixからの復元（[`crate::prefix`]）が、
    /// 所有権を持たずにtypeを判定するために使う。綴りが両方向で一致していることは、
    /// `tests/conformance.rs`が共有fixtureの`expect.type`に対して検査する。
    #[must_use]
    pub fn known_type_name(name: &[u8]) -> Option<&'static str> {
        Self::TYPE_NAMES
            .into_iter()
            .find(|known| known.as_bytes() == name)
    }

    /// 受理するtype名の一覧。[`Self::type_str`]と同じ綴りを使う。
    const TYPE_NAMES: [&'static str; 6] = ["boot", "hello", "ping", "get_status", "status", "ack"];

    /// 上限のある値が範囲内かを検査する。
    ///
    /// # Errors
    ///
    /// string fieldがbyte上限を超える場合は[`ErrorCode::OutOfRange`]を返す。
    pub fn check_bounds(&self) -> Result<(), DecodeError> {
        match self {
            Self::Boot(boot) => boot.check_bounds(),
            Self::Hello(hello) => hello.check_bounds(),
            Self::Status(status) => status.check_bounds(),
            Self::Ack(ack) => ack.check_bounds(),
            Self::Ping | Self::GetStatus => Ok(()),
        }
    }

    /// type固有のfield間整合を検査する。
    ///
    /// [`Self::check_bounds`]が値の範囲を見るのに対し、こちらはfieldの組み合わせを見る。
    /// [`crate::decode_line`]と[`crate::encode_line`]の両方がこの関数を呼ぶ。**送信側と
    /// 受信側で規則を二重実装しないための単一の入口である。**新しいtypeがshape規則を
    /// 持ったときは、ここへ足せば両経路へ同時に効く。
    ///
    /// # Errors
    ///
    /// 組み合わせが§6を満たさない場合は[`ErrorCode::InvalidPayload`]を返す。
    pub(crate) fn check_shape(&self) -> Result<(), DecodeError> {
        match self {
            Self::Ack(ack) => ack.check_shape(),
            Self::Boot(_) | Self::Hello(_) | Self::Status(_) | Self::Ping | Self::GetStatus => {
                Ok(())
            }
        }
    }
}

/// 上限のあるstring fieldを検査する。
fn check_len(field: &str, value: &str, max: usize) -> Result<(), DecodeError> {
    if value.len() > max {
        return Err(DecodeError::new(
            ErrorCode::OutOfRange,
            format!("`{field}` is {} bytes, limit is {max}", value.len()),
        ));
    }
    Ok(())
}

/// `boot`のpayload（§4.1）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boot {
    /// Firmware version／build identity。
    pub firmware: String,
    /// Firmware board-configuration ID。
    pub board: String,
    /// Machine-readableなreset reason。
    pub reset_reason: String,
}

impl Boot {
    fn check_bounds(&self) -> Result<(), DecodeError> {
        check_len("firmware", &self.firmware, limits::MAX_FIRMWARE_BYTES)?;
        check_len("board", &self.board, limits::MAX_BOARD_BYTES)?;
        check_len(
            "reset_reason",
            &self.reset_reason,
            limits::MAX_RESET_REASON_BYTES,
        )
    }
}

/// `hello`の`reason`（§5.1）。
///
/// `reason`と`sid`の整合（`startup`は新しい`sid`、`port_reopen`／`resync`は現在の`sid`）は
/// session stateを持つ受信側が判定する。この型は列挙値だけを保証する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HelloReason {
    /// Piのprocessを起動した。新しい`sid`を使う。
    Startup,
    /// serial portを開き直した。現在の`sid`を維持する。
    PortReopen,
    /// stateの再取得を求める。現在の`sid`を維持する。
    Resync,
}

/// `hello`のpayload（§5.1）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Host process identity。
    pub host: String,
    /// Host version／build identity。
    pub version: String,
    /// このhelloを送る理由。
    pub reason: HelloReason,
}

impl Hello {
    fn check_bounds(&self) -> Result<(), DecodeError> {
        check_len("host", &self.host, limits::MAX_HOST_BYTES)?;
        check_len("version", &self.version, limits::MAX_VERSION_BYTES)
    }
}

/// `ack.status`（§6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AckStatus {
    /// 要求を検証し、処理対象として受け入れた。物理動作の完了は意味しない。
    Ok,
    /// 要求を拒否した。`code`を伴う。
    Rejected,
}

/// `ack`のpayload（§6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    /// Acknowledgeする要求messageの送信session ID。
    pub reply_sid: u32,
    /// Acknowledgeする要求message ID。
    pub reply_to: u32,
    /// 受理したか拒否したか。
    pub status: AckStatus,
    /// 拒否理由。`status`が`rejected`のとき必須である。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
    /// 診断用の短い説明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Ack {
    fn check_bounds(&self) -> Result<(), DecodeError> {
        if let Some(detail) = &self.detail {
            check_len("detail", detail, limits::MAX_DETAIL_BYTES)?;
        }
        Ok(())
    }

    /// `status`と`code`の組が§6を満たすかを検査する。
    ///
    /// `ok`に`code`が付いていても拒否しない。§6が要求しているのは
    /// 「`rejected`の場合はcodeを含める」ことだけであり、それ以上に厳しくすると
    /// 仕様上妥当なpeerを拒否する。
    pub(crate) fn check_shape(&self) -> Result<(), DecodeError> {
        if matches!(self.status, AckStatus::Rejected) && self.code.is_none() {
            return Err(DecodeError::new(
                ErrorCode::InvalidPayload,
                "`ack` with status `rejected` requires `code`",
            ));
        }
        Ok(())
    }
}

/// `status.payload.display`（§4.6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayStatus {
    /// Display state名。
    pub state: String,
    /// 現在のexpression名。
    pub expression: String,
}

/// `status.payload.servo`（§4.6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServoStatus {
    /// Servo state名。
    pub state: String,
}

/// `status.payload.sensors`（§4.6）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorStatus {
    /// Touch入力の状態。
    pub touch: String,
    /// 加速度sensorの状態。
    pub acceleration: String,
    /// 環境sensorの状態。
    pub environment: String,
}

/// `status.payload.protocol`のcounter群（§4.6）。
///
/// 各fieldの意味は§4.6のcounter対応表を正本とする。[`ErrorCode::counter_field`]が
/// error codeからここのfield名を返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtocolCounters {
    /// UTF-8／JSON／envelope不正、および相関ACKを構成できなかったmessage。
    pub parse_errors: u32,
    /// type固有schemaまたは現在stateとの整合で拒否した件数。
    pub invalid_payloads: u32,
    /// Protocol major versionの不整合で拒否した件数。
    pub unsupported_versions: u32,
    /// 最大line長の超過。
    pub oversize_lines: u32,
    /// 未知のmessage type。方向を問わず計上する。
    pub unknown_types: u32,
    /// 受理上限、session遷移budget／cooldown、servoの受理command数超過の合算。
    pub rate_limited: u32,
    /// resourceの一時的な占有による拒否。
    pub busy: u32,
    /// 上限のある値が許容範囲外だった拒否。
    pub out_of_range: u32,
    /// 承認されていない`sid`による拒否。
    pub stale_sessions: u32,
    /// 初期化が完了していないhardwareへのcommandによる拒否。
    ///
    /// `busy`と混同しない。§7のとおり`busy`は初期化済みのresourceが一時的に塞がっている
    /// 状態であり、待てば受け付けられる。こちらは待っても受け付けられない。
    pub hardware_unavailable: u32,
    /// 保持履歴から失われたduplicateを再実行できず拒否した件数。
    ///
    /// codeの意味は§7が定める。**発火条件は未確定である。**§9の`TBD`
    /// 「Duplicateが保持履歴より古い場合の動作」と、履歴の保持期間・件数・evict後の
    /// 扱い（`PROTO-TBD-005`）で決まる。**このcounterはそれらの値を先取りしない。**
    pub duplicate_expired: u32,
    /// 実際に発生したsession遷移。
    pub session_switches: u32,
    /// 受信機会に即時送出しなかった応答の総数。
    pub suppressed_responses: u32,
}

/// `status`のpayload（§4.6）。
///
/// field集合は`PROTO-TBD-006`で確定する。現在は§4.6が例示するgroupに合わせている。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Firmware version／build identity。
    pub firmware: String,
    /// Machine-readableなreset reason。
    pub reset_reason: String,
    /// Displayの状態。
    pub display: DisplayStatus,
    /// Servoの状態。
    pub servo: ServoStatus,
    /// Sensorの状態。
    pub sensors: SensorStatus,
    /// Protocol counter。
    pub protocol: ProtocolCounters,
}

impl Status {
    fn check_bounds(&self) -> Result<(), DecodeError> {
        check_len("firmware", &self.firmware, limits::MAX_FIRMWARE_BYTES)?;
        check_len(
            "reset_reason",
            &self.reset_reason,
            limits::MAX_RESET_REASON_BYTES,
        )?;
        check_len(
            "display.state",
            &self.display.state,
            limits::MAX_STATE_NAME_BYTES,
        )?;
        check_len(
            "display.expression",
            &self.display.expression,
            limits::MAX_STATE_NAME_BYTES,
        )?;
        check_len(
            "servo.state",
            &self.servo.state,
            limits::MAX_STATE_NAME_BYTES,
        )?;
        check_len(
            "sensors.touch",
            &self.sensors.touch,
            limits::MAX_STATE_NAME_BYTES,
        )?;
        check_len(
            "sensors.acceleration",
            &self.sensors.acceleration,
            limits::MAX_STATE_NAME_BYTES,
        )?;
        check_len(
            "sensors.environment",
            &self.sensors.environment,
            limits::MAX_STATE_NAME_BYTES,
        )
    }
}
