# Raspberry Pi Rust Toolchain

> 状態: 調査済み。Raspberry Pi実機は未検証
> 調査日: 2026-07-27
> 更新: 2026-08-15 float ABIの判定基準と`arm-unknown-linux-gnueabihf`の適用条件を追加（[#62](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/62)）。**実機確認は未実施**
> 対象board: Raspberry Pi Zero W（V1.1。ヘッダなし版にpin headerをハンダ付けした個体）

## 結論

Raspberry Pi Zero W は Zero 2 W とは異なる。公式 product information では BCM2835、single-core Arm11 / Armv6、512 MB の機種である。したがって、64-bit Arm や Armv7 を前提にしない。

32-bit Raspberry Pi OS と hard-float ABI を実機で確認できた場合、Rust の candidate target は次である。

```text
arm-unknown-linux-gnueabihf
```

Rust 公式の platform support では、この target は Armv6 Linux hard-float とされている。target 名の architecture 部分 `arm` は Armv6 を指し、同資料は Armv6 の実装例として Raspberry Pi Zero の BCM2835 が積む ARM1176JZF-S を挙げている（[Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html) の Architecture Component）。実機の OS、ABI、glibc が一致するまでは確定しない。

## `arm-unknown-linux-gnueabihf` を適用してよい条件

次をすべて満たしたときに限り、この target を適用してよい。一つでも満たせない、または確認できない項目があれば適用しない。**`eabihf` の実行物は `eabi` の system では正しく動作しない**ため、条件 3 の判定を省略できない（[Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html) の ABI Component）。

| # | 条件 | 確認手段 |
|---|---|---|
| 1 | board が Raspberry Pi Zero W である | 現物の silkscreen と `/sys/firmware/devicetree/base/model` |
| 2 | userspace が 32-bit である | `getconf LONG_BIT` が `32` |
| 3 | float ABI が hard-float である | [runbook の float ABI 判定](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定)。複数手段の結果が一致すること |
| 4 | CPU architecture が Armv6 相当である | `uname -m` と `readelf -A` の `Tag_CPU_arch`。target 名の `arm` が Armv6 を指す |
| 5 | libc が glibc である | `ldd --version` と `getconf GNU_LIBC_VERSION` |
| 6 | `rustc -Vv` の `host` が `arm-unknown-linux-gnueabihf` である | 1 から 5 と矛盾しないこと |

条件 3 と条件 4 は独立している。**hard-float であることは Armv6 であることを意味しない。**条件 3 だけを根拠に条件 4 を満たしたと扱うと、Armv7 前提の target と取り違える。根拠となる出力は [runbook の float ABI 判定](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定)にある。

**これは適用してよいかを判断する gate であり、target の確定ではない。**確定は [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) の範囲である。条件を満たせない場合、または ABI を判別できない場合の扱いは [runbook](../runbooks/raspberry-pi-development-machine-setup.md#abiを判別できなかったとき)に従い、[「候補 toolchain」表](#候補-toolchain)の状態欄を `ABI 確認待ち` のまま据え置く。

## 初期方針

最初は Raspberry Pi Zero W 上の native build を試す。

理由:

- target、linker、glibc の組合せを単純にできる。
- 実際の OS で build と run を同時に確認できる。
- 初期の小さな service では cross toolchain の保守を先に増やさない。

Pi Zero W は CPU と RAM が限られるため、build 時間と storage 使用量を必ず記録する。実用上困難と確認できた場合だけ cross compilation を別 Issue で導入する。

## 実機で確定する情報

次は物理 Raspberry Pi と起動中の OS から取得する。

| 項目 | 目的 |
|---|---|
| Board model と revision | Zero W と Zero 2 W の取り違え防止。現物は`Raspberry Pi Zero W V1.1`（[hardware-bom.md](../hardware/hardware-bom.md) SBC-01） |
| OS name、release、32/64 bit | 配布物と support の基準 |
| `uname -m` | kernel architecture |
| `getconf LONG_BIT` | userspace bitness |
| float ABI（hard-float / soft-float） | 候補 target の適用可否。判定手順は [runbook](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定) |
| `readelf -A` の `Tag_CPU_arch` | Armv6 と Armv7 の区別。float ABI とは別の条件 |
| libc と version | GNU target の互換性 |
| 空き storage と memory | native build の実用性 |
| Rust host triple | rustup が選択した host |
| Rust、Cargo、linker version | 再現性 |

端末名、ユーザー名、IP address、Wi-Fi 情報、machine ID は記録しない。

## 候補 toolchain

| 項目 | 候補 | 状態 |
|---|---|---|
| OS | Raspberry Pi OS 32-bit | 実機確認待ち |
| Rust channel | stable | 採用候補 |
| Native host/target | `arm-unknown-linux-gnueabihf` | ABI 確認待ち |
| Linker | OS の native GNU linker | version 確認待ち |
| Build method | Pi 上の direct build | #8 で検証 |
| Cross compilation | 保留 | direct build の計測後に判断 |

ESP32 用の `esp` toolchain を Pi service の build に使わない。

## Cross compilationへ移る条件

次のいずれかを実測し、Issue に証拠を残した場合に検討する。

- clean build が開発サイクルとして許容できない
- dependency build が memory 不足で安定しない
- storage 消費や書込み負荷が大きい
- 複数 Pi への配布を自動化する必要が生じた

cross compilation では Rust target の追加だけでなく、Armv6 hard-float に対応する linker と target libc が必要になる。build host の library へ誤って link しないことを、`file`、ELF metadata、実機実行で確認する。生成物の float ABI と `Tag_CPU_arch` は、[runbook](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定)と同じ手段で読む。

## 確定条件

- [ ] 物理 board model と revision を記録した
- [ ] OS、kernel、userspace bitness、libc を記録した
- [ ] float ABI を、判定に使った command と実出力つきで記録した
- [ ] `Tag_CPU_arch` を記録し、Armv6 であることを float ABI とは別に確認した
- [ ] Rust stable の host triple を記録した
- [ ] 最小 Rust program が Pi 上で build できた
- [ ] 生成物が同じ Pi 上で実行できた
- [ ] clean build 時間、peak memory、storage 使用量を記録した
- [ ] project の test command を記録した
- [ ] direct build を継続するか cross compilation へ移るか決定した

## 公式資料

- [Raspberry Pi computer hardware](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html)
- [Raspberry Pi product catalogue](https://datasheets.raspberrypi.com/product/product-catalogue-2023.pdf)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html)
- [Rustup installation](https://rustup.rs/)
