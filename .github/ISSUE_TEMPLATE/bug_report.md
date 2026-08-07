---
name: 不具合報告
about: 再現可能な誤動作を報告する
labels: "type:bug"
assignees: ""
---

## 概要

<!-- 一つの誤動作を説明してください。秘密情報を含めないでください。 -->

## 期待する動作

## 実際の動作

## 再現手順

1.

再現率:

## 範囲

対象:

対象外:

## 環境

- Git commit:
- Firmware version／build:
- Host version／build:
- ESP32 board／revision:
- Raspberry Pi model／OS:
- 影響を受ける正確な部品:
- 配線revision:
- 電源／電流制限:
- Toolchain／SDK:

問題に影響しないことが明らかな項目だけ`N/A`としてください。

## 完全な証拠

<!-- 失敗前から復旧までの完全なerrorと関連logを添付し、秘密情報を除去してください。 -->

```text

```

測定値・capture:

## 変更と試行

- 最後に正常だった状態:
- 関連する直近の変更:
- 実施済みの試行:

## 受け入れ条件

- [ ] 再現をtestまたは手順として記録した
- [ ] root causeまたは限定された失敗条件を特定した
- [ ] 修正後に対象確認が通る
- [ ] 関連する回帰確認が通る
- [ ] 必要なハードウェア証拠を記録した

## 安全

- [ ] 資格情報や非公開脆弱性の詳細を含んでいない
- [ ] 危険な動作があった場合はactuator電源を切った
- [ ] 追加の再現が安全かつ範囲限定されている

## 起票時の設定

- [ ] milestoneを設定した
- [ ] `priority:*` labelを1つ設定した
- [ ] assigneeを設定した
- [ ] Projects v2の`deskcat` boardへ追加し、`Status`を設定した
