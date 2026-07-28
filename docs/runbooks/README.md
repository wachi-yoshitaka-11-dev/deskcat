# Runbook

このディレクトリには、再現可能な運用・開発手順を置く。

利用可能なrunbook:

- [Repository安全基準](repository-safety-baseline.md)
- [実装開始可否review](implementation-readiness-review.md)
- [ESP32開発端末セットアップ](esp32-development-machine-setup.md)
- [Raspberry Pi開発端末セットアップ](raspberry-pi-development-machine-setup.md)
- [GitHub Pages公開](github-pages-publishing.md)

予定runbook:

- build、flash、serial monitor
- ハードウェアbring-up
- fault記録とdebug
- release手順

runbookには、前提条件、正確なcommand、期待する証拠、失敗時の扱い、安全上の注意を記載する。

`Status: Draft`のrunbookにあるcommandは、DeskCatでの成功が未確認である。対象profileの端末で実行結果を記録し、review後にのみ検証済みcommandとする。
