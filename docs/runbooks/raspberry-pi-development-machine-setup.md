# Raspberry Pi開発端末Setup

> 状態: Draft。対象のRaspberry Pi Zero Wでは未実行
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

`Raspberry Pi Zero 2 W`、64-bit userspace、soft-float ABI、glibc 以外であった場合は候補 target をそのまま適用せず、toolchain 文書を更新する。

## 2. Native build事前要件

対象 Raspberry Pi OS の公式 package 管理手順に従い、最低限次を用意する。

- Git
- C compiler と native linker
- C library development files
- dependency が要求する system library
- download と build に必要な CA certificate

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

`rustc -Vv` の `host` が OS と ABI に一致しない場合は先へ進まない。

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
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Rust Arm Linux targets](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html)
- [Rustup installation](https://rustup.rs/)
