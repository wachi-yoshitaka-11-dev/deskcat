# ADR-0015: 後始末を宣言専用の区分として足し、直接commitの基準を到達できる形へ変える

> 状態: Accepted
> 日付: 2026-08-25

## 背景

**`CLASS=minor`は構造的に出ない。**`review_gate.py`の`classify`は、変更されたpathが
`INSTRUCTION_SOURCES`のいずれかに該当した時点で`review-required`を決め、行の中身
（`LINE_DENY`）の判定へ届かない。`INSTRUCTION_SOURCES`には`docs/hardware/`、
`docs/protocol/`、`docs/decisions/`、`scripts/`、`CONTRIBUTING.md`、`.github/`が入っており、
**この repository の文書作業はほぼ全部が該当する。**

実測した。[PR #206](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/206)は
`hardware-bom.md`の購入待ちリスト節を削る変更だが、分類の理由はpathの2行だけだった。

```text
CLASS=review-required
  reason: docs/hardware/hardware-bom.md: instruction source
  reason: docs/hardware/tbd-register.md: instruction source
```

**帰結が2つある。**

1. `CONTRIBUTING.md`のIssue／Pull Request要否表で、`CLASS=minor`ならPull Requestが不要と
   する行が**実際には空である**
2. `AGENTS.md`が「`CLASS=minor`と判定されない変更を、IssueとPull Requestなしで`develop`へ
   入れない」と定めているため、**直接commitできる変更が存在しない**

`57734371`以降の35 commitのうち`Change-Class: review-required`が32件、`minor`は0件である。

## 判断要因

- **`INSTRUCTION_SOURCES`の縮小と`LINE_DENY`の緩和は、[ADR-0010](0010-change-class-and-review-declaration.md)の
  見直し条件が同じ1文で禁じている。**「偽陰性が運用の妨げになる場合は、
  `INSTRUCTION_SOURCES`の縮小ではなく、軽微経路の廃止（すべてPull Requestにする）を
  先に検討する。deny規則を緩めない」
- **path判定を内容判定へ移しても効かない。**PR #206の差分でpathの近道を外して`LINE_DENY`を
  直接当てると**72件**当たる（`hardware-bom.md`に70、`tbd-register.md`に2。
  `digit`／`inline code`／`fenced block`）。`docs/hardware/*.md`は表・リンク・数字で
  できているため、内容判定へ移しても`review-required`のままである。**実際に軽微へ落とすには
  `LINE_DENY`の緩和も必要になり、それは同じ1文が禁じている**
- **人間の意図は差分に現れない。**「新しい判断」と「前の判断の後始末」を区別するcodeは
  書けない。ADR-0010が選択肢Aとして退けたのと同じ理由である
- 一方で、**後始末には後始末の対象が存在する。**これは機械で確かめられる

## 検討した選択肢

### 選択肢A: `INSTRUCTION_SOURCES`の判定を内容依存にする

`docs/hardware/`の表の1行を直すのと`AGENTS.md`の指示文を書き換えるのを別扱いにする。

**採らない。**ADR-0010の決定1（意味を判定しない）、選択肢Aの却下、見直し条件の3箇所に
正面から反する。**かつ上の72件の実測どおり、単独では効かない。**

### 選択肢B: `LINE_DENY`を緩める

数値やリンクを含む行の変更を軽微にする。

**採らない。**見直し条件が逐語で禁じている。値の書き換えが軽く通るようになる。

### 選択肢C: 軽微経路を廃止する

`minor`が到達不能であることを認めて廃止し、すべてPull Requestにする。

**ADR-0010の見直し条件が「先に検討する」と指定している選択肢である。**検討した。

採らない理由は、**廃止しても現状と何も変わらないこと**である。`minor`は既に0件であり、
廃止は記述の整理にとどまる。**運用の重さは1つも減らない。**そして「軽い経路が要る」という
必要は残る。廃止を採ると、その必要へ答える手段が無くなる。

**ただし`minor`を残すことにも意味は無い。**到達不能な経路として残る。この決定はそこへ
手を付けない。**`minor`の廃止は、この決定と独立に判断できる。**

### 選択肢D: 宣言専用の区分を足す

`Change-Class`に3つ目の値を足す。`classify`は返さない。宣言したら`Refs`で対象を示させる。

## 決定

**選択肢Dを採る。**

1. `Change-Class`に`fixup`を足す。**`classify`は返さない。宣言専用である**
2. **`fixup`を宣言したら`Refs`で後始末の対象（Issue または Pull Request の番号）を要求する。**
   番号を含まない`Refs`は通さない。**後始末である以上、対象が存在する。示せないものは
   後始末ではない**
   **`Refs`はtrailerの形（`Refs: #204`）で書く。**`git interpret-trailers`はコロンの無い
   行をtrailerとして読まない。この repository が本文へ書いてきた`Refs #204`はtrailerでは
   なく、**さらにtrailer blockと同じ段落へ置くとblockごと無効にする**（実測）。
   この落とし穴を`CONTRIBUTING.md`へ書いた
3. **`classify`／`INSTRUCTION_SOURCES`／`LINE_DENY`を変更しない。**`minor`と
   `review-required`の判定はどちらも変わらない
4. **`fixup`は`minor`経路を主張しない。**「宣言が計算結果より緩くない」検査（`minor`に
   対するもの）の外側に置く
5. **`Instruction-Change`の要求を代替しない。**`fixup`でも指示sourceを触れば宣言が要る
6. `develop`へ Issue と Pull Request なしで入れてよい範囲を、**`CLASS=minor`または
   `fixup`＋`Refs`**とする。範囲の上限は`CONTRIBUTING.md`が正本である
7. 有効値の集合を`CLASS_VALUES`へ集約する。以前は`_check_receipt`と`_check_history`へ
   別々に埋め込んでおり、値を足すときに片方だけを直す事故が起きる形だった

**判定を機械化せず申告に寄せた。**その理由は上の「人間の意図は差分に現れない」である。
機械が確かめるのは整合だけで、その差分が本当に後始末かは判定しない。

## 影響

### 利点

- **軽い経路が実際に到達できるようになる。**申告すれば通る
- **強制点が実在する。**直接commitは`gate`を通らないが、`history`が`develop`から`main`への
  昇格時に範囲の各commitを検査する。宣言が壊れていれば次の昇格で落ちる。
  **`minor`経路が完全に無検証であるのより強い**
- `review-required`の重い経路を1つも緩めていない

### 欠点

- **申告は検証できない。**`fixup`と宣言した差分が本当に後始末かは機械では分からない
- `Change-Class`の値が3つになり、`minor`（到達不能）と`fixup`（申告）と
  `review-required`（計算結果）が同じtrailerに同居する。**軸が揃っていない**
- 直接commitはPull Requestを通らないため、CodeRabbitも自己レビューのthreadも残らない

### リスクと対策

| リスク | 対策 |
|---|---|
| `fixup`が新しい判断へ使われる | `CONTRIBUTING.md`へ対象外を明記する（安全値、protocol、GPIO、電源値、firmware、`crates/`、新しい判断、`docs/hardware/`の値）。**迷うものはPull Requestを通す** |
| `Refs`をコロンなしで書き、trailer blockごと無効にする | `CONTRIBUTING.md`へ実測した例を載せる。**gateは「trailerが無い」と報告するため、原因が`Refs`だと分からない。**そこが危ないので明示する |
| `Refs`が形だけになる | 番号の存在だけを機械で見る。**正しさは見ないと明記する。**中身は`main`昇格のreviewで人が見る |
| `fixup`が`minor`の代わりに使われ、`minor`の検査を回避する | `minor`に対する「宣言が計算結果より緩くない」検査を変更しない。`fixup`はその外側に置く。testで固定する |
| 要否表へ行を足すと、本文の位置参照（「3行目」等）がずれる | **今回ずれた。**位置で指す記述を内容で指す形へ書き換えた。[#201](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/201)が同じ表を走査して「参照先が正しい」と確認していたが、**行の追加でその確認が無効になった。**位置参照を残さないことで再発を止める |
| 値を足したことで`_check_receipt`と`_check_history`がずれる | 有効値を`CLASS_VALUES`へ集約する。両方がこれを見る |
| 直接commitが増え、reviewを通る変更が減る | **範囲の上限が守られているかは、`main`昇格時のreviewで見る。**この決定はそれを代替しない |

## 検証

- `fixup`＋`Refs`が通り、`Refs`なしが落ちること。`receipt`と`history`の両方で
- `CLASS_VALUES`に無い値が落ちること。**診断へ有効値を並べること**
- `fixup`を宣言しても`classify`の出力が変わらないこと。**`CLASS=review-required`のまま
  出て、出力に`fixup`が現れないこと**
- `fixup`でも指示sourceを触れば`Instruction-Change`が要ること
- 上記はすべて`scripts/test_review_gate.py`が持つ。**既存の期待値は書き換えていない**
- 見直し条件: **`fixup`の申告が範囲の上限を超えて使われた実例が出た場合は、この決定を
  改訂する。**`CONTRIBUTING.md`の範囲だけを狭めない。**逆に、`minor`が到達可能になる変更
  （`INSTRUCTION_SOURCES`や`LINE_DENY`の変更）を行う場合は、ADR-0010の見直し条件が先に
  適用される**

## 置き換える決定

なし。**[ADR-0010](0010-change-class-and-review-declaration.md)を置き換えない。**
同ADRの決定1（意味を判定しない）、決定2（規則を持つ経路を軽微経路に入れない）、
決定4（trailerの名前と値の正本はscript）はいずれも維持している。
見直し条件が指定する選択肢C（軽微経路の廃止）は上で検討し、採らない理由を記録した。

**[ADR-0011](0011-issue-optional-pull-request-required.md)も置き換えない。**
Issueの要否とPull Requestの要否を分ける決定は維持し、Pull Request不要の側へ1行足している。
