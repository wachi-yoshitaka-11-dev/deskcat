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

結果は次のいずれかで結論づける。

- `Verified`: 記録した profile の必須 command が成功した
- `Partial`: 一部だけ成功し、未実行項目がある
- `Failed`: 再現可能な失敗証拠がある
- `Incompatible`: support 条件または target が一致しない

失敗を手動 workaround で通した場合、元の失敗と workaround の両方を記録する。
