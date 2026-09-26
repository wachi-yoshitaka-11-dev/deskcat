## 結果

<!-- このpull requestで達成する結果を先に記載してください。 -->

Closes #

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

- [ ] 開始時に同一作業の通算巡数と人間の継続承認を確認した（差分・rebase・セッション変更でもリセットしない）
- [ ] [自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)の観点で見直し、下記の終端状態と未解決欠陥を記録した
- [ ] **要件照合Pass**と**fresh-context Pass**を、同じ最終diffに対して別々に実施した（[2つのPass](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#2つのpass)）

**自動reviewは行わない。**既定ではこれが唯一のreviewである（[ADR-0013](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/decisions/0013-manual-only-coderabbit-review.md)）。

宣言はhead commitのtrailerで行う。**checkboxとtrailerの両方が要る。**checkboxは本文の記述であり、
差分が変わっても残る。trailerはcommitへ結び付くため、差分を変えると無効になる。

対象Issue / 実行経路:

開始時の既実施巡数 / 現在の通算巡数（最終diffの確認回数とは別）:

状態（`stopped` / `capped` / `converged`）と理由 / 未解決欠陥:

最終diff hash / 同じdiffでの両Passと新規欠陥0件の巡:

人間の継続承認（なし、または判断者・判断の出所・対象Issue・承認時の通算・終了巡数・範囲）:

| 通算巡 | Passとdiff | 指摘の種類と出所 | 採否・理由・未解決 |
|---|---|---|---|
| | | defect / out-of-scope / optional; diff / prior-explanation / pre-existing | |

引き継ぎ: `review_gate.py session status`のJSONをこのPRまたは対応Issueへ保存する。
上限による停止は完了ではない。`stopped`をreceiptの終端値にしない。

## Review thread

**この節はmerge直前に確認して更新する。**作成時点ではthreadが無いため空欄でよい。

- [ ] CodeRabbitのcheckの**説明文**を読んだ。checkの色だけで判断しない。**自動reviewは行わない設定であり、`Review skipped`が既定である**（[ADR-0013](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/decisions/0013-manual-only-coderabbit-review.md)）。手動で依頼した場合は`Review completed`であること。`Review rate limited`と`Review skipped`は**`pass`と表示されるがreviewは走っていない**（[GitHubが強制しないもの](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#githubが強制しないもの)）
- [ ] reviewが走らなかった場合、[初回reviewが得られなかったとき](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#merge前の確認)の表に従った。**安全・電気・protocol・firmwareに関わる変更を自己レビューで代替しない。**その範囲の変更では、自己レビューの後で手動でreviewを依頼したかを確認した
- [ ] 未解決のreview threadが0件である（GraphQLの`reviewThreads.isResolved`で確認した。REST APIのcomment一覧では判定できない）
- [ ] 未解決を残す場合は追跡Issueを起票し、下欄と該当threadへ番号を記載した

確認commandと未解決を残す場合の手順は[Merge前の確認](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#merge前の確認)に従う。

追跡Issue:

## Riskと残作業

TBD:

別環境または人間による確認:

既知の制限:
