# ADR-0001: Monorepo構成とworkspace境界

> 状態: Accepted
> 日付: 2026-07-27

> **注記（2026-08-08追記）**: 下の「背景」に書いた機器名のうち、
> `Raspberry Pi Zero WH`と`ESP32-DevKitC-32E`は、その後の現物確認で
> それぞれ`Raspberry Pi Zero W`（V1.1）、`ESP-WROOM-32D開発ボード（秋月電子 M-13628）`
> と判明した（[#55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)）。
> **ADRは決定時点の記録であるため本文は書き換えない。**
> 機器の識別の正本は[hardware-bom.md](../hardware/hardware-bom.md)である。
> なお、この訂正はmonorepo構成とworkspace境界という本ADRの決定内容に影響しない。

## 背景

DeskCatは、二つの異なる実行環境向けsoftwareを含む。

- Linuxを実行するRaspberry Pi Zero WH
- ESP-IDFベースのRust toolchainを使用するESP32-DevKitC-32E

Pi application、domain logic、protocol、設定、API、storage、simulator、firmware、ハードウェア記録、deploy fileのversionを連携させる必要がある。一方で、ESP32固有のtoolchainとtarget依存が、通常のhost workspace commandを不安定にしてはならない。

初期project方針では、Rustを主要言語とし、ESP32 firmwareをhost Cargo workspaceから分離することが推奨されている。

## 判断要因

- ハードウェアとsoftwareの変更を連携できる一つのproject履歴
- 明確なESP32–Pi間の責務境界
- toolchainが許す範囲で再利用可能なdomain logicとprotocol logic
- ESP32 toolchainなしで再現可能なhost test
- 明示的なハードウェア文書と実験根拠
- 個人projectに適した、小さく理解しやすい構成
- 後からsimulator、deploy、設定UIを追加できる拡張性

## 検討した選択肢

### リポジトリを分割する

ESP32 firmware、Pi software、ハードウェア文書を別々のrepositoryで管理する。

利点:

- toolchainを分離できる
- access設定とrelease設定を独立させられる

コスト:

- repositoryをまたぐprotocol変更を一体としてreviewしにくい
- MVPのIssueと文書が分散する
- version互換性に追加の調整が必要になる

### ESP32を含む単一Cargo workspace

すべてのRust packageを一つのworkspaceの通常memberにする。

利点:

- 共有依存の宣言が単純になる
- lockfileとcommandの入口を一つにできる

コスト:

- ESP32は専用targetとtoolchainを使用する
- host専用のCIと開発環境がembedded側の制約を引き継ぐ可能性がある
- 依存またはfeatureの解決によって、関係のないtargetが結合する可能性がある

### ESP32 workspaceを分離したmonorepo

Host packageはroot Cargo workspaceを使用する。ESP32 firmwareは同じGit repository内に置くが、そのworkspaceから除外し、toolchain固有の設定を個別に保持する。

利点:

- 文書とprotocolの変更を連携できる
- host testを単純に保てる
- firmware toolchainを明示できる
- 一つのIssueとrelease履歴で製品全体を説明できる

コスト:

- 二つのbuild環境と、場合によっては二つのlockfileが必要になる
- 共有Rust crateの互換性を検証する必要がある
- CIでhostとfirmwareのcheckを分ける必要がある

## 決定

Monorepoを採用する。

### Root host workspace

将来のroot Cargo workspaceには、次を含める。

```text
apps/deskcatd
crates/deskcat-domain
crates/deskcat-protocol
crates/deskcat-config
crates/deskcat-api
crates/deskcat-storage
simulator/deskcat-sim
```

`apps/deskcatd`はRaspberry Pi serviceとする。Pi側の主要言語はRustとする。

### ESP32 workspace

`firmware/esp32`はroot Cargo workspaceから除外する。生成後は、互換性のあるRust toolchain、ESP-IDF設定、firmware manifest、firmware lockfileをこのdirectoryが管理する。

正確なRust、ESP-IDF、target、crateのversionは、現在の公式文書を使用したtoolchain spikeが完了するまで`TBD`とする。

### Release単位

Host serviceとESP32 firmwareは個別にbuildでき、それぞれ独立したversionを報告する。project releaseでは、test済みの互換pairを公開してよい。互換性は、package versionが等しいという仮定ではなく、対応するprotocol major versionとrelease時の根拠で定義する。

### Protocolの所有

wire上の正規動作は次の文書を正本とする。

```text
docs/protocol/esp32-pi-protocol.md
```

Host側Rust実装の配置予定先は次のとおり。

```text
crates/deskcat-protocol
```

Firmwareが同じcrateをpath dependencyとして直接使用するか、小さいfirmware側実装を使用するかは、互換性spike後に決定する。コードを共有するかどうかにかかわらず、両側が共通JSON fixtureとprotocol conformance testに合格しなければならない。

### ハードウェア情報の所有

ハードウェア情報の正本は`docs/hardware/`に置く。

回路図、PCB source、mechanical CAD等のversion管理対象source artifactは、存在する場合にtop-levelの`hardware/`へ置いてよい。生成された製造dataと大容量captureには、明示的な追跡ポリシーが必要である。

### 補助directory

```text
configs/          安全な設定例と秘密でない発話文
deploy/           Raspberry Pi serviceとinstall用artifact
docs/             Architecture、decision、governance、hardware、protocol、runbook、toolchain
pages/            GitHub Pages固有の入口、設定、404
scripts/          小さく再現可能な開発補助script
tests/hil/        Hardware-in-the-loopのfixtureと手順
.github/          GitHub templateとautomation
```

初期基盤では`web/`と`assets/`を作成しない。設定UIまたはlicense確認済みdesign assetが、承認されたIssueになった時点で追加してよい。

### 初期リポジトリ構成

```text
deskcat/
├─ AGENTS.md
├─ README.md
├─ LICENSE
├─ apps/
│  └─ deskcatd/
├─ crates/
│  ├─ deskcat-domain/
│  ├─ deskcat-protocol/
│  ├─ deskcat-config/
│  ├─ deskcat-api/
│  └─ deskcat-storage/
├─ firmware/
│  └─ esp32/
├─ simulator/
│  └─ deskcat-sim/
├─ configs/
├─ deploy/
├─ docs/
│  ├─ architecture/
│  ├─ backlog/
│  ├─ decisions/
│  ├─ governance/
│  ├─ hardware/
│  ├─ planning/
│  ├─ protocol/
│  ├─ runbooks/
│  └─ toolchains/
├─ hardware/
├─ pages/
├─ scripts/
├─ tests/
│  └─ hil/
└─ .github/
```

Directoryは、説明のない`.gitkeep`ではなく、責務を記載したREADMEによって追跡する。

## 影響

### 利点

- ESP32、Pi、protocol、ハードウェアの変更を一緒にreviewできる。
- 初期のhost開発ではESP32 toolchainを必要としない。
- 各directoryの責務を狭く保てる。
- Protocol互換性を製品contractとして扱える。
- ハードウェア情報に専用のSingle Source of Truthを持てる。

### 欠点

- contributorはrootとfirmwareのbuild境界を理解する必要がある。
- 共有crateの依存互換性は保証されない。
- CI setupにhostとfirmwareの個別stageが必要になる。
- release noteにfirmwareとhost両方の互換性を記載する必要がある。

### リスクと対策

| リスク | 対策 |
|---|---|
| 二つのtoolchainが分かりにくくなる | commandを分けて文書化し、versionを固定する |
| Protocol codeが乖離する | 正規仕様、共有fixture、conformance testを使用する |
| 空のarchitectureが早期に肥大化する | 着手可能なIssueがある場合だけpackageを作成する |
| ハードウェア情報が重複する | Single Source of Truthの対応表を強制する |
| Root workspaceへ誤ってfirmwareを含める | manifest作成時に明示的な`exclude`を設定する |

## 保留した判断

- 正確なRust editionとtoolchain version
- ESP32 templateと依存version
- Firmwareからの`deskcat-protocol`直接再利用
- Raspberry Pi向けcross-compile方針
- Deploy package
- 設定UI技術
- Assetの保存・license管理手順

各保留事項は、その実装Issueまたは新しいADRで解決する。

## 検証

次を満たしたとき、ADR-0001を検証済みとする。

- ESP32 toolchainなしでhost workspace commandを実行できる。
- 固定・文書化したtoolchainでfirmwareをbuildできる。
- 両側のprotocol実装が共有fixtureに合格する。
- contributorがdirectory mapから任意のsourceまたは仕様のownerを特定できる。

## 置き換える決定

なし。
