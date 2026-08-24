# Architecture Decision Record

このディレクトリには、DeskCatの複数領域へ影響する判断、または後から戻すコストが高い判断を置く。

## 一覧

| ADR | Status | 判断 |
|---|---|---|
| [ADR-0001](0001-monorepo-layout.md) | Accepted | monorepo構成とworkspace境界 |
| [ADR-0002](0002-role-based-development-environments.md) | Accepted | 役割別の開発環境 |
| [ADR-0003](0003-public-documentation-publishing.md) | Accepted | Repository、Pages、Wikiの公開文書責務 |
| [ADR-0004](0004-main-develop-branch-strategy.md) | Accepted | `main`と`develop`の二段階branch運用 |
| [ADR-0005](0005-standard-development-os.md) | Accepted | 開発環境の標準OSを実機Linuxとする |
| [ADR-0006](0006-validation-script-language.md) | Accepted | 検証scriptの実装言語をPythonとする |
| [ADR-0007](0007-review-scope-and-self-review.md) | Accepted | 自動reviewを高リスク変更へ限定し、自己レビューを主軸とする |
| [ADR-0008](0008-firmware-protocol-crate-reuse.md) | Accepted | Firmwareから`deskcat-protocol`をpath dependencyで再利用する |
| [ADR-0009](0009-pages-own-layout.md) | Accepted | Pagesのlayoutとstylesheetを自前で保持する |
| [ADR-0010](0010-change-class-and-review-declaration.md) | Accepted | 変更の分類を機械化し、自己レビューをcommit trailerで宣言する |
| [ADR-0011](0011-issue-optional-pull-request-required.md) | Accepted | Issueの要否とPull Requestの要否を分ける |
| [ADR-0012](0012-milestones-count-issues-only.md) | Accepted | milestoneはIssueだけに設定する |
| [ADR-0013](0013-manual-only-coderabbit-review.md) | Accepted | CodeRabbitの自動reviewを廃止し、手動依頼だけにする |

## 新規作成

1. [0000-template.md](0000-template.md)を複製し、次の未使用IDで採番する。
2. 複製したADR本文のplaceholderを実値へ置き換える。**一覧だけを更新しない。**
   - 見出しの`ADR-XXXX: 判断の題名`を、採番したIDと実際の題名へ
   - `状態: Proposed`を、そのADRの実際の状態へ（起票時点で`Accepted`なら`Accepted`）
   - `日付: YYYY-MM-DD`を、実際の日付（`2026-08-02`形式）へ
3. 上の`一覧`へ、ADRへのlink、Status、判断の要約を1行追加する。
   **一覧の`Status`とADR本文の`状態`を一致させる。**一覧を`Accepted`にしたまま
   本文が`Proposed`のまま残ると、どちらが正か判断できない。
   `YYYY-MM-DD`のようなplaceholderを本文に残さない。
4. 置換する既存ADRがあれば、次の4箇所すべてを更新する。一覧だけでは、本文と状態が食い違う。
   - **後継ADR本文の`## 置き換える決定`節**（置換元ADRへのlinkを追加）
   - 一覧にある置換元ADRの`Status`（`Superseded`など置換済みを示す値へ）
   - **置換元ADR本文の`状態`field**（一覧と同じ置換済み状態へ）
   - **置換元ADR本文の末尾へ追加する`## 後継の決定`節**（後継ADRへのlinkを追加）

`## 置き換える決定`は、後継ADRから置換元ADRを指す節である。置換元ADRへ
後継linkを置くために使わない。置換元の判断本文を履歴として保つため、既存の
`## 決定`節も後継linkの追記先にしない。

`0000`は採番済みIDではなく雛形であり、一覧には載せない。
採番したADRを一覧へ追加しないと、そのADRは一覧から辿れない。

## 命名

4桁の識別子を使う。

```text
0001-short-decision-name.md
0002-next-decision.md
```

識別子を再利用しない。置換されたADRもrepositoryに残し、後継ADRへlinkする。

## ADRを作成する条件

次の場合にADRを作成する。

- repositoryまたはworkspaceの境界
- hardwareとsoftwareの責務変更
- protocol互換方針
- storageまたはdeploymentのアーキテクチャ
- toolchain戦略
- 複数componentへ影響するdependency
- 安全方針の変更

小さな局所実装の選択は、Issueまたはcode reviewへ記録する。
