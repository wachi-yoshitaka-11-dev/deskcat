//! 感情の状態と遷移。

/// 単調な経過時間（ミリ秒）。呼び出し側が渡す。
///
/// 起点は呼び出し側が決める。`deskcatd`はPi側の単調時計から作り、ESP32の`ts_ms`は使わない
/// （ESP32の再起動で0へ戻るため）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Millis(pub u64);

/// 接触event。payload（強さ、加速度の大きさ、時間）は受け取らない。
///
/// `head_touched`の`strength`は`PROTO-TBD-009`、`tapped`の`magnitude_g`はProtocol §4.3で
/// 意味が未確定である。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContactEvent {
    /// 頭を撫でられた（Protocol §4.2 `head_touched`）。
    HeadTouched,
    /// 軽く叩かれた（Protocol §4.3 `tapped`）。
    Tapped,
    /// 持ち上げられた（Protocol §4.4 `lifted`）。§4.4でoptionalのため、`deskcatd`が渡すのは
    /// accelerometer classifierの検証の後である。
    Lifted,
}

impl ContactEvent {
    const COUNT: usize = 3;

    const fn index(self) -> usize {
        match self {
            Self::HeadTouched => 0,
            Self::Tapped => 1,
            Self::Lifted => 2,
        }
    }
}

/// 表情。Protocol §5.2の初期のexpression名と一対一に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Expression {
    /// 既定の表情。起動時と、保つ時間の経過後。
    Neutral,
    /// 喜ぶ。
    Happy,
    /// 驚く。
    Surprised,
}

/// 首の動きの意図。角度、速度、回数を持たない。
///
/// motion名と範囲は`PROTO-TBD-008`で未確定である。wireの`play_motion`へ変換するのは
/// `deskcatd`であり、PROTO-TBD-008が決まるまで送らない（Issue #491の段階2への申し送り）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionIntent {
    /// うなずく。
    Nod,
}

/// 1回の入力に対する判断。
///
/// `expression`は**表情が変わったときだけ**`Some`になる。同じ表情を送り直さない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reaction {
    /// 新しく出す表情。変わらないときは`None`。
    pub expression: Option<Expression>,
    /// 首の動きの意図。無いときは`None`。
    pub motion: Option<MotionIntent>,
}

impl Reaction {
    /// 何もしない。
    pub const NONE: Self = Self {
        expression: None,
        motion: None,
    };
}

/// 時間に関する値。ハードウェアの値ではなく、表示と反応の振る舞いを決める値である。
///
/// **`same_kind_cooldown_ms`は[`MotionIntent::Nod`]を出す頻度も決める。**撫でが続くと、
/// 最短でこの間隔ごとに`Nod`を1回出す。それでも現時点で安全要件の5項目
/// （docs/governance/hardware-safety-policy.md）に効かないのは、`deskcatd`が
/// `PROTO-TBD-008`の決定まで`play_motion`を送らないと決めたためである
/// （<https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491#issuecomment-5945349700>）。
/// **`play_motion`を送り始める時点で、この判定をやり直す。**
/// それ以外の点では、値が外れても、表情が不自然な長さになるか、反応しすぎる・反応を
/// 逃すだけである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// `Happy`を保つ時間（ミリ秒）。経過したら`Neutral`へ戻る。
    pub happy_hold_ms: u64,
    /// `Surprised`を保つ時間（ミリ秒）。経過したら`Neutral`へ戻る。
    pub surprised_hold_ms: u64,
    /// 同じ種類のeventを抑える間隔（ミリ秒）。直前に受け付けた同じ種類のeventから
    /// この時間が経っていなければ抑える。ちょうど経ったものは受け付ける。
    ///
    /// **サーボの受理上限の代わりにしない。**単位時間あたりに受理するmotion command数は
    /// firmwareが強制する（docs/governance/hardware-safety-policy.md §6）。値は`HW-TBD-020`で
    /// 未確定である（docs/hardware/servo-safety-limits.md）。
    pub same_kind_cooldown_ms: u64,
}

impl Timing {
    /// MVPの初期値。**慣行や実測に基づく値ではない。**実機で撫で・tapを試して観察し、
    /// 決める（Issue #491の段階3）。
    pub const MVP: Self = Self {
        happy_hold_ms: 3000,
        surprised_hold_ms: 2000,
        same_kind_cooldown_ms: 1000,
    };

