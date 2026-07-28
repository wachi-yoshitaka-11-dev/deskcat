# Hardware Bill of Materials

> 状態: Draft — 正確なmoduleは現物確認が必要
> 正本とする情報: 部品識別情報と一次資料

## 規則

- 正確なメーカー、model、suffix、module boardの識別情報を記録する。
- 候補例を採用済み部品として扱わない。
- メーカーのデータシートとmodule回路図を個別にリンクする。
- deviceの識別情報が`TBD`の間はdriver開発を開始しない。
- 部品識別に使用した現物の表示または写真referenceを記録する。
- 部品を交換する場合は、既存行を黙って変更せず新しいBOM revisionとして記録する。

## 状態ラベル

| 状態 | 意味 |
|---|---|
| `Selected` | 初期DeskCat製作で使用する予定 |
| `Required` | 機能は必要だが、正確な部品は未選定 |
| `Candidate` | 評価中 |
| `Deferred` | 初期MVPには含めない |
| `Not used` | 初期製作の対象外と明示済み |

## 計算機とstorage

| Ref | 機能 | 部品／module | 状態 | 正確なメーカー／suffix | 公式文書 | 電源 | Peak電流 | 根拠／残作業 |
|---|---|---|---|---|---|---|---|---|
| MCU-01 | Real-time I/O controller | ESP32-DevKitC-32E | Selected | Board revisionと搭載module suffixはTBD | TBD | Board文書から確認するためTBD | TBD | 実boardの両面を確認する |
| SBC-01 | 高水準controller | Raspberry Pi Zero WH | Selected | Board revisionはTBD | TBD | 5 V入力。connectorと電流予算はTBD | TBD | 実boardと電源経路を確認する |
| SD-01 | Piのbootとstorage | microSD | Required | 容量、endurance class、状態はTBD | メーカー資料はTBD | SBCから供給 | TBD | 既存cardの確認または購入 |

## Display、入力、sensor

| Ref | 機能 | 部品／module | 状態 | 正確なIC／module | Interface | Address／mode | 電源／logic | 公式文書 | 残作業 |
|---|---|---|---|---|---|---|---|---|---|
| DISP-01 | LCDの顔 | LCD module | Required | TBD | SPI／I2C／parallelはTBD | TBD | TBD | TBD | controller表示、module、解像度、pinを確認する |
| TOUCH-01 | 撫で操作／選択入力 | Capacitive touch module／controller | Required | TBD | GPIO／I2C／SPIはTBD | TBD | TBD | TBD | 現物moduleの確認または選定 |
| ACCEL-01 | 軽打／持ち上げ／姿勢 | Accelerometer module | Required | TBD | I2C／SPIはTBD | TBD | TBD | TBD | 表示とaddress設定を確認する |
| ENV-01 | 温度／湿度／気圧 | Environmental sensor module | Required | TBD | I2C／SPIはTBD | TBD | TBD | TBD | 表示と対応測定量を確認する |
| COLOR-01 | 環境色 | Color sensor module | Deferred | TBD | TBD | TBD | TBD | TBD | 初期MVPでは選定しない |

部品候補は、正確な現物識別と公式文書の確認後に追加する。候補例や類似部品を採用済みとして扱わない。

## 駆動系と電源

| Ref | 機能 | 部品／module | 状態 | 正確なmodel | 定格電圧 | Peak／stall電流 | 公式文書 | 残作業 |
|---|---|---|---|---|---|---|---|---|
| SERVO-01 | 首振り | Servo | Required | TBD | 外部5 V系をproject方針とする。正確な定格はTBD | TBD | TBD | model、表示、horn、機械的可動域を確認する |
| PSU-PI-01 | Piとlogic電源 | Regulated wired supply | Required | TBD | TBD | TBD | TBD | connector、regulation、供給可能電流を確認する |
| PSU-SERVO-01 | サーボ電源 | Regulated supplyまたはDC/DC | Required | TBD | 5 V候補。正確なサーボ定格に従う | TBD | TBD | 実際のpeak電流から容量を決める |
| PROTO-01 | 試作用配線 | Breadboard／wire／connector | Required | TBD | 回路に従う | 回路に従う | 該当製品資料 | 許容電流とconnector方向を確認する |

## 初期製作の明示的な対象外

| 部品 | 状態 | 理由 |
|---|---|---|
| Arduino Uno | Not used | 初期のreal-time controllerはESP32とする |
| Raspberry Pi Pico | Not used | 初期のreal-time controllerはESP32とする |
| Camera | Not used | Projectのprivacyとscopeに関する決定 |
| Microphone | Not used | Projectのprivacyとscopeに関する決定 |
| Audio output | Not used | 静かなdesktop動作を維持する |

## 部品受け入れchecklist

各行を`Selected`へ変更する前に、次を確認する。

- [ ] 現物に記載された正確な表示を読んだ
- [ ] Module boardの識別情報が明確である
- [ ] Revision／dateを含む公式データシートへリンクした
- [ ] Module回路図または検証済みpinoutへリンクした
- [ ] 供給電圧とlogic電圧が明確である
- [ ] 通常電流とpeak電流が明確、または測定予定がある
- [ ] Connector方向とpin 1が明確である
- [ ] Bus設定と起動sequenceが明確である
- [ ] 必要なpull-up、capacitor、level shiftingが明確である
- [ ] 入手性と代替リスクを記録した
- [ ] `gpio-assignment.md`と`power-budget.md`を更新した

## Revision履歴

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-07-27 | 0 | 初期project方針からinventoryを作成。周辺部品の正確なmodelは引き続きTBD | [DeskCat マイコン開発技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)、ADR-0001 |
