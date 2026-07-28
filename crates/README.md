# 共通Rust crate

このディレクトリには、host workspace用のlibraryを置く。

| 予定crate | 責務 |
|---|---|
| `deskcat-domain` | 感情、接触event、行動判断、純粋ロジック |
| `deskcat-protocol` | ESP32–Pi間のmessage型とserialization |
| `deskcat-config` | 型付き設定と検証 |
| `deskcat-api` | 文章生成の境界とfallback動作 |
| `deskcat-storage` | snapshot、event log、rotation、復旧 |

各Cargo packageは、最初の着手可能なIssueを実装するときに作成する。
