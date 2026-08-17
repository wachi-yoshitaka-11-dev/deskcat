# 共通Rust crate

このディレクトリには、host workspace用のlibraryを置く。

| crate | 責務 | 状態 |
|---|---|---|
| [`deskcat-protocol`](deskcat-protocol/README.md) | ESP32–Pi間のmessage型、serialization、byte列からの上限付き受信 | 作成済み（#9、#10） |
| [`deskcat-serial`](deskcat-serial/README.md) | host側のserial session。上限付きI/O、切断の観測、再接続の上限とrate limit、上限のある送信queue。**実deviceのopenは含まない** | 作成済み（#11） |
| `deskcat-domain` | 感情、接触event、行動判断、純粋ロジック | 予定 |
| `deskcat-config` | 型付き設定と検証 | 予定 |
| `deskcat-api` | 文章生成の境界とfallback動作 | 予定 |
| `deskcat-storage` | snapshot、event log、rotation、復旧 | 予定 |

各Cargo packageは、最初の着手可能なIssueを実装するときに作成する。作成したcrateはroot `Cargo.toml`の`members`へ追加する。
