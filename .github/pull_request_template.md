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

## 自己レビュー

- [ ] [自己レビュー](../CONTRIBUTING.md#自己レビュー)の観点で見直し、新規指摘0件が2 round続いた

自動reviewを行わないPull Requestでは、これが唯一のreviewである。

## Review thread

**この節はmerge直前に確認して更新する。**作成時点ではthreadが無いため空欄でよい。

- [ ] 自動review（CodeRabbit）の**説明文**を読んだ。checkの色だけで判断しない。allowlistのlabelを持つPull Requestは`Review completed`であること。`Review rate limited`と`Review skipped`は**`pass`と表示されるがreviewは走っていない**（[GitHubが強制しないもの](../CONTRIBUTING.md#githubが強制しないもの)）
- [ ] `Review skipped: excluded by label configuration`は意図した対象外である。rate limitとその他のskipは、再依頼するか自己レビューのみで通す判断を本文へ書いた
- [ ] 未解決のreview threadが0件である（GraphQLの`reviewThreads.isResolved`で確認した。REST APIのcomment一覧では判定できない）
- [ ] 未解決を残す場合は追跡Issueを起票し、下欄と該当threadへ番号を記載した

確認commandと未解決を残す場合の手順は[Merge前の確認](../CONTRIBUTING.md#merge前の確認)に従う。

追跡Issue:

## Riskと残作業

TBD:

別環境または人間による確認:

既知の制限:
