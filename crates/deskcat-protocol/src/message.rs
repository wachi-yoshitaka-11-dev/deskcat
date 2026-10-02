//! Message typeとtype固有payload。
//!
//! Issue #4がreviewで承認した最小集合（`boot`、`hello`、`ping`、`get_status`、`status`、`ack`）に、
//! Issue #527で`set_expression`、`show_text`、`head_touched`、`tapped`、`lifted`、`environment`を
//! 足した。足したtypeのfieldは、安全要件の5項目（`docs/governance/hardware-safety-policy.md`）の
//! どれにも効かない。5項目のうちmessageが関わりうるのはservoの持続的拘束だけであり、
//! servoを動かす経路は`play_motion`（§5.3）だけで、それを含めないためである。
//!
//! 含めないtypeと理由:
//!
//! - `play_motion`と§4.7の`motion_completed`／`motion_stopped`: motion名と範囲が`PROTO-TBD-008`
//! - §5.3の(b)のservoのfault event: `PROTO-TBD-014`。§4.7の`sensor_fault`: 仕様にschemaが無い
//! - `show_choices`: touchとlayoutの選定待ち（§5.5）
//! - `protocol_fault`: 仕様にpayload schemaが無い（`PROTO-TBD-018`）
//!
//! **仕様が意味を決めていないfieldは型に持たない。**`head_touched.strength`（§4.2、
//! `PROTO-TBD-009`）と`tapped.magnitude_g`（§4.3）である。受けたときは§3の
//! 「未知の追加payload fieldは無視してよい」により読み捨てる。意味が決まったら、
//! §11のoptional fieldの追加として足す。
//!
//! **時間の値（`transition_ms`、`duration_ms`）には数値の上限を置かない。**§5.2と§5.4は
//! その上限をdisplayとfirmwareの実装に置いており、ここで数を決めれば推測になる。
//! 型の幅（`u32`）で縛る。

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
    /// Pi→ESP32。表情を切り替える（§5.2）。
    SetExpression(SetExpression),
    /// Pi→ESP32。文字列を表示する（§5.4）。
    ShowText(ShowText),
    /// ESP32→Pi。頭を撫でられた（§4.2）。
    HeadTouched(HeadTouched),
    /// ESP32→Pi。軽く叩かれた（§4.3）。
    ///
    /// payloadは`{}`である。§4.3の例にある`magnitude_g`は、意味が決まっていないため
    /// 型に持たない（module doc）。
    Tapped,
    /// ESP32→Pi。持ち上げられた（§4.4）。
    ///
    /// 型があることは送ってよいことを意味しない。§4.4は、accelerometerのclassifierを
    /// 検証した後にだけ有効にすると定めている。
    Lifted(Lifted),
    /// ESP32→Pi。環境sensorの測定値（§4.5）。
    Environment(Environment),
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
            Self::SetExpression(_) => "set_expression",
            Self::ShowText(_) => "show_text",
            Self::HeadTouched(_) => "head_touched",
            Self::Tapped => "tapped",
            Self::Lifted(_) => "lifted",
            Self::Environment(_) => "environment",
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
    const TYPE_NAMES: [&'static str; 12] = [
        "boot",
        "hello",
        "ping",
        "get_status",
        "status",
        "ack",
        "set_expression",
        "show_text",
        "head_touched",
        "tapped",
        "lifted",
        "environment",
    ];

    /// 上限のある値が範囲内かを検査する。
    ///
    /// # Errors
    ///
    /// string fieldがbyte上限を超える場合、または`show_text.text`が制御文字を含む場合は
    /// [`ErrorCode::OutOfRange`]を返す。
    pub fn check_bounds(&self) -> Result<(), DecodeError> {
        match self {
            Self::Boot(boot) => boot.check_bounds(),
            Self::Hello(hello) => hello.check_bounds(),
            Self::Status(status) => status.check_bounds(),
            Self::Ack(ack) => ack.check_bounds(),
            Self::ShowText(show_text) => show_text.check_bounds(),
            Self::Ping
            | Self::GetStatus
            | Self::SetExpression(_)
            | Self::HeadTouched(_)
            | Self::Tapped
            | Self::Lifted(_)
            | Self::Environment(_) => Ok(()),
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
            Self::Boot(_)
            | Self::Hello(_)
            | Self::Status(_)
            | Self::Ping
            | Self::GetStatus
            | Self::SetExpression(_)
            | Self::ShowText(_)
            | Self::HeadTouched(_)
            | Self::Tapped
            | Self::Lifted(_)
            | Self::Environment(_) => Ok(()),
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

/// `set_expression.name`（§5.2の初期のexpression名）。
///
/// 列挙外の名前は、§7の表の「列挙外の値」として[`ErrorCode::InvalidPayload`]で拒否する。
/// §5.2の「Firmwareは未知の名前をrejectする」をschemaで満たす。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExpressionName {
    /// 通常の顔。
    Neutral,
    /// 喜んだ顔。
    Happy,
    /// 驚いた顔。
    Surprised,
}

/// `set_expression`のpayload（§5.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetExpression {
    /// 切り替える先のexpression。
    pub name: ExpressionName,
    /// 切り替えにかける時間（ms）。
    ///
    /// 数値の上限はここに置かない。§5.2は上限をdisplay実装で定義するとしている。
    pub transition_ms: u32,
}

