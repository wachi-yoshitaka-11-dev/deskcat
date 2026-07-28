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

- [ ] `GITHUB_TOKEN`のdefaultをcontents read-onlyにする
- [ ] write権限は必要なjobだけに付与する
- [ ] 第三者actionをreview済みversionまたはcommitへ固定する
- [ ] forkからのpull requestへ秘密情報を公開しない
- [ ] Hardware-in-the-Loopを通常のhosted CIから分離する

## Pages／Wiki

2026-07-28のread-back結果:

- GitHub Pagesは有効
- 公開URL: `https://wachi-yoshitaka-11-dev.github.io/deskcat/`
- Build type: GitHub Actions workflow
- Source metadata: `main`／repository root
- HTTPS enforcement: 有効
- Pages workflowとActions実行履歴: なし
- Custom 404: なし
- Wiki: 有効
- Wiki content: 既定の英語`Home.md` 1件だけ

整備方針:

- [ ] [GH-002 #25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)で`docs/`、Pages、Wikiの責務と正本を決定する
- [ ] Wikiを使用、用途限定、無効化のいずれにするか決定する
- [ ] [GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)で承認済み方針に基づくPages workflowを実装する
- [ ] `main`の承認済み内容だけをdeployする
- [ ] 公開前にsecret、個人path、local専用資料、再配布権を確認する

## 保留

- CODEOWNERS: 安定したreviewer／owner対応が複数になった時点で追加
- Code of Conduct: 外部communityへ積極的に参加を求める前に追加
- Dependabot: Cargo manifest作成後に追加判断
- Release workflow: versionとartifact方針の確定後に追加
- Discussions: community supportにIssueだけでは不足した場合に追加
