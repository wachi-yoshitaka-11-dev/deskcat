# ADR-0002: 役割別の開発環境

> 状態: Accepted
> 日付: 2026-07-27

## 背景

DeskCatは複数端末で開発される。ESP32 firmware、Raspberry Pi service、文書review、flash、実機試験では、必要なtoolとriskが異なる。

すべての端末へRust、ESP-IDF、USB driver、実機制御toolを導入すると、次の問題が生じる。

- 文書確認だけの端末にも不要なsystem changeが発生する
- どの端末で何を検証したか曖昧になる
- system-wide SDK、environment variable、linkerの差がbuildを汚染する
- flashや実機操作が意図しない端末から実行可能になる
- version driftと再現性の管理が難しくなる

## 判断要因

- 複数端末で一貫した開発を行う
- 不要なdependencyと権限を減らす
- build、flash、実機試験の根拠を区別する
- 文書端末を安全に保つ
- toolchain差分をGitでreviewできる

## 検討した選択肢

### すべての端末へ完全環境を導入する

端末間の見かけ上の差は小さくなるが、不要な変更、保守負担、実機操作可能な範囲が増える。

### 一台の専用開発端末だけを使う

環境は集中するが、複数端末での作業とreview、障害時の再現が難しい。

### 作業roleごとに最小環境を定義する

端末は一つ以上のroleを担い、必要なtoolだけを導入する。Gitにはversionと手順を保存し、端末固有情報は保存しない。

## 決定

[Machine Profiles](../toolchains/machine-profiles.md)に定義したrole-based environmentを採用する。

- Docs / Review端末にはbuild toolchainを要求しない。
- Host Rust、ESP32 Build、ESP32 Flash / HIL、Raspberry Pi Runtime / Direct Build、CIを別profileとする。
- 一台が複数profileを兼ねることを許可する。
- profileごとに[Version Record](../toolchains/version-record-template.md)を作成する。
- 「調査済み」「採用候補」「検証済み」「確定」を区別する。
- system-wide toolの導入と大きな更新には、対象Issueと人間の確認が必要である。
- flashと実機試験をbuild-only検証から分離する。

## 影響

### 利点

- 文書端末に不要な開発環境を導入せずに済む。
- どの検証をどの端末roleで行ったか追跡できる。
- buildと実機操作の権限境界が明確になる。
- 別端末で再現すべき情報をGitへ集約できる。

### 欠点

- profileごとに初期setupとversion recordが必要になる。
- 一台で成功しても、別profileの端末を自動的に検証済みにはできない。
- toolchainの組み合わせを複数保守する必要がある。

### リスクと対策

| リスク | 対策 |
|---|---|
| 端末ごとのversion drift | lockfile、toolchain指定、version record |
| 個人情報やsecretの混入 | 公開禁止fieldをtemplateとpolicyに明記する |
| 文書のcommandを未検証のまま実行 | `Draft`と`Verified`を分離し、実行端末で結果を記録する |
| Build端末から誤ってflash | profileとIssueを分離し、hardware gateを必須にする |
| `IDF_PATH`等によるSDK汚染 | environment overrideをversion recordに記録する |

## 検証

この決定は、次を満たすことで検証する。

- Docs / Review端末だけでMarkdown検証を完了できる。
- ESP32 Build profileの別端末で、文書化したsetupからclean buildを再現できる。
- Raspberry Pi Direct Build profileで、実機buildとrunを再現できる。
- Flash / HILの根拠をBuild-onlyの根拠と明確に区別できる。
- version recordにsecretや個人pathが含まれない。

## 置き換える決定

なし。
