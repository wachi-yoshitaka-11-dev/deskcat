# 開発環境Version Record

この template は、開発端末で実際に確認した環境を記録する。秘密情報や個人を識別する値を含めない。

## 記録template

```text
Record ID:
Date:
Machine profile:
Operator role:
Repository commit:
Working tree clean: yes / no

OS name:
OS version:
CPU architecture:
Userspace bitness:
Container / VM / native:

Rustup version:
Rust channel:
Rust compiler version:
Rust host:
Installed Rust targets:
Cargo version:
rustfmt version:
Clippy version:
Linker identity and version:

ESP32 only:
  Physical board:
  Module marking:
  Board revision:
  Rust target:
  espup version:
  cargo-generate version:
  ldproxy version:
  espflash version:
  ESP-IDF version:
  ESP-IDF source/commit:
  ESP-IDF tools location mode:
  IDF_PATH present: yes / no
  IDF_TOOLS_PATH present: yes / no
  Template repository:
  Template commit:
  sdkconfig/defaults identity:
  USB-UART identity:

Raspberry Pi only:
  Board model:
  Board revision:
  Kernel architecture:
  libc identity and version:
  Available memory before build:
  Available storage before build:

Commands run:
Expected result:
Actual result:
Build duration:
Peak memory if measured:
Storage delta if measured:
Generated artifact identity:
Log or evidence path:
Known differences from documented profile:
Conclusion:
Next action:
```

## 記録してはいけないもの

- access token、password、秘密鍵
- Wi-Fi SSID と credential
- private repository URL に含まれる credential
- 個人名、端末名、ユーザー名
- IP address、MAC address、machine ID
- 個人の絶対 path
- 公開不要な USB serial number
- `.env` の内容

## 判定

**この節が判定語の正本である。**値も、値の意味も、選び方も、ここだけで定義する。
[Version Records](version-records/README.md) はここを参照する。

結果は次のいずれかで結論づける。**この4つ以外を使わない。**

- `Verified`: 記録した profile の必須 command が成功した
- `Partial`: 一部だけ成功し、未実行項目がある
- `Failed`: 再現可能な失敗証拠がある
- `Incompatible`: support 条件または target が一致しない

**未実行の項目が一つでも残る場合は `Partial` とし、何が未達かを記録内に明記する。**
「必須 command はすべて成功した」だけでは `Verified` にならない。

**範囲を括弧で添えることで、上の4つから外れた判定にしない。**
範囲そのものは `Conclusion` へ書いてよいが、**判定語は4つのいずれかである。**

実例がある。ESP32 Flash / HIL の記録は `Conclusion: Verified（flash と起動記録について）`
だったが、同じ記録が「USB 抜き差しでの再現は未検証である」とも書いていた。
**未実行の項目が残っているため `Verified` にはならない。**
[PR #208](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/208) が
`Partial（… USB 抜き差しによる電源再投入後の起動出力が未検証。…）` へ改めた。
**括弧の中身は残し、判定語だけを規則へ合わせている。**

失敗を手動 workaround で通した場合、元の失敗と workaround の両方を記録する。
