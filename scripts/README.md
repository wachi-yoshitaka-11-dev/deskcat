# 開発script

このディレクトリには、project作業を再現するための小さくreview可能な補助scriptを置く。

規則:

- 前提条件と使用方法を記載する。
- 安全側で失敗し、error時は0以外のstatusを返す。
- 秘密情報を埋め込まない。
- 固定されていないremote contentをdownload・実行しない。
- project固有の必要性がない限り、単純な標準Cargo／ESP-IDF commandを複製しない。
