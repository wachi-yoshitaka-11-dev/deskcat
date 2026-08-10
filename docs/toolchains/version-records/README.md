# Version Records

実際の開発端末で確認した環境記録を置く。様式は [Version Record Template](../version-record-template.md) に従う。

## 記録一覧

file 名と Record ID は初回検証日で固定し、以後の再検証でも変更しない。最新の検証日時は各記録の `最終有効な検証日時` を参照する。

| 記録 | Profile | 判定 | 初回検証日 | 最終検証日 |
|---|---|---|---|---|
| [ESP32 Build (Linux x86_64)](2026-08-06-esp32-build-linux.md) | ESP32 Build | `Partial` | 2026-08-06 | 2026-08-08 |

## 規則

- 一つの記録は、一台の端末と一つの profile に対応させる。ある端末の成功記録を、別端末や別 profile の根拠にしない。
- 判定は `Verified`、`Partial`、`Failed`、`Incompatible` のいずれかで結論づける。未実行の項目が残る場合は `Partial` とし、何が未達かを記録内に明記する。
- 秘密情報、個人名、端末名、個人の絶対 path、USB serial を記録しない。build log を載せる場合は、home directory の path を `<home>` のような表記へ置換する。
- Markdown 以外の拡張子で証拠 file を置かない。`docs/` 配下は Markdown だけが公開対象であり、それ以外は未公開として報告される。log は記録本文へ埋め込む。
