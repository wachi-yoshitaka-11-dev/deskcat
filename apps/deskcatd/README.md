# deskcatd

`deskcatd`は、Raspberry Pi Zero W（V1.1）**向けに設計する**Rustサービスである。
**実機での動作は未検証である。**検証状態の正本は
[Raspberry Pi Rust Toolchain](../../docs/toolchains/raspberry-pi-rust-toolchain.md)であり、
同文書は状態を「調査済み。Raspberry Pi実機は未検証」としている。

責務:

- ESP32とのシリアル接続と状態同期
- 感情・性格状態の管理
- 行動のスケジューリング
- コマンド生成
- アイドル時の独り言
- 設定、API、ストレージの統合

## 現在の実装

[Issue #491](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491)の段階2bとして、libraryを作成した
（`src/`。2b-i）。接触eventから表情を決めてESP32へ送り、sessionのやり直しをprocessの終了で行う。
終了と再開の規則は`src/daemon.rs`のmodule docが持つ。

binary（`src/main.rs`）、stderrのlogger、引数、起動からの期限（watchdog）は2b-iiで足した。**引数は既定値を持たない**
（`--help`を参照）。

#23 の Host 段階では、`--speech-min-interval-ms` と `--speech-duration-ms` を両方指定した場合に限り、
`deskcat-domain` の固定文候補を `show_text` として送る。最短間隔と表示時間の運用値は未確定で、
systemd unit には指定していないため既定は off である。送信前には protocol の文字列制限を検査し、
拒否と無応答を分類して counter と log に残す。firmware は `show_text` を受けて描画へ渡し、ACK を返す（#587）が、
実 LCD での表示は未確認である。

## 実機確認用の表情の送信（`--probe-expression`）

[Issue #514](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/514)の赤LED（`happy`の受理で点く）を、
撫でるeventが無いbuildでも確かめるための引数である。**運用では付けない。**systemdのunitにも指定しない。

```text
deskcatd <必須の引数> --no-companion --probe-expression happy --probe-interval-ms <ms>
```

- 2つの引数は**両方**要る。`--no-companion`も要る（`Companion`も欲しい表情を書き換えるため、併用を拒否する）。
- ESP32のsession確立と`status`の受信の後、`set_expression`を**指定の表情と`neutral`で交互に**送る。切り替えは指定の間隔ごとである
  （同じ表情は`ExpressionSync`が送り直さないため、`neutral`を挟む）。`happy`を受理したときだけ赤が点く（`neutral`など他の表情では点かない。点灯時間は[led-514-demo.md](../../docs/hardware/led-514-demo.md#役割と表示)が正本）。
- **probe中は`play_motion`を送らない**（値があっても積まない）。limiter・heartbeatの経路は変えていない。引数が無ければ従来と同じである。
- 表情名は`happy`などの`set_expression`の名前。知らない名前、`neutral`、間隔0、片方だけの指定は、終了1で拒否する。
- 間隔の値に根拠は無い。赤の点灯時間（[led-514-demo.md](../../docs/hardware/led-514-demo.md#役割と表示)）より長い値で観察する。
- 赤を確かめるfirmwareは、LEDを持つ製品buildか、`bringup-led-514`と`face-21`を併せたbuildである（`no-touch-487`はLEDを持たない）。撫でるeventを読むbuildでは、`Mind`の表情が混ざる。

systemdのunitは`deploy/systemd/deskcatd.service`にある。値は暫定で、根拠は無い。Pi実機で未確認である。
