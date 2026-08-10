# DeskCatへのcontribution

DeskCatはfirmware、Linux software、電子回路、可動機構を組み合わせる。変更はreview可能で、証拠に基づく必要がある。

## 作業開始前

1. [AGENTS.md](AGENTS.md)を読む。
2. [Governance](docs/governance/README.md)を読む。
3. 一つの目的に絞ったIssueを探すか作成する。
4. 依存関係と受け入れ条件を確認する。
5. [ハードウェアTBD](docs/hardware/tbd-register.md)を確認する。
6. 編集前にworking treeを確認する。

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

| 対象 | 扱い |
|---|---|
| typo、リンク修正、表記ゆれ、規約の言い回し、boardのmetadata記入漏れ | **Issue不要。**変更内容の承認を得たうえで`develop`へ直接反映してよい |
| 仕様、安全、電気、protocol、GPIO、電源、toolchain、CI、依存の変更 | **Issue必須** |
| 複数のPull Requestに跨る、または他の作業をblockするもの | **Issue必須** |

Issue不要の側でも、**変更内容の承認は必ず得る。**「Issueを立てない」は「勝手に変えてよい」ではない。
判断に迷うものはIssue必須の側として扱う。安全に関わる範囲は緩めない。

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

作成時に、対応するIssueと同じassignee・label・milestoneを設定し、Projects v2 board
（`deskcat`、`https://github.com/users/wachi-yoshitaka-11-dev/projects/5`）へitemとして
追加して`Status`を設定する。boardが進行管理の正本であり、boardに無いPull Requestは
`Pull request merged` workflowの対象にならず、merge済みかどうかがboard上で追えない。

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

[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)により、自動reviewは高リスク変更（`area:firmware`、
`area:protocol`、`area:raspberry-pi`、`area:hardware`、`type:decision`）に限定している。
**それ以外のPull Requestでは、この自己レビューが唯一のreviewである。**

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

### Merge前の確認

**未解決のreview threadが0件であることを確認するまでmergeしない。**
base が`develop`でも`main`でも適用する。

**これはGitHubが強制する。**`main`と`develop`の双方でbranch protectionの
`Require conversation resolution before merging`を有効にしている（2026-08-10 設定・動作確認済み。
[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)）。
未解決threadが1件でもあれば`mergeStateStatus`が`BLOCKED`になり、mergeできない。
**手作業でthreadを数える必要はない。**

**thread のresolveは、reviewerの応答を読んでから行う。**指摘へ返信しただけで自動でresolveしない。
返信に対する応答（自動reviewなら指摘を取り下げたかの判定）を読み、解決したと確認できるthreadだけ
resolveする。自分の返信をもって直ちにresolveすると、`isResolved`が「対応したという主張」に退化し、
GitHubの強制が意味を失う。

#### GitHubが強制しないもの

**「checkが緑」は「reviewが行われた」を意味しない。**次の2つはcheck状態から見分けられない。

| CodeRabbitの状態 | checkの表示 | reviewの実行 |
|---|---|---|
| `Review in progress` | `pending` | 未完了 |
| **`Review rate limited`** | **`pass`** | **実行されていない** |

`pending`のままmergeしないのは当然として、**`pass`でも中身が`Review rate limited`なら
reviewは走っていない。**merge前にCodeRabbitのcheckの説明文を読む。
[#88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)で実際に発生し、
指摘対応後のcommitがreviewを受けないまま`pass`になった。

**thread 0件は、reviewが終わったことを意味しない。**
[#76](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/76)では0件を確認した**28秒後**に
reviewが届き、actionable comment 2件がmerge済みPull Requestへ付いた。
GitHubはthreadが存在しないものをblockできない。**reviewの到着を待たずに0件を「解決済み」と読まない。**

自動reviewの対象は[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)
で高リスク変更へ限定している。対象外のPull Requestでは[自己レビュー](#自己レビュー)が唯一のreviewである。

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

squash mergeしたbranchはcommit hashが変わるため、`git branch -d`が「未merge」と判定する。
削除前に`git diff <branch> origin/develop`が空であることを確認してから`-D`する。

昇格Pull Requestのmerge後は`origin/develop`が消えていないことを確認する。
[#33](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/33)で`delete_branch_on_merge`が
`develop`自体を削除する事故が起きている。Repository Rulesetで禁止済みだが確認はする。

## Gitと秘密情報

- `.env`、資格情報、token、秘密鍵をcommitしない。
- commit前にstage対象pathとdiffを確認する。
- build生成物をcommitしない。
- 通常作業でforce pushしない。
- 許可なく第三者参考資料を公開しない。

## Security上の報告

脆弱性や秘密情報をpublic Issueで開示しない。[SECURITY.md](SECURITY.md)に従う。
