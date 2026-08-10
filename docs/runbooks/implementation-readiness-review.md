# Implementation Readiness Review

> Review日: 2026-07-27
> 結果: Hardware driver実装のgateは未通過

## 結論

Repository基盤は、管理された開発作業を開始できる状態にある。正確なmodule、GPIO割り当て、電源設計、サーボ制限が不明なため、ハードウェア固有firmwareはまだ着手できない。

公式資料に基づくtoolchain調査は完了している。適切な端末では、次の二つの作業streamを安全に開始できる。

1. 人間が主導するハードウェア現物inventory: #1
2. ESP32 Build profile端末でのRust／ESP-IDF生成とclean build: #5

最初のperipheral driverまたは統合通電までは、これらを独立して進められる。

## Repository gate

| 確認項目 | 結果 | 根拠 |
|---|---|---|
| Commit範囲を分類済み | Pass | `repository-safety-baseline.md` |
| `.env`がignore対象で履歴に存在しない | Pass | `repository-safety-baseline.md` |
| 一時参考fileを削除済み | Pass | Repository検証 |
| Governanceが有効 | Pass | `AGENTS.md`、`docs/governance/` |
| Repository境界を決定済み | Pass | ADR-0001 |
| READMEから必要な正本へlink済み | Pass | Root `README.md` |

## Hardware gate

| 確認項目 | 結果 | 次に必要な根拠 |
|---|---|---|
| 正確なESP32 board revision | Fail／TBD | Board現物確認 |
| 正確なLCDとtouch | Fail／TBD | 現物表示と公式文書 |
| 正確なaccelerometerとenvironment sensor | Fail／TBD | 現物表示と公式文書 |
| 正確なservoと電流 | Fail／TBD | 現物表示、データシート、測定 |
| GPIO割り当て | Fail／TBD | 部品inventoryの完成と回路図review |
| 電源予算 | Fail／TBD | 正確な負荷、電源、配線、測定 |
| サーボ安全制限 | Fail／TBD | 電気的・機械的calibration |

Peripheral driverまたはservo出力は、いずれもこのgateを通過していない。

## Software gate

| 確認項目 | 結果 | 次に必要な作業 |
|---|---|---|
| Rustを主要言語とする | Pass | 既存project decision |
| ESP32 workspaceを分離する | ArchitectureとしてPass | ADR-0001 |
| 公式情報による互換性調査 | Pass | `docs/toolchains/esp32-rust-toolchain.md` |
| 互換性のあるRust／ESP-IDF version | Candidate／未検証 | 開発端末での#5 clean build |
| 再現可能なbuild／flash／monitor command | Fail／TBD | #5と#6 |
| Draft protocolが存在する | Review用としてPass | #4 |
| 承認済みprotocol制限とfixture | Partial | #9でschema群のfixtureとhost実装の合格を確認済み。session判定・duplicate replay・budgetの3群は#12、firmware側の合格は#10 |
| Host／firmware workspaceが存在する | Pass | firmware workspaceは#5／#40、host workspaceは#9で作成した |

Toolchainのbuild-only spikeはperipheral pinの選定なしで進められる。Docs / Review端末にtoolchainを導入する必要はない。

## Project management gate

| 確認項目 | 結果 | 次に必要な作業 |
|---|---|---|
| Issue／PR template | ローカルでPass | Repository更新後に公開 |
| Labelとmilestone | Remoteへ適用済み | 2026-07-28検証済み |
| 初期Issue定義 | ローカルでPass | `docs/backlog/initial-issues.md` |
| GitHub Issue作成 | Pending | 基盤文書を公開後、依存順に作成 |
| Private vulnerability reporting | Enabled | 2026-07-28検証済み |
| Branch protection | 最小保護を適用済み | Force pushと削除を無効化。PR／status要件は保留 |

## 承認済みの次作業

### #1: ハードウェア現物inventoryの確認

現物部品から次が必要である。

- 各module両面の鮮明な表示
- Connector labelとpin headerの方向
- Servo label
- 電源label
- ESP32 moduleとboard revision
- Raspberry PiとmicroSDの識別情報

次を更新する。

- `docs/hardware/hardware-bom.md`
- `docs/hardware/sensor-datasheet-notes.md`
- `docs/hardware/tbd-register.md`

### #5: Rust／ESP-IDF toolchainの検証と固定

完了済みの調査:

- 現在の公式Rust on ESPとtemplateのreview
- 候補target、ESP-IDF version、生成時dependency baseline
- Role-based machine policyとversion record
- Draft setup手順

残っている検証:

- ESP32 board現物の識別情報を確認する
- ESP32 Build profile端末でsetupを実行する
- Review済みtemplate commitから生成する
- Clean buildとdependency lockfileを記録する

許可する範囲:

- 現在の公式文書の調査
- Toolchain互換性の判断
- 最小firmware project
- Build-only検証
- Setupとversionの文書化

このIssueで禁止すること:

- Peripheral GPIO
- LCD／sensor driver
- Servo PWM
- 推測したハードウェア値

### GitHub設定の適用

2026-07-28に完了した。Repositoryへtokenを保存せずGitHub認証を復旧した。Label、milestone、private vulnerability reporting、`main`の最小保護を適用し、read-back確認した。

## Gate判断

```text
Foundation work: READY
Toolchain spike: READY
Physical inventory: READY for human inspection
Protocol document review: READY
Hardware drivers: NOT READY
Servo output: NOT READY
Full implementation: NOT READY
Remote GitHub configuration: BASE SETTINGS APPLIED
GitHub Issue migration: PENDING foundation document publication
```

上のgateが`Fail`／`TBD`／`Partial`としているIssueすべてに根拠が揃った後、このreviewを再実行する。
対象は#1、#2、#3、#5（開発端末検証の残り）、#6、#10、#12である。

`#6`（再現可能なbuild／flash／monitor command）を落とさない。Software gateがこれをblockerに
挙げており、揃わないまま再実行しても同じ`Fail`を繰り返すだけになる。

「承認済みprotocol制限とfixture」は#9で`Partial`まで進んだ。**残りをblockerとして扱うのは
#9ではなく#10と#12である。**#9はschema群のfixtureとhost実装の合格までを担当し、
firmware側の合格は#10、session判定・duplicate replay・budget群のfixtureは#12が担う。
再実行時に#9を対象へ戻さない。
