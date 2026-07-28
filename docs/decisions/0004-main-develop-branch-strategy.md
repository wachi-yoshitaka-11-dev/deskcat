# ADR-0004: mainとdevelopの二段階branch運用を採用する

> 状態: Accepted
> 日付: 2026-07-28

## 背景

DeskCatは、ESP32 firmware、Raspberry Pi software、protocol、hardware文書を複数端末で並行して開発する。
開発基盤の初期整備までは`main`だけで進めたが、本格実装では未完成のcomponentを統合しながら、公開・安定状態の`main`を維持する必要がある。

`main`はGitHubのdefault branchであり、GitHub Pagesのdeploy元でもある。
日常的な統合途中の変更を直接`main`へ集約すると、公開文書と安定版の境界が曖昧になる。

## 判断要因

- 複数端末と複数componentの変更を統合できる
- `main`を安定した公開基準として維持できる
- 一Issue一目的のreview可能な差分を保てる
- GitHub Pagesを承認済みの`main`だけから公開できる
- Full GitFlowほどbranchとrelease操作を増やさない
- Hotfix後に統合branchとの不一致を残さない

## 検討した選択肢

### mainだけを使用する

短期branchから直接`main`へPull Requestするtrunk-based workflowである。
単純だが、複数componentの統合途中の状態と、公開・安定状態を同じbranchで扱うことになる。

### mainとdevelopを使用する

`main`を安定版、`develop`を統合先とし、Issue branchを`develop`へ集約する。
本格実装の並行作業と、安定版への昇格を分離できる。

### Full GitFlowを使用する

`main`、`develop`に加えてrelease branchとhotfix branchを常設規則として運用する。
明確だが、現段階の個人開発にはbranch管理とmerge操作が過剰になる。

## 決定

`main`と`develop`を使用する最小の二段階branch workflowを採用する。

### main

- GitHubのdefault branchとして維持する。
- 安定版、公開基準、将来のrelease基準とする。
- GitHub Pagesは`main`だけからdeployする。
- 通常の実装作業を直接commitしない。
- `develop`からの昇格、または明示的に承認したhotfixとrepository運用変更だけを受け入れる。

### develop

- 次の安定版に向けた長期統合branchとする。
- このADRを含む更新済み`main`から作成する。
- 通常のIssue branchとPull Requestのbaseとする。
- 統合途中であっても、各変更は対象Issueの受け入れ条件と実行可能な検証を満たす。
- releaseまたはmilestoneの基準を満たした時点で、Pull Requestにより`main`へ昇格する。

### Issue branch

- 通常は最新の`develop`から作成する。
- `feature/`、`fix/`、`docs/`、`chore/`、`experiment/`の接頭辞とIssue識別子を使用する。
- 一branchを一Issue、一つのreview可能な目的に限定する。
- 通常はPull Requestで`develop`へ統合する。
- Pull Requestをmergeした後、不要なIssue branchは削除してよい。

### Hotfix

- 公開中の`main`へ緊急修正が必要な場合は、`main`から`hotfix/<issue>-<short-name>`を作成する。
- 検証後、Pull Requestで`main`へ統合する。
- 同じ修正を直ちに`develop`へ取り込み、branch間の再発と欠落を防ぐ。

### 初回導入

`develop`が存在しないため、このADRと運用文書は`main`へ直接記録する。
記録commitをpushした後、そのcommitから`develop`を作成する。
以後は上記の通常workflowへ移行する。

Branch protectionとrequired status checkの追加は、branch作成とは別のGitHub設定変更として扱う。

## 影響

### 利点

- 統合途中の変更と安定版を分離できる。
- ESP32、Raspberry Pi、protocolの並行作業を一つの統合branchで確認できる。
- Pagesへ公開する変更を`main`への昇格時に限定できる。
- Issue branchのbaseとPull Request先が明確になる。

### 欠点

- `develop`から`main`への追加のPull Requestが必要になる。
- Hotfixとrepository運用変更を`develop`へ反映する作業が増える。
- Default branchが`main`のため、通常Pull Requestではbaseを`develop`へ明示する必要がある。

### リスクと対策

| リスク | 対策 |
|---|---|
| Pull Requestを誤って`main`へ作成する | Pull Request作成時にbaseを確認し、Contribution文書へ通常baseを記載する |
| `main`と`develop`が意図せず乖離する | `main`へのhotfixと運用変更を`develop`へ直ちに反映する |
| `develop`へ未検証変更が蓄積する | 一Issue一目的、受け入れ条件、利用可能なcheckを各Pull Requestで要求する |
| 二段階mergeで履歴が複雑になる | Full GitFlowは導入せず、release branchを常設しない |
| `develop`が無保護で変更される | Force push、削除、required checkを別のGitHub設定変更として評価する |

## 検証

- GitHubのdefault branchが`main`のままである。
- GitHub Pages workflowが`main`だけをdeployする。
- `develop`が、このADRを含む`main`のcommitから作成されている。
- GitHub上で`main`と`develop`のSHAをread-backできる。
- Development Workflow、CONTRIBUTING、Repository設定のbranch責務が一致している。

## 置き換える決定

なし。
