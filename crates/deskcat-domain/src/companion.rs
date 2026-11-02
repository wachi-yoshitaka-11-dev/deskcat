//! 温湿度・気圧・揺れ・時刻から、吹き出し・表情・首振りの要求を決める（Issue #685の前半）。
//!
//! 接触event（[`crate::Mind`]）とは別の入力を扱う。表情の名前は[`Face`]で持ち、protocolの
//! expression名（Protocol §5.2は`neutral`・`happy`・`surprised`だけ）への対応は、描画（#487、#21）と
//! 一緒に決める。**この型を`set_expression`へ送る経路は、まだ無い。**
//!
//! 時計を読まない。単調時刻（[`Millis`]）と、時刻の分（[`LocalMinutes`]）は呼び出し側が渡す。
//!
//! # 振る舞い
//!
//! - **環境**: 暑い・寒い・乾燥・気圧低下を、しきい値を越えたときに1回だけ出す。戻る側のしきい値を
//!   別に持つ（hysteresis）。しきい値の付近で値が揺れても、出し直さない
//! - **揺れ**: [`Companion::on_shake`]で目を回す表情と首振りを出す。**首振りを要求してから
//!   `motion_settle_ms`の間の揺れは、自分の振動として無視する。**首振りの完了eventは無い
//!   （Protocol §4.7は未導入）ため、起点は要求した時刻である。首振りが`motion_settle_ms`より
//!   長く続くと、その振動を揺れとして数えうる。首振りの長さは#19・#20で決まるまで`TBD`である
//! - **休憩**: 起きている間、`rest_interval_ms`ごとに吹き出しと首振りを出す
//! - **寝起き**: 夜の時間帯は寝顔にして、吹き出しも首振りも出さない。朝の時間帯で起きる
//!
//! # 値
//!
//! [`Config::MVP`]の値はすべて一般値（慣行として使われる値）の初期値であり、安全要件の5項目
//! （docs/governance/hardware-safety-policy.md）に効かない。実機で観察して変える。首の角度・速度・
//! 回数は持たない。[`NeckRequest`]は意図だけで、安全制限の範囲に収めるのはESP32である。

use core::cmp::Ordering;

use crate::Millis;

/// 1日の中の時刻（0時からの分）。`0..1440`。呼び出し側がPiの時計から作る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalMinutes(u16);

impl LocalMinutes {
    /// 1日の分数。
    pub const PER_DAY: u16 = 24 * 60;

    /// 時・分から作る。範囲外は`None`。
    #[must_use]
    pub const fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour < 24 && minute < 60 {
            Some(Self(hour as u16 * 60 + minute as u16))
        } else {
            None
        }
    }

    const fn get(self) -> u16 {
        self.0
    }
}

/// 出す表情。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// 既定の表情。
    Neutral,
    /// 暑い。
    Hot,
    /// 寒い。
    Cold,
    /// 乾燥や気圧低下に困る。
    Worried,
    /// 揺らされて目を回す。
    Dizzy,
    /// 寝顔。
    Sleeping,
}

/// 吹き出しの文言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bubble {
    /// 暑い。
    Hot,
    /// 寒い。
    Cold,
    /// 乾燥。
    Dry,
    /// 気圧低下。
    LowPressure,
    /// 休憩のお知らせ。
    TakeBreak,
    /// 寝る。
    GoodNight,
    /// 起きる。
    GoodMorning,
}

impl Bubble {
    /// 表示する文。`show_text`の上限（Protocol §5.4）に収まる短い文である。
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Hot => "暑いにゃ",
            Self::Cold => "寒いにゃ",
            Self::Dry => "乾燥してるね",
            Self::LowPressure => "雨が来そう",
            Self::TakeBreak => "ひと休みしよう",
            Self::GoodNight => "おやすみ",
            Self::GoodMorning => "おはよう",
        }
    }
}

/// 首の動きの要求。角度・速度・回数を持たない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeckRequest {
    /// 首を振る。
    Sway,
}

/// 1回の入力に対する出力。`None`の項目は何も出さない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Output {
    /// 新しく出す表情。
    pub face: Option<Face>,
    /// 出す吹き出し。
    pub bubble: Option<Bubble>,
    /// 首振りの要求。**寝ている間は常に`None`である。**
    pub neck: Option<NeckRequest>,
}

impl Output {
    /// 何も出さない。
    pub const NONE: Self = Self {
        face: None,
        bubble: None,
        neck: None,
    };
}

