# ESP32 firmware

このディレクトリには、ESP-WROOM-32D開発ボード（秋月電子 M-13628）用のRust firmwareを置く。基板裏面silkscreenは`ESP32_DevKitc_V4`である（2026-08-15に大文字小文字を訂正。旧記載 `ESP32_DevkitC_V4`）。

専用のESP-IDF/Xtensa toolchainを使用するため、rootのhost workspaceとは別のCargo workspaceとする。

責務:

- LCD
- touch、加速度、環境sensor
- サーボPWMと強制安全上限
- JSON Lines通信
- watchdog、reset reason、診断、fail-safe動作

## 現在の状態

Issue #5 でtoolchainを固定し、最小projectのclean buildを確認した。実装済みなのは`link_patches()`、logger初期化、起動logの出力、Issue #7 の heartbeat と health snapshot、Issue #12 の`crate::protocol::PiSession`（`hello`／`ping`／`get_status`の受信側logic）、および Issue #13 の`crate::display::Ili9341`（`DISP-01`／ILI9341のSPI driver）である。**次段落（`crate::display`のbring-up）は既定build（`pi-protocol-mode` feature無効）についての記述である。**`pi-protocol-mode`は`display`／`protocol`のbring-up・demoを行わない（[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)。詳細は後続段落）。

`crate::display`は`docs/hardware/gpio-assignment.md`の`信号inventory`のうち`LCD-SCLK`／`LCD-MOSI`／`LCD-MISO`／`LCD-CS`／`LCD-DC`／`LCD-RST`／`LCD-BL`の7本だけを配線する。**`main()`は起動直後にcontroller識別（Read ID4）、単色fill、四隅test patternを実行し、結果をlogへ出す。**一次資料の引用はmodule docにある。**この版で確認できているのはESP32 Build profile端末でのbuild成功までである。**flash・実機通電・LCD panelの目視確認は未実施であり、実機のtouch／servoは引き続き未実装のままである（`ADC-*`・`TOUCH-*`はこの版でも一切GPIOへ触れない。`SERVO-PWM`は既定のbuildでは一切GPIOへ触れないが、`bench-servo-test-17` featureを付けたbuildだけは`docs/hardware/servo-safety-limits.md`の`初回動作の実行手順`に沿って触れる）。`ACCEL-*`・`ENV-*`はIssue #15で`crate::accel`・`crate::env`のI2C driverを追加済みであり、上記「未実装」の対象外である（生byteをlogへ出すのみで、値の解釈は別途）。

heartbeat と health snapshot は、いずれのbuildでもloopが動き続けるが、**既定buildでだけ**ESP logger の log にのみ出す（`pi-protocol-mode`はloggingを止めるため出力されない。`src/main.rs`参照）。**「log へ出す」は「serial へ出ない」ではない。**既定buildでのlogger の出力は UART を通って serial monitor に現れる。送らないのは、protocol の message として application の serial link（#11）へ流すことである。周期は `src/config.rs` が持ち、**いずれも暫定値である**（Protocol §5.7 が heartbeat の interval を `TBD` としているため、一次資料の根拠が無い）。health snapshot は `crates/deskcat-protocol` の `Status` を組み立てて JSON 1 行として出す。**`ProtocolCounters` はすべて 0 のままである。**いずれのbuildにも`ProtocolCounters`を増やす経路が無いためである（`src/health.rs`のmodule doc参照）。

`src/protocol.rs`の`PiSession`は`hello`のsession遷移、`ping`／`get_status`への応答、`stale_session`の判定を実装し、`crates/deskcat-serial/src/peer.rs`（Pi側、Issue #12）と対になる。**既定buildでは実serial linkへは配線していない。**`main()`はこの型を自己完結した例（`demonstrate_pi_session`）で1回動かしてlogへ出すだけであり、実際のUART受信loopではない。[gpio-assignment.md](../../docs/hardware/gpio-assignment.md)の`Pi–ESP32間のtransport`節がPi linkをUSB serialに確定させており、board上のUSB-UARTブリッジICが内部でUART0（GPIO1／GPIO3）へ接続するためGPIO headerへの配線は無い。**このUART0は、build時のfeature（`pi-protocol-mode`。[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)）でdebug logとPi–ESP32 protocol streamのどちらかへ排他的に使う。**詳細は`docs/protocol/esp32-pi-protocol.md`§2、`src/console.rs`参照。`pi-protocol-mode`を有効にしたbuildは、ESP32自身が送る`boot` frameを`send_boot_frame_once`が1回だけUART0への書き込みを試みる。**受信loopは無いためsession確立は主張しない。**`sid`の生成方法は`PROTO-TBD-011`が未確定であり、この型（`PiSession`）は決めない。

| 項目 | 確定版 |
|---|---|
| Crate | `deskcat-esp32` |
| Rust target | `xtensa-esp32-espidf` |
| Rust channel | `esp-1.95.0.0`（Xtensa Rust 1.95.0.0） |
| ESP-IDF | `v5.5.3`（tools install dirは`workspace`） |
| linker | `ldproxy` |

採用根拠と確定条件は[ESP32 Rust toolchain](../../docs/toolchains/esp32-rust-toolchain.md)、導入手順は[ESP32開発端末セットアップ](../../docs/runbooks/esp32-development-machine-setup.md)、実測環境は[Version Record](../../docs/toolchains/version-records/2026-08-06-esp32-build-linux.md)にある。

## Build

**commandの正本は[検証済みコマンド](../../docs/toolchains/verified-commands.md)である。**
ESP32 Build profileの端末で、このディレクトリにて実行する。**ここへcommandを写さない**（[ADR-0018](../../docs/decisions/0018-instruction-file-structure.md)）。

