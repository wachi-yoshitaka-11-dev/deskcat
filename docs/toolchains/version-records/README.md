# Version Records

実際の開発端末で確認した環境記録を置く。様式は [Version Record Template](../version-record-template.md) に従う。

## 記録一覧

file 名と Record ID は初回検証日で固定し、以後の再検証でも変更しない。最新の検証日時は各記録の `最終有効な検証日時` を参照する。

| 記録 | Profile | 判定 | 初回検証日 | 最終検証日 |
|---|---|---|---|---|
| [ESP32 Build (Linux x86_64)](2026-08-06-esp32-build-linux.md) | ESP32 Build | `Partial` | 2026-08-06 | 2026-08-11 |
| [ESP32 Build (CI)](2026-08-10-esp32-build-ci.md) | CI | `Partial` | 2026-08-10 | 2026-08-10 |
| [Host Rust Development (Linux x86_64)](2026-08-10-host-rust-linux.md) | Host Rust Development | `Verified` | 2026-08-10 | 2026-08-10 |
| [Host Rust Development (実機 Linux x86_64)](2026-08-15-host-rust-native-linux.md) | Host Rust Development | `Verified` | 2026-08-15 | 2026-08-15 |
| [ESP32 Build (実機 Linux x86_64)](2026-08-15-esp32-build-native-linux.md) | ESP32 Build | `Partial` | 2026-08-15 | 2026-08-15 |
| [host workspace format／lint／test (CI)](2026-08-15-host-rust-ci.md) | CI | `Verified` | 2026-08-15 | 2026-08-15 |

**同じ profile の記録が複数あるのは、端末が違うためである。**`2026-08-06`と`2026-08-10`は VM 上、
`2026-08-15`の2件は実機で取得した。`Container / VM / native:`が異なるため、
下の規則に従い別記録としている。**片方をもう片方の根拠にしない。**

`2026-08-15`の2件は同じ端末だが profile が違うため、これも別記録である。

CI の記録は開発端末の記録を置き換えない。**別環境で再現したことの記録**であり、
[ESP32 Rust Toolchain](../esp32-rust-toolchain.md) の確定条件のうち
「別の開発端末または clean environment で再現した」に対応する。
host workspace 側も同じ位置づけで、`2026-08-15-host-rust-ci` が開発端末の 2 記録を再現している。

## 規則

- 一つの記録は、一台の端末と一つの profile に対応させる。ある端末の成功記録を、別端末や別 profile の根拠にしない。
- 判定は `Verified`、`Partial`、`Failed`、`Incompatible` のいずれかで結論づける。未実行の項目が残る場合は `Partial` とし、何が未達かを記録内に明記する。
- 秘密情報、個人名、端末名、個人の絶対 path、USB serial を記録しない。build log を載せる場合は、home directory の path を `<home>` のような表記へ置換する。
- Markdown 以外の拡張子で証拠 file を置かない。`docs/` 配下は Markdown だけが公開対象であり、それ以外は未公開として報告される。log は記録本文へ埋め込む。
