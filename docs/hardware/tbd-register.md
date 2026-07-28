# Hardware TBD Register

> 状態: Active
> 目的: 安全な実装を妨げる未確定情報を追跡する

## 優先度

| 優先度 | 意味 |
|---|---|
| P0 | 電源、配線、または最初の関連driverを妨げる |
| P1 | Featureの受け入れまたは安全な統合を妨げる |
| P2 | 後のmilestoneまで保留できる |

## 未解決項目

| ID | 優先度 | 不足している情報／判断 | 必要な根拠 | 妨げる対象 | Owner | 状態 |
|---|---|---|---|---|---|---|
| HW-TBD-001 | P0 | 正確なESP32 board revisionとmodule suffix | 現物確認＋公式board回路図 | 最終GPIO割り当て | Human | Open |
| HW-TBD-002 | P0 | 正確なLCD module／controller | 現物確認＋公式文書 | LCD driver、SPI pin、電源 | Human | Open |
| HW-TBD-003 | P0 | 正確なtouch module／controller | 現物確認＋公式文書 | Touch driver、pin、gestureしきい値 | Human | Open |
| HW-TBD-004 | P0 | 正確なaccelerometer module／IC | 現物確認＋公式文書 | Accelerometer driverとしきい値 | Human | Open |
| HW-TBD-005 | P0 | 正確なenvironmental sensor module／IC | 現物確認＋公式文書 | Environment driverとbus計画 | Human | Open |
| HW-TBD-006 | P0 | 正確なservo model | 現物確認＋公式データシート | 電源容量とPWM | Human | Open |
| HW-TBD-007 | P0 | Logic電源とservo電源のmodel | 表示／仕様＋配線計画 | 初回統合通電 | Human | Open |
| HW-TBD-008 | P0 | GPIO割り当て | Board／module回路図＋競合review | すべてのhardware driver | Joint | 001–006によりBlocked |
| HW-TBD-009 | P0 | 電源予算とbackfeed review | 部品電流＋回路図＋測定計画 | Servoと全体統合 | Joint | 001–007によりBlocked |
| HW-TBD-010 | P1 | サーボの機械的可動域とneutral | 監視下calibration | 首振り動作の受け入れ | Human | 006–009によりBlocked |
| HW-TBD-011 | P1 | サーボの速度／加速度制限 | 電流・動作試験 | Motion profile | Joint | 010によりBlocked |
| HW-TBD-012 | P1 | Touch gestureのしきい値 | 取得したraw sample | 撫で動作の受け入れ | Joint | 003、008によりBlocked |
| HW-TBD-013 | P1 | 軽打／持ち上げしきい値 | サーボ動作を含む取得済みraw sample | 軽打／持ち上げ動作の受け入れ | Joint | 004、008によりBlocked |
| HW-TBD-014 | P1 | 最終serial baudと最大line length | Pi／ESP32 transport test | Protocol v1の受け入れ | Joint | 候補値あり |
| HW-TBD-015 | P1 | Pi microSDの識別情報と状態 | 現物確認／health check | Deployと耐久性 | Human | Open |
| HW-TBD-016 | P2 | Color sensorの識別情報と役割 | MVP review＋部品選定 | 将来の環境色feature | Human | Deferred |

## 解決手順

1. 現物の正確な表示を記録する。
2. 公式文書を添付またはリンクする。
3. 関連する制限を正本文書へ記録する。
4. 文書だけでは不十分な場合は必要な測定を実行する。
5. 実験記録をリンクする。
6. 影響するGPIO、電源、protocol、安全文書を更新する。
7. 関連Issueをblockedからreadyへ変更する。
8. 解決referenceを付けてTBD行をcloseする。履歴は削除しない。

## 解決済み項目

| ID | 解決内容 | 根拠 | Close日 |
|---|---|---|---|
| — | — | — | — |
