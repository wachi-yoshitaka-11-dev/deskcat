# DeskCatへのcontribution

DeskCatはfirmware、Linux software、電子回路、可動機構を組み合わせる。変更はreview可能で、証拠に基づく必要がある。

## 全体の流れ

**各段で1つだけ落としやすいものを並べる。**規則は再掲しない。詳細は各節を参照する。

```text
着手前:  git fetch origin && git rev-parse --short origin/develop
PRの前:  build・test・lintの出力を示し、/code-reviewと/security-reviewを走らせる
merge前: CIが通り、指摘が解決し、要る承認がある
merge時: --subjectと--body-fileを明示する
review:  CodeRabbitにはfull reviewを使う
```

| 段 | 落としたときに起きること | 正本 |
|---|---|---|
| 着手前 | 古い基点で判断する。**後から入った文書を「存在しない」と読み、そこから誤った断定へ進む** | 下の[作業開始前](#作業開始前) |
| PRの前 | 誤りを見つけないまま、reviewを受ける側へ渡す | [自己レビュー](#自己レビュー) |
| merge前 | **CIが緑でも機械reviewが完走しても承認ではない** | [Pull Requestのレビュー](#pull-requestのレビュー) |
| merge時 | GitHubが合成したmessageでmergeされ、検証の経緯がcommitに残らない | [Merge方式](#merge方式) |
| review | `review`はincrementalで空振りし、枠だけ消費する | [手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する) |

**この表は覚えるためのものではない。**`.claude/settings.json`のhookが、着手前とmerge時を
機械で止める。**hookが止めない段の実行手順は`.claude/skills/deskcat-preflight`が持つ**
（規則ではなく実行だけを持ち、規則はこの文書を参照する）。**merge前の承認は止められない**（人の判断である）。
reviewの依頼は、規則に反する形（`full`の付かない`review`、走った後の`full review`）は`coderabbit_gate.py`が`deny`し、それ以外の`full review`は`ask`で人間に確認する。
hookの一覧と回避手順は[hookが止めたとき](#hookが止めたとき)にある。

## 作業開始前

1. `git fetch origin`し、`origin/develop`を基点にする。
2. 対象baseのcommit SHAを記録し、その版の[AGENTS.md](AGENTS.md)とリンク先の規則を読む。
3. [Governance](docs/governance/README.md)を読む。
4. 一つの目的に絞ったIssueを探すか作成する。
5. 依存関係と受け入れ条件を確認する。
6. [ハードウェアTBD](docs/hardware/tbd-register.md)を確認する。
7. 編集前にworking treeを確認する。

通常開発は`develop`、安定版・公開Pagesは`main`を参照する。公開文書の`blob/main/`リンクは
安定版へのリンクであり、`develop`作業の規則を読むときは記録したbase SHAの同じpathを開く。
相対リンクは同じcheckoutの版を辿る。未mergeの差分は規則として適用しない。

正確な部品、関連GPIO、電源、安全値が`TBD`のときはhardware driverへ着手しない。

## Issueの範囲

Issueには次を含める。

- 背景
- 一つの目的
- 対象範囲と対象外
- 依存関係
- 正式な基準文書
- 測定可能な受け入れ条件（何ができたら終わりか）
- 作業リスト（そのために何をするか。[作業リストの決まり](#作業リストの決まり)）
- PC確認
- 実機確認
- 保存する証拠

無関係なrefactor、新たに見つけた不具合、別実験には別Issueを使う。

### 作業リストの決まり

- Issueには、受け入れ条件（何ができたら終わりか）と作業リスト（そのために何をするか）の両方を書く。
  作業リストの1項目は1つのPull Requestである。ただし、調査や設定の変更のようにPull Requestを伴わない項目は、
  結果をIssueのコメントに残してチェックする。
- Pull Requestは、Issueの作業リストのどれかの項目を埋めるものだけを立てる。本文に、埋める項目を書く。
  例外は[Issueを立てずに直接反映してよい範囲](#issueを立てずに直接反映してよい範囲)でIssueが不要なものだけで、
  そのときは本文の「作業リストの項目」に「Issueなし：」と理由を書く（節は消さない。作成時にhookが節を求める）。この例外が外すのは作業リストの項目を書くことだけで、
  レビューと承認は外さない（[慎重に扱う path](#慎重に扱う-path)に触れる言い回しの直しも、CodeRabbitと所有者の承認が要る）。
  意味を変えないか迷うものは、その表のとおりIssueを立てる側として扱う。
- 途中で要る作業が見つかったら、先に理由を添えて作業リストに足してから着手する。
- 済んだ作業に合わせて、後から項目を足して埋めない。
- mergeしたら、mergeした者がその項目にチェックを付ける。Issueを閉じないPull Requestでは、
  [関連Issueの書き方](#関連issueの書き方)のとおり`Refs: #N`を使い、済んだことと残りのコメントも書く。
  すべての項目が済んでも受け入れ条件が満たされないなら、作業リストを見直す。

### Issueを立てずに直接反映してよい範囲

**すべての気づきをIssueにしない。**記述の維持管理までIssue化すると、保守Issueが製品作業を追い越す。
実際、[#84](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/84)は規約の言い回しを直すために
起票し、直後にcloseした。

**Issueが要るかどうかと、Pull Requestが要るかどうかは別の問いである。**この2つを1つの段に
まとめていたため、規約文書のtypoを直すのにIssueまで要る状態になっていた
（[ADR-0011](docs/decisions/0011-issue-optional-pull-request-required.md)）。列を分ける。

| 対象 | Issue | Pull Request |
|---|---|---|
| boardのmetadata記入漏れ。repositoryの変更ではない | 不要 | 不要。`Status: Done`を含む操作の承認区分は[AI Agent Policy](docs/governance/ai-agent-policy.md#6-責務マトリクス)に従う |
| typo、リンク修正、表記ゆれ、言い回しの修正で、**意味を変えないもの** | 不要 | **必要** |
| 規約、仕様、安全、電気、protocol、GPIO、電源、toolchain、CI、依存の**意味**を変えるもの | **必須** | **必要** |
| 複数のPull Requestに跨る、または他の作業をblockするもの | **必須** | **必要** |

Issue不要の側でも、人間の承認が要るかは[Pull Requestのレビュー](#pull-requestのレビュー)が決める（文書や言葉だけの変更で[慎重に扱う path](#慎重に扱う-path)に当たらないものは、自動のレビューとセキュリティの確認が通れば要らない）。GitHub操作の承認区分は[AI Agent Policy](docs/governance/ai-agent-policy.md#6-責務マトリクス)に従う。
判断に迷うものはIssue必須の側として扱う。安全に関わる範囲は緩めない。

**「言い回しの修正」と「意味の変更」の境界は機械的に判定できない。**
**迷ったら「意味を変えるもの」として扱う。**

**repositoryへの変更は、必ずPull Requestを通す。**Pull Requestを省いて`develop`へ入れる経路は設けない。**これは規則であり、今はGitHubもhookも直接pushを止めない**（`develop`には必須reviewも必須checkも無い。必須checkは[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの8で置く）。
boardのmetadata記入漏れはrepositoryの変更ではないため、Pull Requestという単位が無い。

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
| milestone | 必須 | 対応するM0〜M7 milestoneを設定する |
| label（`type:*`／`priority:*`） | 必須 | それぞれ1つ設定する |
| label（`area:*`） | 対象componentがある場合のみ | 対象componentに対応するlabelを設定する。repository全体の保守作業など特定componentに限らない場合は省略する |
| label（`status:blocked`／`needs:*`） | 該当時のみ | 依存未解決なら`status:blocked`、実機証拠や人間の判断が必要なら`needs:hardware-test`／`needs:decision`を設定する |
| assignee | 必須 | 対応を担当する人を設定する。未定でも起票者自身を暫定assigneeとする |
| project | 必須 | Projects v2 board（`deskcat`、`https://github.com/users/wachi-yoshitaka-11-dev/projects/5`）にitemとして追加し、`Status`を設定する |
| 開始日／終了日（`Start date`／`Target date`） | **必須** | **起票時に両方設定する。**`Start date`は着手予定日、`Target date`は完了目標である。**どちらも予定であり、確定していなくてよい。**確定していないことを理由に空欄にしない。予定が変わったら更新する。**closeした後は、close実施者が`Target date`をJSTのclose実績日へ設定する** |

この表はIssue itemの規約である。**Issue itemとPull Request itemで、2 fieldはどちらも必須である。
違うのは必須かどうかではなく、`Start date`が実績か予定かである。**Pull Request itemの`Start date`は
作成日という**実績**であり、Issue itemの`Start date`は着手予定日という**予定**である。`Target date`は
どちらも予定（Issueは完了目標、Pull Requestはmerge見込み日）であり、mergeまたはcloseの**完了後**に
実績値へ更新する。[Pull request](#pull-request)節を参照する。

**milestoneはIssueだけに設定する。Pull Requestには設定しない**
（[ADR-0012](docs/decisions/0012-milestones-count-issues-only.md)）。milestoneは節目までに
出すものを数える道具であり、Pull Requestはその手段である。両方に付けると、1つのIssueを
何本のPull Requestに割ったかでmilestoneの件数が動く。

boardの`Item closed` workflowは`Status`を`Done`にするが、**日付fieldは更新しない。**
そのためIssueのclose後は、close実施者が`Target date`を実績日へ設定し、APIで読み戻す。
作成・更新・close/reopen後は[metadata操作手順](docs/runbooks/github-metadata.md)の
`check`または`apply`で保存された実値を照合する。checkboxだけを設定済みの証拠にしない。

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

**共有branch（`main`／`develop`）の履歴を書き換えない。**他者がpullした可能性のあるbranchも共有として扱う。
判断に迷うものは共有として扱う。

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

変更に関係する検証commandをすべて実行する。**root READMEとcomponent READMEはcommandを持たない。**

host workspaceはrepository rootで、ESP32 firmwareは`firmware/esp32`で検証する。**確定したcommandの正本は[検証済みコマンド](docs/toolchains/verified-commands.md)である**（[ADR-0018](docs/decisions/0018-instruction-file-structure.md)で`AGENTS.md`とroot READMEから移した）。**まだ正式なcommandが無いのは、Raspberry Piの実機試験（実serial port、ESP32との通信）とHILである。**同文書の「まだコマンドが無いもの」が正本であり、ここでは列挙しない。

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

**配線を変えたら、記録だけで終わらせず、同じPull Requestでブレッドボード配線図（`hardware/breadboard/layout.svg`）も更新する**（`hardware/README.md`）。**図は最終的な配線を描く。測定のためだけの一時配線は描かない。**そのPull Requestは、下の`Wiring-Figure: not-applicable <理由>`で通す。配線図は3つ（`hardware/breadboard/layout.svg`、`hardware/universal-board/`の電源の基板の図、`hardware/overview/layout.svg`）である（電源の基板の図は`plan-front.svg`・`plan-back.svg`の計画で、完成図は[#405](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405)で現物と照合して作る。`overview`は#512で回路図のnetlistから生成する）。`docs/hardware/experiment-log.md`の`Wiring revision`の行を追加・変更し、配線図を更新しないPull Requestは、`Wiring figure sync` workflow（`scripts/check_wiring_figure_sync.py`）が落とす。この検査は、base が`develop`のPull Requestだけで走る。配線を変えていない試験の記録など、図に関係しない場合だけ、Pull Request本文に次の1行を書く。**コロンが要る。**commit messageは読まない。理由は10字以上が要り、名前と値の正本は`scripts/check_wiring_figure_sync.py`である。

```text
Wiring-Figure: not-applicable <配線を変えていない理由>
```

**検査は字句だけで、図の内容が記録と合っているかは判定しない。**照合は人が行う。

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
現在の状態は正本へ集約し、日付付きの根拠・実験・訂正は履歴として参照する。
TBD変更では[状態変更時の参照確認](docs/hardware/tbd-register.md#状態変更時の参照確認)を行う。
本書の[過去事例](docs/runbooks/contributing-history.md)は現在の手順と分けて読む。

## Pull request

Pull requestには次を含める。

- 関連Issueへのlink（`Closes #N`等）
- 埋める作業リストの項目（[作業リストの決まり](#作業リストの決まり)。Issueが不要な変更では、その理由）
- 結果と範囲の説明
- 仕様変更の特定
- 検証証拠
- 残っている実機test
- 新規dependency
- 残存riskと`TBD`
- 戻し方（mergeの後に戻すとき、何を戻せばよいか）
- 規則を消した場合は、消した規則ごとの移した先かやめた理由
- 無関係なformat変更やrefactorがないこと

作成直後に、boardへitemを追加して次を設定し、APIで読み戻す。
更新や終了後も[metadata操作手順](docs/runbooks/github-metadata.md)の検査を行う
（確認者は[誰がいつ確認するか](#誰がいつ確認するか)）。

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

labelは引き続き必須である。仕分けに要る。
boardへのitem追加はPull Requestが存在しないと行えないため、作成直後に行う。

### 関連Issueの書き方

Issue branchからのPull Request（base `develop`）では`Closes #N`を使う。**ただし、[作業リスト](#作業リストの決まり)の
途中の項目を埋め、Issueを閉じないPull Requestでは`Refs: #N`を使う**（下の「IssueをcloseしないPull Request」の手順に従う）。
`Closes #N`はtraceability
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

**closeの前に、受け入れ条件を1つずつ照合し、満たしたものをIssue本文でチェックする。**
これはhookが強制する（`scripts/hooks/gh_metadata_guard.py`。[#693](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/693)）。
`gh issue close`と、`Status`を`Done`にする`gh project item-edit`の前に、Issue本文の
「受け入れ条件」を含む見出し（`##`〜`######`）の節（次の同じかそれより上の階層の見出しまで）に
`- [ ]`が1つでも残っていればdenyし、残っている項目を理由に出す。**その節が無い本文もdenyする。**
満たせない項目があるIssueはcloseせず、openのまま残りを書く。
**方針として閉じる場合だけ、`gh issue close --reason "not planned" --comment <理由>`は節を見ずに通す。**
理由のコメントが無い`not planned`と、完了扱いの`Done`は止める。
Pull Request itemと既にcloseされたIssueの`Done`は止めない。
確認できない場合もdenyする。**解除の手段は無く、`DESKCAT_SKIP_GH_GUARD`も効かない。**

**IssueをcloseしないPull Request（`Refs: #N`だけのもの）をmergeしたら、mergeした者が、
そのIssueへ「このmergeで済んだこと・残っていること」を1コメント書く。**
記録（EXP・docs）を`develop`へ入れても、Issueのコメントは誰も更新しない。timelineにはPull Requestと
commitの参照が自動で出るが、何が済んで何が残ったかは書かれない。最後のコメントを今の状態と読むと、
その後に済んだことを残りと誤る。

- コメントには、このmergeで済んだこと、まだ残っていること（残りが無ければ「残り無し」）、
  参照するPull Requestとcommitを書く。
- **同じ者が、このmergeで満たした受け入れ条件に、Issue本文でチェックを付ける**
  （[#696](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/696)）。本文はIssueの今の状態であり、
  コメントだけ書くと、本文を読んだ人が済んだ条件を残りと読む。チェックを付けるのは、満たしたと確かめた
  条件だけである。満たしていない条件には付けない。
- Issueをcloseする場合は、上の手順に従い、このコメントは要らない。

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
Issue itemでは両fieldが予定であるのに対し、Pull Request itemの`Start date`は
作成時点で確定した実績であり、空欄にする理由がない。

| field | 値 | 設定時期 |
|---|---|---|
| `Start date` | Pull Requestの作成日。作成後は変更しない | 作成時 |
| `Target date` | 作成時はmergeを見込む日（予定）、mergeまたはclose後はその実績日 | 作成時に見込みを設定し、**merge完了後またはclose完了後**に実績値へ更新する |

`Target date`だけが予定から実績へ変わる。merge時に`Status`を`Done`にするworkflowは
日付を書き換えないため、実績値への更新は操作実施者が行う。mergeせずcloseした場合の実績日は
close日である。

**Issue itemでも、完了時には空欄を残さない。**起票時に設定した値は予定であり、
`Status`が`Done`になった時点で開始日と終了日は確定するため、close後に`Start date`へ
着手日、`Target date`へclose日（実績）を入れる。
**Pull Request itemだけが実績を持つと読まない。**

日付はJST（UTC+9）で判断する。GitHubのAPIが返す時刻はUTCであり、JSTの`00:00`から`08:59`に
作成したPull RequestはUTCでは前日の日付になるため、そのまま転記しない。

#### 誰がいつ確認するか

次の担当者が更新後のAPI実値を検査する。ローカルcommandを通さない操作も`check`/全件`audit`で
事後検出できるが、すべての経路を強制停止する仕組みではない。API失敗を合格にしない。

| 時期 | 実施者 | 確認内容 |
|---|---|---|
| Pull Request作成直後 | 作成者 | boardへitemを追加し、`Status`・`Start date`・`Target date`を設定する。`Pull request`節の作成時checklist（この文書の上部）に含めてある |
| **merge後** | merge実施者 | `Target date`をJSTのmerge実績日へ更新する |
| **close後** | close実施者 | `Target date`をJSTのclose実績日へ更新する |
| **reopen後** | reopen実施者 | `Target date`を新しいmerge見込み日へ戻す。再度mergeまたはcloseしたら、上の2行に従って実績日へ更新する |

**更新は「後」であって「直前」ではない。**merge前に実績日を確定できないためである。JSTの日付を
またいだ場合や、mergeを中止した場合に、誤った実績日が残る。

**CIによる日付の自動更新は採らない。**理由は2段ある。

1. Projects v2のboard workflowは日付fieldを更新できない。`Status`を`Done`にするworkflowでは書き換えられない
2. GitHub Actionsから叩く案も採らない。**`GITHUB_TOKEN`はrepository scopeであり、Projects v2へ
   アクセスできない**（[GitHub Docs](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects)）。
   `project` scopeを持つclassic personal access token、またはGitHub Appが必要になる。
   日付2 fieldのためにCIへ長期secretを持ち込む取引は成立しない

操作端末の既存認証を使う明示的なcommandで更新・読み戻す方法を正式手順へ加える（#466）。
従来の手作業限定という結論をこの範囲で改定し、CIへ長期secretを置かない判断は維持する。
UI/APIで直接操作した場合も読み戻して検証する。空欄だけでなく不一致・取得失敗・部分成功を扱い、
予定日・担当者・条件付きlabelを推測で埋めない。操作と再開の詳細は
[metadata操作手順](docs/runbooks/github-metadata.md)へ集約する。
CIのrepository内field検査はProject・日付の保証ではなく、保護設定の必須化もこの変更に含めない。

### 自己レビュー

Pull Requestを作る前に、作業者が次を行う。
基準は[#813](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/813)の「レビューの仕組み」であり、
Anthropicの[Code Reviewの文書](https://code.claude.com/docs/en/code-review)と
[Claude Codeチームのやり方](https://claude.dev/blog/solving-code-review-at-anthropic/)に合わせている。
資料に無い所有者の規則は「（所有者の規則）」と書く。

- 変更は小さく保つ。見やすく、直しやすく、戻しやすくするためである。
- 作業者がbuild・test・lintを走らせ、その出力をPull Request本文に示す。
- 新しいcontextのsubagent（`.claude/agents/fresh-context-reviewer.md`）か`/code-review`で差分をレビューする。
  subagentは`Bash`を持たずgitを使えないため、呼ぶ側がbaseとの差分を
  作業ツリーの外のfileに書き出し、そのpathを渡す。置き場所はセッションのscratchpadにする（subagentが許可なしで読める。ほかの場所は許可を求められ、backgroundのsubagentでは拒否されうる）。
  作業ツリーはレビューするheadと同じcommitにしておく（subagentは周りのコードを作業ツリーから読む）。
  差分がレビューの基準の文書（`AGENTS.md`、`CLAUDE.md`、`CONTRIBUTING.md`、`.claude/`配下、baseの版の`AGENTS.md`の「作業開始時に読むもの」に挙がる文書）を
  変えるときは、変えた文書のうちbaseに在ったものごとにbaseの版（`git show <base>:<path>`）も同じ置き場所に書き出し、元のpathと書き出したfileのpathの組を渡す。
  subagentはmerge前の基準を使わない。baseの版が無ければレビューせずにそう報告し、baseに無い新しい文書は基準にせずレビューの対象としてだけ読む。
  `.claude/agents/fresh-context-reviewer.md`そのものを変えるPull Requestでは、subagentの定義がheadの版で読み込まれ、この手順では防げないため、このsubagentを使わない。
  `/security-review`も走らせる。`/security-review`はoriginの既定のbranch（`main`）との差分を見るため、
  `develop`にあって`main`に無い変更も含まれる。**このPull Requestの差分の外の指摘はPre-existingとして扱う。**
- **`/code-review`のeffortは毎回指定する。**指定しないと前回の値が使われる。安全・電気・サーボの制限・
  配線図と基板の変更は`high`で走らせる。
- **`/code-review`には`--max-findings all`を付ける。**付けないと件数の上限で切られ、残りの指摘が出ない。
- 1巡は、レビューを1回走らせ、その指摘を直すまでとする。巡を重ねるのは危険度の高い変更でよく、
  すべてのPull Requestでは要らない。
- **レビューの記録は、最後のcommitまで途切れずにつなぐ**（[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの9）。
  - commitしてから、`/code-review`をbackgroundのsubagentの中で、範囲`A..B`を明示して走らせる。
    Aは前の記録の終わりのSHA（初回はPull Requestのbaseとのmerge-base）、BはHEADである。
    範囲は1回に1つだけ書く。`...`（3点）とPR番号での実行は記録に数えない。
  - **mainの文脈での実行と、ユーザーが打った`/code-review`は記録に数えない。**hookに
    `agent_id`も結果も届かず、レビューが終わったかを知る手段が無いためである。
  - 記録は`scripts/hooks/review_record.py`が、gitの共通のdirの`deskcat-review-log.jsonl`に書く。
    このlogへ書き込む操作は止める。
  - 完了と数えるのは、報告の本文が指摘の一覧の形（`` ```json ``の囲みがちょうど1つで、中身がobjectの配列。
    空の配列も数える）だったときだけである。利用の上限で止まった実行も、成功として届くためである。
  - **記録はcloneごとにある。Pull Requestは、その範囲をレビューしたcloneでmergeする。**
    同じcloneのworktreeの間では記録を共有し、別のclone（別の端末や環境）とは共有しない。
  - Pull Request本文の「自己レビュー」に、記録の範囲をbacktickで囲んだ`A..B`の形ですべて書く。
  - `gh pr merge`の前に`scripts/hooks/gh_metadata_guard.py`が、記録の範囲がmerge-baseから
    headまでつながっているか、本文の範囲が記録と一致するかを確かめる。
    CI（`review-coverage.yml`）は本文の範囲がつながっているかだけを見る（手元のlogを読めない）。
  - 昇格（`develop`→`main`）では、範囲のつながりの代わりに、範囲の各commitが`develop`へmerge済みの
    Pull Requestのsquashのcommitかを見る。Pull Requestを通らずに入ったcommitは、`<sha>^..<sha>`を
    含む範囲の記録があるときだけ通す。
  - **限界：**hookは、subagentが範囲の全部を実際に読んだかまでは確かめない。logへの書き込みを
    止めるのはうっかりの書き込みだけで、名前を組み立てるscriptを走らせるか、記録のhookに偽の入力を
    流せば記録を偽れる（意図的な偽りは前提にしない）。所有者が画面から
    mergeする経路はhookを通らず、CIは本文に書いた範囲を信じるしかない。CIはPull Request側の
    workflowの版で動くので、workflowを書き換えたPull Requestは自分の検査を変えられる
    （`.github/`は慎重に扱うpathなので、所有者の承認が要る）。hookが時間切れになると、
    止める側に倒れない。rebaseすると前の範囲のSHAがheadの祖先でなくなり、つながらなくなる（レビューし直す）。

**自動reviewは行わない**（[ADR-0013](docs/decisions/0013-manual-only-coderabbit-review.md)）。
[`.coderabbit.yaml`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.coderabbit.yaml)は
`enabled: false`だけを持つ。CodeRabbitへは[慎重に扱う path](#慎重に扱う-path)に触れるときに手動で依頼する。

### Pull Requestのレビュー

- **CI（lint、test、build）が通らない間はmergeしない。**`develop`と`main`のbranch protectionに
  required status checksを置く（所有者の規則）。**まだ置いていない**（[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の
  作業リストの8）。置くまでは、CIが赤でもGitHubはmergeを止めない。
- **すべてのPull Requestで、Claudeのレビューを走らせる。**`/code-review`に`--comment`を付け、
  指摘をPull Requestの行に投稿する。
- **[慎重に扱う path](#慎重に扱う-path)に触れるPull Requestでは、加えてCodeRabbitに`full review`で依頼する**
  （所有者の規則。2つ目の独立したレビュー役として使う）。手順は
  [手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)にある。
- **誤りのレビューとセキュリティのレビューの指摘が未解決の間はmergeしない。**指摘は、直すか、
  理由を書いてresolveする。
- **人間の承認は、作った人ではなく変更の中身で決める。**
  - 文書や言葉だけの変更：自動のレビューとセキュリティの確認が通れば要らない。
    ただし[慎重に扱う path](#慎重に扱う-path)に当たる文書を除く。
  - それ以外：所有者が承認する。AIに作らせたPull Requestでは、作らせた人が承認する。
  - [慎重に扱う path](#慎重に扱う-path)は必ず所有者が承認する。
  - AIの承認は人間の承認に数えない。**承認は、そのcommitにだけ効く。**新しいcommitが入ったら承認し直す。
    所有者は自分のPull RequestをGitHubでapproveできないため、承認はPull Requestのコメントに、
    承認するcommitのSHAを書いて残す（所有者の規則）。
- 人間が見るもの：作るべきかどうか、後の作りを決める設計、示された証拠（testの出力、指摘）。
  検査が何を見逃し得るか言えないときは、すぐ承認せず、しっかりしたレビューを求める。

#### 指摘の重さと扱い

| 重さ | 意味 | 扱い |
|---|---|---|
| Important | mergeの前に直すべき誤り | 直す。指摘が誤りなら、理由を書いてresolveする |
| Nit | 小さな問題。止めない | 直すか、理由を書いてresolveする |
| Pre-existing | このPull Requestが持ち込んだのではない誤り | このPull Requestでは直さない。追跡Issueを作り、似たIssueがあればそこへ足す。resolveする前に作り、指摘に番号を書く。安全・電気・サーボの制限・配線図と基板に関わるものは`priority:high`にし、その作業リストより先に着手する（所有者の規則） |

レビュー役への指示は[AGENTS.md](AGENTS.md)の「レビュー役への指示」が持つ。`/code-review`は`CLAUDE.md`を読み、
`CLAUDE.md`が`AGENTS.md`を読み込むためである。

#### レビューの自己改善

- 指摘を、作業者の対応で4つに分けて数える：直した、指摘が誤り、意図どおり、影響が無い。
  何巡で通ったかと合わせてPull Request本文に残し、レビュー役の誤検出と収束を見る。
- 同じ指摘が繰り返し出たら、元で直す：skill、lintの規則、`AGENTS.md`のどれかに足す。
- mergeの後に見つかった誤りと、戻した変更は、見逃した例として1つのIssueに集める。
  レビューの指示や道具を変えたときは、その例で見逃さないかを確かめる。
- 見逃しを減らすことを先に、巡を減らすことを次に置く。

### 慎重に扱う path

次のpathに触れるPull Requestは、CodeRabbitの`full review`と所有者の承認が要る。
**`gh pr merge`の前に、`scripts/hooks/gh_metadata_guard.py`がCodeRabbitのreviewの完走を確かめる。**
承認を得たら、本文の全体が`承認 <承認したheadの40桁のSHA>`の1行だけのコメントを残す（短いSHAは数えない）。hookはそのコメントがあるかを確かめる
（botのコメントと、他の文を含むコメントは数えない）。承認があったこと・承認者・日時・出所・範囲の記録は、
このコメントに書かず、Pull Requestの本文か別のコメントに書く（[AI Agent Policy](docs/governance/ai-agent-policy.md)の
「ユーザーの発言を書かない」）。**これは記録の確認であって、承認そのものの確認ではない。**所有者とAIは
同じaccountで書くため、書き手で見分けられない（承認の関門は[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの11）。
この一覧とhookの一覧（`REVIEW_TARGET_PREFIXES`・`REVIEW_TARGET_FILES`）は一致させる。
`scripts/test_hooks.py`が一致を検査する。`/`で終わるものはその配下のすべてである。

- CIのworkflow：`.github/workflows/`
- hook：`scripts/hooks/`、`.githooks/`、`.claude/settings.json`、`scripts/review_coverage.py`（hook と CI が共有するレビューの記録の判定）
- 指示と規則：`AGENTS.md`、`CLAUDE.md`、`CONTRIBUTING.md`、`docs/governance/ai-agent-policy.md`、`docs/governance/development-workflow.md`
- 安全・電気・サーボの制限・ピンの割り当て・電源の正本の文書：`docs/governance/hardware-safety-policy.md`、
  `docs/hardware/servo-safety-limits.md`、`docs/hardware/gpio-assignment.md`、`docs/hardware/power-budget.md`
- 基板（KiCadのfile）：`hardware/deskcat.kicad_pcb`、`hardware/deskcat.kicad_sch`、`hardware/deskcat.kicad_pro`、
  `hardware/DeskCat.kicad_sym`、`hardware/DeskCat.pretty/`、`hardware/fp-lib-table`、`hardware/sym-lib-table`
- 配線図、結線表、netlistとその生成の道具：`hardware/overview/`、`hardware/breadboard/`、
  `hardware/breadboard-rebuild/`、`hardware/universal-board/`
- 配線図の検査：`scripts/check_wiring_figure_sync.py`
- ピンと安全の強制：`firmware/esp32/src/`
- サーボの制限：`crates/deskcat-servo/src/`
- 通信が切れたときの止め方：`crates/deskcat-protocol/src/`、`docs/protocol/esp32-pi-protocol.md`

`hardware/`のうち対象外は次だけである（`scripts/test_hooks.py`が、`hardware/`の追跡fileがすべて上の一覧かこの一覧のどちらかに入ることを検査する）：`hardware/README.md`、`hardware/.gitignore`、`hardware/measurement/`、`hardware/bench-18/`、`hardware/wokwi/`。

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
管理者は`BLOCKED`のままmergeできる。**未解決を残してmergeしない。**範囲外の指摘はPre-existingとして
追跡Issueへ回してからresolveする（[指摘の重さと扱い](#指摘の重さと扱い)）。

**threadは、直すか、理由を書いてresolveする。**理由を書かずにresolveしない。

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
（[#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94#issuecomment-5242073164)）。
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

依頼するのは[慎重に扱う path](#慎重に扱う-path)に触れるPull Requestである。PR担当のセッションは投稿前に
**開いている全PRの最新の`@coderabbitai`投稿時刻**を確認する。いずれかが1時間以内なら待ち、
投稿直前に全PRを再確認する。投稿はPR担当のセッションが行い、`coderabbit_gate.py`が止める。
投稿時刻は対象Issueのコメントに残す。
依頼前の`@coderabbitai rate limit`による残数確認は必須としない。

初回は`@coderabbitai full review`を使う。`review`は新しい変更のみを見るため、
reviewがまだ走っていないPRでは空振りしうる。依頼したreviewが走った後の対応commitは
自己レビューで見て、投げ直さない。

**reviewが開始したら依頼は最大1回。**`Review rate limited`で開始されなかった投稿はこの1回に
数えない。枠切れが確定した後は、PMへ知らせ、全PRの最新投稿時刻から1時間以上経ち、
投稿直前にも新しい依頼がないと確認できた場合に限り、PR担当のセッションが`full review`を再依頼できる。
`Review stopped after lock loss`でreviewが完走していないと確定した場合（checkが`failure`で、`Review completed`へ到達していない）も、この1回に数えず、PMへ知らせ、1時間以上の間隔と投稿直前の再確認という同じ条件を満たせば`full review`を再依頼できる。再依頼は1回のlock lossにつき1回で、再び止まった場合も同じ条件を満たすまで待つ。
`Review in progress`など結果が確定していない間は再依頼しない。枠切れのまま再び投稿した場合も
同じ条件を満たすまで待つ。**同じ状態で2回以上投げない。**
[慎重に扱う path](#慎重に扱う-path)に触れる変更は、reviewが完走するまでmergeしない。

**依頼の形は`coderabbit_gate.py`が強制する**（[#781](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/781)）。
`@coderabbitai`と同じ行の`review`が`full review`でなければ`deny`する（`review`は、reviewがまだ走っていないPRでは空振りして枠だけ消える）。
対象PRのどれかのcommitでCodeRabbitのreviewが完走または進行中なら、`full review`も`deny`する。
完走も進行中も無く、checkの説明文が`Review rate limited`か`Review stopped after lock loss`のときは、
`deny`にせず`ask`（人間の確認）になる。対象のPRやcheckを判定できないときも`ask`で、素通りさせない。
reviewがまだ走っていないPRの初回の`full review`も`ask`である。

**`ask`が人間に届くか。**Claude Codeの公式資料（permission modesとhooksの各ページ）は、
auto modeでもhookの`ask`はpermission promptへ送られ、classifierは決めないと書く。
promptに答える人がいない`-p`の実行では拒否される。**資料の上では届く。**
2026-11-19に`ask`を通り抜けた3回の原因は、資料から説明できず**未確認である**
（`dontAsk`と`bypassPermissions`での扱いも、読んだ範囲に記述が無い）。
そのため、規則に反すると字句と記録で決まるものは`deny`にし、`ask`に頼るのは人間の判断が要る`full review`（初回、再依頼の条件に当たりうる場合、判定できない場合）と、本文を読めず判定できない経路に限った。

**mergeの前の検査はhookが強制する**（`scripts/hooks/gh_metadata_guard.py`。[#688](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/688)）。
`gh pr merge`の前に、Pull Requestの変更fileが[慎重に扱う path](#慎重に扱う-path)のどれかを含み、
かつ**どのcommitでも**CodeRabbitのreviewが完走していなければdenyする。完走の後に対応commitを足して
headが`Review skipped`でも通る。base `main`の昇格も同じ検査を受ける。
**解除の手段は無く、`DESKCAT_SKIP_GH_GUARD`も効かない。**確認できない場合もdenyする。

**「reviewを依頼した」を「reviewを受けた」と書かない。**`Review rate limited`は`pass`と表示されるため、
依頼の事実だけで完了扱いにすると検出が抜ける。

#### 昇格Pull Requestでの手動reviewと未解決thread

**base が`main`のPull Request（`develop`からの昇格）について定める。**
定めが無かったため、[#316](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/316)で
手動reviewを2回投げる違反と、未解決を残してmergeしかける判断が起きた（追跡は[#327](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/327)）。

**1. 昇格も、範囲が[慎重に扱う path](#慎重に扱う-path)に触れるときに手動reviewの対象である。**ほかのPull Requestと同じ条件であり、merge前のhookも昇格に同じ条件で掛かる。
**回数は[手動依頼の手順](#手動で依頼する前に状態を確認する)のとおりで、昇格も例外ではない。**

**2. 未解決threadを残してmergeしない。**`main`をbaseとするmerge済みPull Request 18本を全数走査した結果、
`Require conversation resolution before merging`の強制後（2026-08-10）の13本は
**すべて未解決0件**である（#316自身を含む。2026-09-03時点の実測）。未解決を残した2本（#32／#52）は強制より前であり、
[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)が「導入の契機は#40で未解決thread 1件を
残したmergeが起きたこと」と記録している。**例外の2本は前例ではなく、規則が作られた原因である。**

**3. 内容への指摘は`develop`側で直す。**昇格Pull Requestのheadは`develop`であり、
応じると共有branchへの直接pushになる。**Issueを立てて`develop`側のPull Requestで直す。**
本文への指摘は昇格Pull Requestで直す。

**4. 範囲が変わっても、昇格Pull Requestを作り直さない。本文を更新する。**
headは`develop`であり、**`develop`へcommitが入るとPull Requestのheadも一緒に動く。**
古くなるのは本文の測定値とtitleだけである。**closeして作り直す必要は無い。**

**reviewが開始した後は、同じPull Requestで`full review`を投げ直さない**
（[手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)。完走しなかったlock lossの後の再依頼は、その節の条件で行う）。

> **依頼したreviewの指摘に対応したcommitは、自己レビューで見る。ここで投げ直さない。**
> 投げ直すと1つのPull Requestでreviewを何度も消費する。
> （[観測記録](docs/runbooks/coderabbit-review-observations.md)。回数の扱いは[ADR-0025](docs/decisions/0025-coderabbit-rate-limit-retry.md)）

**増えたcommitは自己レビューで見る。**規則1の1回は、その昇格で開始したreviewに対する1回である。
**Pull Requestを作り直した回数ではない。**昇格の差分は1回のreviewで閉じないが、
それは投げ直す理由にならない。**枠は1件/時であり、待ち時間も同じだけ積む。**

[昇格PRを作り直した経緯](docs/runbooks/contributing-history.md#昇格prを作り直した経緯)を参照する。AI同士で受け渡した指示も正本と照合する。

**5. 昇格Pull Requestを作るのは、範囲が確定してからである。**上の3で立てたIssueが
すべて`develop`へ入った後に作る。

**6. 範囲が確定したら、範囲内commitのclosing keywordを走査する。**

```bash
git fetch origin
promotion_head="$(git rev-parse origin/develop)"
python3 scripts/report_promotion_closing_keywords.py --base origin/main --head "$promotion_head"
```

**`origin/develop`をそのまま渡さない。移動するrefである。**走査してから昇格Pull Requestを
作るまでの間に`develop`へcommitが入ると、**走査していないcommitが範囲へ足される。**
そのcommitのclosing keywordはmerge時に発火する。**走査時のSHAを固定し、作った
Pull Requestの`headRefOid`が同じSHAであることを確認する。**

```bash
gh pr view <N> --json headRefOid --jq .headRefOid
```

**食い違ったらmergeしない。**範囲が変わっているため、**走査からやり直して本文を更新する**
（規則4）。**Pull Requestは作り直さない。**

**照合は2回行う。作成直後と、merge を実行する直前である。**昇格Pull Requestのheadは
branch（`develop`）であり、**作成後に`develop`へcommitが入るとPull Requestのheadも動く。**
作成直後の照合だけでは範囲を固定できない。**入ったcommitは未走査かつ未reviewであり、
closing keywordの発火に加えて規則1と規則2の保証も外れる。**下のIssue itemの手当てを
終えたあと、`gh pr merge`を実行する直前にもう一度照合する。

**baseが`main`では、範囲内commit messageのclosing keywordがmerge時に発火する。**
`closingIssuesReferences`とPull Request本文の確認では**この経路を検出できない**
（本文からしか計算されないため。[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。
**履歴は書き換えないため、検出しても消せない。**できるのは、発火を知ってboard側を
先に手当てすることだけである。

**検出したら、参照先Issueと対応するIssue itemの状態を確認する。**boardの
`Auto-close issue`はIssue itemの`Status`が`Done`になったときにcloseする。
**GitHubの自動closeが先に走ると、IssueはCLOSEDだがIssue itemは`Done`にならず、
boardとIssueの状態が食い違う。**

**判定するのはIssueの状態ではなくIssue itemの`Status`である。**参照先が`OPEN`なら、
**mergeする前にIssue itemを`Done`にする。**更新できない場合はmergeしない。
**参照先が既に`CLOSED`でも、Issue itemの`Status`を実測する。**`Done`でなければ手当てが
要る。**`CLOSED`を見て「空振り」と判定しない。**boardには`Item closed` workflowも
有効にしてあるが（[Repository設定](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)）、
**それが`Status`をどの値にするかをここで前提にしない。**見るのは実測した`Status`である。

**走査は昇格Pull Requestを作る前に行う。**CIの`Report closing keywords in the
promotion range` jobも同じ走査を行うが（base=`main`のときだけ）、**作成後にしか出ない。**

### Merge方式

baseで決まる。

| base | 方式 | 理由 |
|---|---|---|
| `develop` | **squash merge** | Issue branchの試行錯誤を1 commitにまとめ、`develop`の履歴を「1 Issue = 1 commit」に保つ |
| `main` | **squashしない。merge commitにする** | 昇格をsquashすると`develop`側の個々のcommitが`main`の履歴から消え、両branchが別系列になる |

**`gh pr merge`には`--subject`と`--body-file`を明示する。**省略するとGitHubが合成したmessageで
mergeされ、検証の経緯や測定値がcommitに残らない（`main`昇格の#383で、merge commit `afc86cf`の
messageがGitHubの既定のまま入った）。`scripts/hooks/gh_metadata_guard.py`が省略を止める。

```bash
gh pr merge <N> --squash --subject "<Pull Requestの題名>" --body-file <path>
```

squash mergeしたbranchはcommit hashが変わるため、`git branch -d`が「未merge」と判定する。
削除前に`git diff <branch> origin/develop`が空であることを確認してから`-D`する。

昇格Pull Requestのmerge後は`origin/develop`が消えていないことを確認する。
[#33](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/33)で`delete_branch_on_merge`が
`develop`自体を削除する事故が起きている。Repository Rulesetで禁止済みだが確認はする。

## hookが止めたとき

`.claude/settings.json`がhookとして起動するscriptのうち、下の表は7つ、**検査は14である。**
**この数は下の表と一致させる。**一致していない間、表に無いhookは「無い」ものとして読まれる。
**数えるのは検査であってscriptではない。**`gh_metadata_guard.py`は1 scriptで8つを見る
（同fileのdocstring「止める対象は8つである」）。`review_record.py`は、止める検査のほかに
レビューの記録を書く（記録は止めない）。公開前の検査（`publication_guard_hook.py`）は、この表とは別に起動する。

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
| `gh issue create`／`gh pr create`の前 | 本文がtemplateの`## `節を持つか | 同上 |
| `gh pr merge`の前 | [慎重に扱う path](#慎重に扱う-path)を含むPull Requestで、CodeRabbitのreviewがどれかのcommitで完走しているか。**解除の手段は無い** | 同上 |
| `gh issue close`の前、およびboardの`Status`を`Done`にする`gh project item-edit`の前 | 対象Issueの`受け入れ条件`の節に`- [ ]`が残っていないか（節が無い本文も止める）。**解除の手段は無い** | 同上 |
| `@coderabbitai`へreviewを投げる前 | 同じ行の`review`が`full review`か（**`full`の付かない`review`は`deny`**）。対象PRのどれかのcommitでreviewが完走・進行中なら`full review`も**`deny`**。完走も進行中も無いPR（`Review rate limited`や`Review stopped after lock loss`の後を含む）への`full review`と、対象PRやcheckを判定できないときは`ask`。[`ask`の扱い](#手動で依頼する前に状態を確認する) | `scripts/hooks/coderabbit_gate.py` |
| `gh pr merge`の前 | `--subject`と`--body`／`--body-file`があるか | `scripts/hooks/gh_metadata_guard.py` |
| `gh pr merge`の前 | レビューの記録が merge-base から head まで途切れずにつながっているか。本文の「自己レビュー」の範囲が手元の log の完了した記録と一致するか。慎重に扱うpathに触れるPull Requestで、本文の全体が`承認 <headの40桁のSHA>`だけの人のコメントがあるか。昇格では、範囲のつながりの代わりに、各commitがPull Requestを通ったか、記録を持つかを見る。**解除の手段は無い**（[自己レビュー](#自己レビュー)） | 同上 |
| PowerShellの前 | commandに語としての`gh`（`gh.exe`を含む）が無いか。あれば読み解かずに止め、Bashで走らせるよう求める（上の`gh`の検査はbashの書き方でcommandを読むため）。**解除の手段は無い** | `scripts/hooks/gh_metadata_guard.py` |
| Write／Edit／MultiEdit／NotebookEdit／Bash／PowerShellの前 | レビューの記録のlog（`deskcat-review-log.jsonl`）へ書き込まないか。shellのcommandに、logの名前が無いか | `scripts/hooks/review_record.py` |
| `git checkout -b`／`git switch -c`の前 | 基点が最新の`origin/develop`か | `scripts/hooks/branch_base_guard.py` |
| `git reset --hard`／`git clean`／checkout・switch・restoreによる破棄の前 | 捨てる範囲に未commitの変更があるか | `scripts/hooks/worktree_guard.py` |
| 応答を終える前 | 完了の主張に、対応するtool呼び出しがあるか | `scripts/hooks/stop_claim_guard.py` |

**判定は字句だけで行う。**意味は判定しない。
**字句だけでは決まらない検査は、repositoryの状態も見る。**`branch_base_guard.py`は
基点を、`worktree_guard.py`は捨てる対象が在るかを、それぞれ子processのgitで読む。
**`git reset --hard`は、汚れたtreeでは破壊的で、汚れていないtreeでは何も失わない。
同じcommandである。**

**改行、heredoc、コメントをどう読むかと、その境界は`scripts/hooks/command_line.py`のdocstringが持つ。
ここへ複製しない。**

### 止まったときにどうするか

**まず、指摘が当たっているかを見る。**当たっていれば、足りないものを足して再実行する。

**誤検知で止まったときは、所有者の承認を得て通す。**`DESKCAT_SKIP_*`の環境変数をcommand行頭に
書いても効かない。2026-08-28に`DESKCAT_SKIP_GH_GUARD=1 gh issue create --help`を実行し、拒否された。
hookは対象commandとは別のprocessとして起動されるため、command行頭の代入はhookのenvironmentへ届かない。

`branch_base_guard.py`は、基点を明示した場合（`git checkout -b <name> <start>`）と
`hotfix/`で始まるbranchを対象外にする。`hotfix/`は`main`から作るのが正しい
（[ADR-0004](docs/decisions/0004-main-develop-branch-strategy.md)）。

### 取り切れていないもの

**hookを「通った」ことを「正しい」と読まない。**次は検査できていない。

- **`--base`の値そのもの。**存在するかだけを見る。`develop`と書くべきか`main`と
  書くべきかはそのPull Requestの目的で決まり、字句からは読めない。
  **`--base main`は昇格では正しく、日常のPull Requestでは誤りである。**
  hookが直すのは「省略して既定の`main`になる」であって、選び間違いではない
  （2026-08-28に[PR #250](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/250)が
  base `main`で作られ、変更まで1時間17分かかった）。
- **`gh issue create`には`--base`を要求しない。**option自体が存在しない。
- **`--help`／`-h`が付いた呼び出しは、`gh_metadata_guard.py`の6つの検査すべてを抜ける。**helpの表示は何も作らず、
  mergeもしないため、誤検知しか生まない。**2026-08-28に`gh issue create --help`、
  `gh pr create --help`、`gh pr merge --help`の3つとも拒否されることを実測した。
  hookが要求するoption名を`--help`で調べる手段そのものが塞がっていた。**
  判定は完全一致であり、`--title -h`のように値として`-h`を渡した呼び出しも
  検査を抜ける。**字句だけで区別する手立てが無い。**
- **`gh api`や画面からIssueをcloseする経路。**closeの検査は`gh issue close`と`gh project item-edit`の字句だけを見る。
  **boardの`Done`のoption idは`Status`fieldのものであり、boardを作り直すと変わる。**
- **squash messageをstdin（`-F -`）で渡す経路。**hookからは読めないため、
  読めないことを理由に止める。`--body-file`を使う。
- **`gh`や`git`を、alias、shell function、`xargs`、`sh -c`の内側から起動した場合。**
  hookはcommandの字句だけを見るため、呼び出しとして拾えない。
  **推測で拾わないのは、誤検知がhookごと無効化される側の失敗だからである。**
- **1行に収めた`$(...)`と、引用の中の`$(...)`、および`eval`の内側。**実行されるのに
  呼び出しとして拾えない。**引用の外で改行を挟む`$( )`の中の行は、行として拾う。**
- **引用の中の`\`改行。**bashは消して1行に繋ぐが、hookは繋がない。
  `--body "@coderabbitai \`改行`full review"`は、bashでは依頼として投稿されるが
  `coderabbit_gate.py`が拾わない（実測）。**#389より前から同じである。**
- **`;`／`&&`の直前に空白が無い形。**`cat x; git push origin develop`は`x;`が1語になり、
  `git`がcommand位置から外れる。**bashは両方を実行する。**`shlex`が空白でしか語を切らない
  ためであり、**改行の対応（[#389](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/389)）では直していない。****#389と同じ失敗型である。**
  **同じ形で`worktree_guard.py`も丸ごと抜ける。**`cd sub; git reset --hard`、
  `(git reset --hard)`、`git reset --hard|cat`はいずれも素通りする（2026-09-17実測）。
  **この穴で失うものは、hookによって違う。**申告の監査漏れで済む側と、
  **戻らない未commitの変更**が消える側がある。`SEPARATORS`に`|`／`(`／`)`は入っているが、
  **空白で囲まれていない限り`shlex`が語を切らないため届かない。**
- **heredocのbodyを、bodyをcommandとして実行する側へ流した場合。**`bash <<'EOF' … EOF`／
  `sh <<EOF`／`ssh host <<EOF`のbodyはbashが実行するが、hookはbodyを落とすため見ない。
  **`cat > file <<EOF`のためにbodyを落としており、受け取る側の綴りで絞っていない。**
  綴りで絞ると、別名や絶対pathで書いた形を落とし損ねる。
- **`<<`をheredocと読み違え、その語と同じ行が後に現れた場合。**
  `echo $((1 << 2))`改行`git push origin develop`改行`2`は、delimiterを`2`と読み、
  3行目を終端行と見て**間のpushを落とす。bashは実行する**（実測）。
  **終端行が現れない場合は1文字も落とさない**（`scripts/hooks/command_line.py`の
  `_skip_heredoc_bodies`）。
- **branchをhook以外の経路で作った場合。**worktreeを外部の道具が作ると
  `git checkout -b`を通らないため、基点は検査されない。
- **`git fetch`ができない環境。**基点の検査は行わず、黙って通る。
- **`git stash drop`／`git stash clear`／`git rm -f`／`git worktree remove --force`／
  `git checkout-index -f`。**いずれも未commitの変更を失いうるが、
  `worktree_guard.py`の対象は[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補1が挙げた3系統
  （`git reset --hard`／checkout系による破棄／`git clean`）だけである。
  **広げるかどうかは判断であり、この変更では判断していない。**
- **汚れていない範囲への破棄は通す**（`worktree_guard.py`）。**cleanなtreeでの
  `git reset --hard <ref>`が落とす未pushのcommitも通る。**この検査が守るのは
  未commitの変更である。
- **`git clean`は、逆に作業treeの状態を見ない**（`-n`／`--dry-run`を除きaskする）。
  **消える対象を`git status`では数え切れないためである。**
- **`cd other && git restore x`のように、commandの中で移動した先のpath。**
  自分のcwdで`git status`を実行する（`git -C <path>`は引き継ぐが、`cd`は字句であって
  実行ではない）。**別のtreeを見て「汚れていない」と読む。**
- **pathspecとして名指しした`pages/assets-manifest.json`だけは通す。**
  [preflight skill](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.claude/skills/deskcat-preflight/SKILL.md)がpush前に`git restore`するよう指示している経路である。
  **`git reset --hard`のようにpathを取らない形では効かない**（範囲全体を見るため、
  このfileだけが汚れていてもaskになる）。
- **staged済みで作業treeがindexと一致する変更も数える。**`git checkout -- <path>`は
  indexから戻すため実際には失われないが、**askする側へ倒している。**
- **`worktree_guard.py`に関する上の各項目について、そう決めた理由は同fileのdocstringが
  持つ。ここへ複製しない。**同じ規則を2箇所で持つと、片方だけが実装とずれる。
  **数で参照しない。**項目が増えたときに数だけが古くなる。

**hookで安全要件を代替しない。**hookが直すのは「忘れる」であって、
[Hardware Safety Policy](docs/governance/hardware-safety-policy.md)が要求する根拠ではない。

## 報告と成果物

**checklistの項目は増やさない。**下の2つは、既存のchecklistが見ない観点を補う手順である
（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。

### 外向き操作の報告を受けたら結果を確認する

「push した」「commit した」「merge した」という報告を受けたら、**その場で結果を見る。**
報告の内容が正確であることと、その操作が引き起こした結果（CIの状態等）が問題無いことは別である。

2026-09-01の実例。担当が「push済み」と報告し、内容は正確だった。だがそのpushでCIが
赤くなっており、報告した側も受け取った側も、その時点では見ていない。気付いたのは
次の照合のときだった。**これは宣言の真偽を検査する話ではない。**受け取る側の手順である。

### 説明は成果物へ書く

PMやreviewerへ説明した内容のうち、成果物（Pull Request本文、commit message、
Issue本文）に残るべきものは、成果物へ書く。**報告と成果物は別のfileであり、
片方に書いたことがもう片方に自動で反映されない。**

2026-08-31の実例。昇格Pull Requestのcheckが`Review skipped`へ戻った経緯を、
担当は報告には正確に書いたが、`main`の履歴に残るPull Request本文には書いていなかった。
説明は正確に行われていた。**宛先が違った。**

## 道具の効かない条件

**道具そのものは正しく動いていても、効かない条件が書かれていなければ誤読する。**
2026-09-01と2026-09-02に、10件の道具でこれが起きた（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。
**ここに8件を記載する。**数を書くなら数える。

- **`git cherry`は、squash mergeされたbranchに対して逆の答えを返しうる。**
  1 commitのbranchでは`-`（取り込み済み）を返すが、複数commitを1つへ畳んだbranchでは
  patch-idが一致せず`+`（未取り込み）を返す。額面どおり読むと「作業が`develop`に
  入っていない」となるが、実際にはblob hashが完全一致していることがある。
  **`git diff`では判定できない場面で`git cherry`へ切り替えても、この条件は消えない。**
  疑わしい場合はblob hashまたは実際のfile内容を突き合わせる。
- **`@coderabbitai rate limit`への返答時間に上限が無い。**6秒で返った例と22分35秒
  かかった例の両方が観測されている（[CodeRabbitのreview状態の観測記録](docs/runbooks/coderabbit-review-observations.md)）。
  **無応答と遅延は区別できない。**これは以前の残数確認運用に関する観測である。
  現行の依頼手順は[手動で依頼する前に状態を確認する](#手動で依頼する前に状態を確認する)を参照する。
- **quoteしていないheredoc（`<<EOF`）は、`\|`とbacktickを含む文書を壊す。**
  このrepositoryの文書はどちらも高頻度で使う。**heredocは`<<'EOF'`で書く。**
- **走査結果を数として報告するときは、patternが何を含み何を落とすかを一緒に書く。**
  「0件」も「N件」も、pattern外側については何も言っていない。2026-09-01に3件の
  実例が出た。

  | 主張したもの | 実際に測ったもの |
  |---|---|
  | 昇格範囲15 fileのbyte合計 | 既存14 fileだけ（新規fileを`NEW`として別行に出し`continue`し、合計から抜けた） |
  | 表のGFMセル数 | パイプ数−1（末尾パイプの無い行で1ずれた） |
  | `members`が増えた時期 | servoが入った時期だけ（patternに`servo`を含めたため、`serial`の日が出なかった） |

- **`gh api /markdown`は、相対linkを書き換えずに返す。**入力に含まれる相対pathの
  文字列がそのまま出力に残るため、GitHub上でrenderされたときに解決できるかは
  この呼び出しだけでは分からない。**ブラウザ側の解決は別に確認する。**
- **`origin/*`を検査するscriptを、自分のworktreeの版で実行すると、遅れた判定になる。**
  `origin/develop`や`origin/main`は`git fetch`しないと更新されない。fetchせずに
  過去の版へ対して検査を走らせると、既に解決済みの問題を「まだ落ちている」
  と読みうる。**実行前に`git fetch origin`する。**
- **`git log -S<文字列>`は、既存の定数へ1行足しただけのcommitを検出しない。**
  `-S`は指定した文字列の**出現数の変化**を見る。定数名
  そのものへ1行足しても、定数名の出現数は変わらないため
  hitしない。**個々の値（SHAや識別子）で検索し直す必要がある。**
- **`closingIssuesReferences`は、本文にclosing keywordがあってもしばらく0件のことがある。**
  **merge時点の0件は最終状態ではない。**反映の遅れであり、幅は測っていない。

  実測:

  - [#313](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/313)・[#318](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/318)は、作成直後は0件で、時間を置くと登録された（2026-09-02。**幅は記録していない**）
  - [#321](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/321)・[#322](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/322)は、merge（2026-09-03 04:10Z／04:11Z）時点で0件。**同日23:45Zの実測では登録済み**（約19時間後）
  - [#326](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/326)は、merge（同日08:34Z）時点で0件。**同じ実測で登録済み**（約15時間後）

  **登録までの幅は測っていない。**上限も下限も持たない。
  #321・#322・#326 は、2026-09-03の`main`昇格作業の時点では
  「作成直後・merge直前・merge後のいずれでも0件」と記録し、
  **「時間を置けば反映されるでは説明が付かない。原因は特定できていない」と書いていた。**
  **測った時刻がmerge直後で、そこで打ち切っていた。**
  **観測しなかった区間で起きていないと扱った誤りである。**
  **0件を「本文にclosing keywordが無い」と読まない。**本文を直接見て判定する。
  この repository はcloseをProjects v2 boardで管理しており、`closingIssuesReferences`に
  依存していない。**0件でも失われる機能は無い**（この節の経緯は[#328](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/328)）。

## Gitと秘密情報

- `.env`、資格情報、token、秘密鍵をcommitしない。
- commit前にstage対象pathとdiffを確認する。
- build生成物をcommitしない。
- **共有branch（`main`／`develop`）へforce pushしない。**自分の未push・未mergeのbranchでは使ってよい（[Development Workflow](docs/governance/development-workflow.md)）。
- 許可なく第三者参考資料を公開しない。

## Security上の報告

脆弱性や秘密情報をpublic Issueで開示しない。[SECURITY.md](SECURITY.md)に従う。
