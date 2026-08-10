# ADR-0007: 自動reviewを高リスク変更へ限定し、自己レビューを主軸とする

> 状態: Accepted
> 日付: 2026-08-10

## 背景

保守作業が製品作業を追い越していた。2026-08-10時点で、初期backlog由来のIssue（#1–#34、製品そのもの）は
29件中6件しかcloseしておらず（21%）、レビューと運用から派生したIssue（#35–）は22件中20件がcloseしている。
派生Issueの91%が`type:maintenance`である。

Pull Requestも同様で、35件中12件（34%）は題名の時点でレビュー対応または記述修正である。
昇格Pull Request [#61](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61) は単体で
#62〜#70 の9件を派生させた。

同じ2日のうちに、[#36](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/36)で
「Machine Profilesにpwsh 7以降の要件を追記する」を処理し、
[#38](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/38)でその前提ごと捨てている。

**最も作業を止めていたのは、全Pull Requestで自動review（CodeRabbit）を走らせる運用である。**
rate limitに掛かり、必要なPull Requestでreviewが返らない。
[PR #55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)では再review依頼が実行されず、
**未reviewのcommitを含んだままmergeされた。**

## 判断要因

- 安全（hardware safety、servo、電源、protocol）に関わる範囲は緩めない
- 手作業を増やす解を採らない。GitHubまたは設定で強制できるものはそちらへ寄せる
- 削除で解決できるものに、新しい規約を足さない
- 変更後、merge前に人手で実行する手順が実際に減ったことを測れる
- 自動reviewの総量を減らす以上、その分の検出を別の手段で担保する

## 検討した選択肢

### 選択肢A: 自動reviewを高リスク変更へ限定し、自己レビューを主軸にする

`.coderabbit.yaml`で自動reviewの発火をlabelで制御し、Rust、protocol、hardware、安全文書、
および設計判断に限定する。文書の記述修正は自己レビューで通す。
あわせて自己レビューの観点をchecklist化し、回数（新規指摘0件が2 round）は維持する。

利点: rate limitの頻度が下がり、必要なPull Requestでreviewが返る。文書修正がreview待ちで止まらない。

コスト: 対象外のPull Requestの品質が自己レビューに依存する。checklistの質が効く。

### 選択肢B: 現状維持（全Pull Requestでreviewを受ける）

利点: 見落としの確率は最も低い。

コスト: rate limitで止まる。実際に#55で不発している。文書1行の修正でもreview待ちが発生し、
保守Issueの生成速度が製品開発を上回る現状が続く。

### 選択肢C: templateを削って記入負荷を下げる

コスト: **採らない。**指摘の出どころを調べると、#63・#64・#65・#70・#82 はいずれも中身の不備
（未検証の断定、一次資料未確認、存在しない照合先）であり、templateの節数が原因ではない。
Pull Request templateの「検証」表と「安全とsecurity」は、何を確認して何を確認していないかを
書かせる装置であり、削ると未検証のまま通る量が増える。**不足があるから指摘が増えるのであって、その逆ではない。**

## 決定

**選択肢Aを採る。**

1. `.coderabbit.yaml`を新設し、auto reviewを**高リスク変更（firmware、protocol、Pi、hardware）**
   を示すlabelを持つPull Requestに限定する。あわせてpushのたびの再reviewを止める。
   **対象labelと設定値の正本は[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)である。**
   本ADRは判断を記録するもので、設定値を再掲しない
2. 自己レビューの観点を`CONTRIBUTING.md`にchecklistとして定める。回数は減らさない
3. Issueを立てずに直接反映してよい範囲（typo、リンク、表記ゆれ、規約の言い回し、metadata記入漏れ）を定める。
   **変更内容の承認は必ず得る**
4. `develop`のbranch protectionで`Require conversation resolution before merging`を有効化し、
   手作業のGraphQL確認を廃止する。**必須reviewと必須status checkは設定しない**
5. **templateは変更しない**

## 影響

### 利点

- 自動reviewの総量が減り、rate limitで必要なreviewが落ちる事象が減る
- 未解決threadを残したmergeをGitHubが止める（[#40](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/40)の再発防止）
- `CONTRIBUTING.md`の「Merge前の確認」から、pagination付きGraphQL手順が不要になる
- 記述の維持管理がIssue起票を伴わなくなる

### 欠点

- 対象外のPull Requestは自動reviewを受けない。検出は自己レビューに依存する
- 自己レビューchecklistが実際に何を拾えるかは未測定である

### リスクと対策

| リスク | 対策 |
|---|---|
| 自己レビューが形骸化し、対象外Pull Requestの品質が落ちる | checklistを観点で定義し、各項目を過去の実際の失敗に紐付ける。回数（2 round）は維持する |
| labelの付け忘れで、高リスク変更がreviewされない | Pull Requestのlabelは既に必須運用である。昇格Pull Requestには対象範囲に応じたlabelを必ず付ける。**ただしlabelは作成時に付ける。**CodeRabbitは対象判定を作成直後に行い、後からのlabel追加では再判定しない（[#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94)でのCodeRabbitの回答。**こちらの実測ではない**）。`gh pr create --label`で作成時に指定する運用を`CONTRIBUTING.md`へ定めた |
| **`auto_incremental_review: false`により、指摘対応後のcommitがreviewされない** | **対応commitは自己レビューで見る。`@coderabbitai review`を投げ直さない。**投げ直すと1つのPull Requestでreviewを何度も消費し、`false`にした意味が無くなる（[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で実際に発生）。手動依頼は**初回のreviewが一度も得られなかったときだけ**とし、安全・電気・protocol・firmwareに関わる変更ではrate limitが解けるまで待つ |
| `Review rate limited`がcheck上`pass`と表示され、reviewされていないのにmergeされる | GitHubは止められない。`CONTRIBUTING.md`の「Merge前の確認」に手作業の確認として明記する。**本ADRを入れるPR #90 自身で2回連続して発生し、機械reviewを受けられなかった** |
| 直接反映の範囲が拡大解釈される | 「Issueを立てない」は「勝手に変えてよい」ではない。承認は必ず得る。迷うものはIssue必須側として扱う |

## 検証

- 対象外label（`area:docs`＋`type:maintenance`）のPull Requestで自動reviewが走らないこと。
  **確認済み。**[PR #95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)と
  [PR #96](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/96)が
  `Review skipped: excluded by label configuration` となった。**labelによる除外である旨が
  文言に表れている。**
  なお[PR #88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)の観測
  （`Review skipped: reviews are disabled for this base branch`）は**この確認にはならない。**
  #88 は本設定を導入したPull Request自身であり、判定の時点で`.coderabbit.yaml`は
  `develop`に無かった。**base branchによるskipであって、labelによる除外ではない。**
- 対象labelのPull Requestで自動reviewが走ること。**発火と完走の双方を確認した。**
  [PR #90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)（`type:decision`を持つ。
  **当時のallowlistは`type:decision`を含んでいた。#91で外した**）では
  `Review skipped`ではなく`Review rate limited`となり、対象判定を通過してから
  rate limitで止まったことが分かる。完走は
  [PR #91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で得た。
  作成時にlabelを指定し、作成の4分47秒後に自動reviewが提出されている
  （手動の`@coderabbitai review`より前である）
- **`enabled: false`が絶対的に効く、という懸念は否定された。**
  [#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94)で確認した。
  allowlistに一致するlabelを作成時に持つPull Requestは、`enabled: false`でも対象判定を通る
- **[PR #89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)がskipされた原因は確定できない。**
  #89 は作成の24秒後にlabel無しで`Review skipped`となり、labelは33分後に付いた。
  しかし**その判定の5秒後に本設定が`develop`へmergeされている**（判定`12:22:00Z`、merge`12:22:05Z`）。
  判定の時点で`.coderabbit.yaml`はbase branchに無く、**labelの未付与と設定の未反映が交絡している。**
  #89 を「labelの後付けが原因」の証拠として扱わない。**切り分けのための検証Pull Requestは作らない**
  （reviewを消費し、運用上の結論が変わらないため）。
  labelを作成時に付ける運用はCodeRabbitの回答に沿った安全側の選択であり、`CONTRIBUTING.md`の
  「labelは作成時に付ける」に定めた
- `develop`で未解決threadがmergeをblockすること。PR #88で確認済み
  （resolved→`CLEAN`、unresolve→`BLOCKED`、再resolve→`CLEAN`）
- 見直し後の最初の3 Pull Requestについて、merge前に人手で実行した手順の数とreviewの待ち時間を記録し、
  見直し前と比較する。**1本目（[PR #90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)）を実施。**
  記録は[#87](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/87)のコメントにある

### 1本目で分かったこと

**手順数は5から1へ減った。**`reviewThreads`のpaginationループが消えたことが最も効いている。

**待ち時間は比較できなかった。**#90 では初回と手動依頼の2回ともrate limitとなり、
**reviewが一度も完走していない。**完走した実績が無い以上、待ち時間が改善したとも
悪化したとも言えない。[PR #91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で
初めて`Review completed`を得たため、比較は2本目以降の記録で行う。

**確実に言えるのは、reviewを要求する回数が減ったことだけである。**
「1回のreviewが返る速さ」への効果は未測定である。

見直し条件: 自己レビューを主軸にした結果、対象外Pull Requestで見落としが続けて発生した場合は、
対象labelの範囲を広げるか、選択肢Bへ戻すことを再検討する。

## 置き換える決定

なし。
