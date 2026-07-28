# 計画milestone

このfileは、GitHubに作成するmilestoneのlocal定義元である。

## M0 Development Foundation

説明: リポジトリ安全、Governance、構成、ハードウェア正本文書、GitHubテンプレート、初期バックログを整備する。

完了条件:

- repository安全基準をreview済み
- Governanceと`AGENTS.md`が有効
- ADR-0001とrepository構成が存在
- ハードウェア情報の正式文書が存在
- READMEとGitHub templateが存在
- 初期backlogが準備済み

## M1 ESP32 Bring-up

説明: 対応するツールチェーンを固定し、最小ファームウェアをビルド／書き込みして、起動診断とheartbeatの根拠を記録する。

完了条件:

- 対応するRust／ESP-IDF toolchainを固定
- 最小firmwareのbuildとflashに成功
- 起動logにfirmware identityとreset reasonを出力
- power cycleをまたいでheartbeatが動作
- debug／release sizeの証拠を保存

## M2 ESP32–Pi Protocol

説明: 上限付きJSON Lines、`boot`、`ping`、`status`、ACK、error、duplicate、再接続動作を実装・検証する。

完了条件:

- 正式なv1 fixtureが存在
- hostとfirmwareが最大長制限付きJSON Linesをparse
- `boot`、`ping`、`get_status`、ACK、error経路が動作
- 分割、結合、不正、最大長超過、重複、reconnect testに成功

## M3 Display and Input

説明: 正確なLCD、タッチ、加速度センサ、環境センサを立ち上げ、観測可能な障害処理を実装する。

完了条件:

- LCDが承認済みMVP表情を描画
- touchから検証済みpetting eventを生成
- 加速度sensorから検証済みtap eventを生成
- sensor faultが分離され観測可能

## M4 Servo Integration

説明: 電源・動作制限を測定して強制上限を適用し、fail-safe動作を検証する。

完了条件:

- 電源と機械制限を実測
- 強制動作制限を適用
- 承認済み動作がbrownoutなしで動作
- 通信断、reset、緊急停止を検証

## M5 DeskCat MVP

説明: 撫で、軽打、表情、安全動作、独り言、再接続のMVPシナリオを統合する。

完了条件:

- pettingにより喜び表情と安全動作を生成
- 軽いtapにより驚き表情を生成
- local fallback付きのアイドル独り言が動作
- reconnect後にPiとESP32の同期が復旧
- MVP受け入れscenarioを記録

## M6 Reliability

説明: 長時間動作、電源再投入、切断、リソース、故障注入、展開、復旧を検証する。

完了条件:

- 目標時間の長時間testに成功
- power cycleとdisconnectのfault injectionに成功
- queue、memory、parser、sensor、reset counterが制限内
- deploymentとrecovery runbookを検証
