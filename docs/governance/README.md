# DeskCat Governance

> 状態: Active
> 適用範囲: リポジトリ全体

このディレクトリには、DeskCatの開発時に人間とAIエージェントが使用する永続的なポリシーを置く。

## 文書一覧

| 文書 | 目的 |
|---|---|
| [AI Agent Policy](ai-agent-policy.md) | 権限、責務、根拠、不確実性、外部操作の境界 |
| [Development Workflow](development-workflow.md) | Issue、実装、検証、文書化、Gitの作業手順 |
| [Hardware Safety Policy](hardware-safety-policy.md) | 電気、機構、ベンチ試験に関する必須安全規則 |
| [公開asset register](published-asset-register.md) | 公開するbinary assetの出所と再配布許諾 |

ルートの[AGENTS.md](../../AGENTS.md)は、AIエージェントが実行時に参照する簡潔な指示である。背景情報を重複させず、このディレクトリのポリシーを参照する。

## 文書言語

リポジトリの文書は、次の基準で表記を統一する。

- 説明、手順、ポリシー、Issue／PRテンプレートの本文は日本語で記載する。
- ファイル名、ディレクトリ名、ソースコード、コマンド、JSONフィールド、label名、milestone名、型名、API名は原則として英語表記を維持する。
- 製品、規格、ライブラリ等の公式名称と、検索・照合に必要な原文は英語表記を維持してよい。
- エラーメッセージやログを引用する場合は原文を改変せず、必要に応じて日本語の説明を添える。
- `Active`、`Draft`、`Accepted`、`Deprecated`、`Superseded`、`TBD`等の状態識別子は、文書間の検索性を保つため英語表記を使用してよい。

この基準は、技術用語を無理に翻訳するためのものではない。読み手が日本語の文脈で一貫して内容を理解でき、同時にコードや外部資料との対応を失わないことを目的とする。

## 権限の優先順位

情報が矛盾する場合は、次の順序を使用する。

1. 現在のユーザー指示
2. 承認済みの安全制限、GPIO割り当て、プロトコル仕様、ADR
3. メーカーのデータシート、ボード回路図、公式SDK文書
4. 再現可能な測定結果と実験記録
5. [DeskCat マイコン開発技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)
6. Issue、Pull Request、コードコメント
7. 一般知識またはAIによる推論

現在の指示であっても、電気的または機械的な安全制限を暗黙に上書きしない。指示が危険と思われる場合は、影響する操作を停止し、その根拠とリスクを提示する。

## Single Source of Truth

各プロジェクト情報には、正本となる場所を一つだけ定める。

| 情報 | 正本 |
|---|---|
| 長期的なアーキテクチャ判断 | `docs/decisions/` |
| 正確な部品識別情報とデータシート根拠 | `docs/hardware/hardware-bom.md` |
| GPIO割り当て | `docs/hardware/gpio-assignment.md` |
| 電源構成と電流予算 | `docs/hardware/power-budget.md` |
| サーボ制限とfail-safe動作 | `docs/hardware/servo-safety-limits.md` |
| ESP32–Pi間のwire protocol | `docs/protocol/esp32-pi-protocol.md` |
| 開発端末の役割とtoolchain選定 | `docs/toolchains/` |
| 開発・運用手順 | `docs/runbooks/` |
| 公開文書のsource | RootのMarkdownと`docs/` |
| 公開binary assetの出所と許諾 | [公開asset register](published-asset-register.md) |
| Pages／Wikiの公開方針 | [ADR-0003](../decisions/0003-public-documentation-publishing.md) |

同じ値を複数の文書で再定義しない。他の文書からは正本の定義へリンクする。

## ポリシー変更

Governanceを変更する場合は、次を満たす。

1. 解決する問題を明記する。
2. 影響するポリシーとworkflowを特定する。
3. 安全境界を維持するか、明示的に強化する。
4. 実行時の指示が変わる場合は`AGENTS.md`を更新する。
5. 必須チェックが変わる場合はtemplateまたはCIを更新する。
6. 関係のないfeature作業とは分離してreviewする。
