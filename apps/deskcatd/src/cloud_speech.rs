//! #522: Claudeで独り言を1文生成する。HTTPと予算fileのI/Oは専用workerだけで行う。
//! APIキー、応答本文、ライブラリのerror文字列はlogへ渡さない。

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use deskcat_domain::Expression;
use deskcat_face::text::TextLayout;
use deskcat_protocol::{Message, ShowText};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

const MODEL: &str = "claude-haiku-5-5";
const API_ROOT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const SYSTEM_PROMPT: &str = "あなたは小さな猫です。日本語で独り言を一文だけ返してください。20文字以内、改行なし。ひらがな・カタカナ・JIS第一水準の漢字と普通の句読点だけを使ってください。説明や引用符は不要です。";
const MAX_INPUT_TOKENS: u32 = 512;
const MAX_OUTPUT_TOKENS: u32 = 128;
const MAX_DAILY_ATTEMPTS: u32 = 300;
/// 生成試行の最短間隔。再起動後も予算fileで維持する。
pub const MIN_INTERVAL_SECS: u64 = 60;
// $0.10 / 1M input token と $0.50 / 1M output token。単位は10^-9 USD。
const MAX_ESTIMATE_NANO_USD: u64 = 512 * 100 + 128 * 500;
const DAILY_ESTIMATE_NANO_USD: u64 = 40_000_000;
const MIN_VALID_UNIX_SECS: u64 = 1_735_689_600; // 2025-01-01 UTC。未同期時計での送信を避ける。
const RESPONSE_LIMIT_BYTES: u64 = 4096;

/// 秘密情報を含まない失敗分類。raw HTTP errorや応答本文は保持しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// キーやsystemd credentialが無い、または不正。
    Credential,
    /// 時計が不正、戻った、または費用・頻度の上限に達した。
    Budget,
    /// 予算fileの読み書きに失敗。安全側で送らない。
    BudgetIo,
    /// HTTPの制限時間を超えた。
    Timeout,
    /// HTTP statusが成功以外。
    Http,
    /// TLS、名前解決など通信の失敗。
    Network,
    /// 入力token数が上限を超えた。
    InputTokens,
    /// 応答のJSON、構造、停止理由、使用量が不正。
    Response,
    /// 表示可能文字・文字数・byte数・制御文字の検査で拒否。
    Display,
    /// workerへ送れない。
    Worker,
}

impl Failure {
    const fn index(self) -> usize {
        match self {
            Self::Credential => 0,
            Self::Budget => 1,
            Self::BudgetIo => 2,
            Self::Timeout => 3,
            Self::Http => 4,
            Self::Network => 5,
            Self::InputTokens => 6,
            Self::Response => 7,
            Self::Display => 8,
            Self::Worker => 9,
        }
    }
}

/// 分類ごとの累計。秘密情報は持たない。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    /// 生成と送信前の表示検査に成功した文。
    pub success: u64,
    /// 原因別の失敗件数。順序は[`Failure::index`]に対応する。
    pub failures: [u64; 10],
}

impl Counters {
    fn note(&mut self, result: &Result<String, Failure>) {
        match result {
            Ok(_) => self.success += 1,
            Err(kind) => self.failures[kind.index()] += 1,
        }
    }

    /// 失敗分類別の累計。
    #[must_use]
    pub const fn count(self, kind: Failure) -> u64 {
        self.failures[kind.index()]
    }
}

/// workerへ渡す情報はenumと粗い時刻帯だけ。時計が無効なら時刻帯を省く。
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// 現在の表情。センサ生値は含めない。
    pub expression: Expression,
    /// UTCの時刻帯。0: 深夜、1: 朝、2: 昼、3: 夜。
    pub time_band: Option<u8>,
}

impl Context {
    fn user_prompt(self) -> String {
        let emotion = match self.expression {
            Expression::Neutral => "neutral",
            Expression::Happy => "happy",
            Expression::Surprised => "surprised",
        };
        let period = match self.time_band {
            Some(0) => "night",
            Some(1) => "morning",
            Some(2) => "day",
            Some(3) => "evening",
            _ => "unknown",
        };
        format!("emotion={emotion}; time_band={period}")
    }
}