/// センサ値1回分。BME280の補正済みの値を、呼び出し側が渡す。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ambient {
    /// 気温（℃）。
    pub temperature_c: f32,
    /// 相対湿度（%）。
    pub humidity_pct: f32,
    /// 気圧（hPa）。
    pub pressure_hpa: f32,
}

/// 値。ハードウェアの値ではない。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// 暑いと判断する気温（℃）以上。
    pub hot_on_c: f32,
    /// 暑い状態を終える気温（℃）未満。`hot_on_c`より低くする。
    pub hot_off_c: f32,
    /// 寒いと判断する気温（℃）以下。
    pub cold_on_c: f32,
    /// 寒い状態を終える気温（℃）超。`cold_on_c`より高くする。
    pub cold_off_c: f32,
    /// 乾燥と判断する湿度（%）以下。
    pub dry_on_pct: f32,
    /// 乾燥を終える湿度（%）超。
    pub dry_off_pct: f32,
    /// 気圧低下と判断する下がり幅（hPa）以上。
    pub pressure_drop_on_hpa: f32,
    /// 気圧低下を終える下がり幅（hPa）未満。
    pub pressure_drop_off_hpa: f32,
    /// 気圧の履歴を1点足す最短の間隔（ミリ秒）。
    pub pressure_sample_ms: u64,
    /// 環境の吹き出しを出す最短の間隔（ミリ秒）。複数が同時に当たったとき、順に出す。
    pub bubble_gap_ms: u64,
    /// 環境の表情を保つ時間（ミリ秒）。
    pub ambient_hold_ms: u64,
    /// 目を回す表情を保つ時間（ミリ秒）。
    pub dizzy_hold_ms: u64,
    /// 首振りを要求してから、揺れを無視する時間（ミリ秒）。完了eventが無いため、要求した時刻から数える。
    /// 首振りがこの時間より長く続くと、その振動を揺れとして数えうる（長さは#19・#20まで`TBD`）。
    pub motion_settle_ms: u64,
    /// 首振りを出す前提の、揺れを受け付ける最短の間隔（ミリ秒）。
    pub shake_cooldown_ms: u64,
    /// 休憩のお知らせの間隔（ミリ秒）。
    pub rest_interval_ms: u64,
    /// 寝る時刻。
    pub sleep_at: LocalMinutes,
    /// 起きる時刻。`sleep_at`より早い時刻なら、日をまたぐ夜と読む。
    pub wake_at: LocalMinutes,
}

impl Config {
    /// MVPの初期値。**一般値で始める調整前の値であり、実測の根拠は無い。**
    pub const MVP: Self = Self {
        hot_on_c: 30.0,
        hot_off_c: 28.0,
        cold_on_c: 15.0,
        cold_off_c: 17.0,
        dry_on_pct: 30.0,
        dry_off_pct: 35.0,
        pressure_drop_on_hpa: 4.0,
        pressure_drop_off_hpa: 2.0,
        pressure_sample_ms: 10 * 60 * 1000,
        bubble_gap_ms: 30 * 1000,
        ambient_hold_ms: 5 * 1000,
        dizzy_hold_ms: 3 * 1000,
        motion_settle_ms: 2 * 1000,
        shake_cooldown_ms: 5 * 1000,
        rest_interval_ms: 25 * 60 * 1000,
        sleep_at: LocalMinutes(22 * 60),
        wake_at: LocalMinutes(7 * 60),
    };
}

/// 気圧の履歴の点数。`pressure_sample_ms`の10分で3時間になる。
const PRESSURE_HISTORY: usize = 18;

/// 計数。捨てた入力を黙って捨てずに数える。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    /// 有限でない、または範囲外のセンサ値の数。
    pub invalid_samples: u64,
    /// 首振りを要求してから`motion_settle_ms`の間、または寝ている間のために無視した揺れの数。
    pub ignored_shakes: u64,
    /// 前回より前の時刻のために処理しなかった入力の数。
    pub clock_regressions: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Condition {
    active: bool,
    pending: bool,
}

impl Condition {
    /// 入る側のしきい値を越えたら有効にして、出す予約を立てる。戻る側を越えたら外す。
    fn update(&mut self, enter: bool, leave: bool) {
        if !self.active && enter {
            self.active = true;
            self.pending = true;
        } else if self.active && leave {
            self.active = false;
            self.pending = false;
        }
    }
}

/// 状態機械。
#[derive(Debug, Clone)]
pub struct Companion {
    config: Config,
    hot: Condition,
    cold: Condition,
    dry: Condition,
    low_pressure: Condition,
    pressure: [f32; PRESSURE_HISTORY],
    pressure_len: usize,
    last_pressure_at: Option<Millis>,
    last_bubble_at: Option<Millis>,
    face: Face,
    face_since: Millis,
    asleep: bool,
    last_neck_at: Option<Millis>,
    last_shake_at: Option<Millis>,
    rest_since: Millis,
    last_now: Millis,
    counters: Counters,
}

