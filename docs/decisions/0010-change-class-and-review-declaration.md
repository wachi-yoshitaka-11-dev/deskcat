# ADR-0010: 変更の分類を機械化し、自己レビューをcommit trailerで宣言する

> 状態: Accepted
> 日付: 2026-08-21

## 背景

[ADR-0007](0007-review-scope-and-self-review.md)で自動reviewを高リスク変更へ限定し、
自己レビューを主軸にした。運用してみて、次の4つが人の記憶だけに載っていることが分かった。

- **どの変更が軽微か**を決めるのは人の判断だけである。`CONTRIBUTING.md`に表はあるが、
  照合する仕組みが無い
- **自己レビューが完了したこと**が、reviewした差分へ結び付いていない。review後にcommitを
  足しても、宣言は何も変わらない
- **Pull Requestの差分に含まれる指示file**を、指示ではなくdataとして扱ったかどうかが
  記録に残らない（[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の
  リスク欄）
- **`main`昇格**が、含まれる全commitの分類を確認していない

いずれも「忘れることが唯一の失敗モード」であり、ADR-0007が採った方針
（GitHubまたは設定で強制できるものはそちらへ寄せる）の延長で扱える。

## 判断要因

- ADR-0007の方針を引き継ぐ。自動reviewの総量は増やさない
- **意味を判定するcodeを書かない。**「言い回しの修正」と「意味の反転」を構文で区別できない
- fail-closedにする。証明できないものは軽微にしない
- GitHubまたはbranch protectionが強制できるものを、scriptで二重に持たない
- 手作業を増やす解を採らない。増やすなら1つに留める
- 規則表を後から育てない。edge caseを追うと近似の穴が開く

## 検討した選択肢

### 選択肢A: 差分の意味を判定するclassifierを書く

Markdownの差分を解析し、typoの修正か意味の変更かを判定する。

コスト: **採らない。**この判定に正解を出せる参照実装をこのrepositoryは持たない
（[ADR-0006](0006-validation-script-language.md)で標準ライブラリのみと決めている）。
`〜しない`を`〜する`へ変える差分は数値もcommandもlinkも含まず、構文では区別できない。
近似で答えると、精度を上げるたびに逆向きの穴が開く。実際に
[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の作業では、
過剰検知を直す変更が新しい抜け道を作った。

### 選択肢B: pathと字句だけでfail-closedに分類し、宣言をcommit trailerで持つ

規則を持つfileそのものを軽微経路から外し、残るfileでは変更行が数値・command・link・表・
見出し・checkbox・fenceに触れていないことだけを字句的に確かめる。分類と自己レビューの
完了は、head commitのtrailerとして宣言し、scriptが照合する。

利点: 判定に正解がある。trailerはcommitへ結び付くため、review後にcommitを足すと宣言が
自動的に無効になる。追跡fileが増えない。直接pushにも同じ規則が適用できる。

**reviewしたSHAを記録して突き合わせる方式は採らない。**記録したSHAは差分が変わったことを
検出するだけで、宣言した分類が正しいかどうかは判定できない。**Gateの時点で実際の範囲から
分類を再計算する**方が強い。あわせて、squash mergeでtreeが変わってもこの照合は成立する。

コスト: 本当は軽微な変更もreview必須側へ落ちる。squash merge時にtrailerを書く手作業が
1つ増える。**散文の意味の反転は検出しない。**

### 選択肢C: 機械化せず、運用の注意として書く

利点: 新しいcodeを持たない。

コスト: **採らない。**ADR-0007の時点で既に運用の注意として書いてあり、それでも上の4つが
残った。`Start date`／`Target date`が3件続けて空のままcloseされた事例と同じ失敗modeである。

## 決定

**選択肢Bを採る。**

1. 変更の分類は`scripts/review_gate.py`が持つ。**このscriptは意味を判定しない。**
   規則を持つfileの列挙（`INSTRUCTION_SOURCES`）と、変更行の字句的なdeny規則だけで
   判定し、**軽微と証明できないものはすべてreview必須にする**
2. **規則・安全・protocol・commandを持つ経路は軽微経路へ入らない。**rootの指示file、
   `.claude/`と`.github/`、governanceとADR、hardwareとprotocolの正本文書、技術ガイド、
   そして検証script自身がこれに当たる。**exact pathの列挙の正本はscriptであり、
   ここでは再掲しない**
3. **ADR-0007の「Issueを立てずに直接反映してよい範囲」から「規約の言い回し」を外す。**
   規則を持つfileの散文は、言い回しの修正と意味の変更を機械的に区別できない。
   Issue #154 の受け入れ条件が「規約の意味変更は軽微経路へ入らない」と定めており、
   そちらを優先する
4. 分類と自己レビューの完了は、変更のhead commitのtrailerで宣言する。
   **trailerの名前と値の正本はscriptである。**宣言が計算結果より緩い場合は通さない
5. 専用workflowが、baseが`develop`と`main`のPull Requestで分類と宣言を検証する。
   **`develop`側でも掛ける。**宣言が無効になる仕組みはPull Requestのhead commitを見て
   初めて働き、`main`側だけに掛けると宣言はsquash commitにしか現れない。
   **未解決threadと必要CIは検証しない。**どちらもbranch protectionが強制しており、
   同じ条件を2箇所で持つと片方だけを見た判断が起きる
6. **規則表は意図して粗いままにする。**偽陰性は意図した失敗方向であり、Pull Requestを
   1本作れば済む。edge caseに合わせて条件を足さない

## 影響

### 利点

- 軽微かどうかの判断が、実行できるcommandの結果になる
- review後にcommitが増えると宣言が無効になる。人が気付く必要が無い
- 指示fileの変更が、dataとしてreviewした宣言なしには`main`へ進めない
- 判定に必要なのはgitだけである。tokenもAPIも要らない

### 欠点

- 本当は軽微な変更でもPull Requestが必要になる場合がある
- squash merge時にtrailerを書く手作業が増える。**忘れると`main`昇格で落ちる**
- **散文の意味の反転は検出しない。**規則を持つfileを軽微経路から外すことで、
  検出できない変更が規則を緩める余地を限定しているだけである
- `INSTRUCTION_SOURCES`に載っていないfileが将来規則を持ち始めた場合、列挙の更新が要る

### リスクと対策

| リスク | 対策 |
|---|---|
| 規則表が偽陽性側へ倒れ、軽微でない変更が軽微と判定される | 各deny規則にfixture testを持たせ、`minor`と判定される条件を明示的に固定する。列挙外のfileは軽微、ではなく**列挙外かつ字句的に無害なMarkdownだけ**が軽微である |
| edge caseを追って規則が育ち、近似の穴が開く | 決定6として明文化した。粗さは欠陥ではなく設計である |
| trailerを書き忘れて`main`昇格が止まる | 落ちる位置が昇格Pull Requestであり、mergeより手前である。`CONTRIBUTING.md`のmerge手順にcommandを載せる |
| 宣言だけを書いて自己レビューをしない | **機械的には防げない。**trailerは宣言であって証拠ではない。分類の照合だけがscriptで検証できる部分である |
| `git commit --amend`でtreeを変えても、trailerがmessageに残る | **検出しない。**ただし分類はGate時点で実際の範囲から再計算するため、宣言した分類が実態より緩ければ落ちる。`Self-Review`の宣言だけがamendを跨いで残る |
| 列挙を緩める変更が、同じPull Request内で自らの根拠になる | `scripts/`自体が`INSTRUCTION_SOURCES`に入っており、列挙の変更は軽微経路へ入らない。あわせて`Instruction-Change`の宣言が要る |

## 検証

- 各deny規則と各宣言について、fixture repositoryでの回帰testを持つ。
  `scripts/test_review_gate.py`が`review_gate.py`を子processとして起動し、分類結果と
  診断出力を検査する。**Windows localで全件成功を確認した（2026-08-21）**
- 実repositoryに対して分類がerrorなく走ること。**確認済み。**結果の値は主張しない
- workflowが実際に起動し、宣言の欠落で落ちること。**`develop`側は本ADRを入れる
  Pull Request自身で実測する。****`main`側は未検証であり、次の昇格Pull Requestが
  最初の実測になる。**
- 見直し条件: 偽陰性が運用の妨げになる場合は、`INSTRUCTION_SOURCES`の縮小ではなく、
  軽微経路の廃止（すべてPull Requestにする）を先に検討する。deny規則を緩めない

## 置き換える決定

なし。**[ADR-0007](0007-review-scope-and-self-review.md)を置き換えない。**
自動reviewの範囲、自己レビューの回数、branch protectionの扱いはADR-0007のままである。
本ADRはその決定3（Issueを立てずに直接反映してよい範囲）を狭め、判定と宣言を機械化する。
