# Workflow

buildを実行するworkflowは、そのbuild commandが確定してから追加する。
再現できないbuildを、検証したとworkflowで主張しない。

追加する順序は次のとおりである。**実施済みは`#3`と`#4`で、`#1`と`#2`は未着手である。**

| # | 対象 | 状態 |
|---|---|---|
| 1 | root Cargo workspace作成後のhost format、lint、unit test | 未着手。workspaceが未生成 |
| 2 | protocol fixture test | 未着手。[#9](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/9)待ち |
| 3 | toolchain固定後のESP32 cross-build | **`firmware.yml`で実施**（[#42](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/42)） |
| 4 | toolとversion選定後の文書検査 | **`pages.yml`で実施**（[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)） |

## 現在のworkflow

### `pages.yml` — 公開文書

- Rust、ESP-IDF、firmware、実機をbuildしない。
- Pull Requestでは、link検査（`validate_doc_links.py`）、validator自身のtest（`test_link_validators.py`）、Pages sourceの生成、Jekyll build、生成物の検査（`validate_pages_output.py`）、公開境界の回帰test（`test_pages_guards.py`）を行う。**deployはしない。**
- `main`で同じcheckに成功した場合だけPages artifactをdeployする。
- 通常権限はread-onlyとし、deploy jobだけに`pages: write`と`id-token: write`を付与する。
- Actionはreview済みのcommit SHAへ固定する。

`paths`は`**/*.md`を含む。`validate_doc_links.py`が追跡下の**全**Markdownを検査するため、
top-level directoryを列挙すると検査対象と起動条件がずれる。実際に`apps/`、`crates/`、
`firmware/`等の14 fileが対象外で、component READMEだけの変更は検査を素通りしていた。

### `firmware.yml` — ESP32 firmware

- `firmware/esp32`で`cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo build --locked`を実行する。
- 権限は`contents: read`だけである。secretとwrite権限を渡さない。
- Xtensa toolchainは`esp-rs/xtensa-toolchain`をcommit SHAへ固定して導入する。
  **GitHub所有ではない唯一のActionであり、`patterns_allowed`へ個別に許可している**
  （理由と確認日は[Repository設定](../REPOSITORY_SETTINGS.md)）。
- `name`は`rust-toolchain.toml`の`channel`と一致させる。不一致だとrustupがcompile前に停止する。
- `.embuild`はcacheしない。理由はworkflow内のコメントを参照する。
- flashとserial monitorは実行しない。

required status checkの必須化はまだ行っていない。CIの安定を確認してから別途判断する。

Hardware-in-the-Loop testは通常のhosted CIから分離する。
