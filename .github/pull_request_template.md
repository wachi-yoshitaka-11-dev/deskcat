## 結果

<!-- このpull requestで達成する結果を先に記載してください。 -->

Closes #

<!-- 作業リストの最後の項目でなく、Issueを閉じないpull requestでは`Refs: #N`を使う（CONTRIBUTINGの「関連Issueの書き方」）。 -->

## 作業リストの項目

<!-- このpull requestが埋める、Issueの作業リストの項目を書く（CONTRIBUTINGの「作業リストの決まり」）。Issueが不要な変更では「Issueなし：」と理由を書く（この節は消さない）。 -->

## 範囲

対象:

-

明示的な対象外:

-

## 変更

-

## 仕様への影響

- [ ] 仕様変更なし
- [ ] ADR
- [ ] Protocol
- [ ] GPIO
- [ ] Power
- [ ] Component identity
- [ ] Servo safety
- [ ] Toolchain／build

更新文書:

## 検証

| 確認 | 結果／証拠 |
|---|---|
| Format | 未実行／結果 |
| Lint | 未実行／結果 |
| Unit test | 未実行／結果 |
| Integration test | 未実行／結果 |
| ESP32 build | 未実行／結果 |
| 実機test | 未実行／結果 |
| 回帰確認 | 未実行／結果 |

ハードウェア構成と測定証拠:

## Dependency

- [ ] 新規dependencyなし
- [ ] 新規dependencyの必要性、support、licenseを確認した

詳細:

## 安全とsecurity

- [ ] 秘密情報またはlocal資格情報を含まない
- [ ] ハードウェア定数に正式な根拠がある
- [ ] サーボと電源の安全制限を維持する
- [ ] 不正入力・最大長超過を有界に処理する
- [ ] `unsafe`を追加していない、または別のreviewをlinkした

## 作成時の設定

- [ ] 対応するIssueと同じassigneeを設定した
- [ ] 対応するIssueと同じlabel（`area:*`／`type:*`／`priority:*`）を設定した
- [ ] Projects v2の`deskcat` boardへitemとして追加した
- [ ] `Status`を設定した
- [ ] `Start date`（作成日）を設定した
- [ ] `Target date`（mergeを見込む日）を設定した

## 自己レビュー

手順は[自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#自己レビュー)と[Pull Requestのレビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#pull-requestのレビュー)に従う。

- [ ] build・test・lintを走らせ、その出力を「検証」に示した
- [ ] `/code-review`（effortを指定し、`--max-findings all`を付けた）か`fresh-context-reviewer`で差分をレビューした
- [ ] `/security-review`を走らせた
- [ ] [慎重に扱う path](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#慎重に扱う-path)に触れる場合は、CodeRabbitに`full review`で依頼した（触れない場合は「触れない」と書く）

使ったレビューとeffort:

レビューの範囲（backtickで囲んだ`A..B`をすべて。[自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#自己レビュー)）:

巡の数:

指摘の対応（[指摘の重さと扱い](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#指摘の重さと扱い)）:

| 対応 | 件数 |
|---|---|
| 直した | |
| 指摘が誤り | |
| 意図どおり | |
| 影響が無い | |

Pre-existingの追跡Issue:

## Review thread

**この節はmerge直前に確認して更新する。**作成時点ではthreadが無いため空欄でよい。

- [ ] CodeRabbitに依頼した場合、checkの**説明文**を読み、`Review completed`であることを確かめた。`Review rate limited`と`Review skipped`は**`pass`と表示されるがreviewは走っていない**（[GitHubが強制しないもの](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#githubが強制しないもの)）
- [ ] 未解決のreview threadが0件である（GraphQLの`reviewThreads.isResolved`で確認した。REST APIのcomment一覧では判定できない）
- [ ] 承認が要る変更では、承認するcommitのSHAを書いた承認のコメントがある。[慎重に扱う path](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md#慎重に扱う-path)に触れる変更では、そのコメントの本文の全体が`承認 <承認したheadの40桁のSHA>`の1行だけで、承認の日時や範囲の記録は、この本文か別のコメントに書いた

## 消した規則

<!-- 規則を消した場合は、消した規則ごとに移した先かやめた理由を書く。消していなければ「無し」と書く。 -->

| 消した規則 | 扱い | 移した先／やめた理由 |
|---|---|---|

## 戻し方

<!-- mergeの後に戻すとき、何を戻せばよいか（revertするcommit、一緒に戻す設定やIssueの記述）。 -->

## Riskと残作業

TBD:

別環境または人間による確認:

既知の制限:
