# 配置・運用

Raspberry Pi用serviceとinstall用artifactをこのディレクトリに置く。

置いたもの（Issue #491 段階2b-iii）:

- `systemd/deskcatd.service`（値は暫定で、根拠は無い。Pi実機で未確認である）、`systemd/deskcatd.env.example`

予定内容:

- install・update手順
- logとrestartの方針（unitの値以外）
- 設定pathの説明

実行file、path、実行user、service動作が確定する前に、動作するdeployment scriptを作成しない。
（`systemd/deskcatd.service`が固定するpath・user・動作は暫定で、確定していない。この文が禁じるのは、install・update用のscriptである）
