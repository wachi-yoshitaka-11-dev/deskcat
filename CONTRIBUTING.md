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
| 開始日／終了日（`Start date`／`Target date`） | 任意 | 着手日・完了目標が具体的に決まった時点でのみProjects v2 board上で設定する。未定なら空欄のままでよい |

この表はIssue itemの規約である。Pull Request itemでは同じ2 fieldが必須であり、`Start date`は
作成日という実績、`Target date`はmerge見込み日という予定を表す。`Target date`はmergeまたは
closeの時点で実績値へ更新する。[Pull request](#pull-request)節を参照する。

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

workspaceはまだ存在しないため、現段階で有効と断定できるCargo commandはない。toolchain Issueで生成・検証後、command文書を更新する。

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
| `Target date` | 作成時はmergeを見込む日（予定）、mergeまたはclose後はその実績日 | 作成時に見込みを設定し、merge時またはclose時に実績値へ更新する |

`Target date`だけが予定から実績へ変わる。merge時に`Status`を`Done`にするworkflowは
日付を書き換えないため、実績値への更新は手作業で行う。mergeせずcloseした場合の実績日は
close日である。

日付はJST（UTC+9）で判断する。GitHubのAPIが返す時刻はUTCであり、JSTの`00:00`から`08:59`に
作成したPull RequestはUTCでは前日の日付になるため、そのまま転記しない。

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
