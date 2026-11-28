# Workflow

buildを実行するworkflowは、そのbuild commandが確定してから追加する。
再現できないbuildを、検証したとworkflowで主張しない。

**この文書が並べる command は、workflow が実際に実行するものの記述である。**
**開発者が実行する command の正本は[検証済みコマンド](../../docs/toolchains/verified-commands.md)であり、
正本が変わったら workflow とこの記述を合わせる**（[ADR-0018](../../docs/decisions/0018-instruction-file-structure.md)）。

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
- Pull Requestでは、指示entrypointの検査（`validate_instruction_entrypoint.py`）とそのtest（`test_instruction_entrypoint.py`）、link検査（`validate_doc_links.py`）、validator自身のtest（`test_link_validators.py`）、Pages sourceの生成、Jekyll build、生成物の検査（`validate_pages_output.py`）、公開境界の回帰test（`test_pages_guards.py`）を行う。**deployはしない。**
- `main`で同じcheckに成功した場合だけPages artifactをdeployする。
- 通常権限はread-onlyとし、deploy jobだけに`pages: write`と`id-token: write`を付与する。
- Actionはreview済みのcommit SHAへ固定する。

`paths`は`**/*.md`を含む。`validate_doc_links.py`が追跡下の**全**Markdownを検査するため、
top-level directoryを列挙すると検査対象と起動条件がずれる。実際に`apps/`、`crates/`、
`firmware/`等の14 fileが対象外で、component READMEだけの変更は検査を素通りしていた。

### `promotion-closing-keywords.yml` — 昇格Pull Requestのclosing keywordの報告

- base が`main`のPull Requestだけで起動する。
- 範囲の各commitに含まれる`Closes #N`等を`scripts/report_promotion_closing_keywords.py`で検出し、**報告だけ行う。**mergeを止めない。
- 範囲の各commitを読むため`fetch-depth: 0`、tokenをscriptから読めないよう`persist-credentials: false`を指定する。

### `wiring-figure-sync.yml` — 配線の記録と配線図の同期確認

- base が`develop`のPull Requestだけで起動する。**`main`への昇格では走らせない。**範囲に多くのcommitを含み、`develop`で通った変更を再検査しても得るものが無く、過去の範囲のずれで昇格が止まるためである。
- `docs/hardware/experiment-log.md`の`Wiring revision`の行が追加・変更され、
  `hardware/breadboard/layout.svg`が同じ範囲で変わっていなければ、`scripts/check_wiring_figure_sync.py`が落とす（[#512](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/512)）。
- 図に関係しない変更は、Pull Request本文の`Wiring-Figure: not-applicable <理由>`
  （理由10字以上）だけで通す。commit messageは読まない。Pull Request本文は環境変数で渡し、宣言の行を探すためだけに読む。
- Pull Request本文の編集（`edited`）でも走り直す。落ちた後に本文へ宣言を足せば通る。
- 判定は字句だけである。図の内容が記録と合うかは判定しない。
- merge baseを求めるため`fetch-depth: 0`、tokenをscriptから読めないよう`persist-credentials: false`を指定する。

### `review-coverage.yml` — レビューの範囲のつながり

- base が`develop`のPull Requestだけで起動する。昇格の commit は squash で入っており、記録の SHA と別物なので、範囲でつなげない。
- Pull Request本文の「自己レビュー」に書いた範囲（backtickで囲んだ`A..B`）が、baseとのmerge-baseからheadまで途切れずにつながっているかを、`scripts/review_coverage.py`で見る（[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの9）。
- 手元の`scripts/hooks/gh_metadata_guard.py`は同じ判定に加えて、本文の範囲が手元のlogの完了した記録と一致するかも見る。CIは手元のlogを読めないので、本文だけを見る。所有者が画面からmergeする経路では、この job だけが見る。
- Pull Request本文の編集（`edited`）でも走り直す。PRのコメントは使わない（`issue_comment`のworkflowは`main`の版で動くため）。
- merge-baseと祖先の関係を求めるため`fetch-depth: 0`、tokenをscriptから読めないよう`persist-credentials: false`を指定する。

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
