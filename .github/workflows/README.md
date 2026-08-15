# Workflow

buildを実行するworkflowは、そのbuild commandが確定してから追加する。
再現できないbuildを、検証したとworkflowで主張しない。

追加する順序は次のとおりである。**`#1`から`#4`まですべて実施済みである。**

| # | 対象 | 状態 |
|---|---|---|
| 1 | root Cargo workspace作成後のhost format、lint、unit test | **`host.yml`で実施**（[#129](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/129)） |
| 2 | protocol fixture test | **`host.yml`で実施**（[#129](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/129)）。fixture testは`crates/deskcat-protocol/tests/`にあり`cargo test --workspace`に含まれるため、`#1`と同じworkflowで満たす |
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

### `host.yml` — host workspace

- **実機、firmware、ESP-IDF、ESP32 toolchainを触らない。**`firmware/esp32`はroot workspaceから
  `exclude`されているため、このworkflowでは検証されない。firmware側は`firmware.yml`が担当する。
- repository rootで`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked`、
  `cargo test --workspace --locked`を実行する。計画表の`#1`と`#2`を1本で満たす。
- `-D warnings`は付けない。lintの水準はroot `Cargo.toml`の`[workspace.lints]`が持ち、
  `clippy::all`が既に`deny`である。commandのoptionで二重に持つと、片方だけ変えたときに水準が食い違う。
- **Rustの版はworkflow内で明示導入する。**runner image既定の版に任せると、
  root `Cargo.toml`が「実際にbuildとtestを通した版だけを宣言する」と定めている`rust-version`と、
  実際に通した版がずれる。上げるときはworkflowの値と[Version Record](../../docs/toolchains/version-records/2026-08-15-host-rust-ci.md)を同時に変える。
- **サードパーティActionを使わない。**rustupはrunner imageに入っているためshellだけで導入でき、
  `patterns_allowed`（＝repository設定）の変更が発生しない。
- 権限は`contents: read`だけである。`persist-credentials: false`でcheckoutがtokenを残さない。

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
