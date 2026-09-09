---
paths:
  - "crates/**"
  - "apps/**"
  - "simulator/**"
  - "Cargo.toml"
  - "Cargo.lock"
---

# host workspace と Raspberry Pi の build

**コマンド、来歴、実測値の正本は[検証済みコマンド](../../docs/toolchains/verified-commands.md)である。**
実行前に開く。**ここへコマンドと数値を写さない。**

踏みやすい点だけを挙げる。**値は正本が持つ。**

- **host の clippy に `-D warnings` を付けない。**水準は root `Cargo.toml` の `[workspace.lints]` が持つ。
- **Raspberry Pi では `-p` で 1 crate ずつ絞る。**memory の実測値と、`--workspace` が未検証であることは正本が持つ。
- **Pi 上で検証済みなのは 2 crate だけである。**どの crate かは正本が持つ。他で通ることは主張しない。
- **CI が実行するのは host workspace だけである。**Raspberry Pi 上の build と実行は CI では担保されない。
