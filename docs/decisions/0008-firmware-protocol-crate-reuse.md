# ADR-0008: Firmwareから`deskcat-protocol`をpath dependencyで再利用する

> 状態: Accepted
> 日付: 2026-08-15

## 背景

[ADR-0001](0001-monorepo-layout.md)は「Firmwareが同じcrateをpath dependencyとして直接使用するか、
小さいfirmware側実装を使用するかは、互換性spike後に決定する」として、この判断を保留した。
同ADRは「各保留事項は、その実装Issueまたは新しいADRで解決する」とも定めている。

[Issue #10](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/10)（上限付きfirmware line parser）が、
その互換性spikeを兼ねる最初の実装Issueである。#10の受け入れ条件は
「Targetが対応する範囲で共有fixtureに合格する」であり、
[protocol仕様](../protocol/esp32-pi-protocol.md) §12.1も「firmware側の合格は#10」と書いている。
つまり**この判断を先送りすると、その要求の所有者が居なくなる。**

ADR-0001は同時に「コードを共有するかどうかにかかわらず、両側が共通JSON fixtureとprotocol
conformance testに合格しなければならない」と定めている。共有するかどうかは、その義務の
達成手段の選択である。

## 判断要因

- 仕様（`docs/protocol/esp32-pi-protocol.md`）とconformance fixtureは1つであり、
  実装が2つに分かれると乖離が起きる。§12.1のfixture置き場も1箇所に固定されている
- ESP32とhostでtoolchainが異なる。両立できなければ共有は成立しない
- firmwareのflash sizeとbuild時間は有限である
- root workspaceの`exclude = ["firmware/esp32"]`は、firmware専用のtarget、`rust-toolchain.toml`、
  `.cargo/config.toml`を守るためにある。これを壊す解決策は採れない

## 検討した選択肢

### 選択肢A: firmware側に小さい独自実装を持つ

`firmware/esp32`の中に、firmware専用のparserを書く。

利点:

- host側の依存（`serde`、`serde_json`）がfirmwareへ入らない
- toolchainの制約を互いに持ち込まない

コスト:

- **同じ仕様の実装が2つになる。**§7のerror code対応表、§8の検証順序、上限値を二重に持つ
- host workspaceでtestが回らないため、共有fixtureへの合格を示しにくい。
  `firmware/esp32`は`.cargo/config.toml`が`xtensa-esp32-espidf`を指定し、
  `[[bin]] harness = false`であるため、`cargo test`をそのまま実行できない
- 乖離は「両側が同じfixtureに合格する」という要求そのものを壊す

### 選択肢B: `deskcat-protocol`をpath dependencyで共有する

`firmware/esp32/Cargo.toml`から`path = "../../crates/deskcat-protocol"`で依存する。
root workspaceの`exclude`は維持し、lockfileは2つに分かれたままにする。

利点:

- 仕様の実装が1つになる。fixtureも1箇所に置ける
- host workspaceでtestが回るため、受け入れ条件を示せる
- ADR-0001の保留事項に、実測を伴う答えが出る

コスト:

- firmwareのbuild graphへ`serde_json`が入り、flash sizeとbuild時間が増える
- **共有crateのMSRVが、両toolchainの下限に縛られる**
- `crates/`の変更がfirmwareのbuildを壊しうる

### 選択肢C: 共有部分を第3のcrateへ切り出す

`crates/deskcat-lines`のような新crateを作り、firmwareはそれだけに依存する。

利点:

- `deskcat-protocol`をstateを持たない単一line codecのまま保てる

コスト:

- ADR-0001が挙げたcrate一覧に無い名前であり、workspace境界の追加になる
- §12.1がfixtureの置き場を`crates/deskcat-protocol/tests/fixtures/`に固定しているため、
  新crateのtestは隣のcrateのdirectoryを`include_str!`することになる
- firmwareの依存先が2つになるが、`deskcat-protocol`への依存はどのみち残る
  （error codeの分類を共有するため）

## 決定

**選択肢Bを採る。**`firmware/esp32`は`deskcat-protocol`をpath dependencyで直接使用する。

- root `Cargo.toml`の`exclude = ["firmware/esp32"]`は**維持する**。
- lockfileはroot `Cargo.lock`と`firmware/esp32/Cargo.lock`の2つに分かれたままとする。
- `deskcat-protocol`の`rust-version`は、workspaceからの継承をやめ、
  **両toolchainの下限を明示する**。
- 受信器は`framing`（byteのみ、serde非依存）、`prefix`（純関数）、`receiver`（`decode_line`との結線）に
  分ける。将来firmwareがJSON層を要らないと判断した場合、`framing`だけを使う余地を残すためである。

## 影響

### 利点

- protocol実装が1つになり、共有fixtureへの合格が両側で同じ根拠になる。
- ADR-0001の保留事項が、実測を伴って解決する。
- host workspaceでtestが回るため、firmware向けのlogicをESP32 toolchainなしで開発できる。

### 欠点

- firmwareのbuild graphが増える。`firmware/esp32/Cargo.lock`への追加entryは
  `deskcat-protocol`の1件だけであった（`serde`と`serde_json`は既にlockへ解決済みだった）が、
  **実際にcompileされるのは今回が初めてである。**`cargo tree -i serde_json`が示すとおり、
  `serde_json`をbuild graphへ引き込んでいるのは`deskcat-protocol`だけである。
  flash sizeへの影響は、実機起動を伴う
  [#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)以降で測る。
- `deskcat-protocol`へhost専用の依存を足すと、firmwareのbuildが壊れる。
- 共有crateのMSRVが、より低い側のtoolchainに縛られる。

### リスクと対策

| リスク | 対策 |
|---|---|
| ESP toolchainのrustcが共有crateのMSRVを満たさない | `deskcat-protocol`の`rust-version`を両toolchainの下限で明示し、実際にその版でbuildとtestを通す。ESP toolchainの版を上げるときに再確認する |
| `crates/`の変更がfirmware buildを壊す | `.github/workflows/firmware.yml`の`paths`へ`crates/deskcat-protocol/**`を加え、CIで検知する |
| host専用のdependencyが無自覚に入る | `framing`をserde非依存に保つ。新しいdependencyは[Development Workflow](../governance/development-workflow.md)の手順で判断する |
| flash sizeが想定を超える | #6の実機bring-upで測る。超える場合は選択肢Cへ退避し、`framing`だけを共有する |

## 検証

次を満たしたとき、この決定を検証済みとする。

- [x] host workspaceで`cargo test --workspace --locked`が通る。
      根拠は[PR #127](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/127)の検証欄。
- [x] ESP toolchainのrustc（`esp-1.95.0.0` = rustc 1.95.0-nightly）で、
      host workspaceの`cargo test --workspace --locked`が通る。共有crateのMSRVの根拠である。

      ```bash
      cargo +esp-1.95.0.0 test --workspace --locked
      ```

      **このcommandのVersion Recordは無い。**Version Recordは端末とprofileの環境記録であり、
      ここで確かめたのは「その版のrustcで共有crateが通るか」という一回きりのMSRV確認である。
      ESP toolchain自体の環境は
      [2026-08-06 ESP32 Build](../toolchains/version-records/2026-08-06-esp32-build-linux.md)、
      host環境は
      [2026-08-10 Host Rust](../toolchains/version-records/2026-08-10-host-rust-linux.md)が記録している。
      実行結果はPR #127の検証欄にある。**AGENTS.mdの検証済みcommandには加えない。**
      ESP toolchainの版を上げるときは、この確認をやり直す。
- [x] `firmware/esp32`で`cargo build --locked`が通る。root workspaceの`exclude`を外さずに
      path dependencyを張れること、lockfileが2つに分かれたままで矛盾しないことを含む。
      別端末での再現は`.github/workflows/firmware.yml`が`ubuntu-24.04`で行う。
- [ ] 実機でfirmwareが起動し、共有fixtureに合格する。**起動は2026-08-20に#6で確認した**
      （[Version Record](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)）。
      **共有fixtureへの合格は未実施である。**#6の最小firmwareはProtocol sessionを確立せず、
      `Boot` messageを送っていない。**この項目は残る。**

最後の項目が満たされるまで、「両側が共有fixtureに合格した」とは言わない。
**現時点の根拠は、cross compileが通ることと、2026-08-20の実機起動記録までである**
（[Version Record](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)）。
**起動は確認したが、`Boot` messageの送出とfixture照合は行っていない。**

**flash sizeは2026-08-20の#6の実測で問題にならなかった。**release imageは381,344 bytesで、
partitionの4,128,768 bytesに対し9.24 %である（`espflash`の`App/part. size`の表示）。
**したがってこの理由での見直しは要らない。**`deskcat-protocol`がhost専用の依存を必要と
した場合は、この決定を見直す。

## 置き換える決定

なし。[ADR-0001](0001-monorepo-layout.md)の「保留した判断」のうち
「Firmwareからの`deskcat-protocol`直接再利用」を解決するものであり、ADR-0001を置き換えない。
