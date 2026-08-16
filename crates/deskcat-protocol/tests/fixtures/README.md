# Conformance fixture

`docs/protocol/esp32-pi-protocol.md` §12が要求する共有fixtureを置く。

**Rust側に期待値を埋め込まず、言語に依存しないJSON fileとして持つ。**
ADR-0001が「コードを共有するかどうかにかかわらず、両側が共通JSON fixtureとprotocol
conformance testに合格しなければならない」と定めているためである。firmware側の実装
（Issue #10）は、この同じfileを読んで検証する。

## File

| File | 内容 |
|---|---|
| `valid.json` | decodeに成功し、round-tripできるline |
| `invalid.json` | 分類済みerror codeで失敗するline |
| `framing.json` | byte列を受信器へ流したときの結果の並び（#10） |

`valid.json`と`invalid.json`は次の形を持つ。`framing.json`は別の形であり、下に節を分ける。

```json
{ "fixture_schema": 1, "cases": [ ... ] }
```

`fixture_schema`はこのfile形式のversionであり、wire protocolの`v`とは無関係である。
形式を変えるときは値を上げ、読み手側の対応を同時に更新する。

### `valid.json`のcase

| Key | 意味 |
|---|---|
| `name` | case名。file全体で一意 |
| `line` | wire上のline。改行を含む |
| `canonical` | `true`なら、再encodeした結果がbyte単位で`line`と一致しなければならない |
| `expect.type` | 期待するmessage type |
| `expect.sid` / `expect.id` | 期待するenvelope identity |
| `note` | 任意。そのcaseが何を固定しているか |

`canonical`が`false`なのは、未知の追加payload fieldを含むcase（§3によりdecodeで無視され、
再encodeで落ちる）と、CRLFで終わるcase（再encodeでは`\n`になる）である。
これらは値の一致だけを要求する。

### `invalid.json`のcase

| Key | 意味 |
|---|---|
| `name` | case名。file全体で一意 |
| `line` | wire上のline |
| `expect_error` | §7のerror code文字列 |
| `note` | 任意 |

### `framing.json`

`valid.json`／`invalid.json`の`line`はJSON文字列である。**したがってinvalid UTF-8も、
byte単位の分割も表現できない。**形が根本的に違うため、既存fileの`fixture_schema`を上げるのではなく
別fileにした。`fixture_schema`はfileごとの形式versionなので、この新fileは`1`から始まる。

```json
{ "fixture_schema": 1, "cases": [ ... ] }
```

| Key | 意味 |
|---|---|
| `name` | case名。file全体で一意 |
| `note` | そのcaseが何を固定しているか |
| `capacity` | bodyの最大byte数。省略時は`MAX_LINE_BODY_BYTES` |
| `prefix_budget` | 任意。oversize時にidentity復元へ渡すprefixのbyte数。省略時は行buffer全体 |
| `chunks` | 1回のreadに相当するbyte列の並び |
| `expect` | 期待する結果の並び |
| `expect_pending_bytes` | 全chunkを流し終えた時点で、bufferに残っているbody byte数 |

`chunks`の各要素は`utf8`と`hex`の**どちらか一方だけ**を持つ。ASCIIのcaseを読めるまま保ちつつ、
invalid UTF-8をhexで表せるようにするためである。両方書くと同じbyte列の正本が2つになる。
`framing_fixture_chunks_have_exactly_one_encoding` testが検査する。

`expect`の各要素は`outcome`で形が決まる。

| `outcome` | Key |
|---|---|
| `frame` | `type`、`sid`、`id` |
| `rejected` | `code`（§7）、`cause`、任意の`identity`（`[sid, id]`）と`type` |

`cause`は`invalid_utf8`／`decode`／`oversize`のいずれかである。§4.6がUTF-8とJSONの不正を
まとめて`parse_errors`とするため、`code`だけでは受け入れ条件「Invalid UTF-8とJSONを分類できる」を
fixtureから確認できない。`cause`はそのためにある。

#### 切り方を変えても結果は変わらない

`framing.json`の中心的な不変条件である。runnerは各caseを、**`chunks`のとおり／全部まとめて1回／
1 byteずつ／すべての2分割**の4通りで回し、同じ`expect`の並びを要求する。§12が求める
「すべてのbyte境界で分割したmessage」を、Rustのtest codeではなくfixtureの契約として置くためである。
firmware側の実装も同じ義務を負う。

`capacity`をcase単位で持たせているのは、oversizeのcaseを`MAX_LINE_BYTES`から切り離すためである。
`capacity: 64`なら超過caseが人の読める長さで書ける。§2の候補値が動いてもfixtureは生き残る。

## 分担

§12は5群のfixtureを要求する。**このdirectoryが現在持つのはSchema群とFraming／parse群である。**
残る3群は受信側のstateに依存するため#12を待つ。

| 群 | 対象 | 担当Issue |
|---|---|---|
| Framing／parse | 分割受信、CRLF、invalid UTF-8、line長境界 | **作成済み。**line長境界とCRLFは#9、byte単位の分割受信とinvalid UTF-8は#10（`framing.json`） |
| Schema | envelope、type固有payload、上限、未知version／type | #9（このdirectory） |
| Session判定 | 遷移の成否、`stale_session`、retired session、再送 | #12 |
| Duplicate replay | 同一`(sid, id)`のretry、保持結果の返却 | #12 |
| Budgetと応答 | 受理上限、遷移budget／cooldown、予約枠、送出上限 | #12。値が`PROTO-TBD-011`／`012`／`017`で未確定 |

session判定・duplicate・budgetの群は、単一lineのschema検証でも1本のbyte streamの組み立てでもなく、
**session間にまたがる受信側のstate**に依存する。`decode_line`はstateを持たず、#10の受信器が持つstateも
1本のstreamの組み立て途中だけである。したがってこれらのcaseをここへ置いても実行できない。
`PROTO-TBD-012`のbudget値も未確定である。

## 追加するときの規則

- `name`はfile全体で一意にする。`fixture_case_names_are_unique` testが検査する。
- 期待値をRust側のtest codeへ書かない。fileへ書く。
- 上限の境界は「ちょうど」と「1 byte超過」を対にして入れる。片方だけでは境界を固定できない。
- **string上限のcaseはASCIIだけで書かない。**上限はUTF-8 byte長であって文字数ではない。
  ASCIIだけでは両者が一致するため、byte数を文字数と取り違えた実装をfixtureが検出できない。
  `multibyte_string_over_limit_but_under_char_count`が、その取り違えを名指しで失敗させる。
- `line`のbyte長に意味があるcaseは、手計算せず生成して確認する。
- case を削るときは`fixtures_are_not_empty`／`framing_fixtures_are_not_empty` testの下限も
  一緒に見直す。下限は、fixtureが空でもtestが成功してしまう事故を防ぐためにある。
- `framing.json`のbyte長に意味があるcaseは、`capacity`との関係が分かるように`note`へ書く。
  境界のcaseは「ちょうど」と「1 byte超過」を対で置く。
