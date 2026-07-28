# Repository設定計画

> 最終remote確認: 2026-07-28

## 確認済み

- Repository: `wachi-yoshitaka-11-dev/deskcat`
- 公開範囲: public
- Default branch: `main`

## 認証後の適用結果

- [x] Issueが有効
- [x] 利用目的が生じるまでDiscussionsを無効
- [x] Private vulnerability reportingを有効
- [x] Secret scanningとpush protectionを有効
- [x] GitHub標準label 9件のdescriptionとDeskCat固有label 16件を`.github/labels.yml`に同期
- [x] `.github/MILESTONES.md`のM0–M6 title／descriptionを同期
- [x] `main`へのforce pushを禁止
- [x] `main`の削除を禁止

## Branch protectionの時期

CI導入前:

- [x] solo bootstrap中はpull requestを必須にしない
- [x] 存在しないstatus checkを必須にしない
- 必須承認review数: solo bootstrap中は`0`

安定したCI導入後:

- [ ] 実在するformat／lint／test checkを必須化
- [ ] CIの信頼性が十分な場合にbranchの最新化を必須化
- [ ] 外部contributionにreviewを必須化
- 必須承認review数: 外部contribution受付時は`1`
- [ ] signed commitとlinear historyを別途評価

## Actions権限

workflow追加時:

- [x] `GITHUB_TOKEN`のdefaultがread-onlyであることを確認する
- [x] write権限はPagesのdeploy jobだけに付与する
- [x] 使用するGitHub公式Actionをreview済みcommitへ固定する
- [x] Pull Requestのbuild jobにsecretとwrite権限を渡さない
- [x] Hardware-in-the-Loopを通常のhosted CIから分離する

## Pages／Wiki

2026-07-28のread-back結果:

- GitHub Pagesは有効
- 公開URL: `https://wachi-yoshitaka-11-dev.github.io/deskcat/`
- Build type: GitHub Actions workflow
- Source metadata: `main`／repository root
- HTTPS enforcement: 有効
- Pages workflow: [run 30338761812](https://github.com/wachi-yoshitaka-11-dev/deskcat/actions/runs/30338761812)でbuild／deploy成功
- 公開物の`404.html`: HTTP 200でread-back確認済み
- Pages APIの`custom_404` metadata: `false`
- Wiki: 有効
- Wiki content: 既定の英語`Home.md` 1件だけ

承認済み方針:

- [x] [ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)で`docs/`、Pages、Wikiの責務と正本を決定する
- [x] RootのMarkdownと`docs/`を正本とし、Pagesを正本から生成する
- [x] Wikiを日本語の入口ページに限定し、独自の技術仕様やlive statusを置かない
- [x] [GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)でPages workflowを実装する
- [ ] [GH-004 #27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)でWikiの既定Homeを入口ページへ置き換える
- [x] Workflowと`github-pages` environmentの両方でdeploy元を`main`に限定する
- [x] Whitelist stagingでsecret、個人path、local専用資料、未承認形式を検査する

## 保留

- CODEOWNERS: 安定したreviewer／owner対応が複数になった時点で追加
- Code of Conduct: 外部communityへ積極的に参加を求める前に追加
- Dependabot: Cargo manifest作成後に追加判断
- Release workflow: versionとartifact方針の確定後に追加
- Discussions: community supportにIssueだけでは不足した場合に追加
