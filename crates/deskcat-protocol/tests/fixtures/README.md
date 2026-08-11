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

どちらも次の形を持つ。

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

## 分担

§12は5群のfixtureを要求する。**このdirectoryが現在持つのはSchema群と、Framing／parse群のうち
line長境界・CRLF・invalid JSONだけである。**

| 群 | 対象 | 担当Issue |
|---|---|---|
| Framing／parse | 分割受信、CRLF、invalid UTF-8、line長境界 | line長境界とCRLFは#9。byte単位の分割受信とinvalid UTF-8は#10 |
| Schema | envelope、type固有payload、上限、未知version／type | #9（このdirectory） |
| Session判定 | 遷移の成否、`stale_session`、retired session、再送 | #12 |
| Duplicate replay | 同一`(sid, id)`のretry、保持結果の返却 | #12 |
| Budgetと応答 | 受理上限、遷移budget／cooldown、予約枠、送出上限 | #12。値が`PROTO-TBD-011`／`012`／`017`で未確定 |

session判定・duplicate・budgetの群は、単一lineのschema検証ではなく**受信側のstate**に
依存する。#9の`decode_line`はstateを持たないため、これらのcaseをここへ置いても実行できない。
`PROTO-TBD-012`のbudget値も未確定である。

## 追加するときの規則

- `name`はfile全体で一意にする。`fixture_case_names_are_unique` testが検査する。
- 期待値をRust側のtest codeへ書かない。fileへ書く。
- 上限の境界は「ちょうど」と「1 byte超過」を対にして入れる。片方だけでは境界を固定できない。
- **string上限のcaseはASCIIだけで書かない。**上限はUTF-8 byte長であって文字数ではない。
  ASCIIだけでは両者が一致するため、byte数を文字数と取り違えた実装をfixtureが検出できない。
  `multibyte_string_over_limit_but_under_char_count`が、その取り違えを名指しで失敗させる。
- `line`のbyte長に意味があるcaseは、手計算せず生成して確認する。
- case を削るときは`fixtures_are_not_empty` testの下限も一緒に見直す。下限は、
  fixtureが空でもtestが成功してしまう事故を防ぐためにある。
