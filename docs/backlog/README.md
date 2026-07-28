# Backlog

このディレクトリには、GitHub Issue作成前の初期backlogを置く。

## 正式な管理先の移行

- repository準備中は、[初期Issue](initial-issues.md)をreview対象の定義元とする。
- GitHub認証が利用可能になり、基盤文書が公開された後、承認済みIssueとmilestoneを作成する。
- 作成後はGitHub Issueを現在statusの正式な管理先とする。
- このディレクトリは初期計画と対応表として残すが、live statusを重複管理し続けない。

## Issue品質

各Issueには次を含める。

- 一つの目的
- 対象範囲と対象外
- 依存関係
- 正式な基準文書
- 測定可能な受け入れ条件
- PC確認と実機確認
- 保存する証拠
- 安全と秘密情報の扱い

必要な部品型番、GPIO、電源値、安全制限が`TBD`の場合は`status:blocked`を使用する。
