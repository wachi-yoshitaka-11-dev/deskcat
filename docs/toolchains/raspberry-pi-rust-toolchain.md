# Raspberry Pi Rust Toolchain

> 状態: 調査済み。Raspberry Pi実機は未検証
> 調査日: 2026-07-27
> 対象board: Raspberry Pi Zero WH

## 結論

Raspberry Pi Zero WH は Zero 2 W とは異なる。公式 product information では BCM2835、single-core Arm11 / Armv6、512 MB の機種である。したがって、64-bit Arm や Armv7 を前提にしない。

32-bit Raspberry Pi OS と hard-float ABI を実機で確認できた場合、Rust の candidate target は次である。

```text
arm-unknown-linux-gnueabihf
```

Rust 公式の platform support では、この target は Armv6 Linux hard-float とされている。実機の OS、ABI、glibc が一致するまでは確定しない。

## 初期方針

最初は Raspberry Pi Zero WH 上の native build を試す。

理由:

- target、linker、glibc の組合せを単純にできる。
- 実際の OS で build と run を同時に確認できる。
- 初期の小さな service では cross toolchain の保守を先に増やさない。

Pi Zero WH は CPU と RAM が限られるため、build 時間と storage 使用量を必ず記録する。実用上困難と確認できた場合だけ cross compilation を別 Issue で導入する。

## 実機で確定する情報

次は物理 Raspberry Pi と起動中の OS から取得する。

| 項目 | 目的 |
|---|---|
| Board model と revision | Zero WH と Zero 2 W の取り違え防止 |
| OS name、release、32/64 bit | 配布物と support の基準 |
| `uname -m` | kernel architecture |
| `getconf LONG_BIT` | userspace bitness |
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
| Build method | Pi 上の direct build | M1-004 で検証 |
| Cross compilation | 保留 | direct build の計測後に判断 |

ESP32 用の `esp` toolchain を Pi service の build に使わない。

## Cross compilationへ移る条件

次のいずれかを実測し、Issue に証拠を残した場合に検討する。

- clean build が開発サイクルとして許容できない
- dependency build が memory 不足で安定しない
- storage 消費や書込み負荷が大きい
- 複数 Pi への配布を自動化する必要が生じた

cross compilation では Rust target の追加だけでなく、Armv6 hard-float に対応する linker と target libc が必要になる。build host の library へ誤って link しないことを、`file`、ELF metadata、実機実行で確認する。

## 確定条件

- [ ] 物理 board model と revision を記録した
- [ ] OS、kernel、userspace bitness、libc を記録した
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