`export-esp.sh`を読み込まないと失敗する。`ESP_IDF_TOOLS_INSTALL_DIR=workspace`のため、ESP-IDF本体とmanaged toolは`.embuild/`（約4.4 GB）へ展開される。`.embuild/`と`target/`は`.gitignore`で除外し、applicationの`Cargo.lock`は追跡する。

環境に`IDF_PATH`が設定されていると、`.cargo/config.toml`で選んだESP-IDF versionを上書きする。`[env]`の`force = true`はこの変数を保護できないため、**`build.rs`が設定を検出してbuildを止める。**意図した外部SDKを使う場合だけ`DESKCAT_ALLOW_EXTERNAL_IDF_PATH=1`（値は厳密に`1`。`0`や`false`では通らない）を設定して通し、出力される`cargo:warning`をVersion Recordの`IDF_PATH present`へ記録する。

**止めるのが`build.rs`とは限らない。**`IDF_PATH`の指す先が実在しない場合は、依存の`esp-idf-sys`が先に別のerror（`could not determine esp-idf version from ...`）で失敗する。どちらでもbuildは止まるが、`build.rs`のguardのmessageは出ない。実測は[Version Record](../../docs/toolchains/version-records/2026-08-06-esp32-build-linux.md)にある。

## host crateの再利用

`deskcat-protocol`をpath dependencyで使う。wire protocolの実装を両側で1つに保つためであり、
判断の記録は[ADR-0008](../../docs/decisions/0008-firmware-protocol-crate-reuse.md)にある。

```toml
deskcat-protocol = { path = "../../crates/deskcat-protocol" }
```

- root workspaceの`exclude = ["firmware/esp32"]`は**維持する。**lockfileはroot `Cargo.lock`と
  このディレクトリの`Cargo.lock`の2つに分かれたままでよい。
- **共有crateの`rust-version`は、host（1.97.1）とESP toolchain（rustc 1.95.0-nightly）の
  両方を満たす下限にしてある。**`crates/deskcat-protocol/Cargo.toml`が理由込みで宣言している。
  ここを上げるとfirmwareのbuildが`rustc 1.95.0-nightly is not supported`でcompile前に止まる。
- `crates/deskcat-protocol/**`の変更でも`.github/workflows/firmware.yml`が発火する。
  host側だけの変更でfirmware buildが壊れるのを検知するためである。

`src/health.rs`がこのcrateの`Status`と`ProtocolCounters`を組み立てる。ただし**送信はしない。**
serial deviceは[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)、
session stateとserial taskの配線は
[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)の担当である。
現時点で示せているのは、固定toolchainで`xtensa-esp32-espidf`向けにcross compileできること、
および同crateの`status` payloadを実機上でserializeできることまでで、
**共有fixtureへの合格は主張しない。**

実機でのserializeの証拠は
[Version Recordの「2026-08-25 再検証」](../../docs/toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)
にある（`status` payloadのJSON 1行と`serialize失敗 0`）。**同記録の2026-08-20の確認とは別である。**
そちらが示したのは#6の起動出力とchip名までであり、`deskcat-protocol`を呼んでいなかった。

## 未確定の前提

機種と搭載moduleは確定している（ESP-WROOM-32D開発ボード／秋月電子 M-13628。基板にrevision表示は無い）。根拠は[hardware-bom.md](../../docs/hardware/hardware-bom.md)のMCU-01である。

**公式pin表と現物pin表記の照合は2026-08-15に完了し、[HW-TBD-001](../../docs/hardware/tbd-register.md)はcloseした**（38pinヘッダ両側のsilkが公式`J2`／`J3`と19pin×2列すべてで一致した）。照合先は[ESP32-DevKitC V4公式回路図](https://dl.espressif.com/dl/schematics/esp32_devkitc_v4-sch.pdf)と公式guideのpin description表である（秋月商品ページの添付はモジュールとチップのdatasheetのみで、boardのpin配列表を含まない）。**ただしGPIO割り当てを伴う変更は、まだ入れない。**[gpio-assignment.md](../../docs/hardware/gpio-assignment.md)は実機での電源off導通check、MSP2807のlogic IO levelの現物確認、servo起動時状態の安全review待ちで`Blocked`のままである。**中核chipの識別（`HW-TBD-031`）は2026-08-20に満たした。**中核chipは半田付けされた金属シールドの内側にあり刻印を読めないため、要件は2026-08-15にesptoolの報告で満たす形へ再定義され（[esp32-rust-toolchain.md](../../docs/toolchains/esp32-rust-toolchain.md)の`chip識別の満たし方`）、#6のflash時に`esptool`が`ESP32-D0WD`を報告した。**値と未加工の出力はそちらとVersion Recordが持ち、ここへ再掲しない。**

ESP-WROOM-32D datasheet v2.7にはPSRAM内蔵variantの記載が無いため、PSRAMを前提とする設定は不要である。

**flash、serial monitor、実機起動は2026-08-20に#6で実施した**（記録は[Version Record](../../docs/toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)）。**確認したのは起動出力とchip名までであり、周辺回路とservoには触れていない。****USB抜き差しによる電源再投入後の起動出力は未検証である**（host側のserial portがUSB enumerate後にしか存在せず、その時点で起動出力が終わっているため。再現は`espflash`のresetによる4回で示した）。

**Issue #13の`DISP-01`driverはこの版ではESP32 Build profile端末でのbuild確認までである。**flash・実機通電はESP32 Flash / HIL profile端末と人間の監視を要する（[Hardware Safety Policy](../../docs/governance/hardware-safety-policy.md)、初回通電）。実機での識別結果（Read ID4の一致）、単色fillの色、四隅patternの向きとcolor order、更新timingの実測値は、実機試験後に`docs/hardware/experiment-log.md`へ記録する（**この版ではまだ記録していない**）。
