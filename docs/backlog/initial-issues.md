# 初期Issue

> 状態: GitHub移行済み
> Remote作成: 2026-07-28に初期Issue 24件（[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)〜[#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24)）と文書公開基盤のfollow-up 3件（[#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)〜[#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)）を作成した

> **注記（2026-08-10追記）**: 下の各Issue定義に書いた機器名のうち、
> `Raspberry Pi Zero WH`と`ESP32-DevKitC-32E`は、その後の現物確認で
> それぞれ`Raspberry Pi Zero W`（V1.1）、`ESP-WROOM-32D開発ボード（秋月電子 M-13628）`
> と判明した（[#55](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/55)）。
> 該当するのは[#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)（ESP32 toolchain）の`目的`と、
> [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8)（Raspberry Pi Rust環境）の前書きおよび`目的`である。
> **本文書は承認した初期backlogの履歴資料であるため本文は書き換えない。**
> 機器の識別の正本は[hardware-bom.md](../hardware/hardware-bom.md)である。
> `#1`〜`#24`のIssue本文はいずれもこの文書を`定義元`として参照しているため、
> **Issueから辿ってこの文書の旧型番を読んだ場合は、この注記を優先する。**
> なお、両Issueの受け入れ条件は型番を直接参照していない。訂正が及ぶのは上記の記述であり、
> 実機の識別は各Issueの範囲で確認する。

## 移行後の運用

- Issueの状態、担当、議論、追加checklistは[GitHub Issues](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues)を正本とする。
- このfileは、承認した初期backlogの定義と作成順序を残す履歴資料であり、live statusを重複管理しない。
- Repositoryのlabel、milestone、security設定の適用はGitHub基盤整備として直接完了しているため、遡及Issueを作成していない。
- GitHub Issue本文では依存関係を実際のIssue番号へ置き換えた。ローカル定義からも対応Issueへ辿れるよう、依存関係をlink化している。
- 例外は**cargo workspace root**である。`#9`の`root Rust workspace`と`#23`の`Root host workspace`に対応するIssueは無い。[ADR-0001](../decisions/0001-monorepo-layout.md)が**着手可能なIssueがある場合だけpackageを作成する**と定めているためである。存在しないIssueを探さない。workspaceは最初に必要とするIssueが作る。実際にfirmware workspaceは`#5`、host workspaceは`#9`が作成しており、[Implementation Readiness Review](../runbooks/implementation-readiness-review.md)のSoftware gate「Host／firmware workspaceが存在する」は現在`Pass`である。

> このfile内の`起票時の状態`、`Labels`、受け入れ条件のcheckboxは、**起票時点のsnapshot**である。
> `status:blocked`等のlabelも当時の値であり、現在の付与状況とは一致しない。
> 現在の進捗として読まない。closeされたIssueでも、ここのcheckboxは当時のまま残る。
> 現在の状態は必ずGitHub Issueで確認する。
>
> **起票時点のものはこの3項目に限らない。**`### 目的`や`### 対象範囲`などの本文も、
> 承認した初期backlogの定義そのものであり、当時の記述のまま残す。
> したがって、**本文に出てくる機器の型番も現在の値とは限らない。**
> 型番については冒頭の訂正注記を先に読む。正本は[hardware-bom.md](../hardware/hardware-bom.md)である。

`#28`は本文書にlocal定義を持たない。#25〜#27の後続として
GitHub Issue側だけで起票・管理しており、目的と受け入れ条件は[#28](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/28)を参照する。
本文書は承認した初期backlogの履歴資料であり、後から追加したIssueの定義を遡って複製しない。

| Milestone | GitHub Issues |
|---|---|
| M0 | [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#4](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/4)、[#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)、[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)、[#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)、[#28](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/28) |
| M1 | [#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)、[#7](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/7)、[#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8) |
| M2 | [#9](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/9)、[#10](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/10)、[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)、[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12) |
| M3 | [#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)、[#14](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/14)、[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)、[#16](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/16) |
| M4 | [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、[#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)、[#19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)、[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20) |
| M5 | [#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)、[#22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)、[#23](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23) |
| M6 | [#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24) |

## 依存関係の概要

```text
M0 foundation
├─ hardware inventory
│  ├─ GPIO assignment
│  ├─ power budget
│  └─ servo safety inputs
├─ protocol draft review
└─ public documentation policy
   ├─ GitHub Pages deployment
   └─ Wiki landing page

M1 ESP32 toolchain
├─ minimal build/flash
├─ boot diagnostics
└─ heartbeat

M2 protocol
├─ shared fixtures and host model
├─ firmware bounded parser
├─ USB serial integration
└─ reconnect/status sync

M3 display/input
├─ LCD
├─ touch
├─ accelerometer
└─ environment sensor

M4 servo
├─ electrical characterization
├─ mechanical calibration
├─ trajectory limiter
└─ fail-safe integration

M5 MVP integration
└─ M6 reliability
```

## 作成順序

既に作成したIssue番号を依存関係から参照できるよう、この文書の順序で作成する。

---

## #1: ハードウェア現物inventoryの確認

- Milestone: M0 Development Foundation
- Labels: `area:hardware`、`type:experiment`、`priority:critical`、`needs:hardware-test`
- 起票時の状態: 人間が部品を確認できるまでBlocked
- 依存関係: なし

### 目的

初期DeskCat製作で使用するすべての物理moduleを、正確なメーカー、model、suffix、module boardの識別情報によって特定する。

### 対象範囲

- ESP32 board revisionとmodule suffix
- Raspberry Pi revision
- LCDとtouch
- Accelerometer
- Environmental sensor
- Servo
- Pi／logic電源とservo電源
- microSD

### 対象外

- Driver実装
- 配線または初回通電
- Color sensor選定

### 正本文書

- `docs/hardware/hardware-bom.md`
- `docs/hardware/sensor-datasheet-notes.md`
- `docs/hardware/tbd-register.md`

### 受け入れ条件

- [ ] すべての初期部品について現物表示を記録した
- [ ] メーカーのデータシートをリンクした
- [ ] Module pinout／回路図の情報源をリンクした
- [ ] 供給電圧、logic電圧、peak電流の根拠を記録した
- [ ] 候補例を選定済み部品として提示していない
- [ ] 解決したTBD行から根拠を参照できる

### 証拠

部品写真またはローカル識別子、正確な文書link／revision、更新済みBOM。

---

## #2: 初期GPIO割り当ての承認

- Milestone: M0 Development Foundation
- Labels: `area:hardware`、`area:firmware`、`type:decision`、`priority:critical`、`status:blocked`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)

### 目的

Boot、flash、UART、board機能、他deviceと競合させず、必要なESP32信号をすべて割り当てる。

### 受け入れ条件

- [ ] 正確なboard回路図を使用した
- [ ] LCD、touch、sensor、servo、UARTの全信号を記載した
- [ ] 起動時状態とpull動作を文書化した
- [ ] SPI chip selectが独立している
- [ ] I2C addressとpull-upに互換性がある
- [ ] Bootstrap pinと使用制限pinをreviewした
- [ ] 最初のbring-up範囲について`docs/hardware/gpio-assignment.md`に未解決GPIOがない

### 実機check

電源offでの導通とpin header対応だけを確認する。Actuatorは動作させない。

---

## #3: 電源architectureと予算の完成

- Milestone: M0 Development Foundation
- Labels: `area:hardware`、`type:decision`、`priority:critical`、`status:blocked`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)

### 目的

安全なlogic／Pi電源経路とservo電源経路、電流容量、接地、decoupling、connector、測定制限を定義する。

### 受け入れ条件

- [ ] 通常、最大、同時動作時に想定されるpeak負荷をすべて記載した
- [ ] Servo電源をESP32 board regulatorから分離した
- [ ] Common GND経路を明示した
- [ ] USB／外部電源間のbackfeedをreviewした
- [ ] 電源、wire、connectorの定格にmarginを含めた
- [ ] Oscilloscope測定点と数値による受け入れ制限を定義した
- [ ] `docs/hardware/power-budget.md`を初回通電用として承認した

---

## #4: Draft protocol v1のreview

- Milestone: M0 Development Foundation
- Labels: `area:protocol`、`type:decision`、`priority:high`
- 起票時の状態: 文書reviewに着手可能
- 依存関係: なし

### 目的

Toolchainとprotocol実装のIssueに必要な最小限の意味を承認する。

### 対象範囲

- Envelope
- `boot`、`ping`、`get_status`、`status`、`ack`
- 上限のある受信動作
- Error code
- Retryとduplicate policy
- Reconnect sequence
- Versioning

### 対象外

- 最終LCD text制限
- 最終motion名
- ハードウェアから決まるtouch／acceleration field

### 受け入れ条件

- [ ] 候補のtransport制限を明確に表示した
- [ ] ACKによる受け入れと物理動作の完了を区別した
- [ ] 不正、oversize、duplicate、stale入力の動作を定義した
- [ ] Reconnect時に古いmotionを再実行できない
- [ ] 必要なconformance fixtureを列挙した
- [ ] 未決定事項をprotocol TBDとして記載した

---

## Repositoryのlabel、milestone、security設定を適用

- Milestone: M0 Development Foundation
- Labels: `type:maintenance`、`area:docs`、`priority:normal`、`status:blocked`
- 記録: 2026-07-28に直接完了。監査記録が必要な場合を除き、遡及的なIssueは作成しない
- 依存関係: 有効なGitHub認証 — 解決済み

### 目的

Review済みのローカルGitHub設定をpublic repositoryへ適用する。

### 正本file

- `.github/labels.yml`
- `.github/MILESTONES.md`
- `.github/REPOSITORY_SETTINGS.md`
- `SECURITY.md`

### 受け入れ条件

- [x] Repositoryへtokenを保存せずGitHub CLI認証が有効
- [x] Labelが`.github/labels.yml`と一致する
- [x] 説明付きのM0–M6 milestoneが存在する
- [x] Private vulnerability reportingが有効
- [x] `main`でforce pushと削除が無効
- [x] 存在しないCI statusを必須にしていない
- [x] 別途決定しない限りrepository visibilityをpublicのまま維持する

---

## #25: GitHub Pages／Wikiの運用方針を決定

- Milestone: M0 Development Foundation
- Labels: `area:docs`、`type:decision`、`priority:normal`
- 起票時の状態: 方針reviewに着手可能
- 依存関係: なし

### 背景

- Repositoryはpublicで、`docs/`を技術文書の正本として運用している。
- GitHub Pagesは`build_type: workflow`で有効だが、Pages workflowと実行履歴は存在しない。
- GitHub Wikiは有効だが、既定の英語`Home.md`だけが存在する。

### 目的

GitHub PagesとWikiの責務、正本、公開範囲、更新方法を決定し、文書の二重管理を防ぐ。

### 対象範囲

- Pagesで公開する文書と公開しない文書
- `docs/`と公開siteの正本関係
- Wikiを使用するか、無効化するか、用途を限定するか
- Review、更新、link、archiveの運用
- Pages生成物とworkflowの管理方針

### 対象外

- Pages workflowの実装
- Wikiへの本格的なcontent作成
- Custom domain
- Firmwareまたはhardware実装

### 参考資料

- [Configuring a publishing source for your GitHub Pages site](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)
- [About wikis](https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis)

### 受け入れ条件

- [ ] `docs/`を正本として維持するか明記した
- [ ] Pagesの対象範囲とnavigation方針を決めた
- [ ] 公開対象外のlocal資料、secret、個人path、再配布不可資料を除外する規則を決めた
- [ ] Wikiの採用、用途限定、無効化のいずれかを決定した
- [ ] Wikiを使用する場合、`docs/`との重複を防ぐ昇格・archive規則を決めた
- [ ] Pagesのbuild方式、Actions権限、version固定方針を決めた
- [ ] 決定をGovernanceまたはADRへ記録した
- [ ] [#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)の実装入力が揃った

---

## #26: GitHub Pages公開基盤を構築

- Milestone: M0 Development Foundation
- Labels: `area:docs`、`type:maintenance`、`priority:normal`、`status:blocked`
- 起票時の状態: [#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)の方針承認待ち
- 依存関係: [#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)

### 目的

承認済みの公開方針に従い、`docs/`を正本とするGitHub Pages siteを再現可能かつ最小権限で公開する。

### 対象範囲

- Static site生成方式の選定と記録
- Navigation、top page、404 page
- Pull Requestでのbuild／link検証
- `main`からのPages deploy
- GitHub Actions権限とenvironment保護
- READMEから公開siteへの導線

### 対象外

- Wiki contentの整備
- Custom domain
- Firmware／hardwareのbuild CI
- 再配布権が不明なPDF、image、fontの公開

### 参考資料

- [Using custom workflows with GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)

### 受け入れ条件

- [ ] 追加tool／依存の必要性、保守状況、license、代替を記録した
- [ ] Workflowで使用するActionをreview可能なversionへ固定した
- [ ] 通常権限はread-onlyとし、deploy jobだけにPages用権限を限定した
- [ ] Pull Requestではbuild／link checkだけを実行しdeployしない
- [ ] `main`の承認済み変更だけをPagesへdeployする
- [ ] Overview、architecture、Governance、安全、文書indexへnavigationできる
- [ ] Custom 404 pageがある
- [ ] Secret、個人path、local専用資料、再配布不可資料を公開しない
- [ ] 公開siteの相対link checkが成功する
- [ ] HTTPSの公開URLをREADMEへ記載した
- [ ] Pages workflowと公開結果をread-back確認した

---

## #27: Wikiを公開文書の入口として整備

- Milestone: M0 Development Foundation
- Labels: `area:docs`、`type:maintenance`、`priority:normal`、`status:blocked`
- 起票時の状態: [#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)の方針確定済み、[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)のPages公開待ち
- 依存関係: [#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)、[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)

### 目的

Wikiを仕様の正本にせず、DeskCatの公開文書へ案内する日本語の入口ページとして整備する。

### 対象範囲

- 既定の英語`Home.md`を日本語の入口ページへ置き換える
- GitHub Pages、repository README、主要文書、Issuesへのlink
- `docs/`が正本であることの明記
- Wikiへ置いてよい情報と置かない情報の短い案内

### 対象外

- 技術仕様、ADR、runbookのWikiへの複製
- Live status、Issue checklist、release noteの二重管理
- 独自の長文Wiki content
- GitHub Pagesの実装

### 参考資料

- [About wikis](https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis)

### 受け入れ条件

- [ ] 既定の英語Homeを日本語の入口ページへ置き換えた
- [ ] Pages、README、文書index、Issuesへnavigationできる
- [ ] `docs/`が正本でありWikiは案内用であると明記した
- [ ] 技術仕様、設計値、live statusをWikiへ複製していない
- [ ] Secret、個人path、local専用資料を含まない
- [ ] 公開linkをread-back確認した

---

## #5: Rust／ESP-IDF toolchainの検証と固定

- Milestone: M1 ESP32 Bring-up
- Labels: `area:firmware`、`type:experiment`、`priority:high`
- 起票時の状態: 公式情報の調査は完了。ESP32 Build profile端末での生成とbuildは未実施
- 依存関係: ADR-0001、Governance

### 準備済みの調査

- [ESP32 Rust Toolchain](../toolchains/esp32-rust-toolchain.md)
- [Machine Profiles](../toolchains/machine-profiles.md)
- [ESP32開発端末Setup](../runbooks/esp32-development-machine-setup.md)
- [ADR-0002](../decisions/0002-role-based-development-environments.md)

記載したversionは、clean buildを記録するまで候補である。文書端末には開発toolを導入していない。

### 目的

現在の公式Rust on ESP文書とESP-IDF文書を使用して、ESP32-DevKitC-32Eが相互対応するtoolchainを選択する。

### 対象範囲

- Rust toolchain
- ESP-IDF version
- ESP32 target
- Project template
- 必要なhost tool
- 再現可能なversion record

### 対象外

- LCD、sensor、servo driver
- GPIO割り当て
- Protocol実装

### 受け入れ条件

- [ ] 正確なboard targetを確認した
- [x] 公式の互換性情報源をリンクした
- [ ] Versionを固定した
- [ ] 最小projectをclean buildできる
- [ ] Setup commandを文書化した
- [ ] Root READMEと`AGENTS.md`へ検証済みcommandを反映した
- [x] 未reviewのハードウェア値を導入していない

### 証拠

Tool version、完全なbuild log、生成設定、正確な公式文書link。

---

## #6: 最小firmwareをflashしてbootを記録

- Milestone: M1 ESP32 Bring-up
- Labels: `area:firmware`、`type:feature`、`priority:high`、`needs:hardware-test`
- 依存関係: [#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)、HW-TBD-001

### 目的

未検証のperipheralを初期化しない安全な最小firmwareをflashし、再現可能な起動出力を取得する。

### 受け入れ条件

- [ ] Flash commandが文書化され、再現可能である
- [ ] Firmware build identityを出力する
- [ ] Board-configuration IDを出力する
- [ ] Reset reasonを出力する
- [ ] 電源再投入後の起動を再現できる
- [ ] Servoまたは未知のoutputをdriveしない
- [ ] Debug／releaseのsize reportを記録した

---

## #7: Task heartbeatとhealth snapshotを追加

- Milestone: M1 ESP32 Bring-up
- Labels: `area:firmware`、`type:feature`、`priority:normal`
- 依存関係: [#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)

### 目的

主要firmware taskが進行し続けていることを、上限のある方法で確認できるようにする。

### 受け入れ条件

- [ ] Heartbeatにrate limitがある
- [ ] Uptimeが単調増加する
- [ ] Reset reasonを引き続き取得できる
- [ ] Loggingがwatchdogの進行をblockしない
- [ ] Counter schemaをprotocol statusへ使用できる

---

## #8: Raspberry Pi Rust環境の検証

- Milestone: M1 ESP32 Bring-up
- Labels: `area:raspberry-pi`、`type:experiment`、`priority:high`
- 依存関係: ADR-0001

調査済み候補と実機checklistは、[Raspberry Pi Rust Toolchain](../toolchains/raspberry-pi-rust-toolchain.md)と[Raspberry Pi開発端末Setup](../runbooks/raspberry-pi-development-machine-setup.md)に準備済みである。Pi Zero WH実機での検証は未実施である。

### 目的

Raspberry Pi Zero WH実機で対応するRust環境を検証し、direct build性能を記録する。

### 受け入れ条件

- [ ] Pi OSとarchitectureを記録した
- [ ] 対応するRust toolchainを固定した
- [ ] Pi上で最小Rust programをbuild・実行できる
- [ ] Build時間とstorage使用量を記録した
- [ ] Direct buildが実用的でない場合を除きcross compilationを保留した
- [ ] Setupにproject secretが含まれていない

---

## #9: Protocol crateとconformance fixtureの作成

- Milestone: M2 ESP32–Pi Protocol
- Labels: `area:protocol`、`area:raspberry-pi`、`type:feature`、`priority:high`
- 依存関係: [#4](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/4)、cargo workspace root（Issue化しない。冒頭の注記を参照）

### 目的

Serial I/Oを含めず、純粋なRustによるprotocol type、validation、共有JSON fixtureを実装する。

### 受け入れ条件

- [ ] Envelopeと最小message typeを型で表現した
- [ ] Integerとstringの上限が明示されている
- [ ] 有効なfixtureをround-tripできる
- [ ] 無効なfixtureが分類済みerrorで失敗する
- [ ] 最大値と最大値超過のcaseがある
- [ ] 未知のversion／typeに対する動作が仕様と一致する
- [ ] ハードウェア依存を導入していない

---

## #10: 上限付きfirmware line parserの実装

- Milestone: M2 ESP32–Pi Protocol
- Labels: `area:protocol`、`area:firmware`、`type:feature`、`priority:high`
- 依存関係: [#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)、[#9](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/9)

### 目的

Firmware向けに、上限のあるincremental JSON Lines receiverを実装する。

### 受け入れ条件

- [ ] 任意位置で分割したmessageをparseできる
- [ ] 1回のreadに含まれる複数lineをparseできる
- [ ] Invalid UTF-8とJSONを分類できる
- [ ] Oversize入力を次の改行まで破棄する
- [ ] 次の有効なlineでparseを再開する
- [ ] Allocationとqueue動作に上限がある
- [ ] Targetが対応する範囲で共有fixtureに合格する

---

## #11: Host serial sessionの実装

- Milestone: M2 ESP32–Pi Protocol
- Labels: `area:protocol`、`area:raspberry-pi`、`type:feature`、`priority:high`
- 依存関係: [#8](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/8)、[#9](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/9)

### 目的

設定したserial deviceをopenし、上限のあるmessageをread／writeし、domain動作を含めずconnection stateを公開する。

### 受け入れ条件

- [ ] Portとbaudをsecretではなく設定として扱う
- [ ] Readerとwriterがpartial I/Oを処理する
- [ ] Disconnectを観測できる
- [ ] Reconnectに上限とrate limitがある
- [ ] 上限のないmessage queueが存在しない
- [ ] Serial simulator testで一般的なfailureを網羅する

---

## #12: Boot、ping、status、ACK、reconnect同期の実装

- Milestone: M2 ESP32–Pi Protocol
- Labels: `area:protocol`、`area:firmware`、`area:raspberry-pi`、`type:feature`、`priority:high`、`needs:hardware-test`
- 依存関係: [#10](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/10)、[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)

### 目的

最小限の2-device sessionとrecovery動作を完成させる。

### 受け入れ条件

- [ ] ESP32が`boot`を送信する
- [ ] Piがsession追跡をresetする
- [ ] `ping`にACKが返る
- [ ] `get_status`にACKとstatusが返る
- [ ] Retryで同じIDを使用する
- [ ] Duplicate commandが2回実行されない
- [ ] Reconnect時に現在状態を同期する
- [ ] 古いrelative motionを再実行できない

---

## #13: LCDの特定とbring-up

- Milestone: M3 Display and Input
- Labels: `area:hardware`、`area:firmware`、`type:feature`、`priority:high`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)

### 目的

正確なLCD module上に、検証可能な色と座標patternを描画する。

### 受け入れ条件

- [ ] Controller識別情報と初期化の根拠を記録した
- [ ] 単色fillが正しい
- [ ] Color orderが正しい
- [ ] 四隅とorientationが正しい
- [ ] 更新timingを測定した
- [ ] 更新中も通信とwatchdogがactiveである

---

## #14: Touch入力の特定とbring-up

- Milestone: M3 Display and Input
- Labels: `area:hardware`、`area:firmware`、`type:feature`、`priority:high`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)

### 目的

Raw touch dataを取得し、再現可能な撫でeventを導出する。

### 受け入れ条件

- [ ] 正確なcontrollerとbus設定を記録した
- [ ] Idle時と操作時のraw sampleを保存した
- [ ] 該当する場合は座標変換をcalibrationした
- [ ] Debounce／state-machine動作をtestした
- [ ] LCD通信による許容できないdata lossがない
- [ ] False positiveと見逃しを測定した

---

## #15: Accelerometerの特定とbring-up

- Milestone: M3 Display and Input
- Labels: `area:hardware`、`area:firmware`、`type:feature`、`priority:high`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)

### 目的

Calibration済みaccelerationを取得し、再現可能な軽打eventを導出する。

### 受け入れ条件

- [ ] Device IDと正確な変換を検証した
- [ ] 静止姿勢のsampleを保存した
- [ ] Offsetとnoiseを測定した
- [ ] 机とサーボの振動sampleを保存した
- [ ] Tapしきい値とretrigger動作を測定した
- [ ] False positiveと見逃しを記録した

---

## #16: Environment sensorの特定とbring-up

- Milestone: M3 Display and Input
- Labels: `area:hardware`、`area:firmware`、`type:feature`、`priority:normal`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#2](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/2)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)、[#6](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/6)

### 目的

対応する環境測定量をfreshnessと分類済みfault付きで取得する。

### 受け入れ条件

- [ ] Device識別情報とcalibration処理を検証した
- [ ] Raw値と変換値を観測できる
- [ ] 値をreferenceと比較した
- [ ] 該当するtimeout、CRC、not-ready、disconnect動作を分類した
- [ ] 遅い変換処理が上位taskをblockしない

---

## #17: サーボ電気動作のcharacterization

- Milestone: M4 Servo Integration
- Labels: `area:hardware`、`type:experiment`、`priority:critical`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)、[#3](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/3)

### 目的

危険な機械負荷を接続せず、サーボPWMと電流動作を検証する。

### 受け入れ条件

- [ ] 正確なservo dataを記録した
- [ ] 接続前にPWMを測定した
- [ ] 外部電源と電流制限を記録した
- [ ] 起動時と小動作時の電流を測定した
- [ ] 5 Vと3.3 Vのtransientを取得した
- [ ] 受け入れ試験中のESP32 reset回数が0である

---

## #18: 機械的可動域と動作制限のcalibration

- Milestone: M4 Servo Integration
- Labels: `area:hardware`、`type:experiment`、`priority:critical`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、首機構の完成

### 目的

Neutral、安全可動域、速度、加速度、通信断時動作を確定する。

### 受け入れ条件

- [ ] 衝突させず低速で物理的制限を確認した
- [ ] Software limitに明示的なmarginがある
- [ ] Neutralを再現できる
- [ ] 速度と加速度制限を測定した
- [ ] PWM-off時の重力による動作が明確である
- [ ] Fail-safe sequenceを選定した
- [ ] `servo-safety-limits.md`を承認した

---

## #19: Servo limiterとtrajectoryの実装

- Milestone: M4 Servo Integration
- Labels: `area:firmware`、`type:feature`、`priority:critical`、`status:blocked`
- 依存関係: [#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)

### 目的

決定論的なhard limitと上限のあるmotion trajectoryを実装する。

### 受け入れ条件

- [ ] 不正なmotionをrejectする
- [ ] Position、velocity、accelerationがhard boundを超えない
- [ ] Debug commandが同じlimiterを使用する
- [ ] Duplicate commandでmotionが二重実行されない
- [ ] Clamp／rejection counterを観測できる
- [ ] 純粋なtrajectory testが境界を網羅する
- [ ] 受け入れる実動作が監視下試験に合格する

---

## #20: Servo fail-safeと統合動作の検証

- Milestone: M4 Servo Integration
- Labels: `area:firmware`、`area:hardware`、`type:feature`、`priority:critical`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)、[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)、[#19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)

### 目的

Disconnect、reset、display／sensor同時動作時の安全な動作を検証する。

### 受け入れ条件

- [ ] 通信断時に承認済みsequenceへ従う
- [ ] Reconnect時にmotionを再実行しない
- [ ] Watchdog／reset時のoutput stateが安全である
- [ ] LCD、sensor、serialの動作が受け入れ範囲内で継続する
- [ ] Brownoutが発生しない
- [ ] 緊急電源遮断が機能する

---

## #21: 撫で操作から喜ぶ反応までの統合

- Milestone: M5 DeskCat MVP
- Labels: `area:firmware`、`area:raspberry-pi`、`type:feature`、`priority:high`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)、[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)、[#14](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/14)、[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)

### 受け入れ条件

- [ ] 検証済みの一つの撫でeventがPiへ到達する
- [ ] Piが感情stateを更新する
- [ ] Piが受け入れ可能なexpression／motion commandを送信する
- [ ] ESP32が安全範囲内で実行する
- [ ] Duplicateによってmotionが反復しない
- [ ] End-to-end latencyを測定した

---

## #22: 軽打から驚く反応までの統合

- Milestone: M5 DeskCat MVP
- Labels: `area:firmware`、`area:raspberry-pi`、`type:feature`、`priority:high`、`status:blocked`、`needs:hardware-test`
- 依存関係: [#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)、[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)、[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)、[#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20)

### 受け入れ条件

- [ ] 検証済みの一つのtap eventがPiへ到達する
- [ ] Piが驚く反応を選択する
- [ ] Expressionと任意のmotionが制限内に収まる
- [ ] サーボ自身の振動でeventを繰り返しtriggerしない
- [ ] End-to-end latencyとfalse eventを測定した

---

## #23: ローカル独り言fallbackの追加

- Milestone: M5 DeskCat MVP
- Labels: `area:raspberry-pi`、`type:feature`、`priority:normal`
- 依存関係: cargo workspace root（Issue化しない。冒頭の注記を参照）、[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)、display text機能

### 受け入れ条件

- [ ] Cloud accessなしでローカル発話文が動作する
- [ ] Output byteとdurationの制限がprotocolと一致する
- [ ] 発話rateに上限がある
- [ ] 設定にsecretが含まれていない
- [ ] 将来cloud連携を追加した場合、そのfailure時にローカルへfallbackする

---

## #24: MVP fault／endurance suiteの実行

- Milestone: M6 Reliability
- Labels: `area:firmware`、`area:raspberry-pi`、`area:hardware`、`type:experiment`、`priority:high`、`needs:hardware-test`
- 依存関係: [#21](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/21)、[#22](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/22)、[#23](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23)

### 目的

長時間動作とfault injectionの条件下で、MVP全体を検証する。

### 受け入れ条件

- [ ] 目標動作時間を定義し、試験に合格する
- [ ] Pi restartとESP32 restartから復旧する
- [ ] USB disconnect／reconnectから復旧する
- [ ] Sensor disconnectの影響を分離できる
- [ ] 不正・高rateのprotocol入力を上限内で処理する
- [ ] Servo動作でbrownoutが発生しない
- [ ] Memory、queue、parser、bus、sensor、reset counterを記録した
- [ ] 既知の制限とrecovery runbookを更新した

---

## GitHub作成checklist

- [x] GitHub CLI認証を復旧する
- [x] Issue作成前にlabelを適用する
- [x] M0–M6 milestoneを作成する
- [x] 依存順にIssueを作成する
- [x] ローカルのsymbolic IDをGitHub Issue linkへ置き換える
- [x] ハードウェアTBDにより着手できないIssueへ`status:blocked`を設定する
- [x] 基盤review後の最初のAI主導実装Issueとして#5を選定する
- [x] 最初の人間主導ハードウェアIssueとして#1を選定する
- [x] 移行後は、このfileでlive statusを重複管理しない
