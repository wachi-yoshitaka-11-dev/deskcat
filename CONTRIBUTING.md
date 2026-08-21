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

**Issueが要るかどうかと、Pull Requestが要るかどうかは別の問いである。**この2つを1つの段に
まとめていたため、規約文書のtypoを直すのにIssueまで要る状態になっていた
（[ADR-0011](docs/decisions/0011-issue-optional-pull-request-required.md)）。列を分ける。

| 対象 | Issue | Pull Request |
|---|---|---|
| boardのmetadata記入漏れ。repositoryの変更ではない | 不要 | 不要 |
| `review_gate.py`が`CLASS=minor`と判定した変更 | 不要 | 不要。承認を得たうえで`develop`へ直接反映してよい |
| typo、リンク修正、表記ゆれ、言い回しの修正で、**意味を変えないもの** | 不要 | **必要** |
| 規約、仕様、安全、電気、protocol、GPIO、電源、toolchain、CI、依存の**意味**を変えるもの | **必須** | **必要** |
| 複数のPull Requestに跨る、または他の作業をblockするもの | **必須** | **必要** |

Issue不要の側でも、**変更内容の承認は必ず得る。**「Issueを立てない」は「勝手に変えてよい」ではない。
判断に迷うものはIssue必須の側として扱う。安全に関わる範囲は緩めない。

**3行目と4行目の境界は機械的に判定できない。**「言い回しの修正」と「意味の変更」を区別するcodeは
無い（[ADR-0010](docs/decisions/0010-change-class-and-review-declaration.md)）。迷ったら4行目にする。
**ただし3行目はPull Requestを通る。**判断を誤っても、Review gateと自己レビューがその差分を見る。
黙って`develop`へ入ることはない。**repositoryへの変更でPull Requestを省けるのは2行目だけであり、
そこはscriptが判定する。**1行目はrepositoryの変更ではないため、Pull Requestという単位が無い。

**scriptが決めるのは`CLASS`だけである。**Issueが要るかどうかはscriptの判定ではない。

```bash
python3 scripts/review_gate.py classify --base origin/develop --head HEAD
```

`CLASS=minor`のときだけ、Pull Requestなしで`develop`へ直接反映してよい。
**`CLASS=review-required`を人の判断で覆さない。**scriptは軽微と証明できるものだけを`minor`に
するため、本当は軽微な変更も`review-required`へ落ちる。それは意図した失敗方向であり、
Pull Requestを1本作れば済む。

**この表を強制する仕組みは無い。**Review gateはPull Requestで起動するため、直接pushした変更は
見ない。`develop`のbranch protectionにも必須reviewと必須status checkを設定していない
（[ADR-0007](docs/decisions/0007-review-scope-and-self-review.md)の決定4）。
`classify`を実行する場面は、`develop`へ直接反映すると決めた場面と同じである。

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

### labelは作成時に付ける

**labelは`gh pr create --label`で作成時に指定する。作成後に付け足さない。**

```bash
gh pr create --base develop --title "<title>" --body-file <path> \
  --label "<area:*>" --label "<type:*>" --label "<priority:*>" \
  --assignee "<login>" --milestone "<milestone>"
```

