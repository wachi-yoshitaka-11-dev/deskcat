# 共通Rust crate

このディレクトリには、host workspace用のlibraryを置く。

| crate | 責務 | 状態 |
|---|---|---|
| [`deskcat-protocol`](deskcat-protocol/README.md) | ESP32–Pi間のmessage型とserialization | 作成済み（#9） |
| `deskcat-domain` | 感情、接触event、行動判断、純粋ロジック | 予定 |
| `deskcat-config` | 型付き設定と検証 | 予定 |
| `deskcat-api` | 文章生成の境界とfallback動作 | 予定 |
| `deskcat-storage` | snapshot、event log、rotation、復旧 | 予定 |

各Cargo packageは、最初の着手可能なIssueを実装するときに作成する。作成したcrateはroot `Cargo.toml`の`members`へ追加する。
