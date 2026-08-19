# Raspberry Pi Rust Toolchain

> 状態: 実機検証済み。native host/target を確定
> 調査日: 2026-07-27
> 更新: 2026-08-15 float ABIの判定基準と`arm-unknown-linux-gnueabihf`の適用条件を追加（[#62](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/62)）
> 更新: 2026-08-17 Raspberry Pi Zero W 実機で 8 条件すべてを確認し、`arm-unknown-linux-gnueabihf`を確定（[#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8)）。根拠は [Version Record](version-records/2026-08-17-pi-direct-build-native.md)。**依存を持つbuildとworkspace buildは未測定である**。この更新は[PR #144](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/144)として2026-08-19に`develop`へmergeした
> 対象board: Raspberry Pi Zero W（V1.1。ヘッダなし版にpin headerをハンダ付けした個体）

## 結論

Raspberry Pi Zero W は Zero 2 W とは異なる。公式 product information では BCM2835、single-core Arm11 / Armv6、512 MB の機種である。したがって、64-bit Arm や Armv7 を前提にしない。

32-bit Raspberry Pi OS と hard-float ABI を実機で確認できた場合、Rust の candidate target は次である。**2026-08-17 に実機で確認し、これを確定した。**

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
| 4 | **物理 CPU** が Armv6 である | board model と SoC の公式情報（BCM2835 の ARM1176JZF-S）。`/sys/firmware/devicetree/base/model` と条件 1 |
| 5 | 対象 system の ELF が Armv6 を超える命令セットを要求しない | `readelf -A` の `Tag_CPU_arch`。**これは ELF 側の要件であり、物理 CPU の測定値ではない** |
| 6 | libc が glibc であり version が **2.17 以上**である | `ldd --version` と `getconf GNU_LIBC_VERSION` |
| 7 | kernel が **3.2 以上**である | `uname -r`（1節では `uname -a` で採取する） |
| 8 | `rustc -Vv` の `host` が `arm-unknown-linux-gnueabihf` である | `rustc -Vv` の出力。1 から 7 のいずれとも矛盾しないこと |

条件 6 と条件 7 の下限は Rust 公式の platform support が示す値である（`arm-unknown-linux-gnueabihf` は `Armv6 Linux, hardfloat (kernel 3.2+, glibc 2.17)`）。

条件 3 は、条件 4 とも条件 5 とも独立している。**hard-float であることは Armv6 であることを意味しない。**条件 3 だけを根拠に条件 4 または条件 5 を満たしたと扱うと、Armv7 前提の target と取り違える。根拠となる出力は [runbook の float ABI 判定](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定)にある。同じ armhf でも、Raspbian の検体は `Tag_CPU_arch` が `v6`、Debian の検体は `v7` であった。

**条件 4 と条件 5 も別物である。**`Tag_CPU_arch` は ELF の build attribute であり、その binary が要求する architecture を示す。**CPU を測定した値ではないため、物理 CPU の証拠に使わない。**同様に `uname -m` は kernel 側の値であり、64-bit kernel と 32-bit userspace の組合せでは userspace の architecture を示さない。物理 CPU の根拠は board model と SoC の公式情報に置く。

**この表は適用してよいかを判断する gate であり、それ自体は target の確定ではない。**確定は [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) の範囲であった。

**2026-08-17 に #8 の実機作業で 8 条件すべてを確認し、target を確定した。**実出力は [Version Record](version-records/2026-08-17-pi-direct-build-native.md) にある。float ABI は判定手段 6 つすべてが hard-float で一致し、`Tag_ABI_VFP_args` は判定表が hard と定める `VFP registers` そのものであった。**この機体では「判定不能」に落ちなかった。**

**確定したのは検証した 1 機体についてである。**別の機体、別の OS image、再 install 後の環境では、この 8 条件を改めて確認する。条件を満たせない場合、または ABI を判別できない場合の扱いは [runbook](../runbooks/raspberry-pi-development-machine-setup.md#abiを判別できなかったとき)に従い、その機体について候補 target を適用しない。[「候補 toolchain」表](#候補-toolchain)の確定は検証済みの機体に対する記録であって、未検証の機体へ引き継げるものではない（[Machine Profiles](machine-profiles.md) の「検証の移送」と同じ扱いである）。

**実機で現れた読み違えの罠を 1 つ記録する。**この機体の `/proc/cpuinfo` は `CPU architecture: 7` と表示する。**これを Armv7 と読んではならない。**同 file の `CPU part: 0xb76` は ARM1176JZF-S であり Armv6 である。条件 4 の根拠は board model と SoC の公式情報に置き、`/proc/cpuinfo` の `CPU architecture` 行を物理 CPU の architecture の根拠に使わない。`uname -m`（この機体では `armv6l`）を根拠にしないのと同じ理由である。

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
| `readelf -A` の `Tag_CPU_arch` | 対象 system の ELF が要求する architecture。**物理 CPU の測定値ではない。**float ABI とも物理 CPU とも別の条件 |
| libc と version | GNU target の互換性 |
| 空き storage と memory | native build の実用性 |
| Rust host triple | rustup が選択した host |
| Rust、Cargo、linker version | 再現性 |

端末名、ユーザー名、IP address、Wi-Fi 情報、machine ID は記録しない。

## 候補 toolchain

状態欄は 2026-08-17 の実機検証（[Version Record](version-records/2026-08-17-pi-direct-build-native.md)）で更新した。

| 項目 | 候補 | 状態 |
|---|---|---|
| OS | Raspberry Pi OS 32-bit | 確定。Raspbian GNU/Linux 13 (trixie)、userspace 32-bit |
| Rust channel | stable | 確定。rustup 経由の stable（rustc 1.97.1） |
| Native host/target | `arm-unknown-linux-gnueabihf` | **確定**（8 条件を実機で確認） |
| Linker | OS の native GNU linker | 確定。`cc` 14.2.0、GNU ld 2.44 |
| Build method | Pi 上の direct build | 最小 program で検証済み。**依存を持つ build は未測定** |
| Cross compilation | 保留 | **保留を維持**（移行条件のいずれにも当たらない） |

ESP32 用の `esp` toolchain を Pi service の build に使わない。

**distribution package の `rustc` は使わない。**2026-08-17 時点の apt の候補版は `1.85.0+dfsg3-1+rpi1` であり、root `Cargo.toml` の `rust-version = "1.97"` を満たさない。rustup の stable は 1.97.1 でこれを満たす。

## Cross compilationへ移る条件

次のいずれかを実測し、Issue に証拠を残した場合に検討する。

- clean build が開発サイクルとして許容できない
- dependency build が memory 不足で安定しない
- storage 消費や書込み負荷が大きい
- 複数 Pi への配布を自動化する必要が生じた

2026-08-17 の実機計測では、**4 条件のうち評価できた 3 つが当たらず、残る 1 つは未評価であるため、移行する根拠が無いと判断して保留を維持する。****4 条件すべてを否定したのではない。**根拠は [Version Record](version-records/2026-08-17-pi-direct-build-native.md) にある。

| 条件 | 実測 | 判断 |
|---|---|---|
| clean build が許容できない | 最小 program の clean build は debug 4〜5 秒台、release 3.5 秒台 | 評価した。当たらない |
| dependency build の memory 不足 | **未評価。**計測した program の依存は 0 件である | **判断できない。**「当たらない」とは言えない |
| storage 消費や書込み負荷 | toolchain 約 820 MiB、`target/` 4.6 MiB、空き 24.8 GiB | 評価した。当たらない |
| 複数 Pi への配布の自動化 | 対象は 1 台 | 評価した。当たらない |

**「dependency build が memory 不足で安定しない」は未評価のまま残る。**依存 0 件の program しか測っていないため、426 MiB の機体で依存を伴う build が通るかはこの計測から言えない。cross compilation の判断を確定させるには、依存を持つ crate の build 計測が別途必要である。

cross compilation では Rust target の追加だけでなく、Armv6 hard-float に対応する linker と target libc が必要になる。build host の library へ誤って link しないことを、`file`、ELF metadata、実機実行で確認する。生成物の float ABI と `Tag_CPU_arch` は、[runbook](../runbooks/raspberry-pi-development-machine-setup.md#float-abiの判定)と同じ手段で読む。

## 確定条件

記録は [Version Record](version-records/2026-08-17-pi-direct-build-native.md) にある。

- [x] 物理 board model と revision を記録した
- [x] OS、kernel、userspace bitness、libc を記録した
- [x] float ABI を、判定に使った command と実出力つきで記録した
- [x] `Tag_CPU_arch` を記録し、ELF が Armv6 を超える命令セットを要求しないことを float ABI とは別に確認した
- [x] 物理 CPU が Armv6 であることを board model と SoC の公式情報で確認した（`Tag_CPU_arch` と `uname -m` を物理 CPU の根拠にしない）
- [x] kernel version と glibc version が Rust 公式の下限（kernel 3.2、glibc 2.17）を満たすことを記録した
- [x] Rust stable の host triple を記録した
- [x] 最小 Rust program が Pi 上で build できた
- [x] 生成物が同じ Pi 上で実行できた（reboot 後も実行できた）
- [x] clean build 時間、peak memory、storage 使用量を記録した
- [ ] project の test command を記録した — **未実施。**#8 の範囲は最小 Rust program であり、このrepositoryのcrateとworkspaceを Pi 上で build していない。host workspace の検証済み command が Pi で通るかは不明である
- [ ] direct build を継続するか cross compilation へ移るか決定した — **暫定で direct build を継続。**移行条件のうち「dependency build が memory 不足で安定しない」が未評価であり、確定していない

## 公式資料

- [Raspberry Pi computer hardware](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html)
- [Raspberry Pi product catalogue](https://datasheets.raspberrypi.com/product/product-catalogue-2023.pdf)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html)
- [Rustup installation](https://rustup.rs/)