**CodeRabbitは対象判定をPull Requestの作成直後に行い、後からのlabel追加では再判定しない。**
[#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94#issuecomment-5242077436)での
CodeRabbit自身の回答である。**こちらの実測ではない。**

実測で確かめたのは次の2点である。いずれも
[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)が
`develop`にある状態で作成したPull Requestである。

| 作成時のlabel | Pull Request | 結果 |
|---|---|---|
| allowlistに一致する（当時は`type:decision`） | [#90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)・[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91) | **対象判定を通過した。**#91 は作成の4分47秒後に自動reviewが提出され、完走した（手動依頼より前） |
| labelはあるがallowlistに一致しない | [#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)・[#96](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/96) | `Review skipped: excluded by label configuration` |

**「後から付けたので発火しなかった」ことを実測した記録は無い。**
[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)は作成の24秒後にlabel無しで
`Review skipped`となり、labelは33分後に付いた。しかし**その判定の5秒後に`.coderabbit.yaml`が
`develop`へmergeされている**（判定 `12:22:00Z`、merge `12:22:05Z`）。
**判定の時点でこの設定はbase branchに無く、labelの未付与と設定の未反映が切り分けられない。**
Issue `#89` を「後付けが原因」の証拠として使わない。

**それでも作成時に付ける。**CodeRabbitの回答に沿う運用であり、費用は`--label`を足すだけで、
取り落としの可能性を消せる。**切り分けられない以上、確実な側に倒す。**

**この規則が要るのはlabelだけである。**assignee・milestoneも同じcommandで指定するが、
自動reviewの判定に関わらないため、後から設定しても失うものは無い。boardへのitem追加は
Pull Requestが存在しないと行えないため、上記のとおり作成直後に行う。

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

**このうちlabelだけは作成後では間に合わないおそれがある。**自動reviewの対象判定が
作成直後に終わるためである。[labelは作成時に付ける](#labelは作成時に付ける)に従い、
`gh pr create --label`で指定する。

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

[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)により、
自動reviewは高リスク変更（firmware、protocol、Raspberry Pi、hardware）を示すlabelを持つ
Pull Requestに限定している。**それ以外のPull Requestでは、この自己レビューが唯一のreviewである。**
**対象labelを持つPull Requestでも、labelを作成後に付けた場合は自動reviewを
受けられないおそれがある**（[labelは作成時に付ける](#labelは作成時に付ける)）。
その場合もこの自己レビューが唯一のreviewになる。

**対象labelの正本は`.coderabbit.yaml`であり、ここでは再掲しない。**値を2箇所に書くと
片方だけを見た判断が起きる。実際、[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で
`type:decision`をallowlistから外した後も、この節には5つ目として残っていた。

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

**「checkが緑」は「reviewが行われた」を意味しない。**次の3つはcheck状態から見分けられない。

| CodeRabbitの状態 | checkの表示 | reviewの実行 | 原因 |
|---|---|---|---|
| `Review in progress` | `pending` | 未完了 | — |
| **`Review rate limited`** | **`pass`** | **実行されていない** | 毎時の枠切れ。**対象判定は通っている** |
| **`Review skipped`** | **`pass`** | **実行されていない** | 対象判定で外れた。allowlistのlabelが**作成時に**無かった場合を含む |

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

**`Review skipped`の説明文で、skipの原因を切り分けられる。**これまでに観測した文言は次のとおりである。
**文言はCodeRabbit側のものであり、将来変わりうる。**一致しない文言を見たら、推測せず実際の表示を記録する。

**説明文の出どころは2つあり、同じ事象でも文言が違う。**commit statusの`description`と、
CodeRabbitがPull Requestへ投稿するcommentの本文である。たとえば前者は
`Review skipped: automatic reviews are disabled`、後者は`Auto reviews are disabled on this repository.`と表示される。
**記録するときはどちらで見たかを併記する。**commentの本文はCodeRabbitが後から書き換えるため、
review完走後には最初の文言が残らない。**後から検証できるのはcommit statusの履歴だけである。**
GitHubのcheck欄は文脈ごとの最新1件しか表示しないため、履歴は次で読む。

```bash
gh api --paginate "repos/<owner>/<repo>/commits/<sha>/statuses?per_page=100" \
  --jq '.[] | select(.context=="CodeRabbit") | [.created_at,.state,.description] | @tsv'
```

| 説明文 | 読み方 | 観測した Pull Request |
|---|---|---|
| `excluded by label configuration` | **設定は効いている。**labelは付いていたが、allowlistに一致しなかった。**対象外として正しくskipされた** | [#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)・[#96](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/96)（`area:docs`＋`type:maintenance`） |
| comment: `Auto reviews are disabled on this repository.`<br>status: `Review skipped: automatic reviews are disabled` | **設定が定着する前の観測である。**[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)の判定時点では`.coderabbit.yaml`がまだ`develop`に無かった。**同じ文言を設定の定着後に見たら5行目である** | #89（判定の5秒後に設定がmergeされた） |
| `reviews are disabled for this base branch` | baseが対象外と判定された | [#88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)（`.coderabbit.yaml`を`develop`へmergeする前） |
| `manual review required for this OSS repository` | **labelの判定では説明できない。**allowlistのlabelが作成時から付いており、1行目には当たらない。#127では`@coderabbitai rate limit`が`Reviews are available now`を返したためrate limitでもなかった。**`@coderabbitai full review`を投げると実際にreviewが走った。****2026-08-16に原因が判明した。**[#135](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/135)でCodeRabbitが投稿したcommentが`Reviews should be triggered manually for repositories with fewer than 10 stars.`と述べている。**star数による条件であり、設定の誤りではない。**この文言を見たら設定を疑わず、手動で`full review`を投げる | [#127](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/127)（`area:firmware`＋`area:protocol`）・[#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)（後の2件は5行目と同時に観測） |
| **2行目と同じ文言を、設定の定着後に観測した**<br>status: `Review skipped: automatic reviews are disabled` | **設定は読まれている。**2行目の読み方（設定が未反映）を当てはめない。**原因は未特定。**対応は4行目と同じで、`@coderabbitai full review`でreviewが走った。**なお4行目の原因（star数）が2026-08-16に判明したが、この文言との関係は確かめていない。**同一Pull Requestに両方出るため（#123・#125・#135）同じ原因である可能性はあるが、CodeRabbitはこの文言について何も述べていない。**推測で4行目へ畳まない** | [#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)（いずれも2026-08-15、作成直後）・[#124](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/124)（push後。後述） |

**行ごとに意味が違う。**同じ`Review skipped`でも取るべき対応が違う。

- 1行目: `.coderabbit.yaml`の意図どおりのskipである。[自己レビュー](#自己レビュー)で通す。
  ただし**変更の内容に対してlabelの付け方が誤っていないかは確認する。**安全・電気・protocol・
  firmwareに関わる変更が`area:docs`だけになっていれば、labelが誤っており1行目に該当しない
- 2行目・3行目: **allowlistの判定まで届いていない。**設定が`develop`にあるか、baseが
  `base_branches`に含まれるかを確認する。対象範囲の変更なら
  [手動で依頼する](#手動で依頼する前に状態を確認する)。安全に関わる変更では自己レビューで代替しない
- 4行目・5行目: **labelもrate limitも原因ではない。**
  [手動で依頼する](#手動で依頼する前に状態を確認する)と得られる。`review`ではなく
  **`full review`**を使う。自己レビューで代替しない。
  ただし**「自動reviewは二度と起動しない」と決めつけない。**#125ではこの2文言の約2分後に
  自動で`Review in progress`へ移っている（その回はrate limitで止まった）

**2行目・3行目は、いずれも`.coderabbit.yaml`が`develop`に無かった時期の観測である。**4行目・5行目は設定が定着した後の観測であり、原因が別である。
設定が定着した後にこの文言を見たら、**それは新しい事象である。**推測で1行目と同じ扱いにしない。
5行目はその事象であり、確認できた事実を[5行目の観測](#5行目の観測)にまとめてある。

**thread 0件は、reviewが終わったことを意味しない。**
[#76](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/76)では0件を確認した**28秒後**に
reviewが届き、actionable comment 2件がmerge済みPull Requestへ付いた。
GitHubはthreadが存在しないものをblockできない。**reviewの到着を待たずに0件を「解決済み」と読まない。**

自動reviewの対象は[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)
で高リスク変更へ限定している。対象外のPull Requestでは[自己レビュー](#自己レビュー)が唯一のreviewである。

**指摘に対応したcommitは、自己レビューで見る。**`auto_incremental_review`を`false`にしているため、
pushしても再reviewは走らない。**ここで`@coderabbitai review`を投げ直さない。**
投げ直すと1つのPull Requestでreviewを何度も消費し、`false`にした意味が無くなる。
[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で実際にそうなり、2回目はrate limitで終わった。

**自動reviewが一度も走らなかった場合だけ、手動で依頼する。**該当するのは
`Review rate limited`または`Review skipped`で初回のreviewが得られなかったときである。

| 変更の種類 | 初回reviewが得られなかったとき |
|---|---|
| 安全、電気、protocol、firmware | **rate limitが解けるまで待つ。**自己レビューで代替しない |
| 上記以外 | **自己レビューで通してよい。**Pull Request本文へ機械reviewを通していない旨と、その判断の根拠を書く |

**`Review stopped after lock loss`もこの表の対象である。**`state`が`failure`でcheckは赤くなるため`Review rate limited`／`Review skipped`とは表示で見分けられるが、**reviewが完走していない点は同じ**である。したがって初回reviewが得られなかった場合として扱い、上の表に従う。**安全・電気・protocol・firmwareに関わる変更では、`Review completed`へ到達するか手動の`full review`が完走するまでmergeしない。**赤いcheckを「reviewは走ったが失敗しただけ」と読み替えない。観測例は[GitHubが強制しないもの](#githubが強制しないもの)にある。

##### 5行目の観測

上の「設定が定着した後に2行目の文言を見たら新しい事象である」に**実際に当たった**。
**1件限りではない。**同日の[#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)と
[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)で、いずれも作成直後に同じ並びが出ている。
まず#123について、確認できた事実だけを次に記録する。

| 確認したこと | 根拠 |
|---|---|
| **設定は読まれている** | reviewの`Run configuration`が`Configuration used: Path: .coderabbit.yaml`（`Review profile: CHILL`、`Plan: Pro Plus`）を示す |
| 設定は`main`・`develop`の両方にあり、内容も同一 | 両branchでblob sha `816e60d` |
| baseは`develop`で、`base_branches`に含まれる | Pull Requestのbase |
| allowlistのlabelが**作成時**に付いている | `13:19:16Z`に`area:firmware`・`area:hardware`（作成は`13:19:15Z`。`gh pr create --label`で指定） |
| それでもskipされた | commit statusの履歴（下記） |

commit statusは`Review queued`が6件出たあと、**4行目の文言が5件と5行目の文言が1件**という内訳になった。

| 時刻（UTC） | `description` |
|---|---|
| `13:19:23Z`〜`13:19:33Z` | `Review skipped: manual review required for this OSS repository`（5件） |
| `13:19:35Z` | `Review skipped: automatic reviews are disabled`（1件） |

**この2文言は排他ではない。**同じPull Requestの同じ時刻帯に両方出る。
**どちらか一方だけを見て切り分けたと判断しない。**

解決は4行目と同じく`@coderabbitai full review`だが、**1回目は空振りした。**

| 時刻（UTC） | 出来事 |
|---|---|
| `13:21:20Z` | `@coderabbitai rate limit` → `More reviews will be available in 26 minutes.` |
| `13:50:36Z` | `@coderabbitai full review` → `13:50:41Z`のreplyが`Review rate limited.`（`next included review will be available in 59 minutes`）。commit statusも`13:50:45Z`に`Review rate limited` |
| `14:53:54Z` | `@coderabbitai rate limit` → `Reviews are available now.` |
| `14:54:26Z` | `@coderabbitai full review` → `14:54:43Z`のreplyが`Full review finished.`、`14:58:39Z`にcommit statusが`Review completed`。review本文は`Actionable comments posted: 3` |

**`full review`自体もrate limitで空振りする。**空振りしても同じCodeRabbit checkが
`Review rate limited`という**別の説明文**で`pass`になるため、投げっぱなしにすると走ったように見える。
[手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)の手順を省かない。
`26 minutes`の案内どおりに待っても枠が空いていなかった点にも注意する。**案内の時刻は保証ではない。**

**#125（作成`13:41:11Z`、allowlistのlabel`area:hardware`を`13:41:13Z`に付与）でも同じ並びが再現した。**
`13:41:17Z`〜`13:41:19Z`に`Review queued`が5件、`13:41:19Z`〜`13:41:27Z`に4行目の文言が4件、
`13:41:30Z`に5行目の文言が1件である。**#123と同じ形であり、単発の事故ではない。**

**ただし#125では、その後に自動reviewが自力で起動している。**同じcommit（`be52d9b`）に対し、
手動依頼も新しいpushも無いまま`13:43:30Z`に`Review in progress`へ移り、`13:43:33Z`に
`Review rate limited`で止まった。**同じことが`13:50:15Z`と`15:08:54Z`にも起きている。**
最終的に`Review completed`へ至ったのは手動の`full review`（`17:26:49Z`）である。

**したがって「5行目＝自動reviewは二度と動かない」ではない。**skipの後に自動で再試行されることがあり、
そのときrate limitに当たると`Review rate limited`へ変わる。**表示が変わったことを「解決した」と読まない。**

**#124でも5行目の文言を観測しているが、条件が違う。**作成直後ではなく、
commit日時`2026-08-16T00:12:31Z`のcommitに対する`00:13:58Z`の判定である。
**#124に現存するcommitはすべてPull Request作成より後のものであり、作成時点のcommitに
付いたstatusはもう辿れない。**作成直後の観測として数えられるのは#123と#125の2件である。

##### `enabled: false`と`labels`の関係（公式文書の確認）

**5行目を「設定の誤りだ」と読まないため、CodeRabbitの公式文書を確認した。**
設定schema（[`schema.v2.json`](https://coderabbit.ai/integrations/schema.v2.json)。
`.coderabbit.yaml`冒頭の`$schema`が指すもの）の`reviews.auto_review.labels`の説明は次を含む。

> When `enabled` is false, a positive label match (for example ['review-ready']) triggers a review;
> negative-only labels such as ['!wip'] remain exclusion filters and do not opt PRs in by themselves.

**`enabled: false`＋`labels`によるopt-inは、文書どおりの使い方である。**
`.coderabbit.yaml`は誤設定ではなく、**5行目を設定変更で解消する根拠は無い。**

実測もこれと矛盾しない。[#90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)・
[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)は
`area:docs`＋`type:decision`＋`priority:*`で作成され、**当時のallowlistには`type:decision`が入っていた**
（`.coderabbit.yaml`から外したのは[#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)）。
一致するlabelがあり、`enabled: false`のまま自動reviewが走っている。
**「#90・#91はallowlistに一致していないのに通った」ではない。**現在のlabel一覧だけを見て
過去の観測を読み替えない。

**したがって5行目の原因は未特定のまま残る。**CodeRabbit側の挙動か、設定以外の要因である。
**推測を書き足さない。**次に同じ文言を見たら、上と同じ形で観測を追記する。

#### 手動で依頼する前に状態を確認する

**投げる前に必ず次を実行する。**これは**reviewを消費せず**、残数と次に空く時刻を返す。

```text
@coderabbitai rate limit
```

| 確認結果 | 行動 |
|---|---|
| 残数が0 | **投げない。**返ってきた時刻まで待つ |
| 残数があり、**初回reviewがskipされた** | **`@coderabbitai full review` を使う。**`review`はincrementalであり、skip後は空振りしうる |
| 残数があり、初回reviewは走ったが対応commitが未review | `@coderabbitai review` |

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

`--body-file`の内容には、末尾へ次のtrailerを置く。**指示sourceを変更していない場合、
3行目は書かない。**

```text
Change-Class: review-required
Self-Review: converged
Instruction-Change: reviewed-as-data
```

**trailerの名前と値の正本は`scripts/review_gate.py`である。**

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
