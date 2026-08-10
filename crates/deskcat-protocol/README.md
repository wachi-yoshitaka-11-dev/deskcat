# deskcat-protocol

ESP32–Raspberry Pi protocolのRust実装。型付きenvelope、message、error code、1 lineの
decode／encode、および共有conformance fixtureを提供する。

**wire仕様の正本は[`docs/protocol/esp32-pi-protocol.md`](../../docs/protocol/esp32-pi-protocol.md)である。**
このcrateは仕様を置き換えない。仕様がDraftである間、`v: 1`が一致してもwire互換は保証されず、
互換の根拠は共有fixtureの一致である（§11、§12.1）。

## 範囲

含むもの:

- envelope（§3）と、Issue #4が承認した最小message type
  （`boot`、`hello`、`ping`、`get_status`、`status`、`ack`）
- error code（§7）と、それを計上する`status`のcounterへの対応付け
- 1 lineのdecodeとencode、およびその検証順序
- 共有conformance fixture（[`tests/fixtures/`](tests/fixtures/README.md)）

含まないもの:

- serial I/O、byte受信、分割lineの結合、invalid UTF-8の分類（Issue #10、#11）
- session state、duplicate履歴、受理budget、遷移cooldown（Issue #12）
- hardwareに依存する値と処理

`set_expression`、`play_motion`、`show_text`、`show_choices`、sensor eventの型は起こしていない。
上限値が`PROTO-TBD-007`／`008`／`009`／`014`で未確定であり、値を推測しないためである。

## 検証順序

`decode_line`は次の順で判定する。**順序自体がconformanceの対象**であり、入れ替えると
返すべきcodeが変わる。§8手順7および§5.1「遷移は完全な検証を通してから確定する」に一致させている。

| # | 判定 | 失敗時のcode |
|---|---|---|
| 1 | 改行を含むline長 | `line_too_long` |
| 2 | JSONとして解析でき、envelope fieldが型どおり | `invalid_envelope` |
| 3 | `payload`がobject（`null`不可） | `invalid_envelope` |
| 4 | `v`が対応major version | `unsupported_version` |
| 5 | `type`が既知 | `unknown_type` |
| 6 | type固有payload schema | `invalid_payload` |
| 7 | 上限のある値の範囲 | `out_of_range` |

未知の追加payload fieldは§3に従って無視する。

**返すcodeは分類であって、送出の判断ではない。**そのcodeを相手へ返すかどうかは、方向（§8）と
§8.2の送出上限に従い、session stateを持つ層（#11、#12）が決める。とくに`line_too_long`は
§7により`(sid, id)`を復元できた場合にだけ返してよく、`unknown_type`は方向によって
相関ACKを返すか計数だけにするかが変わる。**このcrateは上限付きprefixからのidentity復元を
行わない。**行全体を受け取る以上、上限超過時にどこまで保持するかはbyte受信を持つ層（#10）の
判断だからである。

`unknown_type`と`invalid_payload`を区別するため、envelopeのpayloadは
`serde_json::value::RawValue`として未解釈のまま受け、`type`を解決してから
type固有schemaでdeserializeする。serdeのadjacently tagged enumでは失敗理由が
「どのvariantにも一致しない」に潰れ、この区別ができない。

## 上限

`src/limits.rs`にまとめている。いずれも仕様が`Candidate`または`TBD`としている段階の
暫定値であり、確定値として扱わない。

- `MAX_LINE_BYTES`は改行を含む1 lineの上限（§2の候補値、`PROTO-TBD-002`）。
- string上限は勘で置かず、`MAX_LINE_BYTES`から導出できることを`tests/limits.rs`が検査する。
  各message typeについて、全string fieldを上限まで、全integerを最大値まで詰めたlineが
  収まることを確認している。

integer widthは`PROTO-TBD-003`として保留されていたものを確定した。`v`は`u16`、
`sid`と`id`は`u32`、`ts_ms`は`u64`である。根拠は`src/envelope.rs`のdoc commentに書いている。

## 検証

repository rootで実行する。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
```

lint levelはroot `Cargo.toml`の`[workspace.lints]`が持つため、`-D warnings`は要らない。
`cargo fmt`は`--locked`を受け付けない。
