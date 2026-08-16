# Raspberry Pi開発端末Setup

> 状態: Draft。対象のRaspberry Pi Zero Wでは未実行
> 更新: 2026-08-15 1節へfloat ABIの判定手順を追加（[#62](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/62)）。**手順が実機で判別できることは未確認**
> 適用範囲: Raspberry Pi RuntimeとRaspberry Pi Direct Build profile

## 目的

実物の Raspberry Pi Zero W で OS、ABI、Rust host を確認し、最小 Rust program の native build と実行を記録する。

候補と判断基準は [Raspberry Pi Rust Toolchain](../toolchains/raspberry-pi-rust-toolchain.md) を参照する。

## 開始条件

- [ ] 実物が Raspberry Pi Zero W であることを外観と model 表示で確認する（基板裏面silkscreenは`Raspberry Pi Zero W V1.1`。pin headerは別途ハンダ付けしたもの）
- [ ] microSD の必要データを backup した
- [ ] OS package の変更が許容される
- [ ] 安定した電源と network がある
- [ ] 十分な空き storage がある
- [ ] project secret を端末や shell history へ書かない

## 1. BoardとOSの記録

read-only command で環境を記録する。

```sh
tr -d '\0' < /sys/firmware/devicetree/base/model
uname -a
uname -m
getconf LONG_BIT
cat /etc/os-release
ldd --version
getconf GNU_LIBC_VERSION
free -h
df -h
```

出力を公開する前に hostname、username、IP address などを除く。

`ldd --version` と `getconf GNU_LIBC_VERSION` は glibc の identity と version を示すだけで、**float ABI を判別しない。**ABI は[float ABIの判定](#float-abiの判定)で判定する。

`Raspberry Pi Zero 2 W`、64-bit userspace、soft-float ABI、glibc 以外であった場合は候補 target をそのまま適用せず、toolchain 文書を更新する。soft-float か hard-float かの判定は[float ABIの判定](#float-abiの判定)による。

### float ABIの判定

`arm-unknown-linux-gnueabihf` は hard-float ABI の target である。適用してよいかは実機の ABI に依存するため、ここで判定する。適用条件の全体は [Raspberry Pi Rust Toolchain](../toolchains/raspberry-pi-rust-toolchain.md#arm-unknown-linux-gnueabihf-を適用してよい条件) を参照する。

手段は入手性が異なるため、A から順に試す。**権威的なのは B である。**A は OS の申告であり、C は B の tool を導入できない場合の代替である。**A と B は両方とも実行し、結果を突き合わせる。**C を使うのは B を実行できないときだけである。

先に `getconf LONG_BIT` が `32` であることを確認する。以下は 32-bit userspace と ELF32 を前提とする。

```sh
# A. OSが申告するarchitecture
dpkg --print-architecture

# A. hard-float用dynamic loaderの有無
ls -l /lib/ld-linux-armhf.so.3 /lib/ld-linux.so.3

# B. 実行中systemのbinaryのELFから読む（binutilsが要る）
readelf -h /bin/sh
readelf -A /bin/sh
readelf -l /bin/sh

# B. binutilsが無い場合、interpreterだけはfileで確認できる
file /bin/sh

# C. binutilsもfileも無い場合の最終手段（coreutilsだけで足りる）
od -An -tx1 -j 36 -N 4 /bin/sh
```

各手段が何を出力し、その出力のどこを見て判断するかを次に示す。

| 手段 | 見る箇所 | hard-float | soft-float |
|---|---|---|---|
| `dpkg --print-architecture` | 出力そのもの | `armhf` | `armel` |
| `ls -l /lib/ld-linux*` | loader の有無 | `/lib/ld-linux-armhf.so.3` がある | `/lib/ld-linux-armhf.so.3` が無い |
| `readelf -h` | `Flags:` 行 | `hard-float ABI` を含む | `soft-float ABI` を含む |
| `readelf -A` | `Tag_ABI_VFP_args` | `VFP registers` | **その行自体が出力されない** |
| `file` / `readelf -l` | `interpreter` の path | `/lib/ld-linux-armhf.so.3` | `/lib/ld-linux.so.3` |
| `od -An -tx1 -j 36 -N 4` | 4 byte のうち 2 番目 | `04` | `02` または `00` |

補足:

- `dpkg --print-architecture` が `arm64` を返した場合、userspace は 64-bit であり、hard/soft 以前に候補 target の前提を満たさない。1節冒頭の `getconf LONG_BIT` と併せて判断する。
- `ls -l /lib/ld-linux*` の判定は、**system の実行ファイルが要求する interpreter がその path に存在するはずである**という関係に基づく。両方の loader が存在する場合は multiarch 構成であり、この手段では判定できない。手段 B へ進む。
- `readelf -A` の `Tag_ABI_VFP_args` は、**存在すれば hard-float の強い根拠**である。一方、不在は soft-float の根拠としては弱い（tag を出力しない toolchain の可能性を排除できない）。soft-float 側の判定は `readelf -h` の `Flags:` を優先する。
- `od` が読むのは ELF32 header の `e_flags`（offset 36、little endian の 4 byte）である。hard-float なら `00 04 00 05`、soft-float なら `00 02 00 05` の形になる（**いずれも実際に確認した出力である**）。両 bit が 0 の場合は 2 番目の byte が `00` になる（**この形は未確認であり、次の項の仕様からの帰結である**）。ELF64 では offset が異なるため、`getconf LONG_BIT` が `32` でない場合はこの手段を使わない。
- `e_flags` の該当 bit は `EF_ARM_ABI_FLOAT_HARD`（`0x00000400`）と `EF_ARM_ABI_FLOAT_SOFT`（`0x00000200`）である。**両方の bit が 0 の場合は base 標準、すなわち soft-float とみなす**（[AAELF32](https://github.com/ARM-software/abi-aa/blob/main/aaelf32/aaelf32.rst) の Arm-specific `e_flags`）。したがって 2 番目の byte が `00` でも判別不能ではない。
- これらの bit は executable file header（`e_type` が `ET_EXEC` または `ET_DYN`）にのみ設定される。object file や一部の library では判定に使えないため、判定対象には `/bin/sh` のような実行可能ファイルを選ぶ。
- `file` は float ABI を語として出力しない。`interpreter` の path だけが手掛かりであり、interpreter を持たない静的 link binary では判断できない。その場合は `readelf -h` を使う。
- `readelf -A` の `Tag_ABI_FP_*` は hard-float と soft-float の**両方に出力される**ため、判定に使わない。判定に使うのは `Tag_ABI_VFP_args`（[Addenda32](https://github.com/ARM-software/abi-aa/blob/main/addenda32/addenda32.rst) の tag 28）だけである。
- **hard-float であることと Armv6 であることは別の条件である。**`readelf -A` の `Tag_CPU_arch` を併せて記録する。

判定に使う出力の形は次である。

```text
Flags:                             0x5000400, Version5 EABI, hard-float ABI
  Tag_CPU_arch: v6
  Tag_FP_arch: VFPv2
  Tag_ABI_VFP_args: VFP registers
      [Requesting program interpreter: /lib/ld-linux-armhf.so.3]
```

soft-float の場合は次の形になる。`Tag_ABI_VFP_args` が現れない点が hard-float との違いである。

```text
Flags:                             0x5000200, Version5 EABI, soft-float ABI
  Tag_CPU_arch: v5TE
      [Requesting program interpreter: /lib/ld-linux.so.3]
```

> **この節の出力例の出所。**Raspberry Pi 実機の出力ではない。Linux x86_64 の端末（`readelf` は GNU binutils 2.38）で、配布 package から取り出した binary を読んで得た出力である。検体は Raspbian の `coreutils_8.32-4_armhf.deb`、Debian の `coreutils_9.7-3_armhf.deb`、Debian の `coreutils_9.7-3_armel.deb` の 3 種で、armel を soft-float の対照に使った。package の `Architecture` field と ELF の float ABI が対応することも同時に確認した。確認日は 2026-08-15 である。
>
> このとき `Tag_CPU_arch` は、同じ armhf でも Raspbian の検体が `v6`、Debian の検体が `v7` であった。**hard-float であることは Armv6 であることを意味しない。**
>
> **実機で同じ出力が得られることは未確認である。**手順が実際に判別できるかの確認は [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) の実機作業で行う。

### ABIを判別できなかったとき

次のいずれかに当てはまる場合、**判別できたものとして扱わない。**

- 実行した手段の結果が互いに食い違う
- `readelf` と `file` が無く、2節の前提要件としても導入できない
- 対象 binary が静的 link であり、`readelf -h` も実行できない

そのときは次を行う。

- 候補 target を適用せず、先へ進まない（3節の停止規則と同じ扱いとする）。
- 得た出力を、匿名化したうえで [#62](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/62) と [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) へ記録する。判別できなかったこと自体を、手段と出力つきで残す。
- **確定させる所有者は [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) である。**記録だけして2節以降へ進まない。
- [Raspberry Pi Rust Toolchain](../toolchains/raspberry-pi-rust-toolchain.md#候補-toolchain) の「候補 toolchain」表にある状態は `ABI 確認待ち` のまま据え置き、確定として書き換えない。

## 2. Native build事前要件

対象 Raspberry Pi OS の公式 package 管理手順に従い、最低限次を用意する。

- Git
- C compiler と native linker
- C library development files
- dependency が要求する system library
- download と build に必要な CA certificate
- ELF を読む tool（`readelf`。1節の ABI 判定で手段 B を使えなかった場合、これを導入してから1節へ戻る）

最初から不要な SDK、cross compiler、ESP32 toolchain を導入しない。

## 3. Rust stable

[rustup.rs](https://rustup.rs/) の公式 installer が実機の host を support することを実行時点で再確認し、stable の default profile を候補とする。導入前に既存の OS package 版 Rust との競合を確認する。

導入後に記録する。

```sh
rustup -V
rustup show
rustc -Vv
cargo -V
cc --version
```

`rustc -Vv` の `host` が OS と ABI に一致しない場合は先へ進まない。一致は次で判定する。

- host triple の ABI 部分が `gnueabihf` なら hard-float、`gnueabi` なら soft-float を意味する（[Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html) の ABI Component）。1節で判定した float ABI と突き合わせる。**eabihf の実行物は eabi の system では正しく動作しない。**
- host triple の architecture 部分を `readelf -A` の `Tag_CPU_arch` と突き合わせる。同資料では `arm` が Armv6、`armv7` が Armv7 に対応する。**`uname -m` は kernel 側の値であり、userspace の architecture を示すとは限らないため参考にとどめる。**
- 突き合わせた結果が食い違う場合は、1節の[ABIを判別できなかったとき](#abiを判別できなかったとき)と同じ扱いにする。rustup が選んだ host を根拠に ABI を確定しない。

## 4. 最小native検証

root workspace が未生成の間は、対象 Issue 専用の最小 binary crate を使う。既存 source を上書きせず、実際に実行した command と source を evidence に残す。

確認項目:

- clean build が成功する
- binary が同じ Pi で実行できる
- debug と release の build 時間
- build 前後の空き storage
- build 中の memory pressure
- binary の architecture と dynamic dependency
- reboot 後も同じ binary が実行できる

project workspace 生成後は、root で合意した format、lint、unit test、build command に置き換える。

## 5. Cross compilationの判断

native build の計測前に cross compilation を導入しない。導入する場合は別 Issue で次を確定する。

- build host OS
- Rust target
- Armv6 hard-float 対応 linker
- target sysroot と glibc compatibility
- native dependency の cross build
- artifact transfer と integrity check
- 実機実行 test

`rustup target add` だけでは system linker と target C library は用意されない。

## 6. 証拠

[Version Record Template](../toolchains/version-record-template.md) を埋め、次を添える。

- 匿名化した board / OS 情報
- float ABI の判定に使った command と、その実出力（1節）。判別できなかった場合はその事実と、試した手段
- tool versions
- command と終了 status
- build duration、memory、storage
- binary identity
- 実行 output
- failure と workaround
- native build 継続または cross compilation 検討の結論

## 失敗時

- out-of-memory、storage 不足、link error、ABI mismatch を区別する。
- swap の追加や OS 設定変更を、無記録の一時 workaround にしない。
- machine 全体の package upgrade は、対象 Issue の範囲と backup を確認する。
- 実行できない binary を生成できても完了扱いにしない。

## 公式資料

- [Raspberry Pi documentation](https://www.raspberrypi.com/documentation/)
- [ELF for the Arm Architecture (AAELF32)](https://github.com/ARM-software/abi-aa/blob/main/aaelf32/aaelf32.rst) — `EF_ARM_ABI_FLOAT_HARD` と `EF_ARM_ABI_FLOAT_SOFT`
- [Addenda to the ABI for the Arm Architecture](https://github.com/ARM-software/abi-aa/blob/main/addenda32/addenda32.rst) — build attribute `Tag_ABI_VFP_args`
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html)
- [Rustup installation](https://rustup.rs/)
