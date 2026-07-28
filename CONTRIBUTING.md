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

- 関連Issueへのlink
- 結果と範囲の説明
- 仕様変更の特定
- 検証証拠
- 残っている実機test
- 新規dependency
- 残存riskと`TBD`
- 無関係なformat変更やrefactorがないこと

## Gitと秘密情報

- `.env`、資格情報、token、秘密鍵をcommitしない。
- commit前にstage対象pathとdiffを確認する。
- build生成物をcommitしない。
- 通常作業でforce pushしない。
- 許可なく第三者参考資料を公開しない。

## Security上の報告

脆弱性や秘密情報をpublic Issueで開示しない。[SECURITY.md](SECURITY.md)に従う。
