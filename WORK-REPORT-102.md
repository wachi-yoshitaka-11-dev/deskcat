# 実行報告: #102 — `build.rs`の`IDF_PATH` guard 実機検証

- Issue: [#102](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/102)
- 実行日: 2026-08-11（UTC 11:08〜11:14）
- 対象commit: `develop` の `e20c2a2`
- 実行者: AIエージェント（Linux端末セッション）
- 指示書: `WORK-INSTRUCTIONS-102.md`

このファイルは報告用であり、リポジトリの正本文書ではない。`git` には追加していない。

---

## 1. 結論

**guard のロジックは正しい。ただし指示書の5シナリオは、guard を一度も検証していなかった。**

| # | 論点 | 判定 |
|---|---|---|
| 1 | **guard が通常の build を壊すか**（最大の懸念） | **壊さない。**`IDF_PATH` 未設定の build は `exit=0` |
| 2 | `~/export-esp.sh` が `IDF_PATH` を設定するか | **設定しない**（推測ではなく実測で確認） |
| 3 | guard の override 判定 | **正しい。**`1` 完全一致のみ通し、`0`／`false` は通さない |
| 4 | guard は実 build で発火するか | **発火する**（`IDF_PATH` が実在する ESP-IDF を指す場合） |
| 5 | 指示書の5シナリオが guard を検証したか | **していない。**`esp-idf-sys` が先に落ちて結論が決まっていた |
| 6 | 空文字を未設定として扱う配慮 | **実 build では到達しない。**comment が実態と食い違う |

**#99 の merge を止める理由は、この検証結果には見当たらない。** 懸念されていた「guard が全 build を panic で止める」は起きなかった。

---

## 2. 実行環境

```text
Date: 2026-08-11
Repository commit: e20c2a2（develop == origin/develop、ahead 0 / behind 0）
Build対象のsource tree: fb4425a82a0ceedca9cd6beb5016551d7a0ef8fa
Working tree clean: tracked に内容差分なし。untracked は作業指示 file 1件のみ（git管理外）
Machine profile: ESP32 Build
OS name: Ubuntu
OS version: 22.04.5 LTS / kernel 5.15.0-187-generic
CPU architecture: x86_64
Container / VM / native: VM（systemd-detect-virt = microsoft）
Rust channel: esp-1.95.0.0（rust-toolchain.toml による override）
Rust compiler version: rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0) / LLVM 21.1.3
  commit-hash: 95e5bda868c960c607597bc03ed9e8f0ad26226d
Cargo version: cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
IDF_PATH present: no
IDF_TOOLS_PATH present: no
```

> **注意: 本記録は VM 上の実行であり、実機 Linux の根拠にはならない。**
> 既存 Version Record と同一環境である。[Machine Profiles](docs/toolchains/machine-profiles.md) は
> ESP32 の build-only 検証を VM／container でも認めているため、#102 の範囲（flash を含まない）は満たす。

### `~/export-esp.sh` の実測内容

```text
export LIBCLANG_PATH="<home>/.rustup/toolchains/esp-1.95.0.0/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
export PATH="<home>/.rustup/toolchains/esp-1.95.0.0/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"
```

**2行のみで、`IDF_PATH` は設定されていない。** Version Record が `IDF_PATH present: no` としていた根拠は、
これまで推測だったが、本検証で実測に置き換わった。

### 準備手順の変更（指示元へ報告）

指示された準備 command のうち **`git reset --hard origin/develop` は実行していない。**
`AGENTS.md`／`CLAUDE.md` の「Git と公開」が `git reset --hard` と強制 checkout を明示的に禁止しているためである。

そして今回は不要であった。実測で `HEAD` ／ `develop` ／ `origin/develop` がいずれも `e20c2a2` で
ahead 0 / behind 0、tracked file の内容差分は 0 件であった。
`git status` に出ていた 44 件の `M` は**全件が `mode change 100644 => 100755`** で、共有 mount 由来の
file mode（`744`）の見え方の差であり、中身は 1 byte も違わない。

`git config core.fileMode false`（local 設定のみ、破壊なし）で status を空にして検証し、
終了時に元の `filemode = true` を復元した。

---

## 3. 手順1: 環境と識別子の採取

```text
--- date ---
2026-08-11T11:08:57Z
--- HEAD ---
e20c2a2
--- firmware tree ---
fb4425a82a0ceedca9cd6beb5016551d7a0ef8fa
--- worktree clean? ---
?? WORK-INSTRUCTIONS-102.md
--- toolchain ---
esp-1.95.0.0 (overridden by '<home>/deskcat/firmware/esp32/rust-toolchain.toml')
rustc 1.95.0-nightly (95e5bda86 2026-04-15) (1.95.0.0)
binary: rustc
commit-hash: 95e5bda868c960c607597bc03ed9e8f0ad26226d
commit-date: 2026-04-15
host: x86_64-unknown-linux-gnu
release: 1.95.0-nightly
LLVM version: 21.1.3
cargo 1.95.0-nightly (f2d3ce0bd 2026-03-21) (1.95.0.0)
--- IDF_PATH in env ---
IDF_PATH=[<unset>]
```

**最終行は `IDF_PATH=[<unset>]`。指示書の「想定どおり」に該当する。**
guard が現在の開発手順を壊しているという事態は発生していない。

---

## 4. 6シナリオの結果

| # | 条件 | 期待 | 実測 | 判定 |
|---|---|---|---|---|
| 1 | 未設定 | `exit=0` | `exit=0`（44.39s） | **一致** |
| 2 | 設定・override無し | 非0 ＋ guardのpanic message | `exit=101`、**guardのpanic message無し** | **不一致** |
| 3 | 設定・`=1` | panic出ない／`cargo:warning`出る | panic出ない、**`cargo:warning`も出ない**、`exit=101` | **不一致** |
| 4 | 設定・`=0` | 非0 ＋ guardのpanic message | `exit=101`、**guardのpanic message無し** | **不一致** |
| 4b | 設定・`=false`（追加） | 非0 ＋ guardのpanic message | `exit=101`、**guardのpanic message無し** | **不一致** |
| 5 | 空文字 | `exit=0` | **`exit=101`** | **不一致** |

シナリオ4b は指示書に無いが、**Issue #102 の受け入れ条件が「`=0` および `=false` で通らないこと」を
明示している**ため追加した。

**シナリオ2・4・4b は「保護が働いていない」ではない。** build は止まっている。
止めたのが guard ではなく `esp-idf-sys` であった、という不一致である。

### シナリオ1: `IDF_PATH`未設定 → 通る

```bash
env -u IDF_PATH cargo build --locked
```

```text
2026-08-11T11:09:11Z
   Compiling esp-idf-sys v0.37.2
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
   Compiling esp-idf-hal v0.46.2
   Compiling esp-idf-svc v0.52.1
    Finished `dev` profile [optimized + debuginfo] target(s) in 44.39s

real	0m44.791s
exit=0
```

### シナリオ2: `IDF_PATH`設定・override無し → 止まる

```bash
IDF_PATH=/tmp/fake-idf cargo build --locked 2>&1 | tail -20
```

```text
2026-08-11T11:10:05Z
   Compiling esp-idf-sys v0.37.2
warning: esp-idf-sys@0.37.2: Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
error: failed to run custom build command for `esp-idf-sys v0.37.2`

Caused by:
  process didn't exit successfully: `<home>/deskcat/firmware/esp32/target/debug/build/esp-idf-sys-a536e28a16c467d7/build-script-build` (exit status: 1)
（中略: esp-idf-sys の BuildConfig dump）
  Using custom user-supplied esp-idf repository at '/tmp/fake-idf' (detected from env variable `IDF_PATH`)
  Error: Could not install esp-idf

  Caused by:
      0: could not determine esp-idf version from '/tmp/fake-idf/tools/cmake/version.cmake'
      1: No such file or directory (os error 2)
exit=101
```

**全出力56行を検索したが、guard の panic message は1件も存在しない。**

```text
$ grep -n "It overrides the ESP_IDF_VERSION|panicked|DESKCAT_ALLOW_EXTERNAL_IDF_PATH" <全出力>
(guardのpanic messageは全出力に存在しない)
```

### シナリオ3: `IDF_PATH`設定・`=1` → guardは止めない

```bash
IDF_PATH=/tmp/fake-idf DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1 cargo build --locked 2>&1 \
  | grep -nE "IDF_PATH|warning|error|panicked|Finished" | tail -25
```

```text
2026-08-11T11:10:45Z
2:warning: esp-idf-sys@0.37.2: Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
3:error: failed to run custom build command for `esp-idf-sys v0.37.2`
16:  cargo:rerun-if-env-changed=IDF_PATH
20:  cargo:warning=Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
51:  Using custom user-supplied esp-idf repository at '/tmp/fake-idf' (detected from env variable `IDF_PATH`)
56:      1: No such file or directory (os error 2)
exit=101
```

指示書の期待表に対する判定は次のとおり。

| 見るもの | 期待 | 実測 |
|---|---|---|
| `warning: IDF_PATH=/tmp/fake-idf overrides the pinned ESP-IDF...` | 出る | **出ない** |
| `IDF_PATH is set to /tmp/fake-idf. It overrides the ESP_IDF_VERSION...` | 出ない | 出ない |

`exit=0` にならないこと自体は指示書が想定済みだが、**確認したかった「guard が止めなかったこと」は
このシナリオでは判定できない。** guard がそもそも実行されていないためである。
20行目の `cargo:warning` は `esp-idf-sys` 自身のもので、guard のものではない。

### シナリオ4: `=0` → 止まる

```bash
IDF_PATH=/tmp/fake-idf DESKCAT_ALLOW_EXTERNAL_IDF_PATH=0 cargo build --locked 2>&1 | tail -20
```

```text
2026-08-11T11:10:54Z
（シナリオ2と同一の esp-idf-sys 失敗）
  Using custom user-supplied esp-idf repository at '/tmp/fake-idf' (detected from env variable `IDF_PATH`)
  Error: Could not install esp-idf

  Caused by:
      0: could not determine esp-idf version from '/tmp/fake-idf/tools/cmake/version.cmake'
      1: No such file or directory (os error 2)
exit=101
```

### シナリオ4b: `=false` → 止まる（Issue #102 受け入れ条件、指示書に無い追加分）

```bash
IDF_PATH=/tmp/fake-idf DESKCAT_ALLOW_EXTERNAL_IDF_PATH=false cargo build --locked 2>&1 | tail -20
```

```text
2026-08-11T11:11:00Z
（シナリオ2・4と同一の esp-idf-sys 失敗）
exit=101
```

### シナリオ5: `IDF_PATH`が空文字 → **期待に反して落ちた**

```bash
IDF_PATH= cargo build --locked
```

```text
2026-08-11T11:11:05Z
   Compiling esp-idf-sys v0.37.2
warning: esp-idf-sys@0.37.2: Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
error: failed to run custom build command for `esp-idf-sys v0.37.2`
（中略）
      idf_path: Some(
          "",
      ),
（中略）
  Using custom user-supplied esp-idf repository at '' (detected from env variable `IDF_PATH`)
  Error: Could not install esp-idf

  Caused by:
      0: could not determine esp-idf version from 'tools/cmake/version.cmake'
      1: No such file or directory (os error 2)

real	0m0.287s
exit=101
```

**`esp-idf-sys` は `IDF_PATH=` を「未設定」ではなく「空 path の custom repository」と解釈する**（`idf_path: Some("")`）。

**これは guard が起こした回帰ではない。** 失敗は `esp-idf-sys` の build script 内で完結しており、
`deskcat-esp32` は compile すらされていない。guard を削除しても同じ結果になる。

---

## 5. 切り分けA: build script 単体での検証

シナリオ2〜5 で guard が実行されなかったため、`cargo` を介さず、
シナリオ1 で生成された build script の実体を直接起動して guard 単体の挙動を確認した。

```bash
BS=$(ls -t target/debug/build/deskcat-esp32-*/build-script-build | head -1)
env -u IDF_PATH "$BS"
IDF_PATH=/tmp/fake-idf "$BS"
...
```

| 条件 | exit | 出力 | 判定 |
|---|---|---|---|
| 未設定 | `0` | `cargo:rerun-if-env-changed=` 2行のみ | 設計どおり |
| `/tmp/fake-idf`・override無し | `101` | `panicked at build.rs:26:9` ＋ guard message | 設計どおり |
| `=1` | `0` | `cargo:warning=IDF_PATH=/tmp/fake-idf overrides the pinned ESP-IDF. Record this override in the Version Record.` | 設計どおり |
| `=0` | `101` | `panicked at build.rs:26:9` ＋ guard message | 設計どおり |
| `=false` | `101` | `panicked at build.rs:26:9` ＋ guard message | 設計どおり |
| 空文字 | `0` | 出力なし | 設計どおり |

guard の panic message 全文（実測）:

```text
thread 'main' panicked at build.rs:26:9:
IDF_PATH is set to /tmp/fake-idf. It overrides the ESP_IDF_VERSION pinned in .cargo/config.toml, so this build would not match the recorded toolchain. Unset IDF_PATH, or set DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1 (exactly "1") and record the override in a Version Record.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

`cargo:rerun-if-env-changed=IDF_PATH` と `=DESKCAT_ALLOW_EXTERNAL_IDF_PATH` の2行は全 case で出力されている。

**6分岐すべてが `build.rs` の設計どおりである。**

---

## 6. 切り分けD: `IDF_PATH` に実在する ESP-IDF を指定した実 build

「guard が実 build で発火することはあるのか」を決着させるため、
`.embuild/espressif/esp-idf/v5.5.3`（既に workspace へ展開済みの実在する ESP-IDF）を `IDF_PATH` に指定した。
**新しい path は作成していない。**

### D1: 実在ESP-IDF・override無し

```bash
IDF_PATH="$PWD/.embuild/espressif/esp-idf/v5.5.3" cargo build --locked
```

```text
2026-08-11T11:12:13Z
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
error: failed to run custom build command for `deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)`
  thread 'main' panicked at build.rs:26:9:
  IDF_PATH is set to <home>/deskcat/firmware/esp32/.embuild/espressif/esp-idf/v5.5.3. It overrides the ESP_IDF_VERSION pinned in .cargo/config.toml, so this build would not match the recorded toolchain. Unset IDF_PATH, or set DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1 (exactly "1") and record the override in a Version Record.

real	0m28.881s
exit=101
```

**guard が実 build で発火した。** `esp-idf-sys` を通過した後、`deskcat-esp32` の build script で停止している。

### D2: 実在ESP-IDF・`=1`

```bash
IDF_PATH="$PWD/.embuild/espressif/esp-idf/v5.5.3" DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1 cargo build --locked
```

```text
2026-08-11T11:12:54Z
   Compiling deskcat-esp32 v0.1.0 (<home>/deskcat/firmware/esp32)
warning: deskcat-esp32@0.1.0: IDF_PATH=<home>/deskcat/firmware/esp32/.embuild/espressif/esp-idf/v5.5.3 overrides the pinned ESP-IDF. Record this override in the Version Record.
    Finished `dev` profile [optimized + debuginfo] target(s) in 10.95s

real	0m10.986s
exit=0
```

**override が意図どおり機能し、記録用の `cargo:warning` も出力された。**

> **この2 case の限界:** 指定した ESP-IDF は pin 済みと同じ v5.5.3 である。
> guard は path の存在も版も見ないため、発火の確認としては十分だが、
> **「記録と異なる版の外部 SDK で build される事態を防げる」ことの証明ではない。**
> 別版の ESP-IDF を用意した検証は実施していない。

---

## 7. 分析: なぜ指示書のシナリオで guard が実行されなかったか

`esp-idf-sys` は `deskcat-esp32` の依存であり、**依存の build script が先に実行される。**
`esp-idf-sys` は `IDF_PATH` を検出すると、`.cargo/config.toml` が pin した `ESP_IDF_VERSION` を破棄して
その path を custom repository として使う。

```text
warning: esp-idf-sys@0.37.2: Ignoring configuration setting `ESP_IDF_VERSION` (Tag v5.5.3): custom esp-idf repository detected via $IDF_PATH
```

したがって `IDF_PATH` の指す先で挙動が二分される。

| `IDF_PATH` の指す先 | 何が起きるか | guard は |
|---|---|---|
| 実在しない path（`/tmp/fake-idf`、空文字） | `esp-idf-sys` が版を判定できず停止 | **実行されない** |
| 実在する ESP-IDF | `esp-idf-sys` は成功。`deskcat-esp32` の build script へ到達 | **発火する** |

**指示書は「guard は path の存在を見ない」から `/tmp/fake-idf` を実在させない設計にしていたが、
`esp-idf-sys` は path の存在を見る。** ここが検証設計の盲点であった。

実害の評価:

- **意図しない外部 SDK で build される事態は防げている。**実在する外部 ESP-IDF が指定された場合、
  guard が発火して build を止める（D1 で実証）
- 実在しない path の場合も **build は止まる**ので安全側に倒れている。
  劣るのは診断 message の分かりやすさだけである
- 依存の build script の実行順は Cargo が決めるため、**`build.rs` 側では解決できない**

---

## 8. Issue #102 受け入れ条件への対応

| 受け入れ条件 | 結果 |
|---|---|
| `IDF_PATH`未設定 → buildが通る | **確認済み**（シナリオ1、`exit=0`） |
| `IDF_PATH`設定・override未設定 → buildがpanicで止まる | **条件付きで確認。**実在 ESP-IDF なら guard の panic で停止（D1）。実在しない path では `esp-idf-sys` が先に停止し、guard は実行されない（シナリオ2） |
| `IDF_PATH`設定・`=1` → buildが通り`cargo:warning`が出る | **条件付きで確認。**実在 ESP-IDF で `exit=0` ＋ guard の `cargo:warning`（D2）。実在しない path では到達しない（シナリオ3） |
| `=0` および `=false` で通らないこと | **確認済み。**build script 単体で両方とも `exit=101` ＋ guard message（切り分けA）。実 build でも `exit=101` |
| 空文字は未設定として扱われること | **成立しない。**guard 単体では未設定扱いだが（切り分けA）、`esp-idf-sys` が空文字を空 path の custom repo と解釈して先に失敗する（シナリオ5） |
| 結果を Version Record へ記録する | 未実施（Windows 側で実施） |
| 記録に command と panic message を含める | 本報告に全文を掲載済み |
| `docs/toolchains/` 索引の最終検証日を更新する | 未実施（Windows 側で実施） |

---

## 9. `build.rs` の修正要否

**必須の修正は無い。**懸念された「guard が通常の build を壊す」は発生せず、
override 判定も設計どおりである。ただし次の2点は実態と食い違っている。

### (a) 空文字に関する comment が実態と合わない

`firmware/esp32/build.rs:17-20`:

```rust
// 空文字は未設定として扱う。shellの初期化で`export IDF_PATH=`だけが走る場合がある。
let external = std::env::var("IDF_PATH")
    .ok()
    .filter(|p| !p.trim().is_empty());
```

この配慮は**実 build では到達しない。**`export IDF_PATH=` だけが走った端末では、
guard の手前で `esp-idf-sys` が停止するため、いずれにせよ build できない。

選択肢:

| 案 | 内容 | 評価 |
|---|---|---|
| A | 何もしない | 実害なし。ただし comment が読み手を誤解させ続ける |
| B | comment を訂正し、「空文字は `esp-idf-sys` 側が先に失敗させる」と明記 | 最小の変更で誤解を解ける |
| C | `.filter()` を外し、空文字も guard で止める | **実態に最も合う。**どのみち build できないので、`esp-idf-sys` の分かりにくい error より guard の明示的な message で止める方が親切。`build.rs` の comment も素直になる |

**C を推奨する。** ただし挙動上の実害は無いため、B に留める判断も妥当である。

### (b) 文書が「guard が最初に止める」ように読める

`firmware/esp32/README.md:44` と `docs/runbooks/esp32-development-machine-setup.md:211` は
**`build.rs` が build を止める**と記述している。実際には `IDF_PATH` の指す先によって
`esp-idf-sys` が先に別の error で止めることがある。1行の補足を入れると実態に合う。

いずれも**指示元（Windows 側）の判断に委ねる。**本セッションでは `build.rs` を変更していない。

---

## 10. Version Record 転記用

### 記録本体へ反映する項目

```text
最終有効な検証日時: 2026-08-11T11:13:22Z
Repository commit: e20c2a2
Build対象のsource tree: fb4425a82a0ceedca9cd6beb5016551d7a0ef8fa
  （従来値 cf3fcdb4... から変更。#101 で build.rs と firmware/esp32/README.md が変わったため）
Working tree clean: no
  tracked file に内容差分は無い。untracked の作業指示 file 1件（git管理外）のみが残る
IDF_PATH present: no（shell環境。~/export-esp.sh が設定しないことを実測で確認した）
Generated artifact identity:
  path: firmware/esp32/target/xtensa-esp32-espidf/debug/deskcat-esp32
  type: ELF 32-bit LSB executable, Tensilica Xtensa, version 1 (SYSV), statically linked, with debug_info, not stripped
  size: 13,660,584 bytes
  sha256: 54a2adec71f0af045f880af867a36c9cfcf732196cbf7765f65aa02f3ffef375
  実行日時: 2026-08-11T11:13:22Z（IDF_PATH 未設定の復帰build）
Cargo.lock: unchanged
```

`sha256` は既存記録の `0f9d4918...` と異なる。source tree が変わっているうえ、
**入力を揃えても一致しないことは既存記録の「再現性実験log」で測定済み**であり、矛盾しない。

### 追記する節の案

`## IDF_PATH guard の検証log（#102 追記）` として、本報告の 3〜7 節を転記する。
**実施していない項目を「成功」と記録しないこと。** とくに次は明記が必要である。

- シナリオ2・3・4・4b・5 は **guard を検証できていない**（`esp-idf-sys` が先に停止）
- guard の発火確認は、`IDF_PATH` が**実在する ESP-IDF** を指す場合のみ（D1／D2）
- D1／D2 で指定した ESP-IDF は pin 済みと**同じ v5.5.3** であり、
  別版の外部 SDK を防げることの証明にはなっていない
- 本記録は **VM 上**であり、実機 Linux の根拠にならない

---

## 11. 実行しなかった検証

- **flash、serial monitor、実機接続、通電、配線**（#102 の範囲外。指示書の「絶対に守ること」に従い一切実施せず）
- `cargo fmt` / `cargo clippy`（#102 は guard の挙動確認が対象のため未実行）
- `cargo clean` からの full clean build（cache を利用した incremental build のみ）
- **別版の ESP-IDF を指定した guard 検証**（用意していない）
- 実機 Linux（bare metal）での再現（本セッションは VM）
- Version Record と `docs/toolchains/` 索引の更新、Pull Request 作成（Windows 側で実施予定）

---

## 12. 残存リスクと TBD

| 項目 | 状態 |
|---|---|
| `esp-idf-sys` が guard より先に走る順序依存 | **未解決。**Cargo が決める順序のため `build.rs` では解決できない。実害は診断 message の分かりやすさに限られる |
| 別版の外部 ESP-IDF に対する guard の有効性 | **未検証。**guard は path の存在も版も見ないため理屈上は発火するが、実測していない |
| 空文字 `IDF_PATH` の扱い | **実態と comment が不一致。**9-(a) の対応待ち |
| 実機 Linux での再現 | **未実施。**本記録は VM |
| HW-TBD-001（回路図と現物pin表記の照合、chip刻印） | 既存 TBD。本作業の範囲外 |

---

## 13. 環境の後始末

- `/tmp/fake-idf` は**作成していない。**embuild も作成していないことを確認済み（`No such file or directory`）
- 検証後に `IDF_PATH` 未設定で復帰 build を実行し、pin 済み SDK による成果物を再生成した（`exit=0`、36.82s）
- `Cargo.lock` は unchanged
- **branch 作成・`git add`・commit・push・Pull Request は一切行っていない**
- `git config core.fileMode` は元の `filemode = true` を復元し、session 開始時と同じ状態に戻した。
  `~/.gitconfig` は無変更（mtime 2026-08-06）
- `firmware/esp32/build.rs` を含め、**追跡下のファイルは1件も変更していない**

> 補足: `~/.gitconfig` には元から `core.fileMode = false` があり、repository local の `true` が
> それを上書きしている。local 側を削除すれば 44件の mode 差分は恒久的に消えるが、
> 実際の実行 bit 変更も無視されるようになるため、元の状態へ戻してある。
> 恒久的に消したい場合は `git config --unset core.fileMode` の一行で切り替えられる。