/// UTC時刻を取得できるときだけ時刻帯を返す。料金の日付判定は別に行う。
#[must_use]
pub fn time_band(now: SystemTime) -> Option<u8> {
    let secs = now.duration_since(UNIX_EPOCH).ok()?.as_secs();
    if secs < MIN_VALID_UNIX_SECS {
        return None;
    }
    let hour = (secs % 86_400) / 3600;
    Some(match hour {
        0..=5 => 0,
        6..=11 => 1,
        12..=17 => 2,
        _ => 3,
    })
}

/// テストではこの境界を差し替え、実APIへ接続しない。
pub trait Client: Send + 'static {
    /// 送信予定promptの入力token数。
    ///
    /// # Errors
    ///
    /// 通信、HTTP、応答形式の失敗。
    fn count_tokens(&mut self, context: Context) -> Result<u32, Failure>;
    /// 一文を生成する。
    ///
    /// # Errors
    ///
    /// 通信、HTTP、応答形式の失敗。
    fn generate(&mut self, context: Context) -> Result<String, Failure>;
}

/// 固定のAnthropic endpoint以外へ要求しないHTTP client。
pub struct ClaudeClient {
    agent: ureq::Agent,
    key: Secret,
}

// `Debug`を実装しない。errorやpanicからキーが出るのを防ぐ。
struct Secret(String);

impl ClaudeClient {
    /// systemdが渡したcredential fileを読む。pathや値はlogに出さない。
    ///
    /// # Errors
    ///
    /// credentialが無いか形式が不正な場合。
    pub fn from_credential_dir(dir: &Path) -> Result<Self, Failure> {
        let file =
            fs::File::open(dir.join("anthropic-api-key")).map_err(|_| Failure::Credential)?;
        if file.metadata().map_err(|_| Failure::Credential)?.len() > 514 {
            return Err(Failure::Credential);
        }
        let mut key = String::new();
        file.take(514)
            .read_to_string(&mut key)
            .map_err(|_| Failure::Credential)?;
        while key.ends_with('\n') || key.ends_with('\r') {
            key.pop();
        }
        if key.is_empty() || key.len() > 512 || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(Failure::Credential);
        }
        let tls = TlsConfig::builder()
            .provider(TlsProvider::NativeTls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .https_only(true)
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .new_agent();
        Ok(Self {
            agent,
            key: Secret(key),
        })
    }

    fn post(&self, path: &str, body: Value) -> Result<Value, Failure> {
        let url = format!("{API_ROOT}{path}");
        let mut response = self
            .agent
            .post(&url)
            .header("x-api-key", &self.key.0)
            .header("anthropic-version", API_VERSION)
            .send_json(body)
            .map_err(|error| classify_http(&error))?;
        if response.status() != 200 {
            return Err(Failure::Http);
        }
        response
            .body_mut()
            .with_config()
            .limit(RESPONSE_LIMIT_BYTES)
            .read_json()
            .map_err(|error| classify_body(&error))
    }
}

fn classify_http(error: &ureq::Error) -> Failure {
    match error {
        ureq::Error::Timeout(_) => Failure::Timeout,
        ureq::Error::StatusCode(_) => Failure::Http,
        _ => Failure::Network,
    }
}

fn classify_body(error: &ureq::Error) -> Failure {
    match error {
        ureq::Error::Timeout(_) => Failure::Timeout,
        ureq::Error::Io(_) | ureq::Error::BodyStalled => Failure::Network,
        _ => Failure::Response,
    }
}

fn base_request(context: Context) -> Value {
    json!({
        "model": MODEL,
        "system": SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": context.user_prompt()}],
        "thinking": {"type": "disabled"},
    })
}

impl Client for ClaudeClient {
    fn count_tokens(&mut self, context: Context) -> Result<u32, Failure> {
        #[derive(Deserialize)]
        struct Count {
            input_tokens: u32,
        }
        let reply: Count =
            serde_json::from_value(self.post("/count_tokens", base_request(context))?)
                .map_err(|_| Failure::Response)?;
        Ok(reply.input_tokens)
    }

