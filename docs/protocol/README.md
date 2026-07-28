# Protocol文書

このディレクトリは、正式なESP32–Raspberry Pi間wire動作を定義する。

初期仕様:

- `esp32-pi-protocol.md`

Rustの型とserializerはこの仕様を実装するものであり、仕様の代わりにはならない。

protocol変更では次を明記する。

- versionへの影響
- 互換動作
- 境界値・不正入力test
- reconnect動作
- firmware側とhost側の実装状態
