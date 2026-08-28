# DeskCatへのcontribution

DeskCatはfirmware、Linux software、電子回路、可動機構を組み合わせる。変更はreview可能で、証拠に基づく必要がある。

## 全体の流れ

**各段で1つだけ落としやすいものを並べる。**規則は再掲しない。詳細は各節を参照する。

```text
着手前:  git fetch origin && git rev-parse --short origin/develop
merge前: 人間の承認を得る
merge時: squash messageへChange-ClassとSelf-Review 3値を入れる
merge後: 入ったことをmerge commitで確認する
review:  full reviewを使う
```

| 段 | 落としたときに起きること | 正本 |
|---|---|---|
| 着手前 | 古い基点で判断する。**後から入った文書を「存在しない」と読み、そこから誤った断定へ進む** | 下の[作業開始前](#作業開始前) |
| merge前 | **CIが緑でも機械reviewが完走しても承認ではない** | [Merge前の確認](#merge前の確認) |
| merge時 | trailerがsquash commitへ引き継がれない | [Merge方式](#merge方式) |
| merge後 | 引き継がれなかったことに次の昇格まで気付かない | [Merge方式](#merge方式) |
| review | `review`はincrementalで空振りし、枠だけ消費する | [手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する) |

**この表は覚えるためのものではない。**`.claude/settings.json`のhookが、着手前・merge時・
merge後を機械で止める。**hookが止めない段の実行手順は`.claude/skills/deskcat-preflight`が持つ**
（規則ではなく実行だけを持ち、規則はこの文書を参照する）。**merge前の承認とreviewの投げ方は止められない**（人の判断であり、
`review`と`full review`はどちらも同じ枠を消費するため機械で選べない）。
hookの一覧と回避手順は[hookが止めたとき](#hookが止めたとき)にある。

## 作業開始前

1. `git fetch origin`し、`origin/develop`を基点にする。
2. [AGENTS.md](AGENTS.md)を読む。
3. [Governance](docs/governance/README.md)を読む。
4. 一つの目的に絞ったIssueを探すか作成する。
5. 依存関係と受け入れ条件を確認する。
6. [ハードウェアTBD](docs/hardware/tbd-register.md)を確認する。
7. 編集前にworking treeを確認する。

正確な部品、関連GPIO、電源、安全値が`TBD`のときはhardware driverへ着手しない。

## Issueの範囲

Issueには次を含める。

- 背景
- 一つの目的
- 対象範囲と対象外
- 依存関係
- 正式な基準文書
- 測定可能な受け入れ条件
- PC確認
- 実機確認
- 保存する証拠

無関係なrefactor、新たに見つけた不具合、別実験には別Issueを使う。

### Issueを立てずに直接反映してよい範囲

**すべての気づきをIssueにしない。**記述の維持管理までIssue化すると、保守Issueが製品作業を追い越す。
実際、[#84](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/84)は規約の言い回しを直すために
起票し、直後にcloseした。

**Issueが要るかどうかと、Pull Requestが要るかどうかは別の問いである。**この2つを1つの段に
まとめていたため、規約文書のtypoを直すのにIssueまで要る状態になっていた
（[ADR-0011](docs/decisions/0011-issue-optional-pull-request-required.md)）。列を分ける。

| 対象 | Issue | Pull Request |
|---|---|---|
| boardのmetadata記入漏れ。repositoryの変更ではない | 不要 | 不要 |
| `review_gate.py`が`CLASS=minor`と判定した変更 | 不要 | 不要。承認を得たうえで`develop`へ直接反映してよい |
| **既にmergeされreviewを通った作業の後始末。**`Change-Class: fixup`と`Refs: #<番号>`を宣言する | 不要 | 不要。承認を得たうえで`develop`へ直接反映してよい |
| typo、リンク修正、表記ゆれ、言い回しの修正で、**意味を変えないもの** | 不要 | **必要** |
| 規約、仕様、安全、電気、protocol、GPIO、電源、toolchain、CI、依存の**意味**を変えるもの | **必須** | **必要** |
| 複数のPull Requestに跨る、または他の作業をblockするもの | **必須** | **必要** |

Issue不要の側でも、**変更内容の承認は必ず得る。**「Issueを立てない」は「勝手に変えてよい」ではない。
判断に迷うものはIssue必須の側として扱う。安全に関わる範囲は緩めない。

**「言い回しの修正」と「意味の変更」の境界は機械的に判定できない。**両者を区別するcodeは
無い（[ADR-0010](docs/decisions/0010-change-class-and-review-declaration.md)）。
**迷ったら「意味を変えるもの」として扱う。**
**ただし「言い回しの修正」もPull Requestを通る。**判断を誤っても、Review gateと自己レビューが
その差分を見る。黙って`develop`へ入ることはない。

**repositoryへの変更でPull Requestを省けるのは、`CLASS=minor`と後始末（`fixup`）の2つだけである。**
前者はscriptが判定し、後者は宣言する側が申告する（下の「後始末（`fixup`）の範囲」）。
boardのmetadata記入漏れはrepositoryの変更ではないため、Pull Requestという単位が無い。

**scriptが決めるのは`CLASS`だけである。**Issueが要るかどうかはscriptの判定ではない。

```bash
python3 scripts/review_gate.py classify --base origin/develop --head HEAD
```

`CLASS=minor`のときは、Pull Requestなしで`develop`へ直接反映してよい。
**`CLASS=review-required`でも、後始末として申告する経路がある**（下の
「[後始末（`fixup`）の範囲](#後始末fixupの範囲)」）。**それ以外で
`CLASS=review-required`を人の判断で覆さない。**scriptは軽微と証明できるものだけを`minor`に
するため、本当は軽微な変更も`review-required`へ落ちる。それは意図した失敗方向であり、
Pull Requestを1本作れば済む。

**この表を強制する仕組みは無い。**Review gateはPull Requestで起動するため、直接pushした変更は
見ない。`develop`のbranch protectionにも必須reviewと必須status checkを設定していない
（[ADR-0007](docs/decisions/0007-review-scope-and-self-review.md)の決定4）。
`classify`を実行する場面は、`develop`へ直接反映すると決めた場面と同じである。

### 後始末（`fixup`）の範囲

**`CLASS=minor`は構造的に出ない。**`INSTRUCTION_SOURCES`にpathが該当した時点で
`review-required`が決まり、行の中身の判定へ届かない。**この repository の文書作業は
ほぼ全部が該当する。**そのため上の表の`CLASS=minor`の行は、実際には空である。

そこで**申告による軽い経路**を置いた。`Change-Class: fixup`と`Refs: #<番号>`を宣言する。

対象:

- 既にmergeされreviewを通った作業の**後始末**。typo、リンク切れ、指摘対応の漏れ、記録の整理
- `Refs`でその Issue または Pull Request を指す。**後始末である以上、対象が存在する**

**絶対に対象外:**

- **安全値、protocol、GPIO、電源値、firmware、`crates/`。**新しい判断
- `docs/hardware/`の**値**の変更。**節の整理は可**

**この経路には強制点がある。**直接commitは`gate`を通らない（`gate`はPull Requestで起動する）。
しかし`history`が`develop`から`main`への昇格時に**範囲の各commitを検査する**ため、
宣言が壊れていれば次の昇格で落ちる。**`minor`経路が完全に無検証であるのより強い。**

**`fixup`は「軽微である」と主張しない。**`classify`の出力は変わらず、`review-required`の
まま出る。主張しているのは「後始末である」だけである。**そのため`minor`の判定も
`review-required`の判定も緩んでいない**（[ADR-0015](docs/decisions/0015-fixup-class-and-direct-commit-scope.md)）。

## Issueの命名

Issue titleにprefixを付けない。`<概要>`だけの素の説明文にする。
種別は`type:bug`／`type:feature`／`type:decision`／`type:experiment`／`type:maintenance`labelで表現する。
GitHubのIssue一覧はlabelを常にtitleの横に表示するため、titleにも同じ情報を重複して書く必要がない。

`[Bug]`のようなbracket prefixや、`FND-001`／`GH-005`のような連番付きID形式は使わない。
連番による識別が必要な場合は、GitHub Issue番号（`#1`等）をそのまま使う。

### 起票時に設定する項目

| 項目 | 必須／任意 | 値の決め方 |
|---|---|---|
| title | 必須 | 上記の形式 |
| milestone | 必須 | 対応するM0〜M6 milestoneを設定する |
| label（`type:*`／`priority:*`） | 必須 | それぞれ1つ設定する |
| label（`area:*`） | 対象componentがある場合のみ | 対象componentに対応するlabelを設定する。repository全体の保守作業など特定componentに限らない場合は省略する |
| label（`status:blocked`／`needs:*`） | 該当時のみ | 依存未解決なら`status:blocked`、実機証拠や人間の判断が必要なら`needs:hardware-test`／`needs:decision`を設定する |
| assignee | 必須 | 対応を担当する人を設定する。未定でも起票者自身を暫定assigneeとする |
| project | 必須 | Projects v2 board（`deskcat`、`https://github.com/users/wachi-yoshitaka-11-dev/projects/5`）にitemとして追加し、`Status`を設定する |
| 開始日／終了日（`Start date`／`Target date`） | 着手時は任意、**close後は必須** | 着手日・完了目標が具体的に決まった時点でProjects v2 board上で設定する。未定なら空欄のままでよい。**closeした後は、close実施者が`Target date`をJSTのclose実績日へ設定する** |

この表はIssue itemの規約である。Pull Request itemでは同じ2 fieldが必須であり、`Start date`は
作成日という実績、`Target date`はmerge見込み日という予定を表す。`Target date`はmergeまたは
closeの**完了後**に実績値へ更新する。[Pull request](#pull-request)節を参照する。

**milestoneはIssueだけに設定する。Pull Requestには設定しない**
（[ADR-0012](docs/decisions/0012-milestones-count-issues-only.md)）。milestoneは節目までに
出すものを数える道具であり、Pull Requestはその手段である。両方に付けると、1つのIssueを
何本のPull Requestに割ったかでmilestoneの件数が動く。

boardの`Item closed` workflowは`Status`を`Done`にするが、**日付fieldは更新しない。**
そのためIssueのclose後は、close実施者が手作業で`Target date`を実績日へ設定する。

## Branches

`main`は安定版とGitHub Pages公開元、`develop`は通常開発の統合先である。
通常は最新の`develop`からIssue branchを作成し、Pull Requestのbaseも`develop`にする。
Releaseまたはmilestoneの基準を満たした時点で、`develop`から`main`へPull Requestする。

推奨するIssue branch名:

```text
feature/<issue>-<short-name>
fix/<issue>-<short-name>
docs/<issue>-<short-name>
chore/<issue>-<short-name>
experiment/<issue>-<short-name>
hotfix/<issue>-<short-name>
```

一つのbranchを、一Issue、一つのreview可能な目的に絞る。
`main`から緊急hotfixを行った場合は、同じ修正を`develop`にも取り込む。
詳細は[ADR-0004](docs/decisions/0004-main-develop-branch-strategy.md)と
[Development Workflow](docs/governance/development-workflow.md)を参照する。

## 実装

- 無関係なユーザー変更を保持する。
- 単体componentから統合へ、小さな変更で進める。
- domain logicをhardware I/Oから独立させる。
- GPIOと安全設定を集約する。
- buffer、queue、retry、timeout、入力sizeに上限を設ける。
- 失敗を隠さず、型付きerror、log、counterを追加する。
- 別の安全reviewなしに`unsafe`を追加しない。
- 必要性、support、保守状況、license、代替を確認せずdependencyを追加しない。

## 検証

root READMEとcomponent READMEに記載された、関連するcommandをすべて実行する。

host workspaceはrepository rootで、ESP32 firmwareは`firmware/esp32`で検証する。確定したcommandは[root README](README.md#buildとtest)と`AGENTS.md`の「検証」節にある。Raspberry Pi、HIL、ESP32のflashとserial monitorには、まだ正式なcommandがない。

今後の変更では次を報告する。

- format結果
- lint結果
- unit／integration test結果
- 該当する場合はESP32 build結果
- ハードウェア構成
- 実機test結果
- 実行できなかった確認

PC testはLCD、電気、timing、sensor、機構の検証を代替しない。

## ハードウェア実験

ハードウェア実験Issue templateを使用し、次を記録する。

- 正確な部品とrevision
- 配線revision
- 電源と電流制限
- firmware commit／profile
- 測定器
- 手順
- 期待値と未加工の実測結果
- faultとreset reason
- 次の安全な作業

初回通電、初回サーボ動作、動作範囲拡張、電源・GPIO変更後のtestでは、人間がactuator電源を切れる状態にする。

## 文書

次を変更する場合は正式文書を更新する。

- アーキテクチャ
- GPIO
- 電源
- 部品identity
- protocol
- サーボ安全
- toolchain
- build、flash、test手順

複数componentに影響する判断や、戻すコストが高い判断にはADRを使う。

## Pull request

Pull requestには次を含める。

- 関連Issueへのlink（`Closes #N`等）
- 結果と範囲の説明
- 仕様変更の特定
- 検証証拠
- 残っている実機test
- 新規dependency
- 残存riskと`TBD`
- 無関係なformat変更やrefactorがないこと

作成直後に、boardへitemを追加して次を設定する。**空欄を検出する自動化は無いため、
ここが唯一のgateである**（詳細は[Pull Request itemの開始日／終了日](#pull-request-itemの開始日終了日)の
[誰がいつ確認するか](#誰がいつ確認するか)）。

- [ ] `Status`
- [ ] `Start date`（作成日。JSTで判断する）
- [ ] `Target date`（mergeを見込む日。**merge完了後またはclose完了後**に実績値へ更新する）

### labelは必ず付ける

**labelは`gh pr create --label`で指定する。**`area:*`／`type:*`／`priority:*`を付ける。

```bash
gh pr create --base develop --title "<title>" --body-file <path> \
  --label "<area:*>" --label "<type:*>" --label "<priority:*>" \
  --assignee "<login>"
```

**「作成時でなければならない」という縛りは廃止した**（[ADR-0013](docs/decisions/0013-manual-only-coderabbit-review.md)）。
理由はCodeRabbitが対象判定をPull Requestの作成直後に行うことだけであり、
**自動reviewを廃止したため判定そのものが無い。**当時の実測はADR-0013へ移した。

labelは引き続き必須である。仕分けと`review_gate.py`の分類に要る。
boardへのitem追加はPull Requestが存在しないと行えないため、作成直後に行う。

### 関連Issueの書き方

Issue branchからのPull Request（base `develop`）では`Closes #N`を使う。これはtraceability
目的の記載であり、GitHub純正の自動close機能は使わない。closeを起こすのはdefault branch
（`main`）へのmergeだけであり、base `develop`のmergeでは働かないためである。

**`develop`から`main`への昇格Pull Requestでは`Closes #N`を使わない。** baseが`main`
なのでGitHubの自動closeが実際に働き、boardによるclose管理と二重になる。昇格Pull Requestに
含まれるIssueは`Refs #N`かplain linkで列挙する。

代わりに、Projects v2 board（`deskcat`）でcloseを管理する。
boardでは6つのworkflowが有効である。全一覧はRepository設定に記録しており、
このうちcloseに関わるのは`Auto-close issue`と`Pull request merged`の2つである。

- `Pull request merged`: mergeされた**Pull Request item**の`Status`を`Done`にする
- `Auto-close issue`: **item**の`Status`が`Done`になったとき、そのitemのIssueをcloseする

この2つは別のitemに作用する。Pull Requestのmergeで`Done`になるのはPull Request item側
であり、Issue itemの`Status`は変わらない。したがって**mergeだけではIssueはcloseされない。**
merge後に、対応するIssue itemの`Status`を`Done`にする。これにより`Auto-close issue`が
Issueをcloseする。

作成時に、対応するIssueと同じassignee・labelを設定し、Projects v2 board
（`deskcat`、`https://github.com/users/wachi-yoshitaka-11-dev/projects/5`）へitemとして
追加して`Status`を設定する。boardが進行管理の正本であり、boardに無いPull Requestは
`Pull request merged` workflowの対象にならず、merge済みかどうかがboard上で追えない。
**milestoneは設定しない**（[ADR-0012](docs/decisions/0012-milestones-count-issues-only.md)）。

labelは[labelは必ず付ける](#labelは必ず付ける)に従い、`gh pr create --label`で指定する。
**時期の縛りは無い**（[ADR-0013](docs/decisions/0013-manual-only-coderabbit-review.md)）。

board上のworkflow構成は[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)に記録する。

### Pull Request itemの開始日／終了日

Pull Request itemの`Start date`／`Target date`は必須とし、次の値を設定する。
Issue itemでは両fieldが予定であり未定なら空欄でよいが、Pull Request itemの`Start date`は
作成時点で確定した実績であり、空欄にする理由がない。

| field | 値 | 設定時期 |
|---|---|---|
| `Start date` | Pull Requestの作成日。作成後は変更しない | 作成時 |
| `Target date` | 作成時はmergeを見込む日（予定）、mergeまたはclose後はその実績日 | 作成時に見込みを設定し、**merge完了後またはclose完了後**に実績値へ更新する |

`Target date`だけが予定から実績へ変わる。merge時に`Status`を`Done`にするworkflowは
日付を書き換えないため、実績値への更新は手作業で行う。mergeせずcloseした場合の実績日は
close日である。

**Issue itemでも、完了時には空欄を残さない。**上の「未定なら空欄でよい」は**未定である間の
扱い**である。`Status`が`Done`になった時点で開始日と終了日は確定しているため、空欄でよい
根拠が消える。close後に`Start date`へ着手日、`Target date`へclose日（実績）を入れる。
**Pull Request itemだけが実績を持つと読まない。**

日付はJST（UTC+9）で判断する。GitHubのAPIが返す時刻はUTCであり、JSTの`00:00`から`08:59`に
作成したPull RequestはUTCでは前日の日付になるため、そのまま転記しない。

#### 誰がいつ確認するか

この2 fieldを強制する自動化は無い。**空欄を検出する仕組みが無いため、次の手作業をgateとする。**

| 時期 | 実施者 | 確認内容 |
|---|---|---|
| Pull Request作成直後 | 作成者 | boardへitemを追加し、`Status`・`Start date`・`Target date`を設定する。`Pull request`節の作成時checklist（この文書の上部）に含めてある |
| **merge後** | merge実施者 | `Target date`をJSTのmerge実績日へ更新する |
| **close後** | close実施者 | `Target date`をJSTのclose実績日へ更新する |
| **reopen後** | reopen実施者 | `Target date`を新しいmerge見込み日へ戻す。再度mergeまたはcloseしたら、上の2行に従って実績日へ更新する |

**更新は「後」であって「直前」ではない。**merge前に実績日を確定できないためである。JSTの日付を
またいだ場合や、mergeを中止した場合に、誤った実績日が残る。

**自動化しない。**理由は2段ある。

1. Projects v2のboard workflowは日付fieldを更新できない。`Status`を`Done`にするworkflowでは書き換えられない
2. GitHub Actionsから叩く案も採らない。**`GITHUB_TOKEN`はrepository scopeであり、Projects v2へ
   アクセスできない**（[GitHub Docs](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects)）。
   `project` scopeを持つclassic personal access token、またはGitHub Appが必要になる。
   日付2 fieldのためにCIへ長期secretを持ち込む取引は成立しない

したがって**手作業を正式な手順とする。**これは妥協ではなく判断である。
以後「自動化できるはず」として再検討しない。前提が変わるのは、Projects v2のworkflowが
日付fieldを扱えるようになったときだけである。

**忘れることが唯一の失敗モードである。**実際に[#71](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/71)・
[#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72)・[#73](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/73)で
3件続けて空のままcloseした。**空欄を検出する仕組みは無い。**closeの操作と同じ場面で設定する。

### 自己レビュー

pushする前に、作成者自身が差分を見直す。**新規指摘が0件の状態が2 round続くまで繰り返す。**

**自動reviewは行わない**（[ADR-0013](docs/decisions/0013-manual-only-coderabbit-review.md)）。
[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)は
`enabled: false`だけを持つ。**したがって、この自己レビューが既定で唯一のreviewである。**

**意味上criticalな変更では、自己レビューの後で手動でreviewを依頼する。最大1回。**
判断は人が行い、機械的な判定は置かない。安全・電気・protocol・firmwareに関わる変更を
自己レビューで代替しない。手順は[手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)にある。

回数だけを守っても、同じ観点を繰り返しなぞるだけになる。次の観点で見る。

- [ ] **受け入れ条件を1つずつ差分と突き合わせた。**「だいたい満たしている」で通さない
- [ ] **正本文書と矛盾する記述を新たに作っていない。**同じ定数・同じ規約を2箇所に書いていない。
      片方だけを見た判断が起きる
- [ ] **未確認の値を確定として書いていない。**実測していない値、一次資料で確かめていない型番、
      未検証の動作を、断定形で書いていない（[AGENTS.md](AGENTS.md)の推測禁止を自分の差分へ適用する）
- [ ] **参照先が実在する。**リンク先の文書・節・表が存在し、そこに書いてあると主張した内容が実際にある
- [ ] **公開されない路への相対linkを張っていない。**`CONTRIBUTING.md`、`.coderabbit.yaml`、
      `.github/`配下などはGitHub Pagesへ公開されない。公開文書からこれらへ相対linkを張ると
      `validate_doc_links.py`が失敗する。**絶対URLを使う。**
      [PR #90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)ではこの誤りだけでCIを3回落とした
- [ ] **差分に、このPull Requestの目的と無関係な変更が混ざっていない**
- [ ] **Pull Request本文の記述が、実際の差分と一致している。**対象file数、含まれる変更、
      検証欄の「実行した／していない」が実態と合っている

各項目は過去に実際に起きた失敗に対応する。順に
[#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72)（規則を守っているか一度も照合していなかった）、
[hardware-bom.md Revision 20](docs/hardware/hardware-bom.md)（同じ条件を2文書に書き、式が食い違った）、
[#63](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/63)・[#82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)（未検証の動作を断定した）、
[#82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)（存在しない照合先を参照していた）、
[#61](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61)（本文が「4 Pull Request、14 file」のまま、実際は9 commitへ増えていた）である。

#### 2つのPass

回数とは別に、**最終diffに対して次の2つを別々に実施する。**同じ読み方を2回繰り返しても、
拾えるものは増えない。

**要件照合Pass。**受け入れ条件、Issueの対象範囲、上のchecklistの観点を、差分と1つずつ
突き合わせる。「だいたい満たしている」で通さない。**満たしていない項目は、満たしていないと
書く。**このPassは、何を作るはずだったかを手元に置いて読む。

**fresh-context Pass。****何を作るつもりだったかを一旦忘れて、差分だけを読む。**
作成者は意図を頭に持っているため、書いていないことを読み取って補完してしまう。
初めてこのrepositoryを見た人が、その差分だけで同じ結論に辿り着くかを見る。
このPassで拾えるのは、意図を知らないと意味が通らない記述、宣言だけで根拠が無い主張、
前提の書き漏れである。

**2つのPassは同じ最終diffに対して行う。**どちらかの後に差分が変わったら、
**両方が無効**になる。差分を変えたら2つとも実施し直す。

宣言はcommit trailerで行う。**書式の例は[Merge方式](#merge方式)にあり、値の正本は
`scripts/review_gate.py`である。**trailerはcommitへ結び付くため、差分を変えると宣言が
自動的に無効になる。**収束の宣言も同じtrailerで行う。**回数と2つのPassは別の軸であり、
1つの値にまとめるとどちらをやっていないのかが分からなくなる。

### Merge前の確認

**未解決のreview threadが0件であることを確認するまでmergeしない。**
base が`develop`でも`main`でも適用する。

**これはGitHubが強制する。**`main`と`develop`の双方でbranch protectionの
`Require conversation resolution before merging`を有効にしている（2026-08-10 設定・動作確認済み。
[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)）。
未解決threadが1件でもあれば`mergeStateStatus`が`BLOCKED`になり、mergeできない。
**手作業でthreadを数える必要はない。**

**ただし強制されるのは管理者以外である。**`develop`は`enforce_admins`を`false`にしており
（[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)）、
管理者は`BLOCKED`のままmergeできる。**この経路を使う場合は自動化が何も止めないため、
[未解決を残してmergeする場合](#未解決を残してmergeする場合)の手順を必ず踏む。**
「GitHubが止めるはずだ」を根拠にしない。

**thread のresolveは、reviewerの応答を読んでから行う。**指摘へ返信しただけで自動でresolveしない。
返信に対する応答（自動reviewなら指摘を取り下げたかの判定）を読み、解決したと確認できるthreadだけ
resolveする。自分の返信をもって直ちにresolveすると、`isResolved`が「対応したという主張」に退化し、
GitHubの強制が意味を失う。

#### GitHubが強制しないもの

**自動reviewは行わない**（[ADR-0013](docs/decisions/0013-manual-only-coderabbit-review.md)）。
そのため`Review skipped`は**既定の状態であり、異常ではない。**以下は**手動で依頼した
reviewの結果を読むため**、および過去の観測を履歴として残すためにある。
allowlistに言及する記述は、廃止前の設定に対する観測である。

**「checkが緑」は「reviewが行われた」を意味しない。**次の3つはcheck状態から見分けられない。

| CodeRabbitの状態 | checkの表示 | reviewの実行 | 原因 |
|---|---|---|---|
| `Review in progress` | `pending` | 未完了 | — |
| **`Review rate limited`** | **`pass`** | **実行されていない** | 毎時の枠切れ。**対象判定は通っている** |
| **`Review skipped`** | **`pass`** | **実行されていない** | 自動reviewを行わない設定である（ADR-0013）。**自動review側では既定の状態であり、異常ではない。**手動依頼が空振りした場合もこの表示になる |

`pending`のままmergeしないのは当然として、**`pass`でも中身が`Review rate limited`または
`Review skipped`ならreviewは走っていない。**merge前にCodeRabbitのcheckの説明文を読む。
[#88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)では`Review rate limited`が実際に発生し、
指摘対応後のcommitがreviewを受けないまま`pass`になった。

**この2つを混同しない。**`Review rate limited`は**対象判定を通過した後**の枠切れであり、
枠が空けば同じPull Requestで実行できる。`Review skipped`は**対象判定で外れている**ため、
枠が空いても自動では走らない。
[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)では**両方が別々の理由で観測されている**
（[#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94#issuecomment-5242073164)。
枠を使い切った経緯は後述の[手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)にある）。
**「skipされた」と「rate limitに当たった」を同じ言葉で記録しない。**

**`Review skipped`でも`Review rate limited`でもない中断もある。**
[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)では`2026-08-15T16:34:34Z`に
`Review stopped after lock loss`（`state`は`failure`）を観測した。**これは`pass`ではなくcheckが赤くなる**ため、
上の3つと違い表示で気付ける。ただし**reviewは完走していない。**
[手動で依頼する](#手動で依頼する前に状態を確認する)。#125では`full review`で`Review completed`へ到達した。

**`Review skipped`の説明文で、skipの原因を切り分けられる。**
観測した文言と、それぞれの読み方・対応は
[CodeRabbitのreview状態の観測記録](docs/runbooks/coderabbit-review-observations.md)にある。
**文言はCodeRabbit側のものであり、将来変わりうる。**一致しない文言を見たら、
推測せず実際の表示を記録し、同記録へ追記する。

**同記録は履歴である。**読むのは、見た文言が既知かどうかを調べるときだけでよい。
**merge前に必要なのは上の3つの区別だけである。**

#### 手動で依頼する前に状態を確認する

**投げる前に必ず次を実行する。**これは**reviewを消費せず**、残数と次に空く時刻を返す。

```text
@coderabbitai rate limit
```

| 確認結果 | 行動 |
|---|---|
| 残数が0 | **投げない。**返ってきた時刻まで待つ |
| 残数があり、**まだ一度もreviewが走っていない** | **`@coderabbitai full review` を使う。**自動reviewを行わないため、依頼はいつもこの状態から始まる。`review`はincrementalであり、skip後は空振りしうる |
| 残数があり、依頼したreviewは走ったが対応commitが未review | `@coderabbitai review`。**ただし対応commitは自己レビューで見るのが既定であり、投げ直さない**（[Merge前の確認](#merge前の確認)） |

`review`と`full review`は別物である。`review`は**新しい変更のみ**、`full review`は**全fileを最初から**
review する。**どちらも同じ毎時上限を消費する。**`full review`は枠を回避する手段ではない。

**同じ状態で2回以上投げない。**空振りなのかrate limitなのかを判別せずに繰り返すと、
枠だけを消費して**他のPull Requestのreviewを止める。**
実際、[#88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)・
[#90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)・
[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で計4回投げ、3回がrate limitに当たり、
[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)のreviewを遅らせた。

上限は開発者identity単位のrolling windowである（Pro 5件／時、Pro+ 10件／時）。
一定時刻にまとめて戻るのではなく、古いreviewが枠から抜けるたびに1件ずつ空く。

**「reviewを依頼した」を「reviewを受けた」と書かない。**`Review rate limited`は`pass`と表示されるため、
依頼の事実だけで完了扱いにすると検出が抜ける。

#### 未解決を残してmergeする場合

branch protectionの`enforce_admins`は無効であり、管理者は強制mergeできる。その場合は次をすべて行う。
**追跡Issueなしにmergeしない。**

1. 追跡Issueを起票する
2. 該当threadへ返信し、追跡Issue番号を書く
3. Pull Request本文の`Review thread`節の`追跡Issue`欄へ番号を書く
4. merge報告に未解決だったことと追跡Issue番号を記載する

[#40](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/40)では26 threadのうち1件が未解決のまま
mergeされ、追跡も無かった。後から[#74](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/74)として
起票し直している。

### Merge方式

baseで決まる。

| base | 方式 | 理由 |
|---|---|---|
| `develop` | **squash merge** | Issue branchの試行錯誤を1 commitにまとめ、`develop`の履歴を「1 Issue = 1 commit」に保つ |
| `main` | **squashしない。merge commitにする** | 昇格をsquashすると`develop`側の個々のcommitが`main`の履歴から消え、両branchが別系列になる |

**squash merge時に、分類と自己レビューをtrailerで宣言する。**trailerはcommitへ結び付くため、
review後にcommitが増えると宣言が自動的に無効になる（[ADR-0010](docs/decisions/0010-change-class-and-review-declaration.md)）。
`main`昇格workflowが、`develop`のtip commitにこの宣言があることを要求する。

```bash
gh pr merge <N> --squash --subject "<Pull Requestの題名>" --body-file <path>
```

**`--subject`と`--body-file`を省略しない。**引数なしの`gh pr merge --squash`はGitHubが合成した
messageでmergeし、**Pull Requestのhead commitが持っていたtrailerは引き継がれない。**
mergeは成功し、警告も出ない。

`--body-file`の内容には、末尾へ次のtrailerを置く。**指示sourceを変更していない場合、
`Instruction-Change`の行は書かない。**残る`Change-Class` 1行と`Self-Review` 3行はすべて要る。

```text
Change-Class: review-required
Self-Review: requirements-pass
Self-Review: fresh-context-pass
Self-Review: converged
Instruction-Change: reviewed-as-data
```

**`Change-Class: fixup`を宣言する場合は、`Refs`で後始末の対象を示す。**
番号を含まない`Refs`は通らない。

```text
Change-Class: fixup
Self-Review: requirements-pass
Self-Review: fresh-context-pass
Self-Review: converged
Refs: #206
```

**`Refs`にはコロンが要る。**`git interpret-trailers`はコロンの無い行をtrailerとして
読まない。この repository のcommit messageは`Refs #204`（コロンなし）を本文の段落として
書いてきたが、**それはtrailerではない。**

**そしてコロンの無い行をtrailer blockと同じ段落へ置くと、blockごと無効になる。**
`Change-Class`も`Self-Review`も消え、gateは「trailerが無い」と報告する。実測した。

```text
本文

Refs #200
Change-Class: review-required
Self-Review: converged
```

上のmessageから`git interpret-trailers --parse`が返すのは**空である。**
本文の段落として`Refs #204`を書く場合は、**trailer blockと空行で分ける。**
`fixup`の宣言に使う場合は、**block の中へ`Refs: #204`と書く。**

**trailerの名前と値の正本は`scripts/review_gate.py`である。**

**この節は、省略する行を「3行目」と行番号で指定していた。**
[#161](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/161)でこの節を書いたとき上のblockは3行で、3行目は`Instruction-Change`だった。
[#164](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/164)が`Self-Review`を3値へ分けてblockが5行になった際、
**古い記述だけが取り残された。**指す先は`Self-Review: fresh-context-pass`へずれており、
**従うと`receipt`が落ちる**（3値すべてを要求するため）。
**行番号で指定しない。**値が増減するとずれる。

**`main`昇格では、範囲の各commitの宣言も検証される。**squash commitへtrailerを書き忘れると、その回のmergeは通っても次の昇格で落ちる。

**ただし検査に起点がある。**trailer運用を導入したcommit（[#161](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/161)のsquash）より前は検査しない。**宣言を求める規則が存在しなかったためである。**起点より後にも、宣言を持たないことを許しているcommitがある。`AGENTS.md`が共有branchの履歴書き換えを禁じているため、後からtrailerを付けられない。**起点と免除の正本は`scripts/review_gate.py`の`DECLARATION_CUTOVER`と`DECLARATION_EXEMPT`であり、免除の理由は同fileが持つ。**

そのため**`HISTORY_CHECKED`は昇格範囲のcommit数より少なくなりうる。取りこぼしではない。**検査した件数、skipしたmerge commitの件数、免除した件数を必ず出すため、差はその内訳で説明できる。**この数を期待値として引用しない。**範囲は昇格ごとに変わる。

**過去のcommitへの要求は、head commitより軽い。**`Change-Class`が計算結果より緩くないことと、`Self-Review`が1つ以上あることだけを見る。`Self-Review`の値の集合は時期によって変わっており、**過去のcommitを現在の集合で測ると、当時は正しかった宣言が落ちる。**

**Pull Requestのhead commitにも同じtrailerを置く。**Review gate workflowがそれを見る。
head commitへ置いておけば、review後にcommitを足したときにworkflowが落ちる。
**squash commitのtrailerは、mergeするまで存在しないためmerge前に検査できない。**
merge前にlocalで確認できるのは、自分のbranchのhead commitに対する次である。

```bash
python3 scripts/review_gate.py gate --base origin/develop --head HEAD
```

merge後に`develop`のtipへ宣言が入ったことは、次で確認する。ここが`main`昇格の前提になる。

```bash
python3 scripts/review_gate.py receipt --base origin/develop~1 --head origin/develop
```

**この確認をskipすると取り返しがつかない。**`AGENTS.md`が共有branchの履歴書き換えを禁じているため、
mergeしたsquash commitへ後からtrailerを付ける手段は無い。残るのは`DECLARATION_EXEMPT`へ
登録する判断だけであり、それはIssueとPull Requestを1本ずつ要する。
[#197](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/197)で実際に2 commit分を払っている。

squash mergeしたbranchはcommit hashが変わるため、`git branch -d`が「未merge」と判定する。
削除前に`git diff <branch> origin/develop`が空であることを確認してから`-D`する。

昇格Pull Requestのmerge後は`origin/develop`が消えていないことを確認する。
[#33](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/33)で`delete_branch_on_merge`が
`develop`自体を削除する事故が起きている。Repository Rulesetで禁止済みだが確認はする。

## hookが止めたとき

`.claude/settings.json`が4つのscriptをhookとして起動する。**検査は6つである。**

**文書に書いても実行されないことが実測で分かっている。**
[#204](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/204)、
[#205](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/205)、
[#206](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/206)は
**3件とも作成時にboardへ入っておらず、5分55秒〜18分29秒後に追加されている**
（`added_to_project_v2`のevent時刻と作成時刻の差）。規約は当時も同じだった。
そこで[全体の流れ](#全体の流れ)のうち機械で判定できる段をhookで止めている。

| いつ | 何を見る | 実体 |
|---|---|---|
| `gh issue create`／`gh pr create`の前 | `--project`（短縮形`-p`）があるか | `scripts/hooks/gh_metadata_guard.py` |
| `gh pr create`の前 | `--base`（短縮形`-B`）があるか | 同上 |
| `gh pr merge`の前 | squash messageが`Change-Class`と`Self-Review`を持つか | 同上 |
| `git checkout -b`／`git switch -c`の前 | 基点が最新の`origin/develop`か | `scripts/hooks/branch_base_guard.py` |
| `gh pr merge`の後 | squash commitに実際に入ったか | `scripts/hooks/merge_trailer_report.py` |
| `develop`へ直接pushする前 | 押す範囲が`review_gate.py gate`を通るか | `scripts/hooks/push_gate.py` |

**判定は字句だけで行う。**意味は判定しない（`scripts/review_gate.py`と同じ方針）。

### 止まったときにどうするか

**まず、指摘が当たっているかを見る。**当たっていれば、足りないものを足して再実行する。

誤検知のときは環境変数で無効化できる。**逃げ道であって常用するものではない。**
使ったらPull Request本文へ理由を書く。

```bash
DESKCAT_SKIP_GH_GUARD=1 gh pr create --title "..." --body-file body.md
DESKCAT_SKIP_BASE_GUARD=1 git checkout -b experiment/scratch
DESKCAT_SKIP_PUSH_GATE=1 git push origin HEAD:develop
```

> **上の1行目の形は効かない。**2026-08-28に
> `DESKCAT_SKIP_GH_GUARD=1 gh issue create --help`を実行し、**拒否された。**
> hookは対象commandとは別のprocessとして起動されるため、command行頭の環境変数代入は
> hookのenvironmentへ届かない。**例はこれまでの記述のまま残してある。**
>
> **`DESKCAT_SKIP_BASE_GUARD`と`DESKCAT_SKIP_PUSH_GATE`は実測していない。**
> hookの起動のされ方が同じであるため同じ見込みだが、**同じであると断定しない。**
>
> **効く形は未確定である。**hookのprocessが継ぐenvironmentへ入れる必要がある、
> というところまでしか分かっていない。**機構の扱いは別に判断する。**

`branch_base_guard.py`は、基点を明示した場合（`git checkout -b <name> <start>`）と
`hotfix/`で始まるbranchを対象外にする。`hotfix/`は`main`から作るのが正しい
（[ADR-0004](docs/decisions/0004-main-develop-branch-strategy.md)）。

`push_gate.py`は`develop`を更新するpushだけを見る。**他のbranchへのpushは見ない。**
Pull Requestを通る変更は`review-gate.yml`が`gate`を実行するためである。
**直接pushはCIを通らないため、そこだけが検査の無い経路だった。**

**このhookは`gate`を実行するだけで、直接commitしてよい基準そのものは検査しない。**
検査すると、trailerを落としたsquash commitをamendして直す手順
（[Merge方式](#merge方式)）を誤って止める。その手順の対象は
`Change-Class: review-required`を持つため、基準で測ると拒否になる。
**字句で区別する手立てが無い。**

### 取り切れていないもの

**hookを「通った」ことを「正しい」と読まない。**次は検査できていない。

- **`--base`の値そのもの。**存在するかだけを見る。`develop`と書くべきか`main`と
  書くべきかはそのPull Requestの目的で決まり、字句からは読めない。
  **`--base main`は昇格では正しく、日常のPull Requestでは誤りである。**
  hookが直すのは「省略して既定の`main`になる」であって、選び間違いではない
  （2026-08-28に[PR #250](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/250)が
  base `main`で作られ、変更まで1時間17分かかった）。
- **`gh issue create`には`--base`を要求しない。**option自体が存在しない。
- **`--help`／`-h`が付いた呼び出しは、`gh_metadata_guard.py`の3つの検査すべてを抜ける。**helpの表示は何も作らず、
  mergeもしないため、誤検知しか生まない。**2026-08-28に`gh issue create --help`、
  `gh pr create --help`、`gh pr merge --help`の3つとも拒否されることを実測した。
  hookが要求するoption名を`--help`で調べる手段そのものが塞がっていた。**
  判定は完全一致であり、`--title -h`のように値として`-h`を渡した呼び出しも
  検査を抜ける。**字句だけで区別する手立てが無い。**
- **squash messageをstdin（`-F -`）で渡す経路。**hookからは読めないため、
  読めないことを理由に止める。`--body-file`を使う。
- **`gh`や`git`を、alias、shell function、`xargs`、`sh -c`の内側から起動した場合。**
  hookはcommandの字句だけを見るため、呼び出しとして拾えない。
  **推測で拾わないのは、誤検知がhookごと無効化される側の失敗だからである。**
- **branchをhook以外の経路で作った場合。**worktreeを外部の道具が作ると
  `git checkout -b`を通らないため、基点は検査されない。
- **`git fetch`ができない環境。**基点の検査は行わず、黙って通る。
- **merge後の確認は事後である。**入っていなければ履歴書き換えなしには直せない。
  それでも報告するのは、免除へ回す判断を次の昇格まで先延ばしにしないためである。
- **`develop`へ入る経路のうち、`gh pr merge`によるものは`push_gate.py`を通らない。**
  そちらはPull RequestのCIで`gate`が済んでいる。
- **`push_gate.py`が見る`develop`の書き方は`develop`と`refs/heads/develop`だけである。**
  `--mirror`と`--all`のように、refspecを書かずに複数branchを更新する形は拾えない。
- **`gate`が時間内に終わらない場合は通す。**止めないのは、遅い環境で作業を止めないためである。
  **通ったことを、検査したことと読まない。**

**hookで安全要件を代替しない。**hookが直すのは「忘れる」であって、
[Hardware Safety Policy](docs/governance/hardware-safety-policy.md)が要求する根拠ではない。

## Gitと秘密情報

- `.env`、資格情報、token、秘密鍵をcommitしない。
- commit前にstage対象pathとdiffを確認する。
- build生成物をcommitしない。
- **共有branch（`main`／`develop`）へforce pushしない。**自分の未push・未mergeのbranchでは使ってよい（[Development Workflow](docs/governance/development-workflow.md)）。
- 許可なく第三者参考資料を公開しない。

## Security上の報告

脆弱性や秘密情報をpublic Issueで開示しない。[SECURITY.md](SECURITY.md)に従う。