    fn generate(&mut self, context: Context) -> Result<String, Failure> {
        #[derive(Deserialize)]
        struct Block {
            #[serde(rename = "type")]
            kind: String,
            text: Option<String>,
        }
        #[derive(Deserialize)]
        struct Usage {
            input_tokens: u32,
            output_tokens: u32,
        }
        #[derive(Deserialize)]
        struct Reply {
            content: Vec<Block>,
            stop_reason: Option<String>,
            usage: Usage,
        }
        let mut body = base_request(context);
        body["max_tokens"] = json!(MAX_OUTPUT_TOKENS);
        let reply: Reply =
            serde_json::from_value(self.post("", body)?).map_err(|_| Failure::Response)?;
        if reply.stop_reason.as_deref() != Some("end_turn")
            || reply.usage.input_tokens > MAX_INPUT_TOKENS
            || reply.usage.output_tokens > MAX_OUTPUT_TOKENS
            || reply.content.len() != 1
        {
            return Err(Failure::Response);
        }
        let block = reply.content.into_iter().next().ok_or(Failure::Response)?;
        if block.kind != "text" {
            return Err(Failure::Response);
        }
        block.text.ok_or(Failure::Response)
    }
}

/// 表示制約はfirmwareとprotocolの共通crateで検査する。
///
/// # Errors
///
/// 空、制御文字、884 byte超、20文字超、収録外glyphの場合。
pub fn validate_text(text: &str, duration_ms: u32) -> Result<(), Failure> {
    Message::ShowText(ShowText {
        text: text.to_owned(),
        duration_ms,
    })
    .check_bounds()
    .map_err(|_| Failure::Display)?;
    TextLayout::new(text).map_err(|_| Failure::Display)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct BudgetState {
    day: u64,
    attempts: u32,
    estimated_nano_usd: u64,
    last_attempt_secs: u64,
}

struct Budget {
    path: PathBuf,
}

impl Budget {
    fn reserve(&self, now: SystemTime) -> Result<(), Failure> {
        let secs = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::Budget)?
            .as_secs();
        if secs < MIN_VALID_UNIX_SECS {
            return Err(Failure::Budget);
        }
        let day = secs / 86_400;
        let mut state = match fs::File::open(&self.path) {
            Ok(file) => {
                if file.metadata().map_err(|_| Failure::BudgetIo)?.len() > 1024 {
                    return Err(Failure::BudgetIo);
                }
                let mut data = Vec::new();
                file.take(1024)
                    .read_to_end(&mut data)
                    .map_err(|_| Failure::BudgetIo)?;
                serde_json::from_slice::<BudgetState>(&data).map_err(|_| Failure::BudgetIo)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BudgetState {
                day,
                attempts: 0,
                estimated_nano_usd: 0,
                last_attempt_secs: 0,
            },
            Err(_) => return Err(Failure::BudgetIo),
        };
        if day < state.day || secs < state.last_attempt_secs {
            return Err(Failure::Budget);
        }
        if state.last_attempt_secs != 0
            && secs.saturating_sub(state.last_attempt_secs) < MIN_INTERVAL_SECS
        {
            return Err(Failure::Budget);
        }
        if day > state.day {
            state.day = day;
            state.attempts = 0;
            state.estimated_nano_usd = 0;
        }
        if state.attempts >= MAX_DAILY_ATTEMPTS
            || state.estimated_nano_usd > DAILY_ESTIMATE_NANO_USD - MAX_ESTIMATE_NANO_USD
        {
            return Err(Failure::Budget);
        }
        state.attempts += 1;
        state.estimated_nano_usd += MAX_ESTIMATE_NANO_USD;
        state.last_attempt_secs = secs;
        self.persist(state)
    }

    fn persist(&self, state: BudgetState) -> Result<(), Failure> {
        let parent = self.path.parent().ok_or(Failure::BudgetIo)?;
        let name = self.path.file_name().ok_or(Failure::BudgetIo)?;
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::BudgetIo)?
            .as_nanos();
        let tmp = parent.join(format!(
            ".{}.{}.{}.tmp",
            name.to_string_lossy(),
            std::process::id(),
            suffix
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|_| Failure::BudgetIo)?;
        let result = (|| {
            let data = serde_json::to_vec(&state).map_err(|_| Failure::BudgetIo)?;
            file.write_all(&data).map_err(|_| Failure::BudgetIo)?;
            file.sync_all().map_err(|_| Failure::BudgetIo)?;
            fs::rename(&tmp, &self.path).map_err(|_| Failure::BudgetIo)?;
            fs::File::open(parent)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| Failure::BudgetIo)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }
}

fn run_request<C: Client>(
    client: &mut C,
    budget: &Budget,
    context: Context,
    duration_ms: u32,
    now: SystemTime,
) -> Result<String, Failure> {
    // 生成の可能性がある試行を、最初のHTTPより前に耐久化する。
    budget.reserve(now)?;
    let count = client.count_tokens(context)?;
    if count > MAX_INPUT_TOKENS {
        return Err(Failure::InputTokens);
    }
    let text = client.generate(context)?;
    validate_text(&text, duration_ms)?;
    Ok(text)
}

struct Job {
    context: Context,
    duration_ms: u32,
}

/// UART loopがpollする、容量1のworker。thread終了時もローカル文へ戻せる。
pub struct CloudSpeech {
    jobs: SyncSender<Job>,
    results: Receiver<Result<String, Failure>>,
    in_flight: bool,
    counters: Counters,
}

impl CloudSpeech {
    /// workerを起動する。予算fileのpathはsystemdの永続state directory内に置く。
    ///
    /// # Errors
    ///
    /// thread起動に失敗した場合。
    pub fn start<C: Client>(client: C, budget_path: PathBuf) -> Result<Self, Failure> {
        let (jobs, recv) = mpsc::sync_channel::<Job>(1);
        let (send, results) = mpsc::sync_channel::<Result<String, Failure>>(1);
        std::thread::Builder::new()
            .name("cloud-speech".to_owned())
            .spawn(move || {
                let mut client = client;
                let budget = Budget { path: budget_path };
                while let Ok(job) = recv.recv() {
                    let result = run_request(
                        &mut client,
                        &budget,
                        job.context,
                        job.duration_ms,
                        SystemTime::now(),
                    );
                    if send.send(result).is_err() {
                        break;
                    }
                }
            })
            .map_err(|_| Failure::Worker)?;
        Ok(Self {
            jobs,
            results,
            in_flight: false,
            counters: Counters::default(),
        })
    }

    /// 非同期に試行を予約する。送信queueが壊れていれば失敗として扱う。
    ///
    /// # Errors
    ///
    /// workerへ渡せない場合。
    pub fn submit(&mut self, context: Context, duration_ms: u32) -> Result<(), Failure> {
        if self.in_flight {
            return Err(Failure::Worker);
        }
        match self.jobs.try_send(Job {
            context,
            duration_ms,
        }) {
            Ok(()) => {
                self.in_flight = true;
                Ok(())
            }
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => Err(Failure::Worker),
        }
    }

    /// UARTを止めずに結果を読む。完了していなければ`None`。
    pub fn poll(&mut self) -> Option<Result<String, Failure>> {
        if !self.in_flight {
            return None;
        }
        let result = match self.results.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(Failure::Worker),
        };
        self.in_flight = false;
        self.counters.note(&result);
        Some(result)
    }

    /// workerへの配送失敗も分類して数える。
    pub fn note_failure(&mut self, kind: Failure) {
        self.counters.failures[kind.index()] += 1;
    }

    /// 生成中か。
    #[must_use]
    pub const fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// 安全な失敗分類ごとの累計。
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fake {
        calls: Arc<Mutex<Vec<&'static str>>>,
        count: Result<u32, Failure>,
        text: Result<String, Failure>,
    }

    impl Client for Fake {
        fn count_tokens(&mut self, _: Context) -> Result<u32, Failure> {
            self.calls.lock().expect("lock").push("count");
            self.count
        }
        fn generate(&mut self, _: Context) -> Result<String, Failure> {
            self.calls.lock().expect("lock").push("generate");
            self.text.clone()
        }
    }

    fn scenario(
        count: Result<u32, Failure>,
        text: Result<&str, Failure>,
    ) -> (Fake, Arc<Mutex<Vec<&'static str>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Fake {
                calls: Arc::clone(&calls),
                count,
                text: text.map(str::to_owned),
            },
            calls,
        )
    }

    fn context() -> Context {
        Context {
            expression: Expression::Neutral,
            time_band: Some(2),
        }
    }

    fn clock(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn run_case(
        count: Result<u32, Failure>,
        text: Result<&str, Failure>,
    ) -> (Result<String, Failure>, Vec<&'static str>) {
        let dir = std::env::temp_dir().join(format!(
            "deskcat-cloud-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&dir).expect("test dir");
        let budget = Budget {
            path: dir.join("budget.json"),
        };
        let _ = fs::remove_file(&budget.path);
        let (mut fake, calls) = scenario(count, text);
        let result = run_request(&mut fake, &budget, context(), 1000, clock(1_800_000_000));
        let observed = calls.lock().expect("lock").clone();
        fs::remove_dir_all(dir).expect("cleanup");
        (result, observed)
    }

    #[test]
    fn success_and_failure_classes_never_call_real_api() {
        assert_eq!(
            run_case(Ok(42), Ok("こんにちは。")),
            (Ok("こんにちは。".to_owned()), vec!["count", "generate"])
        );
        assert_eq!(
            run_case(Err(Failure::Timeout), Ok("こんにちは。")),
            (Err(Failure::Timeout), vec!["count"])
        );
        assert_eq!(
            run_case(Ok(42), Err(Failure::Http)),
            (Err(Failure::Http), vec!["count", "generate"])
        );
        assert_eq!(
            run_case(Ok(513), Ok("こんにちは。")),
            (Err(Failure::InputTokens), vec!["count"])
        );
        assert_eq!(
            run_case(Ok(42), Ok("😺")),
            (Err(Failure::Display), vec!["count", "generate"])
        );
        assert_eq!(
            run_case(Ok(42), Ok("一\n行")),
            (Err(Failure::Display), vec!["count", "generate"])
        );
    }

    #[test]
    fn reservation_survives_reopen_and_blocks_early_retry() {
        let dir = std::env::temp_dir().join(format!("deskcat-budget-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test dir");
        let budget = Budget {
            path: dir.join("budget.json"),
        };
        let _ = fs::remove_file(&budget.path);
        budget.reserve(clock(1_800_000_000)).expect("first");
        assert_eq!(
            Budget {
                path: budget.path.clone()
            }
            .reserve(clock(1_800_000_059)),
            Err(Failure::Budget)
        );
        budget.reserve(clock(1_800_000_060)).expect("after minute");
        let state: BudgetState =
            serde_json::from_slice(&fs::read(&budget.path).expect("read")).expect("json");
        assert_eq!(state.attempts, 2);
        assert_eq!(state.estimated_nano_usd, 2 * MAX_ESTIMATE_NANO_USD);
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn corrupted_budget_fails_closed() {
        let dir = std::env::temp_dir().join(format!(
            "deskcat-budget-corrupt-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test dir");
        let budget = Budget {
            path: dir.join("budget.json"),
        };
        fs::write(&budget.path, b"bad").expect("write");
        assert_eq!(budget.reserve(clock(1_800_000_000)), Err(Failure::BudgetIo));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn daily_limit_blocks_before_any_http_call() {
        let dir =
            std::env::temp_dir().join(format!("deskcat-budget-full-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test dir");
        let budget = Budget {
            path: dir.join("budget.json"),
        };
        let secs = 1_800_000_000;
        let state = BudgetState {
            day: secs / 86_400,
            attempts: MAX_DAILY_ATTEMPTS,
            estimated_nano_usd: u64::from(MAX_DAILY_ATTEMPTS) * MAX_ESTIMATE_NANO_USD,
            last_attempt_secs: secs - MIN_INTERVAL_SECS,
        };
        fs::write(&budget.path, serde_json::to_vec(&state).expect("serialize")).expect("write");
        let (mut fake, calls) = scenario(Ok(42), Ok("こんにちは。"));
        assert_eq!(
            run_request(&mut fake, &budget, context(), 1000, clock(secs)),
            Err(Failure::Budget)
        );
        assert!(calls.lock().expect("lock").is_empty());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn credential_is_absent_or_safe_and_http_config_is_restricted() {
        let dir =
            std::env::temp_dir().join(format!("deskcat-credential-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("test dir");
        assert!(matches!(
            ClaudeClient::from_credential_dir(&dir),
            Err(Failure::Credential)
        ));
        fs::write(dir.join("anthropic-api-key"), b"fake-test-only-key\n").expect("write");
        let client = ClaudeClient::from_credential_dir(&dir).expect("fake credential");
        let config = client.agent.config();
        assert!(config.https_only());
        assert!(config.proxy().is_none());
        assert_eq!(config.max_redirects(), 0);
        assert_eq!(config.tls_config().provider(), TlsProvider::NativeTls);
        assert!(matches!(
            config.tls_config().root_certs(),
            RootCerts::PlatformVerifier
        ));
        assert_eq!(client.key.0, "fake-test-only-key");
        assert!(!format!("{:?}", Failure::Credential).contains(&client.key.0));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn a_body_timeout_is_not_misclassified_as_invalid_json() {
        assert_eq!(
            classify_body(&ureq::Error::Timeout(ureq::Timeout::RecvBody)),
            Failure::Timeout
        );
        assert_eq!(
            classify_body(&ureq::Error::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionReset
            ))),
            Failure::Network
        );
    }
}
