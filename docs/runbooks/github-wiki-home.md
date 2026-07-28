# GitHub Wiki入口の保守

> 状態: Verified — 2026-07-28に初回更新と公開結果を確認済み
> 対象Issue: [GH-004 #27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)
> 方針: [ADR-0003](../decisions/0003-public-documentation-publishing.md)

## 目的

GitHub Wikiを、DeskCatの公開文書へ案内する日本語の入口ページとして保守する。
技術情報はmain repositoryでreviewし、Wikiを独立した仕様保管場所にしない。

## 管理境界

- 技術情報の正本はmain repositoryのroot Markdownと`docs/`である。
- GitHub Pagesは正本から生成する閲覧用siteである。
- Wikiは別のGit repositoryであり、`Home.md` 1件だけを案内用に保持する。
- 技術仕様、設計値、ADR、runbook、進捗、Issue checklist、release noteをWikiへ複製しない。
- Wikiへ長文が必要になった場合は、先にmain repositoryの正本へ追加し、Wikiからlinkする。

| 対象 | 値 |
|---|---|
| Wiki URL | `https://github.com/wachi-yoshitaka-11-dev/deskcat/wiki` |
| Git remote | `https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git` |
| Branch | `master` |
| 公開page | `Home.md` |

## 更新手順

1. Wiki変更用Issueを用意し、一つの目的に限定する。
2. main repositoryとWiki repositoryの未commit変更、remote、branchを確認する。
3. Wiki repositoryを一時directoryへcloneする。
4. `Home.md`だけを編集し、仕様本文やlive statusを追加していないことをreviewする。
5. すべてのlinkが公開先でHTTP 200を返すことを確認する。
6. Secret様pattern、資格情報、個人path、local専用資料がないことを確認する。
7. Staged diffが`Home.md`だけであることを確認してcommitする。
8. 承認後にWikiの`master`へpushする。
9. Rendered Wikiとraw Markdownをread-backし、日本語本文、正本方針、linkを確認する。
10. Wikiのlocal／remote SHAが一致することを確認し、一時cloneを安全に削除する。

Wikiはmain repositoryのbranch protectionやPages workflowとは別に更新される。
そのため、push前のdiff確認とpush後のread-backを省略しない。

## 初回公開の検証記録

2026-07-28に、既定の英語Homeを日本語の案内ページへ置き換えた。

| 確認対象 | 結果 |
|---|---|
| Wiki commit | `8402a8e8e2622f27af0d7707709aa66b6d3cd0e1` |
| Files | `Home.md` 1件 |
| Rendered Wiki | HTTP 200、日本語の入口と正本方針を確認 |
| Raw Markdown | HTTP 200、日本語の入口と正本方針を確認 |
| Link | 11件を確認し、失敗0 |
| 公開禁止情報 | Secret様pattern 0、個人path 0 |
| 二重管理 | 技術仕様、設計値、live status、Issue checklistなし |
| Git同期 | Wikiのlocal／remote SHA一致 |

## 失敗時

- Link切れや公開禁止情報を認めた場合は完了扱いにしない。
- Wikiへ仕様を直接追記して修正を急がず、main repositoryの正本を先に更新する。
- Remote、branch、対象fileが想定と異なる場合はpushしない。
- 意図しない公開があった場合は、影響範囲を記録し、安全な内容への修正を優先する。
