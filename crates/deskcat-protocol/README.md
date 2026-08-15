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
- byte列からlineを組み立てる上限付きreceiver（§8手順1〜6、Issue #10）
- 共有conformance fixture（[`tests/fixtures/`](tests/fixtures/README.md)）

含まないもの:

- serial deviceのopen、read／write、切断と再接続（Issue #11）
- session state、duplicate履歴、受理budget、遷移cooldown（Issue #12）
- hardwareに依存する値と処理

`set_expression`、`play_motion`、`show_text`、`show_choices`、sensor eventの型は起こしていない。
上限値が`PROTO-TBD-007`／`008`／`009`／`014`で未確定であり、値を推測しないためである。

## 検証順序

`decode_line`は、§8手順7および§5.1「遷移は完全な検証を通してから確定する」に一致する順で判定する。
**順序自体がconformanceの対象**であり、入れ替えると返すべきcodeが変わる。

**判定の順序と失敗時のcodeは`decode_line`のdoc commentに表として置き、ここには複製しない。**
`cargo doc --open`で読む。事象とcodeの対応そのものの正本は仕様§7である。

未知の追加payload fieldは§3に従って無視する。

**返すcodeは分類であって、送出の判断ではない。**そのcodeを相手へ返すかどうかは、方向（§8）と
§8.2の送出上限に従い、session stateを持つ層（#11、#12）が決める。とくに`line_too_long`は
§7により`(sid, id)`を復元できた場合にだけ返してよく、`unknown_type`は方向によって
相関ACKを返すか計数だけにするかが変わる。**`decode_line`単体は上限付きprefixからのidentity
復元を行わない。**行全体を受け取る以上、上限超過時にどこまで保持するかはbyte受信を持つ層の
判断だからである。復元は下の受信器が行う。

`unknown_type`と`invalid_payload`を区別するため、envelopeのpayloadは
`serde_json::value::RawValue`として未解釈のまま受け、`type`を解決してから
type固有schemaでdeserializeする。serdeのadjacently tagged enumでは失敗理由が
「どのvariantにも一致しない」に潰れ、この区別ができない。

## 受信器（byte列からline、#10）

`src/framing.rs`、`src/prefix.rs`、`src/receiver.rs`の3層に分ける。責務の詳細は
`cargo doc --open`で読む。ここには**判断の理由だけ**を残す。

| 層 | 役割 |
|---|---|
| `framing` | byteを蓄えて`\n`でlineへ切る。JSONもUTF-8も解釈しない |
| `prefix` | 途中で切れたJSONから`(sid, id)`と`type`の復元を試みる純関数 |
| `receiver` | UTF-8を検証して`decode_line`へ渡し、結果を§7の分類へ対応づける |

### 上限は型の性質にしている

bufferは構築時に`Box<[u8]>`で確保する。`push`が無いため伸びない。「上限のないbufferを
作らない」を規律ではなく型で保証するためである。queueは持たない。呼び出し側が1件ずつ
取り出すので、溜める場所そのものが無い。

### 容量は`MAX_LINE_BODY_BYTES`（改行を除く）

§2の上限は改行を含む。bufferへ蓄えるのはbodyだけなので、容量は`MAX_LINE_BYTES - 1`とする。
こうすると**bufferを通ったlineは必ず`decode_line`の行長判定も通る**ため、`line_too_long`の
出所がreceiverの1箇所だけになる。両方で判定すると、identityを復元できる経路とできない経路が
同じcodeで並んでしまう。

CRLFで終わる行では`\r`もbody予算を使う。§2は1024 bytesに`\r`を含めるかを書いていないため、
この振る舞いは`PROTO-TBD-002`への入力として`tests/fixtures/framing.json`が固定している。

### oversizeは検知した時点で1回だけ返す

終端の`\n`を待たない。待つと、相手が`\n`を送ってこない限り報告が出ず、§4.1の`boot`送信側が
recovery budgetを無応答のまま使い切る。破棄中にさらに超過が続いても2件目は返さない。

### prefix長の定数は置かない

§8手順6が求める「上限付きprefix」の長さは`PROTO-TBD-002`であり、**仕様は候補値を一つも
示していない。**JSONはkeyの順序を縛らないため、必要byte数に計算可能なworst caseが無い。
そこで受信器は、既に保持している行buffer全体をprefixとして使う。新しい数を決めずに済む
退化した上限である。確定したら`with_prefix_budget`で縮める。縮めることは純粋な制限であり、
いま復元できない行が復元できるようにはならない。

### UTF-8の不正とJSONの不正を`cause`で分ける

§4.6はUTF-8／JSON／envelopeの不正をまとめて`parse_errors`とするため、wire上の`ErrorCode`だけでは
両者を区別できない。一方§7は「parser counterでは区別する」と定めている。そこで`Rejection`は
wireへ出す`code`とは別に`cause`を持つ。**JSONの不正とenvelopeの不正は、まだ区別が付かない。**
`decode_line`がどちらも`invalid_envelope`として返すためである。

## 上限

`src/limits.rs`にまとめている。いずれも仕様が`Candidate`または`TBD`としている段階の
暫定値であり、確定値として扱わない。

- `MAX_LINE_BYTES`は改行を含む1 lineの上限（§2の候補値、`PROTO-TBD-002`）。
- string上限は勘で置かず、**escapeが起きない文字であれば**`MAX_LINE_BYTES`から導出できることを
  `tests/limits.rs`が検査する。各message typeについて、全string fieldを上限まで、
  全integerを最大値まで詰めたlineが収まることを確認している。

**string上限を守っても行長は保証されない。**string上限はUTF-8 byte長で測るが、JSON encodeは
`"`を`\"`、`\`を`\\`、制御文字を`\u00XX`（1 byte→6 byte）へ広げる。全fieldが上限内でも
encode後の行が`MAX_LINE_BYTES`を超えることがあり、その場合`encode_line`が`line_too_long`を返す。
黙って上限超過の行がwireへ出ることはないが、field上限は行長の十分条件ではない。
escape後のwire sizeまで含めた上限の確定は`PROTO-TBD-002`に含める。

integer widthは`PROTO-TBD-003`として保留されていたものを確定した。**値と根拠の正本は仕様§3の表**であり、
ここには複製しない。実装上の宣言は`src/envelope.rs`の型を見る。

## 検証

repository rootで実行する。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
```

lint levelはroot `Cargo.toml`の`[workspace.lints]`が持つため、`-D warnings`は要らない。
`cargo fmt`は`--locked`を受け付けない。
