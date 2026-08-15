//! 途中で切れたJSONのprefixから、envelope identityを復元する（§8手順6）。
//!
//! 行長上限を超えた行は破棄するが、破棄の前に`(sid, id)`の復元を試みる。復元できた場合だけ、
//! §7は`line_too_long`を相手へ返してよい。復元できなければ`oversize_lines`の計数だけを行う。
//! §4.1の`boot`は、この復元ができないと終端応答（`status: rejected`のACK）を組み立てられず、
//! 送信側がrecovery budgetを無応答のまま使い切る。
//!
//! # best-effortであることの意味
//!
//! 入力は**途中で切れたJSON**であり、valid JSONではない。したがってこの走査は
//! 「復元できたら`Some`、少しでも怪しければ`None`」に倒す。**誤ったidentityを返すことは、
//! 復元に失敗することより悪い。**別の要求へ相関したACKを送り、#12のduplicate履歴を汚すためである。
//!
//! 具体的には次を`None`にする。
//!
//! - `sid`と`id`のどちらかでも復元できなかった
//! - 値がprefixの末尾で切れている（`"sid":903`が本当は`9031`だったかもしれない）
//! - 同じkeyがprefix内に2回現れた（`serde_json`はlast-winsだが、prefixから見えている
//!   最後が行全体での最後とは限らない）
//! - top-levelのobjectとして読み進められない綴り
//!
//! # 拾う範囲
//!
//! **depth 1のkeyだけ**を見る。`payload`のようなnested valueは丸ごと読み飛ばすため、
//! `payload`の中の`id`を拾うことはない。keyは完全一致で比較するため、`"sid"`を`"id"`と
//! 取り違えることも起こらない。escape sequenceを含む綴り（`s`を`\u0073`と書いたもの）は
//! 既知keyとして扱わず、そのpairを読み飛ばす。

use crate::message::Message;

/// 上限付きprefixから復元できたenvelope。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixEnvelope {
    /// 送信側のsession ID。
    pub sid: u32,
    /// 同一session内のmessage ID。
    pub id: u32,
    /// protocol major version。prefixへ収まらなければ`None`。
    pub v: Option<u16>,
    /// 既知のmessage type。未知またはprefixへ収まらなければ`None`。
    ///
    /// §7が`boot`だけ`line_too_long`の返し方を変えるため、typeが要る。
    pub type_name: Option<&'static str>,
}

/// 走査を打ち切る理由。
enum Stop {
    /// prefixの末尾に達した。ここまでに拾えたものは使える。
    Truncated,
    /// top-level objectとして読み進められない。拾ったものも捨てる。
    Malformed,
}

/// 途中で切れたJSON objectのprefixから`(sid, id)`の復元を試みる。
///
/// 復元できた場合だけ`Some`を返す。判断基準はmodule levelのdoc commentを参照する。
#[must_use]
pub fn recover_identity(prefix: &[u8]) -> Option<PrefixEnvelope> {
    let mut scan = Scanner {
        bytes: prefix,
        pos: 0,
    };

    scan.skip_whitespace();
    if scan.bump()? != b'{' {
        return None;
    }

    let mut sid = None;
    let mut id = None;
    let mut v = None;
    let mut type_name = None;
    let (mut saw_sid, mut saw_message_id, mut saw_version, mut saw_type) =
        (false, false, false, false);

    loop {
        scan.skip_whitespace();
        match scan.peek() {
            // prefixがここで切れた、またはobjectが閉じた。どちらもここまでで打ち切る。
            None | Some(b'}') => break,
            Some(b'"') => {}
            Some(_) => return None,
        }

        let key = match scan.read_string() {
            Ok(key) => key,
            Err(Stop::Truncated) => break,
            Err(Stop::Malformed) => return None,
        };

        scan.skip_whitespace();
        match scan.bump() {
            None => break,
            Some(b':') => {}
            Some(_) => return None,
        }
        scan.skip_whitespace();

        match key {
            b"sid" | b"id" | b"v" => {
                let value = match scan.read_integer() {
                    Ok(value) => value,
                    Err(Stop::Truncated) => break,
                    Err(Stop::Malformed) => return None,
                };
                match key {
                    b"sid" => {
                        if core::mem::replace(&mut saw_sid, true) {
                            return None;
                        }
                        sid = Some(u32::try_from(value).ok()?);
                    }
                    b"id" => {
                        if core::mem::replace(&mut saw_message_id, true) {
                            return None;
                        }
                        id = Some(u32::try_from(value).ok()?);
                    }
                    _ => {
                        if core::mem::replace(&mut saw_version, true) {
                            return None;
                        }
                        // `v`は診断用である。幅に収まらなくても復元自体は続ける。
                        v = u16::try_from(value).ok();
                    }
                }
            }
            b"type" => {
                if core::mem::replace(&mut saw_type, true) {
                    return None;
                }
                match scan.read_string() {
                    Ok(raw) => type_name = Message::known_type_name(raw),
                    Err(Stop::Truncated) => break,
                    Err(Stop::Malformed) => return None,
                }
            }
            _ => match scan.skip_value() {
                Ok(()) => {}
                Err(Stop::Truncated) => break,
                Err(Stop::Malformed) => return None,
            },
        }

        scan.skip_whitespace();
        match scan.peek() {
            None | Some(b'}') => break,
            Some(b',') => scan.pos += 1,
            Some(_) => return None,
        }
    }

    Some(PrefixEnvelope {
        sid: sid?,
        id: id?,
        v,
        type_name,
    })
}

