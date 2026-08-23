# アーキテクチャ

このディレクトリには、現在有効なアーキテクチャ説明と図を置く。

判断の理由と履歴は`docs/decisions/`に置き、その結果としての現在設計をこのディレクトリに置く。

**このディレクトリには、まだアーキテクチャ文書が無い。**下の一覧は予定であり、対応する文書は存在しない。

ESP32 と Raspberry Pi の責務境界は、[`AGENTS.md`](../../AGENTS.md)の「プロジェクト境界」が正本である。**予定文書を書くときも、同じ内容をここへ複製しない**（[Single Source of Truth](../governance/README.md#single-source-of-truth)）。

予定文書:

- system context
- ESP32とRaspberry Piのcomponent境界
- runtime event flow
- deployment view
- fault・recovery model

GPIO、電源、サーボ制限の正確な値はここに置かず、`docs/hardware/`を参照する。
