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

### `review-gate.yml` — 分類と自己レビューの宣言確認

- base が`develop`と`main`のPull Requestで起動する。
- 含まれる変更の分類と、Pull Request head commitのtrailer（分類、自己レビュー、指示source変更の
  宣言）を`scripts/review_gate.py`で検証する（[ADR-0010](../../docs/decisions/0010-change-class-and-review-declaration.md)）。
- **`develop`側でも掛ける。**宣言が無効になる仕組みは、Pull Requestのhead commitを見て初めて
  働く。`main`側だけに掛けると、宣言はsquash commitにしか現れない。
- **base が`main`のPull Requestでは、範囲の各commitの宣言も検証する**（`history`）。
  範囲に複数のsquash commitが入るため、head commitだけでは宣言を持たないcommitが混ざっても
  通る。**`develop`側では実行しない。**feature branchの中間commitへ宣言を要求しない。
- **未解決threadと必要CIは検証しない。**branch protectionが強制しており、同じ条件を2箇所で持たない。
- commit messageのtrailerを読むため`fetch-depth: 0`でcheckoutする。
- 権限はread-onlyとし、tokenをscriptから読めないよう`persist-credentials: false`を指定する。

### `declaration-audit.yml` — 共有branchへ入った後の宣言確認

- `main`と`develop`への**push**で起動する。**Pull Requestでは起動しない。**
- pushされた範囲（`before..after`）の各commitを`scripts/review_gate.py history`で検証する。
- **`review-gate.yml`との違いは見る位置である。**あちらはPull Requestを見るが、
  **squash commitへtrailerが載るかは merge を実行する経路が決める。**
  `gh_metadata_guard.py`はBash tool経由の`gh pr merge`しか見ないため、
  GitHub MCPの`merge_pull_request`やweb UIからのmergeは素通りする
  （[#373](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/373)で実際に起きた）。
- **阻止ではなく事後検出である。**pushは既に完了している。
  **`main`昇格の直前ではなく、原因を作ったmergeの直後に気付けるようにする**のが目的である
  （[ADR-0021](../../docs/decisions/0021-declaration-audit-on-push.md)）。
- `before`が解決できない場合（branch作成、force push後）は、**3つの状況を分ける。**
  **どの経路でも直前の1 commitへ縮めない。**縮めると、force pushが複数のcommitを
  持ち込んだときに中間のcommitを1つも見ない（[ADR-0021](../../docs/decisions/0021-declaration-audit-on-push.md)の`検証`で実測）。

  | `git merge-base <after> origin/main` | 意味 | 見る範囲 |
  |---|---|---|
  | head と一致する | **`main`自身への push** | 直前の1 commit。**範囲は昇格 Pull Request で検査済みである** |
  | 空を返す | **`origin/main`と共通祖先が無い** | `review_gate.py history --from-root`。**head から辿れるcommitをすべて見る** |
  | それ以外 | 通常の branch 作成・force push | `main`へ入っていない範囲をすべて見る |

  **2つに畳まない。**共通祖先が無い場合を「`main`への push」と同じ扱いにすると、
  中間のcommitを1つも見ない（[#385](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/385)で実測）。
  `--from-root`では`--base`は`CLASS`の計算にだけ使う。**`--not <起点>`は変わらず効くため、
  起点より前を検査しない規則は弱まらない。**
- `fetch-depth: 0`、`persist-credentials: false`は`review-gate.yml`と同じ理由である。
- **`cancel-in-progress`を有効にしない。**pushごとに範囲が違うため、後のpushで
  前の範囲の検査を打ち切ると、その範囲を誰も見なくなる。

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