impl Companion {
    /// 起動する。夜の時間帯なら、最初の[`Companion::tick`]で寝顔を出す。
    #[must_use]
    pub const fn new(config: Config, now: Millis) -> Self {
        Self {
            config,
            hot: Condition {
                active: false,
                pending: false,
            },
            cold: Condition {
                active: false,
                pending: false,
            },
            dry: Condition {
                active: false,
                pending: false,
            },
            low_pressure: Condition {
                active: false,
                pending: false,
            },
            pressure: [0.0; PRESSURE_HISTORY],
            pressure_len: 0,
            last_pressure_at: None,
            last_bubble_at: None,
            face: Face::Neutral,
            face_since: now,
            asleep: false,
            last_neck_at: None,
            last_shake_at: None,
            rest_since: now,
            last_now: now,
            counters: Counters {
                invalid_samples: 0,
                ignored_shakes: 0,
                clock_regressions: 0,
            },
        }
    }

    /// 今の表情。
    #[must_use]
    pub const fn face(&self) -> Face {
        self.face
    }

    /// 寝ているか。
    #[must_use]
    pub const fn is_asleep(&self) -> bool {
        self.asleep
    }

    /// 計数。
    #[must_use]
    pub const fn counters(&self) -> &Counters {
        &self.counters
    }

    /// センサ値を受け取る。しきい値を越えた状態を、1回だけ吹き出しと表情にする。
    pub fn on_ambient(&mut self, sample: Ambient, now: Millis) -> Output {
        if !self.advance_clock(now) {
            return Output::NONE;
        }
        if !plausible(sample) {
            self.counters.invalid_samples = self.counters.invalid_samples.saturating_add(1);
            return Output::NONE;
        }
        let c = self.config;
        self.hot.update(
            sample.temperature_c >= c.hot_on_c,
            sample.temperature_c < c.hot_off_c,
        );
        self.cold.update(
            sample.temperature_c <= c.cold_on_c,
            sample.temperature_c > c.cold_off_c,
        );
        self.dry.update(
            sample.humidity_pct <= c.dry_on_pct,
            sample.humidity_pct > c.dry_off_pct,
        );
        let drop = self.pressure_drop(sample.pressure_hpa);
        self.low_pressure.update(
            drop >= c.pressure_drop_on_hpa,
            drop < c.pressure_drop_off_hpa,
        );
        self.record_pressure(sample.pressure_hpa, now);
        self.emit_ambient(now)
    }

    /// 揺れを受け取る。目を回す表情と首振りを出す。
    pub fn on_shake(&mut self, now: Millis) -> Output {
        if !self.advance_clock(now) {
            return Output::NONE;
        }
        let own_motion = self
            .last_neck_at
            .is_some_and(|t| now.0 - t.0 < self.config.motion_settle_ms);
        if self.asleep || own_motion {
            self.counters.ignored_shakes = self.counters.ignored_shakes.saturating_add(1);
            return Output::NONE;
        }
        if self
            .last_shake_at
            .is_some_and(|t| now.0 - t.0 < self.config.shake_cooldown_ms)
        {
            return Output::NONE;
        }
        self.last_shake_at = Some(now);
        self.last_neck_at = Some(now);
        Output {
            face: self.set_face(Face::Dizzy, now),
            bubble: None,
            neck: Some(NeckRequest::Sway),
        }
    }

    /// 時間を進める。寝起き、休憩、表情の戻り、環境の吹き出しの順送りを扱う。
    pub fn tick(&mut self, now: Millis, local: LocalMinutes) -> Output {
        if !self.advance_clock(now) {
            return Output::NONE;
        }
        let night = in_sleep_window(local, self.config.sleep_at, self.config.wake_at);
        if night && !self.asleep {
            self.asleep = true;
            return Output {
                face: self.set_face(Face::Sleeping, now),
                bubble: self.say(Bubble::GoodNight, now),
                neck: None,
            };
        }
        if !night && self.asleep {
            self.asleep = false;
            self.rest_since = now;
            return Output {
                face: self.set_face(Face::Neutral, now),
                bubble: self.say(Bubble::GoodMorning, now),
                neck: None,
            };
        }
        if self.asleep {
            return Output::NONE;
        }

        let mut out = Output {
            face: self.expire_face(now),
            ..Output::NONE
        };
        if now.0 - self.rest_since.0 >= self.config.rest_interval_ms {
            self.rest_since = now;
            out.bubble = self.say(Bubble::TakeBreak, now);
            out.neck = Some(NeckRequest::Sway);
            self.last_neck_at = Some(now);
            return out;
        }
        let ambient = self.emit_ambient(now);
        out.face = ambient.face.or(out.face);
        out.bubble = ambient.bubble;
        out
    }

