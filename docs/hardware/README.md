# ハードウェア文書

このディレクトリは、softwareとfirmwareが必要とするハードウェア情報の正式な定義元である。

正式文書:

- `hardware-bom.md`
- `gpio-assignment.md`
- `power-budget.md`
- `servo-safety-limits.md`
- `sensor-datasheet-notes.md`
- `sd-health-check.md`
- `experiment-log.md`（実験記録）

正確な部品型番、公式資料、記録済み実測値だけを使用する。不明値は`TBD`とする。

top-levelの`hardware/`は、回路図、PCB source、機構CADなど、version管理する設計sourceが存在する場合に使用する。
