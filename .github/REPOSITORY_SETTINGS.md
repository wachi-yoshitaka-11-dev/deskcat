# Repository設定計画

> 最終remote確認: 2026-08-22（[全面照合](#2026-08-22のdesired-stateとの全面照合)）

## 確認済み

- Repository: `wachi-yoshitaka-11-dev/deskcat`
- 公開範囲: public
- Default branch: `main`
- Stable／Pages branch: `main`
- Integration branch: `develop`
- Branch strategy: [ADR-0004](../docs/decisions/0004-main-develop-branch-strategy.md)

## 認証後の適用結果

- [x] Issueが有効
- [x] 利用目的が生じるまでDiscussionsを無効
- [x] Private vulnerability reportingを有効
- [x] Secret scanningとpush protectionを有効
- [x] GitHub標準label 9件のdescriptionとDeskCat固有label 17件を`.github/labels.yml`に同期（GitHub側 計26件）
- [x] `.github/MILESTONES.md`のM0–M6 title／descriptionを同期
- [x] `main`へのforce pushを禁止
- [x] `main`の削除を禁止
- `main`の`enforce_admins`は**有効化しない**。2026-07-28に一度有効化してread-backで確認したが、2026-08-22の照合では`false`であり、**同日にrepository所有者が「不要」と判断した。**checkboxを外したのは、これが未完了のtaskではなく決定だからである（下記「[2026-08-22のdesired stateとの全面照合](#2026-08-22のdesired-stateとの全面照合)」）
- [x] Repository description、homepage、topicsを設定
- [x] `delete_branch_on_merge`を有効化（2026-07-31に無効化のdriftを検出し、再適用してread-back済み）
- [x] `develop`: Branch protectionで`Require conversation resolution before merging`**だけ**を有効化（2026-08-10。下記「2026-08-10のdevelop branch protection」を参照）。**必須reviewと必須status checkは設定しない**
- [x] `develop`のbranch削除だけをRepository Rulesetで禁止（2026-08-03。経緯は下記「2026-08-03のdevelop branch削除事故と対処」を参照）
- `develop`へのGit／GitHub操作は、Governanceのforce push禁止、通常作業での直接commit禁止、ユーザー承認で管理する

### 管理者除外の解消

2026-07-28の初回設定時、`main`は次の状態だった。

```text
allow_force_pushes: false
allow_deletions:    false
enforce_admins:     false   ← 管理者は上記2つの制限を受けなかった
```

`enforce_admins`が無効な間、force pushと削除の禁止は、**管理者を拘束していなかった**。
Pull Request必須化と違い、force push／削除の禁止はsolo bootstrapの速度を損なわないため、同日中に有効化した。

read-back結果: `enforce_admins.enabled = true`

これにより[AGENTS.md](../AGENTS.md)の「force pushを行わない」は、**`main`に限って**
GitHub側でも実効化され、AIエージェントの誤操作に対する防壁になる。

**この段落は2026-08-22時点では成立しない。**同日の照合で`enforce_admins`は`false`であり、
repository所有者は有効化を「不要」と判断した。**管理者はforce pushとbranch削除の禁止に
拘束されない。**この2つを止めているのはGovernanceの規約とユーザー承認だけである
（下記「[2026-08-22のdesired stateとの全面照合](#2026-08-22のdesired-stateとの全面照合)」）。

**この記述は2026-08-10に更新した。**`develop`にもbranch protectionを設定したため、
force pushとbranch削除はGitHub側で禁止されている（下記「2026-08-10のdevelop branch protection」）。

ただし**必須reviewと必須status checkは設定していない**ため、`develop`への直接commitは引き続き可能である。
その範囲はGovernanceの規約とユーザー承認だけが歯止めであり、repository全体で強制されていると読まない。

- [x] `github-pages` environmentの`can_admins_bypass`を無効化

read-back結果: `can_admins_bypass = false`、protection rule は `branch_policy`、
許可branchは `main` の1件のみ（`deployment-branch-policies`をAPIで確認）

このenvironmentは`branch_policy`による保護が有効である。`can_admins_bypass`が
`true`のままだと、管理者はその方針の外からdeployできる。workflow側の`main`限定は
workflowを変更すれば外せるため、環境側の統制の代わりにならない。
deployは引き続き`main`から実行できる。制限されるのは方針外branchからのdeployだけである。

## Branch protectionの時期

CI導入前:

- [x] solo bootstrap中はpull requestを必須にしない
- [x] 存在しないstatus checkを必須にしない
- 必須承認review数: solo bootstrap中は`0`
- `develop`は`Require conversation resolution before merging`のみ有効化し、required status checkと必須reviewは設定しない（2026-08-10）

安定したCI導入後:

- [ ] 実在するformat／lint／test checkを必須化
- [ ] CIの信頼性が十分な場合にbranchの最新化を必須化
- [ ] 外部contributionにreviewを必須化
- 必須承認review数: 外部contribution受付時は`1`
- [ ] signed commitとlinear historyを別途評価
- [x] `Require conversation resolution before merging`を評価し、有効化した。`develop`は2026-08-10である（下記「2026-08-10のdevelop branch protection」）。`main`も有効であることを2026-08-22の照合で確認したが、**`main`で有効化した時期は本文書に記録が無い。**導入の契機は[PR #40](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/40)で未解決thread 1件を残したmergeが起きたことである

## Actions権限

workflow追加時:

- [x] `GITHUB_TOKEN`のdefaultがread-onlyであることを確認する
- [x] write権限はPagesのdeploy jobだけに付与する
- [x] 使用するGitHub公式Actionをreview済みcommitへ固定する
- [x] Pull Requestのbuild jobにsecretとwrite権限を渡さない
- [x] checkoutに`persist-credentials: false`を指定し、runner上へtokenを残さない（PR側scriptが残存tokenを読めないようにする措置であり、token露出全般を防ぐものではない。`actions/checkout` v6.0.0以降、tokenの保存先は`.git/config`ではなく`$RUNNER_TEMP`配下の別fileであり、`.git/config`からは`includeIf.gitdir`で条件付きincludeされる）
- [x] Hardware-in-the-Loopを通常のhosted CIから分離する
- [x] 固定したActionの更新を受け取るため`.github/dependabot.yml`を追加する

- [x] `sha_pinning_required`を有効化し、SHA固定を設定として強制する
- [x] Vulnerability alertsとDependabot security updatesを有効化する

2026-07-28のread-back結果。`allowed_actions`はこの時点の値であり、
2026-07-31に`selected`へ変更した。現行値は下の「GitHub Actionsの供給元制限」を参照する。

```text
allowed_actions:             all      # 2026-07-31に selected へ変更（履歴）
sha_pinning_required:        true
dependabot_security_updates: enabled
```

`Vulnerability alerts`は別endpointのため上のblockに現れない。
2026-08-01に`GET /repos/{owner}/{repo}/vulnerability-alerts`で確認した。

```text
GET .../vulnerability-alerts -> HTTP 204 No Content   # 有効
```

204は有効、404は無効を意味する。bodyを返さないendpointであり、
他の設定と同じ形式では記録できない。

この確認は「2026-07-31の実行環境」と同一端末・同一profile・同一tool版で行った。
新しい環境記録は追加しない。

SHA固定は改竄耐性を与えるが、更新機構がなければ修正版が届かない。
`dependabot.yml`（月次のversion更新）とsecurity updates（脆弱性検知時の更新）は役割が異なるため、両方を使用する。

`.github/dependabot.yml`の[`target-branch`](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference#target-branch)
`develop`が適用されるのはversion updateである。Dependabot security updateは
repositoryのdefault branchである`main`を対象にする。
Security updateを`main`へmergeした場合は、[Development Workflow](../docs/governance/development-workflow.md#branch)の
hotfix規則に従い、同じ修正を直ちに`develop`へ取り込む。両branchへ自動で反映されると仮定しない。

- [x] `firmware/esp32`のcargo ecosystemを追加する（2026-08-10、[#41](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/41)）

`schedule`、`target-branch`、`open-pull-requests-limit`、`commit-message.prefix`はgithub-actionsと同じ値へ揃えた。
`labels`には`area:firmware`を加えている。対象が`firmware/esp32`に限られ、Pull Requestの仕分けに要るためである。

**上の`target-branch`と security update の使い分けは、cargoにもそのまま当てはまる。**
version updateは`develop`へ、security updateは default branch の`main`へ来る。

label（`type:maintenance`／`area:firmware`）はどちらもGitHub側に実在することを確認した。
存在しないlabelを指定するとDependabotが付与に失敗する。

**GitHub側が設定を認識したかは、この記録の時点では未確認である。**設定fileのmerge後に、
Insightsの Dependabot 画面または実際のDependabot Pull Requestの発生で確認する。
`firmware/esp32`は custom toolchain（`rust-toolchain.toml`）と`build-std`を使うため、
**Dependabotがlockfileを更新できずに失敗する可能性がある。**その場合はerrorの内容を本文書へ記録する。

host workspaceのCargo manifestは未生成のため対象に含めていない。生成した時点で追加する。

- [x] 外部contributorのPull Requestに承認を必須化する

Pages workflowの`pull_request` jobは、PR側の`scripts/*.py`をrunner上で実行する。
GitHub既定の`first_time_contributors`では、一度merge実績のあるcontributorのPRが
承認なしで実行される。`all_external_contributors`へ変更し、fork由来のworkflow実行に
毎回人間の承認を必要とする。

read-back結果: `approval_policy = all_external_contributors`

### GitHub Actionsの供給元制限

- [x] `allowed_actions`を`selected`へ絞る（2026-07-31。当時は`patterns_allowed`が空で、GitHub所有Actionのみだった）

**2026-08-10時点の実際の方針は「GitHub所有Action＋明示的に許可した`esp-rs/xtensa-toolchain`」である。**
`patterns_allowed`へ1件追加したため、`selected`＝GitHub所有のみ、ではなくなった（下記）。

使用中のActionは次のとおりである。

| workflow | Action | 供給元 |
|---|---|---|
| `pages.yml` | `actions/checkout`、`actions/configure-pages`、`actions/jekyll-build-pages`、`actions/upload-pages-artifact`、`actions/deploy-pages` | GitHub所有 |
| `firmware.yml` | `actions/checkout`、`actions/cache` | GitHub所有 |
| `host.yml` | `actions/checkout`、`actions/cache` | GitHub所有 |
| `firmware.yml` | `esp-rs/xtensa-toolchain` | **サードパーティ。`patterns_allowed`で個別に許可** |

`all`のままにすると、workflowを1行変えるだけで任意のサードパーティActionを導入できる。
`sha_pinning_required`は「固定された何か」であることを保証するだけで、供給元は制限しない。
SHA固定と供給元制限は別の統制であり、片方で他方を代替できない。

2026-07-31のread-back結果: `allowed_actions = selected`、`github_owned_allowed = true`、`verified_allowed = false`、`patterns_allowed = []`。
**`patterns_allowed`の現行値は下記のとおり1件である。**

Rust CIでサードパーティActionが必要になった場合は、`patterns_allowed`へ個別に追加し、採用理由と確認日を本文書へ記録する。

#### `esp-rs/xtensa-toolchain` の許可（2026-08-10、[#42](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/42)）

- [x] `patterns_allowed`へ`esp-rs/xtensa-toolchain@*`を追加する

read-back結果:

```text
github_owned_allowed: true
verified_allowed:     false
patterns_allowed:     ["esp-rs/xtensa-toolchain@*"]
allowed_actions:      selected
sha_pinning_required: true
```

**採用理由**: `firmware.yml`がXtensa Rust toolchainを導入するのに必要である。
GitHub所有のActionに同等品が無い。shellで`espup`を導入する案も採れるが、
version、toolchain名、`ldproxy`、環境変数のexportを自前で並べることになり、
`espup`の仕様変更を追う負担がworkflow側へ移る。**採否は供給元制限を1件緩めることとの比較で判断した。**

**緩めていない統制**:

- `github_owned_allowed`は`true`のまま。上表のGitHub所有Actionの扱いは変わらない
- `verified_allowed`は`false`のまま。Verified creatorを一括で許可していない
- `sha_pinning_required`は`true`のまま。**このAction自身もcommit SHAへの固定が強制される**
- 追加したのは1 patternだけである。`esp-rs/*`のようなnamespace全体の許可はしていない

**このAction自体はreviewの対象である。**`v1.7.0`（`ec6d365`）へ固定した。
2026-08-10時点でrepositoryはarchivedでなく、最終pushは2026-04-20である。
版を上げるときは、上げ先のcommitをreviewしてからSHAを差し替える。

### 2026-07-31のdrift確認

Pull Request #29の作業中にremoteを再確認したところ、`delete_branch_on_merge`が
`false`になっていた。適用時のread-backでは`true`だったため、その後に変化している。
変化の経緯は特定できていない。

再適用し、read-backで`true`を確認した。

```text
delete_branch_on_merge: true
```

本設定はmerge可否や安全境界に影響しないため、Pull Request #29のblockerではない。
repository所有者が意図して無効化していた場合は、無効へ戻し、その判断理由を
本文書へ記録する。checklistとremoteが食い違ったまま放置しない。

**この評価は2026-08-03に誤りと判明した。**下記「2026-08-03のdevelop branch削除事故と対処」を参照。
`delete_branch_on_merge`は、PRのheadが使い捨てのfeature branchかどうかを区別せず、
PR mergeのたびにhead branchを削除する。`develop`のような恒久的なbranchがPRのheadになった
場合、この設定は安全境界（branchの存続）に直接影響する。

### 2026-08-10のdevelop branch protection

`develop`にbranch protectionを設定した。目的は**未解決review threadを残したmergeをGitHub側で止める**ことである。

適用値（`PUT /repos/{owner}/{repo}/branches/develop/protection`）:

| 項目 | 値 | 理由 |
|---|---|---|
| `required_conversation_resolution` | **`true`** | 本設定の目的。未解決threadがあると`mergeStateStatus`が`BLOCKED`になる |
| `required_pull_request_reviews` | **`null`** | **設定しない。**設定すると`develop`への直接commitが塞がり、記述の維持管理を直接反映してよい運用（[CONTRIBUTING](../CONTRIBUTING.md)）と両立しない |
| `required_status_checks` | `null` | CIの安定を確認してから別途判断する（上記「Branch protectionの時期」） |
| `allow_force_pushes` | `false` | force pushを禁止 |
| `allow_deletions` | `false` | 2026-08-03の削除事故と同じ事象を、Rulesetに加えてprotection側でも防ぐ |
| `enforce_admins` | `false` | 管理者は強制mergeできる。未解決を残す場合の手順は[CONTRIBUTING](../CONTRIBUTING.md)に従う |

動作確認（2026-08-10、[PR #88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)）:

| 操作 | `mergeStateStatus` |
|---|---|
| thread resolved | `CLEAN` |
| thread を unresolve | **`BLOCKED`** |
| 再び resolve | `CLEAN` |

**設定しただけで確認しない、を避けるため実際にblockされることを確認した。**

これにより`CONTRIBUTING.md`の「Merge前の確認」が要求していた手作業のGraphQL確認は不要になり、同節を縮小した。
ただし**`Review rate limited`はcheckが`pass`と表示されGitHubは止めない**ため、その確認だけは手作業として残している。

### CodeRabbitのauto review設定（2026-08-10。2026-08-22に廃止）

**この設定は2026-08-22に廃止した。**`.coderabbit.yaml`は`reviews.auto_review.enabled: false`
だけを持ち、`labels`と`base_branches`は外した。reviewは意味上criticalな変更に対して
自己レビューの後で手動依頼する（[ADR-0013](../docs/decisions/0013-manual-only-coderabbit-review.md)）。
**以下は廃止前の記録である。**

[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)を新設し、auto reviewを高リスク変更へ限定した
（[PR #88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)、[#87](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/87)）。

| key | 値 |
|---|---|
| `reviews.auto_review.enabled` | `false`（`labels`の正一致でのみ発火） |
| `reviews.auto_review.base_branches` | `[develop]`。既定branchの`main`は設定に関わらず常に対象 |
| `reviews.auto_review.labels` | `area:firmware` `area:protocol` `area:raspberry-pi` `area:hardware`。**`type:decision`は[PR #91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で外した**（ADRとgovernance文書の変更が頻出し、文書変更でreviewを消費し続けるため） |
| `reviews.auto_review.auto_incremental_review` | `false`（pushのたびの再reviewを止める） |

全Pull Requestでreviewを走らせるとrate limitに掛かる。
[PR #55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)では再review依頼が実行されず、
[PR #88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)では2回目の依頼が`Review rate limited`で終わった。

**pathでauto reviewの発火は制御できない。**`reviews.path_filters`はreviewの中で見るfileを絞るだけである
（公式JSON schemaで確認）。発火を制御できるのは`base_branches`／`labels`／`ignore_title_keywords`／
`ignore_usernames`の4つだけであり、本projectはPull Requestにもlabelを付ける運用のためlabelで制御している。

### 2026-08-03のdevelop branch削除事故と対処

[PR #32](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/32)（`develop`→`main`、
[PR #29](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/29)のmain昇格）をmergeした直後、
`delete_branch_on_merge`により`origin/develop`が削除された。`develop`はADR-0004が定める
恒久的な統合branchであり、使い捨てのfeature branchではない。

| 項目 | 内容 |
|---|---|
| 発生 | PR #32のmerge（commit `57768b1`）直後、`origin/develop`が削除された |
| 検出 | merge後の`git fetch --prune`で`- [deleted] (none) -> origin/develop`を観測 |
| データ損失 | なし。削除されたcommit（`ad7a69c`）はmerge commit経由で`main`の履歴に含まれ、かつlocal repositoryの`develop`branchにも残っていた |
| 復旧 | localの`develop`（`ad7a69c`）を`git push origin develop`で再作成。新規branchとしてpushされたが、内容はmain昇格直前と同一 |
| 恒久対処 | `develop`のbranch削除だけを禁止するRepository Rulesetを追加した（下記） |

```text
ruleset id: 20296953
name:       Protect develop from deletion
target:     branch
enforcement: active
conditions.ref_name.include: [refs/heads/develop]
rules:      [{type: deletion}]
bypass_actors: []（current_user_can_bypass: never。管理者も含めbypassできない）
```

このrulesetは削除だけを禁止し、review必須化やstatus check必須化は含まない。
`develop`をbranch protectionのbundle対象外とする既存方針（solo bootstrapの速度を落とさない）は変更しない。
`delete_branch_on_merge`はrepository全体で有効のままとする。無効化すると、
merge済みのfeature branch（`chore/repository-hardening`等）の自動削除が失われ、
手動cleanupの運用に戻るため、ここでは選ばない。

同種の事故は、恒久的なbranchをPRのheadにする運用（develop→mainの昇格PR）を
続ける限り再発しうる。この節のrulesetがそれを防ぐ。

### 実行環境（sanitized）

| 項目 | 値 |
|---|---|
| 実施日 | 2026-07-29 |
| 端末profile | Docs / Review（[Machine Profiles](../docs/toolchains/machine-profiles.md)） |
| 使用tool | `gh` 2.76.0、`git` 2.44.0、PowerShell 7.6.3 |
| 対象 | `wachi-yoshitaka-11-dev/deskcat` |

2026-07-31に実施した確認と適用（`allowed_actions`、`delete_branch_on_merge`、
`github-pages` environmentの`can_admins_bypass`）も、同一端末・同一profile・
同一tool版で行った。上表の環境をそのまま適用する。

| 項目 | 値 |
|---|---|
| 実施日 | 2026-07-31 |
| 端末profile | 同上 |
| 使用tool | 同上 |
| 対象 | 同上 |

端末名、個人path、token、shell履歴は記録しない。
この記録は実行済みの確認結果であり、候補値や未実行の予定を含まない。


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
- Wiki content: 日本語の案内用`Home.md` 1件だけ
- Wiki URL: `https://github.com/wachi-yoshitaka-11-dev/deskcat/wiki`
- Wiki commit（**2026-07-29のgit read-back時点**のhead）: `9ec03b743bbab0b70cdeece179706007b4523a3d`（下記のgit read-backを参照）
- Wiki commit（履歴）: `8402a8e8e2622f27af0d7707709aa66b6d3cd0e1`。2026-07-29のidentity是正で現在の`master`履歴から到達できなくなった

上の2行は2026-07-28時点の値ではなく、2026-07-29のread-back結果である。
- Wiki運用: [GitHub Wiki入口の保守](../docs/runbooks/github-wiki-home.md)

承認済み方針:

- [x] [ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)で`docs/`、Pages、Wikiの責務と正本を決定する
- [x] RootのMarkdownと`docs/`を正本とし、Pagesを正本から生成する
- [x] Wikiを日本語の入口ページに限定し、独自の技術仕様やlive statusを置かない
- [x] [#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)でPages workflowを実装する
- [x] [#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)でWikiの既定Homeを入口ページへ置き換える
- [x] Workflowと`github-pages` environmentの両方でdeploy元を`main`に限定する
- [x] Stagingでsecret、個人path、local専用資料、未承認形式を検査する
- [x] `docs/`はMarkdownだけを複製し、画像等が人手のreviewを経ずに公開されないようにする

### Wikiのgit read-back（2026-07-29）

Wikiは独立repositoryとしてcloneでき、内容は機械的に確認できる。手順は[GitHub Wiki入口の保守](../docs/runbooks/github-wiki-home.md)を参照する。

- Files: `Home.md` 1件
- Head commit: `9ec03b743bbab0b70cdeece179706007b4523a3d`
- Commit identity: 全commitが`wachi-yoshitaka-11-dev`のnoreply address
- Link: 延べ12件（一意11件）すべてHTTP 200。数え方はrunbookを参照
- Secret様pattern／個人path: 0件
- 二重管理: なし（ADR-0003の方針どおり案内のみ）

Wikiのcommit identityは、2026-07-29に既存2 commitを書き換えて是正した。
経緯と対象SHAはrunbookの「Commit identityの是正記録」を参照する。

### Wikiの編集権限（2026-07-29に目視確認）

- [x] Wikiの「Restrict editing to collaborators only」が有効であること

Settings → Features → Wikis の目視結果:

```text
[x] Wikis
[x] Restrict editing to collaborators only
```

Wikiはcollaboratorだけが編集でき、閲覧はpublicのままである。
READMEとPagesの両方からWikiへ導線があるため、この設定が無効だと
任意のGitHubユーザーによる改竄・誘導の経路になる。無効化しない。

この項目はGitHub UIでしか確認できない。REST APIは`has_wiki`、
GraphQLは`hasWikiEnabled`のみを返し、いずれも機能の有効／無効であって
編集権限ではない（2026-07-29にschema introspectionで確認）。
Wikiの設定を変更した場合は、この節を目視結果で更新する。

## Projects

2026-08-06のread-back結果:

- Repositoryは`repository.projectsV2`（GraphQL）で**Projects v2のboard 1件**にlinkされている
  - Owner: `wachi-yoshitaka-11-dev`（**user**単位。organizationではない）
  - Number: `5`、Title: `deskcat`、Visibility: Public
  - URL: `https://github.com/users/wachi-yoshitaka-11-dev/projects/5`
  - Item数: 35
  - 使用中のfield: `Status`（single select）、`Milestone`、`Repository`、`Start date`、`Target date`（他にTitle／Assignees／Labels等のdefault field）
- `Start date`／`Target date`はProjects v2のcustom fieldであり、Issueの開始日・終了日を設定する運用に対応する
- Repository REST APIの`has_projects`は`true`

2026-08-07のworkflow read-back結果（`user.projectV2(number:5).workflows`）:

| Workflow | 有効 |
|---|---|
| `Auto-add sub-issues to project` | true |
| `Auto-close issue` | true |
| `Item added to project` | true |
| `Item closed` | true |
| `Pull request linked to issue` | true |
| `Pull request merged` | true |

`Item status changed`という名前のworkflowは存在しない。`Status`が`Done`になったitemの
Issueをcloseするのは`Auto-close issue`である。

`Pull request merged`が`Status`を`Done`にするのはPull Request item側であり、
Issue itemには波及しない。実測でも、PR #44 merge（2026-08-06T13:37:11Z）とIssue #43
close（13:41:05Z）に3分54秒、PR #45 merge（2026-08-07T00:36:45Z）とIssue #39
close（01:00:20Z）に23分35秒の差があり、workflowによる即時発火ではない。
mergeだけではIssueがcloseされないため、Issue itemの`Status`を`Done`にする操作が必要である。
運用手順は[CONTRIBUTING.md](../CONTRIBUTING.md)のPull request節に記載する。

`has_projects`は`has_issues`／`has_wiki`と同種の機能有効化トグルであり、classic project
（repository単位のclassic project board）が実在するかどうかを示す値ではない。project数0でも
`true`になり得る。Projects v2のboardがlinkされているかどうかも反映しない。三者は別の仕組みである。

```text
GET repos/{owner}/{repo}/projects -> HTTP 404 Not Found
```

GitHubはProjects（classic）のREST APIを2025-04-01にsunsetしており（機能自体のsunsetは
2024-08-23）、`has_projects`の値に関わらずこのendpointは404を返す。つまり`has_projects`は
現在、classic projectの実在や利用可能性を示す情報としてほぼ無意味なlegacy flagである。

方針: `has_projects`は`true`のまま変更しない。classic projectとしては使っていないが、
このflag自体がGitHub側で実質的な意味を失っているため、無効化のための追加操作は行わない。
実際のIssue進行管理は上記Projects v2のboardで行う。

## 2026-08-22のdesired stateとの全面照合

[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の「branch protection／ruleset／Actions／Pages設定をdesired stateと一致させ、API read-backを記録する」に対する照合である。**設定は変更していない。読み出しだけを行った。**

読み出した値である。**本文書がdesired stateとして記録している項目は、下の1件を除いて
すべて一致した。**記録が無い項目（`has_wiki`、merge方式の設定等）は、一致・不一致を
言えないため観測値として並べる。

```text
visibility:                            public
default_branch:                        main
has_issues:                            true
has_discussions:                       false
has_wiki:                              true
delete_branch_on_merge:                true
secret_scanning:                       enabled
secret_scanning_push_protection:       enabled
dependabot_security_updates:           enabled
private_vulnerability_reporting:       true
vulnerability-alerts:                  HTTP 204 (有効)

main   allow_force_pushes:             false
main   allow_deletions:                false
main   required_approving_review_count: 0
main   require_code_owner_reviews:     false
main   required_status_checks:         null
main   required_conversation_resolution: true

develop allow_force_pushes:            false
develop allow_deletions:               false
develop required_pull_request_reviews: null
develop required_status_checks:        null
develop required_conversation_resolution: true

ruleset "Protect develop from deletion"
  target:      branch
  enforcement: active
  include:     refs/heads/develop
  rules:       deletion
  bypass_actors: []

actions enabled:                       true
actions allowed_actions:               selected
actions github_owned_allowed:          true
actions verified_allowed:              false
actions patterns_allowed:              esp-rs/xtensa-toolchain@*
actions sha_pinning_required:          true
default_workflow_permissions:          read
can_approve_pull_request_reviews:      false

github-pages can_admins_bypass:        false
github-pages protection_rules:         branch_policy
github-pages 許可branch:               main (1件)
pages source:                          main /
pages build_type:                      workflow
pages https_enforced:                  true
```

**本文書の記録と食い違っていた項目が1件ある。**照合後の判断を受けて、記録側を現状へ
合わせた。

```text
main enforce_admins:                   false   ← 2026-07-28の記録はtrue。同項目は2026-08-22に不要と判断
```

本文書の「管理者除外の解消」は、2026-07-28に`enforce_admins`を有効化し、read-backで
`enabled = true`を確認したと記録している。**2026-08-22の読み出しでは`false`である。**
その間に無効化されたことになるが、**経緯は特定できていない。**
`delete_branch_on_merge`が2026-07-31に同種のdriftを起こしたときと同じく、変化の理由を
遡って確定する手段が無い。

影響は次に限られる。`main`の`allow_force_pushes: false`と`allow_deletions: false`は
設定として残っているが、**管理者はこの2つに拘束されない。**「管理者除外の解消」が述べた
「[AGENTS.md](../AGENTS.md)の force pushを行わない が`main`に限ってGitHub側でも実効化され、
AIエージェントの誤操作に対する防壁になる」という状態は、**現在は成立していない。**

**この照合では設定を戻していない。**そして2026-08-22に、repository所有者が
**再適用は不要と判断した。**したがって`enforce_admins: false`は**driftではなく
意図した状態である。**次の照合でこれを未解決として再度上げない。

判断の意味は限定して読む。**`main`のforce pushとbranch削除を止めているのは、
GitHubの設定ではなくGovernanceの規約とユーザー承認だけになる。**
[AGENTS.md](../AGENTS.md)は「共有branch（`main`／`develop`）の履歴を書き換えない。
force pushもしない」と定めており、AIエージェントに対してはこの規約が唯一の歯止めである。
**設定で強制されていると読まない。**

`## 確認済み`のcheckboxは外した。**未完了のtaskではなく決定である。**

## merge方式の設定とdesired state（2026-08-22）

```text
allow_squash_merge:  true
allow_merge_commit:  true
allow_rebase_merge:  true
allow_auto_merge:    false
```

[CONTRIBUTING.md](../CONTRIBUTING.md#merge方式)が認めるのは、base `develop`のsquash mergeと
base `main`のmerge commitだけである。**rebase mergeは規約に無いが、設定では選べる。**

これはdriftではない。本文書はこれまでmerge方式の設定についてdesired stateを記録していない。

**2026-08-22に、repository所有者が`allow_rebase_merge`を無効化しないと判断した。**
これをdesired stateとして記録する。**merge方式は設定では絞らず、
[CONTRIBUTING.md](../CONTRIBUTING.md#merge方式)の規約とreviewで守る。**
次の照合で「規約と設定が食い違っている」として再度上げない。

rebase mergeを選んだ場合、それは規約違反であって設定の不備ではない。
**設定が選択肢を残していることと、規約が1つに定めていることは両立する。**

## 追跡file全体のsecret走査（2026-08-22）

[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の受け入れ条件
「credential、個人情報、local secretがcommit／Issue／logへ含まれない」に対する確認である。

`scripts/lib/publish_guards.py`の`secret_like`と`personal_path`を、**追跡下の全fileへ**
当てた。公開対象だけを見る`prepare_pages.py`とは範囲が違う。

結果は次のとおりで、**実在するcredentialと個人pathは検出されなかった。**
一致した箇所はすべて、検出器自身のpattern定義か、その検出力を確かめるfixtureである。

```text
tracked files:          161（追跡下のsymlinkは0件）
scan対象外:             1（pages/assets/deskcat-concept.jpg。binary）
secret_like 一致:       3   すべて test_pages_guards.py のfixture
                            （変数名が secret、値はOUTSIDE-...-MUST-NOT-BE-PUBLISHED）
personal_path 一致:     6   3件は publish_guards.py のpattern定義そのもの
                            3件は test_*.py のfixture（架空のuser名を使う）
実在するsecret:         0
実在する個人path:       0
```

### 自前の全体走査scriptは追加しない

**GitHubのsecret scanningとpush protectionが同じ範囲を既に見ている。**2026-08-22の
read-backで両方`enabled`である（上記「2026-08-22のdesired stateとの全面照合」）。
GitHub側は履歴も含めて走査し、patternの維持もGitHubが行う。**これはGitHubの仕様であり、
こちらの実測ではない。**確認したのは両設定が有効であることだけである。

自前で全体走査を持つと、上の一致9件を通すための除外一覧が必要になる。除外は
**検出器自身のfileとそのfixture**、つまり最も編集されるfileを対象にすることになり、
**除外を保ちながら検出力を保つことができない。**同じ条件を2箇所で持つ弊害でもある。

`publish_guards.py`のpatternが担うのは**公開境界**である。`prepare_pages.py`と
`validate_pages_output.py`が公開対象に対して適用し、GitHub Pagesへ出る内容を止める。
この役割は変えない。

### 残る隙間

**個人pathは、公開対象でないfileについては機械的に見ていない。**GitHubのsecret scanningは
credentialを対象とし、Linuxのhome配下やWindowsのUsers配下を指す絶対pathは検出しない。
**この節に例を書かないのは、書けばそれ自身がpatternに一致するためである。**patternの正本は
`scripts/lib/publish_guards.py`にある。
`AGENTS.md`が「永続文書に個人のlocal pathを記載しない」と定めており、**この範囲は規約と
reviewが担保する。**上の走査で現状0件であることは確認したが、継続的な機械検査は無い。

## 保留

- CODEOWNERS: 安定したreviewer／owner対応が複数になった時点で追加
- Code of Conduct: 外部communityへ積極的に参加を求める前に追加
- Release workflow: versionとartifact方針の確定後に追加
- Discussions: community supportにIssueだけでは不足した場合に追加
- Signed commit: SSH署名で導入コストが小さいため、次のrepository運用変更で評価する
- Milestone due date: M0–M6すべて未設定。blocked Issueの滞留を可視化する目的で設定を検討する