    fn emit_ambient(&mut self, now: Millis) -> Output {
        if self.asleep {
            return Output::NONE;
        }
        if self
            .last_bubble_at
            .is_some_and(|t| now.0 - t.0 < self.config.bubble_gap_ms)
        {
            return Output::NONE;
        }
        let (bubble, face) = if self.hot.pending {
            self.hot.pending = false;
            (Bubble::Hot, Face::Hot)
        } else if self.cold.pending {
            self.cold.pending = false;
            (Bubble::Cold, Face::Cold)
        } else if self.dry.pending {
            self.dry.pending = false;
            (Bubble::Dry, Face::Worried)
        } else if self.low_pressure.pending {
            self.low_pressure.pending = false;
            (Bubble::LowPressure, Face::Worried)
        } else {
            return Output::NONE;
        };
        Output {
            face: self.set_face(face, now),
            bubble: self.say(bubble, now),
            neck: None,
        }
    }

    #[allow(clippy::unnecessary_wraps)] // 出力欄の型（`Option<Bubble>`）に合わせる。
    fn say(&mut self, bubble: Bubble, now: Millis) -> Option<Bubble> {
        self.last_bubble_at = Some(now);
        Some(bubble)
    }

    /// 表情が変わるときだけ`Some`を返す。
    fn set_face(&mut self, face: Face, now: Millis) -> Option<Face> {
        self.face_since = now;
        if self.face == face {
            return None;
        }
        self.face = face;
        Some(face)
    }

    fn expire_face(&mut self, now: Millis) -> Option<Face> {
        let hold = match self.face {
            Face::Hot | Face::Cold | Face::Worried => self.config.ambient_hold_ms,
            Face::Dizzy => self.config.dizzy_hold_ms,
            Face::Neutral | Face::Sleeping => return None,
        };
        if now.0 - self.face_since.0 >= hold {
            return self.set_face(Face::Neutral, now);
        }
        None
    }

    /// 履歴の最大から今の値を引いた下がり幅。履歴が無ければ0。
    fn pressure_drop(&self, now_hpa: f32) -> f32 {
        self.pressure[..self.pressure_len]
            .iter()
            .copied()
            .fold(now_hpa, f32::max)
            - now_hpa
    }

    fn record_pressure(&mut self, hpa: f32, now: Millis) {
        if self
            .last_pressure_at
            .is_some_and(|t| now.0 - t.0 < self.config.pressure_sample_ms)
        {
            return;
        }
        self.last_pressure_at = Some(now);
        if self.pressure_len == PRESSURE_HISTORY {
            self.pressure.copy_within(1.., 0);
            self.pressure_len -= 1;
        }
        self.pressure[self.pressure_len] = hpa;
        self.pressure_len += 1;
    }

    /// 時刻が戻っていれば数えて`false`を返す。同じ時刻は受け付ける。
    fn advance_clock(&mut self, now: Millis) -> bool {
        if now < self.last_now {
            self.counters.clock_regressions = self.counters.clock_regressions.saturating_add(1);
            return false;
        }
        self.last_now = now;
        true
    }
}

/// 有限で、BME280の測定rangeに収まる値か。センサの異常値を判断へ入れない粗い検査である。
/// 範囲は`docs/hardware/sensor-datasheet-notes.md`の`測定range`行（Bosch datasheet Rev 1.24から記録）による。
fn plausible(s: Ambient) -> bool {
    s.temperature_c.is_finite()
        && (-40.0..=85.0).contains(&s.temperature_c)
        && s.humidity_pct.is_finite()
        && (0.0..=100.0).contains(&s.humidity_pct)
        && s.pressure_hpa.is_finite()
        && (300.0..=1100.0).contains(&s.pressure_hpa)
}

fn in_sleep_window(now: LocalMinutes, sleep_at: LocalMinutes, wake_at: LocalMinutes) -> bool {
    let (n, s, w) = (now.get(), sleep_at.get(), wake_at.get());
    match s.cmp(&w) {
        Ordering::Equal => false,
        Ordering::Less => (s..w).contains(&n),
        Ordering::Greater => n >= s || n < w,
    }
}
