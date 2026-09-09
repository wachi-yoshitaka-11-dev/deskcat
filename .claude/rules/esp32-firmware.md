---
paths:
  - "firmware/esp32/**"
  - "crates/deskcat-protocol/**"
---

# ESP32 firmware を触るとき

**コマンドと来歴の正本は[検証済みコマンド](../../docs/toolchains/verified-commands.md)である。**
実行前に開く。**ここへコマンドを写さない。**

踏みやすいものだけを挙げる。

- **`firmware/esp32` は root workspace から `exclude` している。**`--workspace` に入らない。
- **`crates/deskcat-protocol/` を変更したら、host だけでなく ESP32 build も回す。**
  同 crate の `rust-version` は host と ESP toolchain の両方を満たす下限であり、**上げると
  firmware の build が compile 前に停止する**（[ADR-0008](../../docs/decisions/0008-firmware-protocol-crate-reuse.md)）。
- **ESP32 build は ESP32 Build profile の端末でしか実行できない**（[Machine Profiles](../../docs/toolchains/machine-profiles.md)）。
  **一台で成功した build を、別端末でも検証済みと扱わない。**
- **flash と実機起動は build とは別の主張である。**build が通ったことを実機の根拠にしない。
