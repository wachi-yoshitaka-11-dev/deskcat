# 開発script

project作業を再現するための、小さくreview可能な補助scriptを置く。Python 3の標準ライブラリだけを使い、
サードパーティpackageは導入しない（[ADR-0006](../docs/decisions/0006-validation-script-language.md)）。

## 公開情報の検査

`check_publication.py`は投稿本文、commit message、staged additionsを同じ判定で検査する。
Issue／PR本文、コメント、review返信は本文をfileへ保存し、投稿の直前に実行する。

```bash
python3 scripts/check_publication.py --file <body-file>
python3 scripts/check_publication.py --staged
```

標準入力は`--stdin`。終了codeは、検出なしが0、検出が1、読めない・設定不正が2。診断に検出値は出さない。

個人に固有の名前、メール、SSID、識別子は、repository rootの`.publication.local.json`（gitignore済み）へ
置く。UTF-8のJSONで、`names`・`emails`・`ssids`・`identifiers`の4配列（合計256件まで）を持つ。
実際の値はこのfileだけに書く。無ければ共通patternだけを検査し、その旨を表示する。

commit前の検査は`.githooks/pre-commit`と`.githooks/commit-msg`が呼ぶ。追跡されているだけでは動かない。
専用cloneでは`git config --local core.hooksPath .githooks`で有効にする。共有Git設定の環境では
`git -c core.hooksPath=.githooks commit ...`を使う。Claude Codeでは`.claude/settings.json`が
`publication_guard_hook.py`を起動するが、shell展開・editor・GitHub UI・MCP経由の投稿は事前検査できない。
その経路では本文をfileへ保存して上のcommandを通す。

## 現在のscript