    const fn hold_ms(&self, expression: Expression) -> Option<u64> {
        match expression {
            Expression::Neutral => None,
            Expression::Happy => Some(self.happy_hold_ms),
            Expression::Surprised => Some(self.surprised_hold_ms),
        }
    }
}

/// 1種類のeventの計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventCounts {
    /// 受け付けた数。
    pub accepted: u64,
    /// 同じ種類が短い間隔で続いたために抑えた数。
    pub suppressed: u64,
}

/// 計数。抑えた入力も黙って捨てずに数える。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    events: [EventCounts; ContactEvent::COUNT],
    /// 前回より前の時刻が渡されたために処理しなかった入力の数。呼び出し側の誤りを示す。
    pub clock_regressions: u64,
}

impl Counters {
    /// 指定した種類のeventの計数。
    #[must_use]
    pub const fn event(&self, kind: ContactEvent) -> EventCounts {
        self.events[kind.index()]
    }
}

/// 感情の状態機械。
#[derive(Debug, Clone)]
pub struct Mind {
    timing: Timing,
    expression: Expression,
    expression_since: Millis,
    last_accepted: [Option<Millis>; ContactEvent::COUNT],
    last_now: Millis,
    counters: Counters,
}

impl Mind {
    /// `Neutral`から始める。`now`は以後に渡す時刻の起点になる。
    #[must_use]
    pub const fn new(timing: Timing, now: Millis) -> Self {
        Self {
            timing,
            expression: Expression::Neutral,
            expression_since: now,
            last_accepted: [None; ContactEvent::COUNT],
            last_now: now,
            counters: Counters {
                events: [EventCounts {
                    accepted: 0,
                    suppressed: 0,
                }; ContactEvent::COUNT],
                clock_regressions: 0,
            },
        }
    }

    /// 今の表情。
    #[must_use]
    pub const fn expression(&self) -> Expression {
        self.expression
    }

    /// 計数。
    #[must_use]
    pub const fn counters(&self) -> &Counters {
        &self.counters
    }

    /// 接触eventを受け取り、判断を返す。
    ///
    /// 保つ時間が既に過ぎていれば、先に`Neutral`へ戻してからeventを処理する。
    /// eventを抑えた場合も、その戻りは返す。
    pub fn on_event(&mut self, event: ContactEvent, now: Millis) -> Reaction {
        if !self.advance_clock(now) {
            return Reaction::NONE;
        }
        let before = self.expression;
        self.expire(now);

        let i = event.index();
        let counts = &mut self.counters.events[i];
        let within_cooldown =
            self.last_accepted[i].is_some_and(|t| now.0 - t.0 < self.timing.same_kind_cooldown_ms);
        if within_cooldown {
            counts.suppressed = counts.suppressed.saturating_add(1);
            return self.changed_since(before, None);
        }
        counts.accepted = counts.accepted.saturating_add(1);
        self.last_accepted[i] = Some(now);

        let (expression, motion) = match event {
            ContactEvent::HeadTouched => (Expression::Happy, Some(MotionIntent::Nod)),
            ContactEvent::Tapped | ContactEvent::Lifted => (Expression::Surprised, None),
        };
        self.expression = expression;
        self.expression_since = now;
        self.changed_since(before, motion)
    }

    /// 時間を進める。保つ時間が過ぎていれば`Neutral`へ戻し、その判断を返す。
    pub fn tick(&mut self, now: Millis) -> Reaction {
        if !self.advance_clock(now) {
            return Reaction::NONE;
        }
        let before = self.expression;
        self.expire(now);
        self.changed_since(before, None)
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

    fn expire(&mut self, now: Millis) {
        if let Some(hold) = self.timing.hold_ms(self.expression)
            && now.0 - self.expression_since.0 >= hold
        {
            self.expression = Expression::Neutral;
            self.expression_since = now;
        }
    }

    fn changed_since(&self, before: Expression, motion: Option<MotionIntent>) -> Reaction {
        Reaction {
            expression: (self.expression != before).then_some(self.expression),
            motion,
        }
    }
}