/// `show_text`のpayload（§5.4）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowText {
    /// 表示する文字列。
    ///
    /// UTF-8のbyte長を[`limits::MAX_TEXT_BYTES`]で縛り、制御文字（[`char::is_control`]、
    /// Unicodeの一般カテゴリ`Cc`。改行とtabを含む）を1文字も許さない（§5.4、§7）。
    /// 行数とlayout処理量の上限は`PROTO-TBD-007`であり、ここでは縛らない。
    pub text: String,
    /// 表示する時間（ms）。
    ///
    /// 数値の上限はここに置かない。§5.4は上限をfirmwareに置いている。
    pub duration_ms: u32,
}

impl ShowText {
    fn check_bounds(&self) -> Result<(), DecodeError> {
        check_len("text", &self.text, limits::MAX_TEXT_BYTES)?;
        // 制御文字の上限は0である（§7。§5.4は上限を設けると定めるが値を書いていない）。
        if let Some(control) = self.text.chars().find(|c| c.is_control()) {
            return Err(DecodeError::new(
                ErrorCode::OutOfRange,
                format!(
                    "`text` contains the control character U+{:04X}",
                    u32::from(control)
                ),
            ));
        }
        Ok(())
    }
}

/// `head_touched`のpayload（§4.2）。
///
/// §4.2の例にある`strength`は、意味が決まっていないため持たない（module doc）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadTouched {
    /// 触れていた時間（ms）。
    pub duration_ms: u32,
}

/// `lifted`のpayload（§4.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifted {
    /// 持ち上げられていた時間（ms）。
    pub duration_ms: u32,
}

/// 有限の`f32`。`NaN`と無限大を持たない。
///
/// 有限値だけを持つため、`f32`の`PartialEq`は同値関係になり、`Eq`を実装できる
/// （`0.0`と`-0.0`は等しいが、反射律・対称律・推移律は崩れない）。素の`f32`を
/// [`Message`]へ入れると、[`Message`]と、それを含む型の`Eq`が外れる。
///
/// JSONは`NaN`と無限大を表せない。decodeは読んだ数を`f32`へ狭め、有限にならない数
/// （例: `1e39`）を[`ErrorCode::InvalidPayload`]で拒否する。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct FiniteF32(f32);

impl Eq for FiniteF32 {}

impl FiniteF32 {
    /// 有限値なら包んで返す。`NaN`と無限大には`None`を返す。
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// 包んでいる値を返す。
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FiniteF32 {
    /// `f64`として読み、`f32`へ狭める。
    ///
    /// **`f32::deserialize`を直接使わない。**`firmware/esp32`のbuild（`esp-1.95.0.0`、
    /// `xtensa-esp32-espidf`）で、`serde_json`がJSONの数を`f32`として読む関数の生成が
    /// LLVM の Xtensa backend の内部エラー（`Cannot select: ... XtensaISD::PCREL_WRAPPER`）で
    /// 止まった（#527）。`f64`として読む形は同じbuildが通る。
    ///
    /// 狭めた結果が有限でなければ、下の検査で拒否する。
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wide = f64::deserialize(deserializer)?;
        #[allow(clippy::cast_possible_truncation)]
        let value = wide as f32;
        Self::new(value).ok_or_else(|| {
            serde::de::Error::custom(format!("{value} is not a finite 32-bit float"))
        })
    }
}

/// `environment`のpayload（§4.5）。
///
/// §4.5は「選定したsensorが対応する測定量だけを含める」と定めるため、どのfieldも
/// 省略できる。値の範囲はsensor（#16）で決まるため、有限であることだけを検査する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// 気温（℃）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<FiniteF32>,
    /// 相対湿度（%）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidity_pct: Option<FiniteF32>,
    /// 気圧（hPa）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure_hpa: Option<FiniteF32>,
}
