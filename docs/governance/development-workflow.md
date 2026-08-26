# Development Workflow

> 状態: Active
> 適用範囲: 文書、software、firmware、protocol、ハードウェア関連の変更

## 1. 作業単位

一つのIssueには、一つの主目的だけを設定する。

Issueには次を記載する。

- 背景
- 目的
- 対象範囲
- 対象外
- 依存関係
- 正本となる仕様
- 受け入れ条件
- 必要なPC上のcheck
- 必要な実機check
- 保存する根拠

別の不具合またはrefactorを発見した場合、現在の目的を妨げるものでなければ、別Issueを作成または提案する。

**ただし、すべての気づきをIssueにしない。**typo、リンク修正、表記ゆれ、規約の言い回し、
boardのmetadata記入漏れは、変更内容の承認を得たうえで直接反映してよい。
判断基準は[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)に定める。

## 2. Issueの着手条件

次をすべて満たしたとき、Issueを着手可能とする。

- 目的が一つで、結果を観測できる。
- 依存作業が完了しているか、明示的に利用可能である。
- 必要な部品の識別情報が確定している。
- 関連するGPIO、電源、protocol、安全制限が確定している。
- 受け入れ条件を測定できる。
- 必要なハードウェアと測定器が明確である。
- 許可された変更範囲が明確である。

必要なハードウェア情報が`TBD`のままの場合は、`status:blocked`を付ける。

## 3. 変更手順

各Issueでは次の順で作業する。

1. working treeを確認する。
2. `AGENTS.md`とリンクされた仕様を読む。
3. 期待される結果を内部的に言語化する。
4. リスクと`TBD`を特定する。
5. テスト可能な最小の変更を設計する。
6. 可能であれば、先に失敗するtestを追加または更新する。
7. 依頼された動作だけを実装する。
8. 変更に見合う検証一式を実行する。
9. 最終diffに関係のない変更がないか確認する。
10. 仕様を変更した場合は正本文書を更新する。
11. 結果と残っている実機checkを記録する。

## 4. 統合順序

ハードウェア作業は、独立した根拠の取得から統合へ進める。

1. 電源と静的な電気check
2. toolchain、build、flash、起動log
3. UARTまたはUSB serial
4. 一つのbusと一つのdevice
5. raw device data
6. 変換後のdataとerror handling
7. hostから観測可能なeventまたはcommand
8. 2 componentの統合
9. 全体統合
10. 長時間試験とfault injection test

最終版のdriverをすべて生成し、一つの変更で統合してはならない。

## 5. コード規則

### Rust

- Rustを主要言語とする。
- ESP32は、初期段階ではESP-IDFベースのRust環境を対象とする。
- 互換性のあるRust、ESP-IDF、target、crateのversionを、現在の公式文書で確認する。
- 再現可能なapplication buildにlockfileが必要なprojectでは、lockfileを維持する。
- 保守対象のコードではwarningをfailureとして扱う。
- 単位、ID、状態、検証済みの値を型で区別する。
- business logicを物理I/Oから独立させる。
- real-timeまたは長時間動作するpathでは、上限のないallocationを避ける。
- `unsafe`は、invariant、代替案、testを含む個別reviewを行わない限り禁止する。

### ハードウェア依存コード

- board pinとハードウェア設定を一元管理する。
- GPIOや安全値をdriver内に散在させない。
- device register、CRC、calibration、変換処理の詳細はadapter内に閉じ込める。
- 共有busごとに明示的なownerとlocking規則を定める。
- 初期化の依存関係を明示する。
- すべてのbuffer、queue、timeout、retry、message lengthに上限を設ける。

### Interrupt

ISRでは次を実行してよい。

- 発生源の特定とclear
- 最小限のdataまたはtimestampの取得
- queue、task、flagへの通知
- overflow counterの増加

ISRでは次を実行してはならない。

- allocation
- blocking
- JSONのparseまたはserialize
- 長いI2CまたはSPI transaction
- frame描画
- サーボtrajectoryの実行
- 長いlogの出力

## 6. 依存関係のポリシー

依存を追加する前に、次を記録する。

- standard libraryまたは現在の依存では不足する理由
- 公式projectとpackageの識別情報
- 保守状況
- 対応するtargetとtoolchain
- license
- security上の考慮事項
- 該当する場合はbinary sizeとruntimeへの影響
- 検討した、より単純な代替案

可能であれば、依存だけを更新する変更はfeature作業から分離する。

生成された指示に記載されているという理由だけでtoolを導入しない。文書化された用途と再現可能なsetupがあるtoolだけを追加する。

## 7. 検証マトリクス

存在し、変更に関係するcheckを実行する。

| 変更種別 | 最低限のcheck |
|---|---|
| Markdownのみ | 構造、リンク、参照path、整合性 |
| 純粋なRust logic | format、lint、unit test |
| Protocol | format、lint、unit test、境界値、不正入力、互換性test |
| ESP32設定 | buildと文書化されたハードウェアreview |
| Driver | host logic test、ESP32 build、独立したdevice test |
| Servo | build、電気check、人間監視下での限定可動域test |
| Integration | component regression、組み合わせtest、counterとlog |
| Release | clean build、final profile、受け入れtest、fault test |

checkを実行できない場合は次を行う。

- 理由を明記する。
- 成功したものとして扱わない。
- 実行に必要な人または環境を明記する。
- 未実行のcheckが受け入れ条件ならIssueをopenのままにする。

## 8. ハードウェアの根拠

実機試験では次を記録する。

- 日付と作業者
- hardwareとboard revision
- 正確な部品model
- 配線または回路図revision
- 電源と電流制限設定
- firmware commitとbuild profile
- 設定値
- 測定器
- 手順
- raw result
- expected result
- 結論

