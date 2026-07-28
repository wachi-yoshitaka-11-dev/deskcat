# ADR-0003: 公開ドキュメントはPagesで生成しWikiを入口に限定する

> 状態: Accepted
> 日付: 2026-07-28

## 背景

DeskCatは、rootのMarkdownと`docs/`にGovernance、ADR、hardware、protocol、toolchain、runbookを保存している。これらはGitの履歴、Pull Request、link check、公開前reviewの対象である。

GitHub PagesとWikiはどちらも有効だが、2026-07-28の確認時点では次の状態だった。

- Pagesは`build_type: workflow`だが、workflowと実行履歴が存在しない。
- Wikiは既定の英語`Home.md`だけが存在する。
- Repositoryはpublicであり、公開siteとWikiも公開情報として扱う必要がある。

PagesとWikiへ手作業で同じ仕様を複製すると、正本、review経路、更新時期が分かれ、安全制限や`TBD`が不一致になる。

## 判断要因

- 技術情報の正本を一つに保つ
- Gitのreviewと履歴を公開文書にも適用する
- 新規参加者が主要文書へ辿りやすくする
- Secret、個人path、local専用資料、再配布不可資料を公開しない
- Pages workflowの権限とdependencyを最小化する
- Wikiを別の仕様保管場所にしない

## 検討した選択肢

### PagesとWikiへ文書を手作業で複製する

閲覧場所は増えるが、修正漏れと内容の不一致が発生しやすい。正本とreview経路も曖昧になる。

### Pagesだけを使用してWikiを無効化する

二重管理を避けやすいが、GitHub上のWiki tabから文書を探す利用者への入口を失う。

### Repositoryを正本とし、Pagesを生成物、Wikiを入口に限定する

技術文書はGitで管理したまま、Pagesで閲覧性を高める。Wikiには案内とlinkだけを置き、独自の仕様を持たせない。

### Wikiを公開文書の正本にする

Wikiは別のGit repositoryで管理されるため、main repositoryのPull Request、branch protection、link checkと同じworkflowを適用しにくい。

## 決定

Repositoryを唯一の正本とし、PagesとWikiを次の責務に限定する。

### Repository

- RootのMarkdownと`docs/`を公開文書を含む技術情報の正本とする。
- Architecture、Governance、安全制限、hardware値、protocol、runbook、Issue定義をWikiへ複製しない。
- 文書変更は通常のGit／Pull Request／review経路で行う。

### GitHub Pages

- Pagesは正本から生成する公開siteとし、Pages上で直接編集しない。
- Pull Requestではbuildとlink checkだけを実行し、deployしない。
- `main`の承認済み変更だけをdeployする。
- Workflowの通常権限はread-onlyとし、deploy jobだけにPages用権限を付与する。
- Actionとsite generatorのversion、保守状況、license、代替をreviewする。
- Secret、資格情報、個人path、local専用資料、再配布不可資料を生成対象から除外する。
- Custom domainは初期整備の対象外とする。

### GitHub Wiki

- Wikiは公開文書への入口ページに限定する。
- 日本語の`Home.md`からPages、repository README、文書index、Issuesへ案内する。
- 技術仕様、設計値、ADR、runbook、live status、Issue checklistをWiki固有contentとして保持しない。
- Wikiに長文または技術情報を追加する必要が生じた場合は、先に正本へ追加し、Wikiからlinkする。

実装は[GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)と[GH-004 #27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)で分離する。

## 影響

### 利点

- 技術文書のreviewと履歴をmain repositoryへ集約できる。
- Pagesの閲覧性とWiki tabの発見性を利用できる。
- 安全制限や`TBD`を複数箇所で更新する必要がない。
- Wikiを無効化せず、用途を明確に限定できる。

### 欠点

- Pages用workflowとsite generatorを保守する必要がある。
- Wikiの入口linkはPagesや文書構成の変更時に更新が必要になる。
- Pagesの生成失敗時はrepository内のMarkdownを直接参照する必要がある。

### リスクと対策

| リスク | 対策 |
|---|---|
| Pagesへ非公開資料が混入する | 生成対象を明示し、PRで公開範囲とartifactを検査する |
| Workflowの権限が過大になる | default read-only、deploy jobだけに`pages: write`と`id-token: write`を付与する |
| DependencyまたはActionが無断更新される | review済みversionへ固定し、更新を独立した変更として扱う |
| Wikiへ仕様が書き足される | Homeに正本方針を明記し、仕様はrepositoryへ昇格させる |
| Pagesと正本のlinkが壊れる | PR buildとlink check、deploy後のread-backを行う |

## 検証

この決定は、次を満たすことで検証する。

- Pagesが`main`の正本から再現可能に生成される。
- Pull RequestからPagesがdeployされない。
- Deploy job以外にwrite権限がない。
- Pagesの主要navigationと相対link checkが成功する。
- Wikiが日本語の入口ページだけを持ち、独自仕様やlive statusを保持しない。
- PagesとWikiにsecret、個人path、local専用資料、再配布不可資料が含まれない。

## 参考資料

- [Configuring a publishing source for your GitHub Pages site](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)
- [Using custom workflows with GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
- [About wikis](https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis)

## 置き換える決定

なし。
