//! `ErrorCode`とstatus counterの対応が、全codeについて成立していることを検査する。
//!
//! `tests/conformance.rs`の`invalid_fixtures_map_to_a_status_counter`は、
//! **invalid fixtureに現れるcodeしか走査しない。**そこに現れないcodeの対応漏れは
//! 検出できない。実際に`hardware_unavailable`と`duplicate_expired`は
//! `counter_field()`が`None`のまま残っていた。
//!
//! ここでは`ErrorCode::ALL`を走査する。**`ErrorCode`は`#[non_exhaustive]`であり、
//! この別crateで`match`を書いてもwildcard armが必要になるため、variantを足したときの
//! compile errorが起きない。**網羅性はcrate側の`const _`が守り、このtestはその一覧を使う。

use deskcat_protocol::{ErrorCode, ProtocolCounters};

/// 全`ErrorCode`がstatus counterへ対応する（§4.6のcounter対応表）。
#[test]
fn error_codes_all_map_to_a_status_counter() {
    for code in ErrorCode::ALL {
        assert!(
            code.counter_field().is_some(),
            "{code} has no status counter"
        );
    }
}

/// `counter_field()`が返す名前が、`ProtocolCounters`に実在するfieldである。
///
/// **名前は文字列であり、compilerは綴りを検査しない。**存在しないfieldを指していても
/// buildは通り、counterへ計上しようとした受信側の実装だけが後で気づく。
/// serialize結果のkeyと突き合わせて、その経路を塞ぐ。
#[test]
fn counter_fields_exist_in_protocol_counters() {
    let counters =
        serde_json::to_value(ProtocolCounters::default()).expect("counters serialize to an object");
    let counters = counters.as_object().expect("`protocol` is a JSON object");

    for code in ErrorCode::ALL {
        let field = code
            .counter_field()
            .unwrap_or_else(|| panic!("{code} has no status counter"));
        assert!(
            counters.contains_key(field),
            "{code} maps to `{field}`, which is not a field of ProtocolCounters"
        );
    }
}

/// `ALL`は全variantを1回ずつ持ち、`as_str()`の綴りも重複しない。
///
/// 配列の網羅性そのものはcrate側の`const _`がcompile時に守る。ここで見るのは、
/// **wire上の綴りが衝突していない**ことである。2つのvariantが同じ文字列を名乗ると、
/// deserializeでどちらか一方へ潰れる。
#[test]
fn error_code_wire_names_are_unique() {
    let mut names: Vec<&str> = ErrorCode::ALL.iter().map(|code| code.as_str()).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "duplicate error code wire name");
}