/// prefixを前から読む位置つきcursor。allocationを行わない。
struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(byte) if byte.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    /// 開き`"`の位置から、閉じ`"`までの中身を**escapeを解釈せずに**返す。
    ///
    /// escapeを解釈しないのは、既知keyとの比較にも既知type名との比較にも不要だからである。
    /// `\"`を終端と取り違えないよう、backslashの次の1 byteだけは読み飛ばす。
    fn read_string(&mut self) -> Result<&'a [u8], Stop> {
        if self.bump() != Some(b'"') {
            return Err(Stop::Malformed);
        }
        let start = self.pos;
        loop {
            match self.bump().ok_or(Stop::Truncated)? {
                b'"' => return Ok(&self.bytes[start..self.pos - 1]),
                b'\\' => {
                    self.bump().ok_or(Stop::Truncated)?;
                }
                _ => {}
            }
        }
    }

    /// 非負整数を読む。**終端がprefix内にあることを要求する。**
    ///
    /// 末尾で切れた数値を採用すると、`"id":123`が本当は`1234`だった場合に別のmessageへ
    /// 相関したACKを送ってしまう。
    fn read_integer(&mut self) -> Result<u64, Stop> {
        let start = self.pos;
        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
            self.pos += 1;
        }
        let digits = &self.bytes[start..self.pos];

        // `-1`、`1.5`、`1e3`はenvelopeの宣言幅を満たさない。数値として読めても採らない。
        match self.peek() {
            None => return Err(Stop::Truncated),
            Some(byte) if byte == b',' || byte == b'}' || byte.is_ascii_whitespace() => {}
            Some(_) => return Err(Stop::Malformed),
        }
        if digits.is_empty() || (digits.len() > 1 && digits[0] == b'0') {
            return Err(Stop::Malformed);
        }

        let mut value: u64 = 0;
        for &digit in digits {
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(digit - b'0')))
                .ok_or(Stop::Malformed)?;
        }
        Ok(value)
    }

    /// 興味のないvalueを1つ読み飛ばす。
    fn skip_value(&mut self) -> Result<(), Stop> {
        match self.peek().ok_or(Stop::Truncated)? {
            b'"' => self.read_string().map(|_| ()),
            b'{' | b'[' => self.skip_container(),
            _ => {
                while let Some(byte) = self.peek() {
                    if byte == b',' || byte == b'}' || byte == b']' || byte.is_ascii_whitespace() {
                        return Ok(());
                    }
                    self.pos += 1;
                }
                Err(Stop::Truncated)
            }
        }
    }

    /// objectまたはarrayを、対応が取れるまで読み飛ばす。string中の括弧に釣られない。
    fn skip_container(&mut self) -> Result<(), Stop> {
        let mut depth = 0usize;
        loop {
            match self.peek().ok_or(Stop::Truncated)? {
                b'"' => {
                    self.read_string()?;
                }
                byte => {
                    self.pos += 1;
                    match byte {
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::recover_identity;

    fn identity(prefix: &str) -> Option<(u32, u32)> {
        recover_identity(prefix.as_bytes()).map(|envelope| (envelope.sid, envelope.id))
    }

    #[test]
    fn recovers_from_a_truncated_envelope() {
        let recovered = recover_identity(
            br#"{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"boot","payload":{"firmw"#,
        )
        .expect("sidとidはprefixへ収まっている");
        assert_eq!((recovered.sid, recovered.id), (90312, 907));
        assert_eq!(recovered.v, Some(1));
        assert_eq!(recovered.type_name, Some("boot"));
    }

    #[test]
    fn recovers_when_the_prefix_ends_right_after_id() {
        assert_eq!(identity(r#"{"v":1,"sid":7,"id":9,"#), Some((7, 9)));
    }

    /// 終端がprefix内に無い数値は採らない。`907`が本当は`9071`かもしれない。
    #[test]
    fn rejects_a_number_cut_at_the_prefix_boundary() {
        assert_eq!(identity(r#"{"sid":90312,"id":907"#), None);
    }

    #[test]
    fn requires_both_sid_and_id() {
        assert_eq!(identity(r#"{"v":1,"sid":90312,"ts_ms":1}"#), None);
        assert_eq!(identity(r#"{"v":1,"id":907,"ts_ms":1}"#), None);
        assert_eq!(identity("{"), None);
        assert_eq!(identity("{}"), None);
    }

    /// nested objectの中のkeyを拾わない。
    #[test]
    fn ignores_keys_inside_nested_values() {
        assert_eq!(identity(r#"{"payload":{"id":9,"sid":8},"ts_ms":1}"#), None);
        assert_eq!(
            identity(r#"{"payload":{"id":9},"sid":1,"id":2}"#),
            Some((1, 2))
        );
        assert_eq!(
            identity(r#"{"payload":[{"id":9},[{"sid":8}]],"sid":1,"id":2}"#),
            Some((1, 2))
        );
    }

    /// string valueの中の綴りをkeyと取り違えない。
    #[test]
    fn ignores_key_lookalikes_inside_strings() {
        assert_eq!(
            identity(r#"{"note":"\"id\":9,\"sid\":8","sid":1,"id":2}"#),
            Some((1, 2))
        );
        assert_eq!(
            identity(r#"{"note":"{\"id\":9}","sid":1,"id":2}"#),
            Some((1, 2))
        );
    }

    /// 同じkeyが2回現れたら、どちらが行全体でのlast-winsか分からない。
    #[test]
    fn rejects_duplicate_keys() {
        assert_eq!(identity(r#"{"sid":1,"id":2,"id":3}"#), None);
        assert_eq!(identity(r#"{"sid":1,"sid":4,"id":2}"#), None);
    }

    #[test]
    fn rejects_values_outside_the_envelope_widths() {
        assert_eq!(identity(r#"{"sid":-1,"id":2}"#), None);
        assert_eq!(identity(r#"{"sid":4294967296,"id":2}"#), None);
        assert_eq!(identity(r#"{"sid":1.5,"id":2}"#), None);
        assert_eq!(identity(r#"{"sid":1e3,"id":2}"#), None);
        assert_eq!(identity(r#"{"sid":01,"id":2}"#), None);
        assert_eq!(identity(r#"{"sid":true,"id":2}"#), None);
    }

    #[test]
    fn accepts_the_declared_maximums() {
        assert_eq!(
            identity(r#"{"sid":4294967295,"id":4294967295,"#),
            Some((u32::MAX, u32::MAX))
        );
        assert_eq!(identity(r#"{"sid":0,"id":0}"#), Some((0, 0)));
    }

    #[test]
    fn tolerates_whitespace_and_unknown_keys() {
        assert_eq!(
            identity("{ \"unknown\" : [1, 2] , \"sid\" : 1 , \"id\" : 2 }"),
            Some((1, 2))
        );
    }

    /// escapeされたkey綴りは既知keyとして扱わず、そのpairを読み飛ばす。
    #[test]
    fn an_escaped_key_spelling_is_not_treated_as_known() {
        // JSONのkeyとして `\u0073id` と書いたもの。値は `sid` と等しいが、
        // escapeを解釈しないこの走査では既知keyに一致しない。
        let escaped = format!("{{{q}{b}u0073id{q}:1,{q}id{q}:2}}", q = '"', b = '\\');
        assert_eq!(identity(&escaped), None);

        let shadowed = format!(
            "{{{q}{b}u0073id{q}:9,{q}sid{q}:1,{q}id{q}:2}}",
            q = '"',
            b = '\\'
        );
        assert_eq!(identity(&shadowed), Some((1, 2)));
    }

    #[test]
    fn reports_an_unknown_type_as_none() {
        let recovered = recover_identity(br#"{"sid":1,"id":2,"type":"nope","#).expect("identity");
        assert_eq!(recovered.type_name, None);
    }

    #[test]
    fn rejects_input_that_is_not_an_object() {
        assert_eq!(identity(""), None);
        assert_eq!(identity("[1,2]"), None);
        assert_eq!(identity(r#""text""#), None);
    }

    /// prefixをどこで切っても、誤ったidentityを返さない。
    #[test]
    fn no_truncation_point_yields_a_wrong_identity() {
        let line = br#"{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"ping","payload":{}}"#;
        for end in 0..=line.len() {
            if let Some(recovered) = recover_identity(&line[..end]) {
                assert_eq!(
                    (recovered.sid, recovered.id),
                    (90312, 907),
                    "prefix長 {end} で誤ったidentityを返した"
                );
            }
        }
    }
}
