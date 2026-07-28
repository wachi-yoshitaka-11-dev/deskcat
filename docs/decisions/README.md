# Architecture Decision Record

このディレクトリには、DeskCatの複数領域へ影響する判断、または後から戻すコストが高い判断を置く。

## 一覧

| ADR | Status | 判断 |
|---|---|---|
| [ADR-0001](0001-monorepo-layout.md) | Accepted | monorepo構成とworkspace境界 |
| [ADR-0002](0002-role-based-development-environments.md) | Accepted | 役割別の開発環境 |
| [ADR-0003](0003-public-documentation-publishing.md) | Accepted | Repository、Pages、Wikiの公開文書責務 |

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