| Script | 用途 | 実行元 |
|---|---|---|
| `check_publication.py` | 投稿本文・commit message・staged additionsの公開情報検査 | 手動commandとgit hook |
| `test_publication_guard.py` | 上の判定・CLI・hookのtest | Pages workflowとlocal |
| `hooks/publication_guard_hook.py` | `gh`の本文・title・API fieldを投稿前に検査する | Claude Codeのhook |
| `lib/publication_guard.py` | 公開情報の共通判定とローカル設定 | import専用 |
| `validate_doc_links.py` | Markdown相対linkの検査。公開対象の判定は`prepare_pages.py`と揃える。閉じていないcode fenceで打ち切る | Pages workflowとlocal |
| `prepare_pages.py` | 公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する | Pages workflowとlocal |
| `validate_pages_output.py` | 生成済み`_site/`のlink・公開禁止情報・拡張子・sizeの検査。内訳を`EXTENSIONS=`等で出す | Pages workflowとlocal |
| `test_link_validators.py` | 上2つとlink検査のtest | Pages workflowとlocal |
| `test_pages_guards.py` | 公開境界（未宣言asset・symlink・size・拡張子・favicon）の回帰test | Pages workflowとlocal |
| `validate_instruction_entrypoint.py` | `CLAUDE.md`がGit index上で、`@AGENTS.md`だけのmode 100644 fileであることの検査 | Pages workflowとlocal |
| `test_instruction_entrypoint.py` | 上のtest | Pages workflowとlocal |
| `validate_table_columns.py` | Markdownの表で、header行とセル数が食い違う行を検出して止める | Pages workflowとlocal |
| `test_table_columns.py` | 上のtest | Pages workflowとlocal |
| `check_doc_size.py` | 文書の目標の量（行数・文字数・A4のページ数）の検査。仕様は先頭のdocstring | Pages workflowとlocal（ページ数は`--measure-pages`でFirefoxが要る） |
| `check_condense_loss.py` | 書き直しで文書から消えた値・ID・URL・codeの一覧（失敗にしない）。仕様は先頭のdocstring | Pages workflow（Pull Requestのみ）とlocal |
| `test_doc_size_checks.py` | 上記2つのtest | Pages workflowとlocal |
| `report_promotion_closing_keywords.py` | 昇格Pull Requestの範囲内commitにあるclosing keywordの報告（止めない） | Review gate workflowとlocal |
| `test_promotion_closing_keywords.py` | 上のtest | Pages workflowとlocal |
| `scan_procurement_mentions.py` | Issue／PR本文・commentの調達状態の語の報告（止めない）。語は`tbd-register.md`から読む | local（手動実行） |
| `test_procurement_mentions.py` | 上のtest（networkへ出ない） | Pages workflowとlocal |
| `review_gate.py` | 変更範囲を`minor`／`review-required`へ分類し、commit trailer（分類・自己レビュー・指示source変更）を照合する。`gate`・`classify`・`receipt`・`instructions`・`history`・`session` | Review gate workflowとlocal |
| `review_session.py` | 自己レビューの巡の記録と承認の検査 | `review_gate.py session`とClaude Codeのhook |
| `test_review_gate.py` | `review_gate.py`のtest | Pages workflowとlocal |
| `test_review_session.py` | `review_session.py`のtest | local |
| `check_wiring_figure_sync.py` | `Wiring revision`の行が変わり、`hardware/breadboard/layout.svg`が変わっていなければ止める | Wiring figure sync workflowとlocal |
| `test_check_wiring_figure_sync.py` | 上のtest | Pages workflowとlocal |
| `lib/publish_guards.py` | 公開境界の共通部品（secret・個人pathのpattern、追跡file列挙、symlink判定、見出しanchor、link抽出） | import専用 |
| `hooks/gh_metadata_guard.py` | PreToolUse hook。`gh issue create`／`gh pr create`の`--project`欠落、squash messageのtrailer欠落、CodeRabbit未完走の`gh pr merge`を拒否する | Claude Codeのhook |
| `hooks/branch_base_guard.py` | PreToolUse hook。`git checkout -b`の基点が古い`origin/develop`なら拒否する（`git fetch`を伴う） | Claude Codeのhook |
| `hooks/merge_trailer_report.py` | PostToolUse hook。`gh pr merge`の後にmerge commitのtrailerを報告する（止めない） | Claude Codeのhook |
| `hooks/worktree_guard.py` | PreToolUse hook。未commitの変更を捨てる`git reset --hard`・checkout系・`git clean`の前に`ask`する | Claude Codeのhook |
| `hooks/push_gate.py` | PreToolUse hook。`develop`を更新する`git push`の前に`review_gate.py gate`を実行し、落ちたら拒否する | Claude Codeのhook |
| `hooks/review_round_guard.py` | PreToolUse hook。レビューの巡の起動を記録と照らし、承認が無ければ止める | Claude Codeのhook |
| `hooks/command_line.py` | hookが受け取ったcommandから目的のprogramの呼び出しを取り出す（command位置の判定） | import専用 |
| `test_hooks.py` | `hooks/`の回帰test。hookを子processで起動し、拒否・通過と診断を検査する | Pages workflowとlocal |

`hooks/`は`.claude/settings.json`が起動する。hookの一覧と止められたときの手順の正本は
[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#hookが止めたとき)である。
`lib/`と`hooks/command_line.py`は単体で実行しない。判定はそこだけに持ち、各scriptへ複製しない。

## 実行

Localでのbuild前検査:

```bash
python3 scripts/validate_doc_links.py
python3 scripts/validate_instruction_entrypoint.py
python3 scripts/validate_table_columns.py
python3 scripts/check_doc_size.py
python3 scripts/review_gate.py classify --base origin/develop --head HEAD
python3 scripts/prepare_pages.py
python3 -m unittest discover --start-directory scripts --pattern "test_*.py"
```

Pages CIは`test_*.py`を、runnerの一時directoryから絶対pathで起動する。repository root以外のcurrent
directoryでも成功しなければならない。`validate_pages_output.py`は`_site/`を対象とするため、localで
実行するにはJekyll build（Ruby・Jekyll・GitHub Pages gem）が要る。`Gemfile`は追跡せず、`bundle exec`は
使わない。localの成功をCIの成功の根拠にしない。Jekyll環境の無い端末では、出力検査はCIに任せる。

## 規則

- 前提条件と使用方法を記載する。
- 安全側で失敗し、errorは0以外のstatusで返す。
- 秘密情報を埋め込まない。
- 固定されていないremote contentをdownload・実行しない。
- project固有の必要性がない限り、標準Cargo／ESP-IDF commandを複製しない。
