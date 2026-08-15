# Version Record: host workspace format／lint／test（CI）

> 対象: [#129](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/129)
> 位置づけ: 開発端末での記録を、**別環境で再現したかどうか**の記録

host workspace の検証は、これまで開発端末でしか行っていなかった。
[2026-08-10 の記録](2026-08-10-host-rust-linux.md)と[2026-08-15 の実機記録](2026-08-15-host-rust-native-linux.md)は
いずれも 1 台ずつの端末であり、`AGENTS.md` も「別端末での再現は未検証である」と書いていた。

本記録は、Pull Request [#130](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/130) の tree を
GitHub Actions の `ubuntu-24.04` runner で実行した結果である。

**run は commit ごとに走る。**`host.yml` の `paths` に文書を含めていないにもかかわらず
文書だけの commit でも走るのは、`pull_request` の `paths` 判定が「その commit の差分」ではなく
**「Pull Request 全体の差分」**に対して行われるためである。Pull Request が
`.github/workflows/host.yml` を含む限り、以後の commit すべてで再 run する。

| # | run | commit | 内容 | 結果 | 所要 |
|---|---|---|---|---|---|
| 1 | 31891323975 | f16472b9 | `host.yml` の追加 | success | 31 s（cache 無し） |
| 2 | 31891594404 | a7ae254a | 文書と本記録の追加 | success | 20 s（cache 有り） |

**本記録は run 2 を根拠とする。**run 2 以降の commit は本記録を含む**文書だけ**であり、
`host.yml`、`Cargo.toml`、`Cargo.lock`、`crates/` に差分は無い。
したがって run 2 の結果は、この Pull Request の最終状態に対しても有効である。
（同じ扱いを [ESP32 Build (CI)](2026-08-10-esp32-build-ci.md) も採っている。）

## 記録

```text
Record ID: 2026-08-15-host-rust-ci
Date: 2026-08-15
最終有効な検証日時: 2026-08-15T15:02:50Z（run 31891594404、commit a7ae254a）
Machine profile: CI（[Machine Profiles](../machine-profiles.md)）
Operator role: 自動実行（人間の介在なし）
Repository commit: a7ae254a（Pull Request #130 の最終 head）
Working tree clean: yes（checkout 直後。workflow は tree を変更しない）

OS name: Ubuntu
OS version: 24.04（runner image `ubuntu-24.04`、image 20260810.271.1、provisioner 20260729.566）
CPU architecture: x86_64
Userspace bitness: 64
Container / VM / native: VM（GitHub-hosted runner）

Rustup version: runner image 既定（記録せず）
Rust channel: 1.97.1（workflow が `rustup toolchain install 1.97.1` で明示導入し default にする）
Rust compiler version: rustc 1.97.1 (8bab26f4f 2026-07-14)
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu（`--profile minimal`。追加 target なし）
Cargo version: cargo 1.97.1 (c980f4866 2026-06-30)
rustfmt version: rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
Clippy version: clippy 0.1.97 (8bab26f4f6 2026-07-14)
Linker identity and version: runner image 既定の cc／ld を使用（host build のため明示導入なし）

ESP32 only: 該当なし（このworkflowはESP32 toolchainを導入せず、`firmware/esp32`をbuildしない）
Raspberry Pi only: 該当なし

Commands run:
  rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy
  rustup default 1.97.1
  rustc --version / cargo --version / cargo fmt --version / cargo clippy --version
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --locked
  cargo test --workspace --locked
Expected result: format、lint、test の 3 command がすべて成功する
Actual result: 3 command すべて成功。test は 76 passed / 0 failed
  内訳: lib 52 / conformance 11 / framing 5 / limits 5 / doc 3
Build duration: 20 s（run 全体。15:02:30Z 開始、15:02:50Z 終了）。1 回目は cache 無しで 31 s
Peak memory if measured: 未測定
Storage delta if measured: 未測定
Generated artifact identity: 未取得。**本記録は artifact の同一性を主張しない**
Log or evidence path: GitHub Actions run 31891594404（および 31891323975）
  https://github.com/wachi-yoshitaka-11-dev/deskcat/actions/runs/31891594404
Known differences from documented profile:
  - 実機 Linux ではなく GitHub-hosted の VM である
  - `--profile minimal` のため、開発端末に入っている rust-docs 等は入っていない
Conclusion: Verified（host workspace の検証済み command 3 つがすべて成功した）
Next action: required status check の必須化を別途判断する
未実行・未測定の項目:
  - peak memory と storage delta（template では任意項目）
  - 実機、flash、serial monitor（CI profile の対象外）
```

## 開発端末との一致

**compiler の実体が開発端末と同一である。**

| 項目 | 開発端末（2026-08-10） | CI（本記録） |
|---|---|---|
| rustc | 1.97.1 (8bab26f4f 2026-07-14) | 1.97.1 (8bab26f4f 2026-07-14) |
| cargo | 1.97.1 (c980f4866 2026-06-30) | 1.97.1 (c980f4866 2026-06-30) |
| host | x86_64-unknown-linux-gnu | x86_64-unknown-linux-gnu |

**これは偶然ではなく、workflow が版を固定しているためである。**
runner image の既定版に任せると、記録した版と違うもので通ったことになる。
版を上げるときは workflow の値と本記録を同時に変える。

**同一版で通ったことは「版を変えても通る」ことを意味しない。**本記録が示すのは、
同じ commit と同じ lockfile が別の環境（GitHub-hosted VM）で通ることである。

## 「検証の移送」との対応

[Machine Profiles](../machine-profiles.md) の「検証の移送」が別端末へ求める項目を、本記録が満たすかを示す。

| 求められる項目 | 本記録 |
|---|---|
| OS と CPU architecture | Ubuntu 24.04 / x86_64 |
| container、VM、実機のどれか | **VM**（GitHub-hosted runner） |
| toolchain と target | Rust 1.97.1 / x86_64-unknown-linux-gnu |
| linker と SDK | runner image 既定の cc／ld。SDK 不要 |
| repository commit | a7ae254a |
| lockfile が変更されていないこと | `--locked` が成功 |
| clean build の結果 | 成功（fresh runner） |

同文書は「host Rust の build、test、lint」について **Docker 上の Linux を使ってよい**と定めており、
USB と実機を触らないためこの記録は profile の想定内である。

## 制限

- **実機動作を一切主張しない。**このworkflowは`firmware/esp32`も実機も触らない
- runner image は GitHub が更新する。将来の run が同じ image とは限らない
- **`cargo test` は host 上の test だけである。**Raspberry Pi 上での build と実行は
  [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) の範囲であり、本記録は何も主張しない
- **cache の効果は 2 点しか測っていない。**cache 無し 31 s、cache 有り 20 s である。
  試行 1 回ずつであり、runner 側のばらつきと区別できない。**この 2 点から比率を主張しない**
- peak memory と storage delta は未測定である