写真、screenshot、waveformだけでは、試験条件を記した文書の代わりにならない。

## 9. 文書更新

次を変更する場合は、正本を更新する。

- Architecture
- GPIO
- 電源
- Protocol
- サーボ安全
- 部品識別情報
- Toolchain
- Build／flash command
- 受け入れ条件

同じ定数を複数のMarkdownファイルへコピーしない。代わりに正本へリンクする。

元に戻すコストが高い判断、複数componentに影響する判断、projectの慣例を定める判断にはADRを使用する。

## 10. Git workflow

### Working tree

- 編集前にstatusを確認する。
- 関係のない未commitのユーザー変更を維持する。
- 関係のないfileを整形しない。
- 未使用に見えるという理由だけで不明なfileを削除しない。
- 可能な限り復元可能な操作を使用する。

### Branch

[ADR-0004](../decisions/0004-main-develop-branch-strategy.md)に従い、
`main`を安定版、`develop`を通常開発の統合先とする。

- GitHubのdefault branchとGitHub Pagesのdeploy元は`main`のまま維持する。
- 通常のIssue branchは最新の`develop`から作成し、Pull Requestで`develop`へ統合する。
- Releaseまたはmilestoneの基準を満たした変更は、`develop`から`main`へのPull Requestで昇格する。
- `main`からhotfixした場合は、同じ修正を直ちに`develop`へ取り込む。
- 通常作業を`main`または`develop`へ直接commitしない。
- Repository運用のbootstrapなど例外的な直接変更には、明示的なユーザー承認とread-backを必要とする。

Issue branchの命名規則は次のとおり。

```text
feature/<issue>-<short-name>
fix/<issue>-<short-name>
docs/<issue>-<short-name>
chore/<issue>-<short-name>
experiment/<issue>-<short-name>
hotfix/<issue>-<short-name>
```

一つのbranchは一Issue、一つのreview可能な目的に対応させる。
Pull Request作成時は、通常のbaseが`develop`であることを確認する。

### Commit

- 一つのcommitには一つのまとまった目的だけを含める。
- 命令形のsummaryを使用する。
- 生成物、secret、ローカル設定を含めない。
- commit前にstaged diffを確認する。
- 指示なしにユーザーのcommitをamendまたはrewriteしない。
- AIは依頼された場合だけcommitする。

### Gitの外部操作

- 明示的な指示なしにpush、force-push、tag、release、visibility変更を行わない。
- **共有branch（`main`／`develop`）へは、指示があってもforce-pushしない。**履歴を書き換えない。
- **他者がpullした可能性のあるbranchは共有として扱う。**
- 外部へ書き込む前に正確なremoteとbranchを確認する。

#### 未mergeのbranchでのrebaseとforce-push

**自分の未push・未mergeのbranchでは、rebaseとforce-pushを使ってよい。**
ここで許すのは操作の種類であって、**指示なしにpushしてよいという意味ではない。**
上の「明示的な指示なしにpush、force-pushを行わない」はそのまま適用される。

以前は一律で禁止していた。守るべきものは共有履歴であって、自分のbranchの整理ではない。

**一律の禁止は、既に守られていなかった。**feature branchを`develop`へ追随させるrebaseは、
記録として残っている。

| 記録 | 内容 |
|---|---|
| [ESP32 Build (Linux x86_64)](../toolchains/version-records/2026-08-06-esp32-build-linux.md) | `develop`（#44／#45／#46反映後）へrebaseし、「rebase後のフル再検証log」を残している |
| [host workspace (CI)](../toolchains/version-records/2026-08-15-host-rust-ci.md) | Pull Request #130を`origin/develop`へrebaseし、SHAが振り直されたことを記録している |
| [SDのhealth check](../hardware/sd-health-check.md) | #113のmerge後の`develop`へrebaseしたことを記録している |
| [Raspberry Pi Direct Build](../toolchains/version-records/2026-08-17-pi-direct-build-native.md) | `develop`の更新へ追従してrebaseしたことを記録している |

**いずれも未mergeのfeature branchであり、共有履歴は書き換えていない。害も出ていない。**
規則が禁じ、運用が行い、記録がそれを残している状態は、**厳格に守るより悪い。**
守っている側だけが、基点をそろえる正しい対処を躊躇する。基点をそろえずに進めると、
後から入った文書を「存在しない」と読み、そこから誤った断定へ進む。

**線引きの基準は「他の誰かがその履歴を持っているか」である。**push済みで他者が
pullした可能性があるbranchは、未mergeでも共有として扱う。判断に迷うものは共有として扱う。

`git reset --hard`と強制checkoutは、**履歴書き換えではなく未commitの変更を失う操作である。**
別枠では禁止しない。[Working tree](#working-tree)の「関係のない未commitのユーザー変更を
維持する」と「可能な限り復元可能な操作を使用する」が守る対象と同じものを守っている。
**実行前にstatusを確認する。**

## 11. Review

次の観点を分けてreviewする。

- 仕様適合性
- 電気的な仮定
- 機械的安全性
- error handlingと復旧
- concurrencyとownership
- memoryとresource上限
- protocol互換性
- securityとsecret
- regression
- test不足

AIによるreviewの指摘は、コード、公式文書、test、測定のいずれかで裏付けられるまでは仮説として扱う。

## 12. Definition of Done

次をすべて満たしたとき、変更を完了とする。

- 目的と受け入れ条件を満たしている。
- 範囲がIssue内に限定されている。
- 必要なcheckが成功している。
- 必要な実機checkが記録されている。
- 既存動作に対して変更に見合うregression testを実行している。
- 安全制限が引き続き強制される。
- 新しいリスクと`TBD`を記録している。
- 文書と実装が一致している。
- 最終報告を別のcontributorが再現できる。
