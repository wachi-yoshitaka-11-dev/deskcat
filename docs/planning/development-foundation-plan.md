# DeskCat 開発基盤整備計画

> 状態: Active
> 対象範囲: 本格的なファームウェア実装へ入る前の基盤整備
> 作成日: 2026-07-27

## 1. この計画の目的

この計画では、DeskCat の開発を AI エージェントと安全かつ継続的に進めるため、次の状態まで整備する。

- Git 管理・GitHub 公開対象が安全に整理されている
- AI エージェントの権限、判断基準、作業手順が明文化されている
- ESP32、Raspberry Pi、共通プロトコルを配置できるリポジトリ構成がある
- ハードウェア仕様を推測せず管理する資料がある
- README、Issue、Pull Request、ラベル、マイルストーンの運用基盤がある
- 最初の実装 Issue に、安全な受け入れ条件と依存関係が定義されている

この計画の完了は「DeskCat の機能実装完了」ではなく、「実装を安全に開始できる状態」を意味する。

## 現在の進捗

次の表は基盤整備時の記録であり、現在のMVP実行順は下の
「[次のMVP成果と依存（2026-09-27照合）](#次のmvp成果と依存2026-09-27照合)」で扱う。

| Phase | 状態 | 証拠／残作業 |
|---|---|---|
| 0: リポジトリ安全確認 | 完了 | `docs/runbooks/repository-safety-baseline.md` |
| 1: AI エージェント基盤 | 完了 | `AGENTS.md`、`docs/governance/` |
| 2: リポジトリ構成 | 完了 | ADR-0001、責務 README |
| 3: ハードウェア基準資料 | 進行中 | 資料作成済み。実部品の型番・回路・実測待ち |
| 4: README／GitHub 基盤 | 完了 | ラベル、マイルストーン、脆弱性報告、`main`最小保護を適用済み。文書言語を日本語中心に統一し、commit `19853d0`で`origin/main`へ公開済み |
| 4A: 公開ドキュメント基盤 | 完了 | [ADR-0003](../decisions/0003-public-documentation-publishing.md)に従い、[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)のPagesと[#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)のWiki入口を公開し、read-backを確認済み |
| 5: 初期バックログ | 完了 | 初期Issue 24件を[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)〜[#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24)として作成し、GitHubへlive statusを移行済み |
| 6: 実装開始ゲート | レビュー済み・未通過 | `docs/runbooks/implementation-readiness-review.md` |
| #5の作業区分a: 開発環境の情報整理 | 完了 | `docs/toolchains/`、役割別 setup runbook、ADR-0002 |
| #5の作業区分b: 開発端末での検証 | 未着手 | ESP32 Build profile 端末での生成・clean build 待ち |

## 次のMVP成果と依存（2026-09-27照合）

[#464](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/464)の計画整理。
基点は`develop`の`bb5cfc290d996f3f773bd69fae3f97b491a1314f`。
Issue本文・コメントは2026-09-27 JSTに再取得した。以後の実施時は最新baseと照合する。
この節は実験結果でも通電承認でもない。安全値・gateの正は既存のhardware文書に置く。

### 次に観測する成果

**[#13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13)で、実LCDの識別、単色fill、色順、
四隅・orientation、SPI転送時間を記録する。**既存受け入れ条件1〜5に対応する一つの実機sessionを
次の製品作業とする。成功時は「panelに何が表示されたか」を写真とlogで示す。
失敗時は最初の失敗段階・log・停止理由を記録し、成功や条件達成へ読み替えない。

- 実施者はIssue assigneeの`wachi-yoshitaka-11-dev`（現物操作・開始／停止判断）と、
  実機LinuxのESP32 Flash / HIL担当（build・log取得）。AIは準備と記録整理を担当する。
  Windows／VM／containerからflash・serial monitorを行わない。
- 手順は[power-budgetのDISP-01追加接続bring-up](../hardware/power-budget.md)の同名節。
  `EXP-015`のACCEL-01／ENV-01接続済み構成へLCDを追加し、PC USBでESP32へ給電する範囲である。
  実際の配線がその構成と同じかは現場で確認する。Pi接続、servo、B-2合算測定を混ぜない。
- 開始には同節の条件(1)〜(7)と手順を満たす。特に非通電check、対象featureのbuild記録、
  既存配線・負荷・環境条件、待機上限と遮断準備が残る。
  条件(5)の独立性確認・bus容量等が未達なら、その節の範囲で人間が現場判断を記録する。
  `EXP-015`でのPM判断を新しい配線への人間承認として流用しない。
- 条件(7)の実行承認は[#13の2026-09-24記録](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13#issuecomment-5804046159)
  にある。対象構成と許可範囲を照合する。条件(1)〜(6)や当日の開始判断まで承認済みとはしない。
- 完了証拠は`display_madctl`（firmwareはcontrollerのIDを読まない。`EXP-018`）、5色の`display_fill`、`display_corner_pattern`、`elapsed_us`、
  panel写真、既存sensorのID log、配線revision・firmware commit／feature・環境・停止の有無。
  [実験記録](../hardware/experiment-log.md)へ保存する。`elapsed_us`はSPI転送までの時間であり、
  panel応答時間ではない。起動直後の旧imageと書き込み後の結果を区別する。
- このsessionは**2026-09-30まで**の実施枠とした（**この枠は過ぎている。**当時の記録であり、現在の状態は#13を参照する）。ユーザーが2026-09-27に実施可能と回答した。
  当日の開始判断と成功を確約したものではない。
  条件が揃わなければ#13へ不足と再予定を残す。#13全体の完了日と混同しない。

**#13の条件6（描画更新中の通信とwatchdog）は未達のまま残す。**
#487で`bringup-display-13`のbuildも描画の間にPi linkの受信とheartbeatを回すようになったが、実機での確認はまだ無い。
[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)の実sessionと、#13自身が持つ
同時動作の実機確認が要る（同時動作のfirmware変更は#487で入れた。入れたのは描画の間のPi linkの受信（`boot`→ACKと、`hello`／`ping`／`get_status`への応答）とheartbeatである）。これらの完了を上の条件1〜5の開始へ逆流させない。

### 依存図の変更と各辺の根拠

変更前のIssue番号だけの読み方では、`#3完了 → #13〜#16完了 → B-2 → #405 → #3完了`が循環する。
さらに#13完了に#446実機sessionを要求すると、案Aの構成では段階C・#247を通って#405へ戻る。別電源＋UARTの構成（#446）では通らない。
前半の設計／単体成果と、後半の実機統合の完了条件を分ける。

```text
#461の通常接続判断 + #13の当日条件 → #13条件1〜5（次の成果）
#445の2点限定構成                 → #15/#16の単体sampling・変換

B-2の経路別条件（未解決）→ 3点定常電流 + 電源/pin/単体動作の証拠
                         → #405非通電実装・star point確認
                         → #3の4文書照合・残る設計判断・人間承認
                         → 段階Cの全条件（案Aを最終の給電に採る場合の再試験は、#405の見直しの候補として引き継いだ）
#446 PR C（PC側準備） ─────────────────┬→ #446実機boot/ACK
                                      └→ #11/#12実機通信
#446実session + #13同時動作実装 ─────────→ #13条件6

#17の初回駆動方法の人間判断 + 電源/servo gate → #17実測
#17実測 + #34機構 → #18校正 → #19実機limiter → #20統合安全
#12/#13/#14 + #20 → #21撫で反応
#12/#13/#15 + #20 → #22軽打反応
#23文字表示fallback + #21/#22 → #24 fault/endurance
```

矢印は以下の条件付き依存であり、図だけでgateを開かない。#17の校正と有効化条件の循環は
**未解決として残す**。図の「人間判断」は承認済みを意味しない。

| 辺・扱い | 分類 | 根拠と開始／完了の区別 |
|---|---|---|
| #3全close → 単体bring-upを外す | 管理上の順序 | [power-budget](../hardware/power-budget.md)「合成給電を部品が揃うまで行わない理由」「単体bring-upとB-2の区別」。PC給電の限定構成は段階Cを待たない。#445/#461の範囲を適用し、他経路へ広げない |
| #461 → LCD通常接続 | 物理的安全条件に関する限定判断 | [TBD台帳](../hardware/tbd-register.md) HW-TBD-024とPR #462。上限値の解明・下流moduleの保護の証明ではない。B-2/B-2bのgateは残る |
| B-2の条件 → 3点電流測定 | 物理的安全条件 | power-budget「段階B-2の測定」、HW-TBD-024/025。MSP2807の現物回路確認は未達。B-2bの電源・設定上限等も未充足。通常接続の許可では測定を開始しない |
| B-2の電流・電源/pin/単体動作の証拠 → #405 | 設計の確定・成果物の利用、実装順は管理判断 | #405「消費電流が分かってから実装する」。全#13〜#16のcloseではない。#13条件6や#15のservo振動sampleは非通電電源実装の入力でなく、逆に下流の段階Cを待つ。必要な単体確認と電流測定は削除しない |
| #405 → #3の最終照合 | 成果物の利用 | HW-TBD-034(5)(7)と#3条件9。star pointの電源off確認は#405、証拠を使う4文書照合は#3。#3の机上作業は先行できるが、完了は実配線の証拠を待つ |
| #3の定義・承認 + #405等 → 段階C | 設計の確定・物理的安全条件 | power-budget「経路部品と定格」「過電流保護（段階Cのgate）」。margin・受け入れ数値・測定系・実装の全条件が必要。#405 closeだけでも#3 closeだけでも通電許可にならない |
| 段階C → 案A再試験（案Aを最終の給電に採る場合のみ。#405の見直しの候補として引き継いだ） | 物理的安全条件・成果物の利用 | power-budget「ESP32の給電経路」、#247、[Safety Policy §11](../governance/hardware-safety-policy.md#11-緊急時の対応)。安全な再開条件と観測結果が必要。#247という番号のcloseだけでは判定しない。host実装・PC testは先行可能。#446/#11/#12の実機通信は別電源＋UARTの構成（`gpio-assignment.md`の`Pi–ESP32間のtransport`節）で行い、この辺を通らない |
| #446 → #13条件6 | 成果物の利用 | #13条件6、[protocol §4.1](../protocol/esp32-pi-protocol.md)、firmwareの統合（#487。feature排他は廃止した）。PR #476/#480はmerge済み。暫定sid・送信1回だけという旧説明は更新し、PR Cと実機・同時動作を残す |
| servo gate ↔ 初回校正 | 未解決の安全・設計依存 | [servo-safety-limits「承認の状態」](../hardware/servo-safety-limits.md#承認の状態)、#17/#474。現在は「実行しない」。HW-TBD-010の測定に出力が要る一方、出力gateは010の解決を要求する。#405追加だけで解けず、人間が最初の駆動方法を判断する |

### 既存Issueの開始条件と完了条件

対象は2026-09-27時点で#461以外すべてopen。以前のblocked 10件もすべて再確認した。
`In Progress`は部分作業が進行中という意味であり、blockedの解消や実機成功を意味しない。
以下は既存受け入れ条件への道筋であり、条件を削除する表ではない。

| Issue | 今からできること／必要環境 | 完了を阻むもの・解除証拠 |
|---|---|---|
| #3 | Docs / Reviewでmargin・数値制限・4文書照合の準備 | HW-TBD-007/009/021/022/028/034等の該当残件、#405の実配線証拠、最終人間承認。机上だけでcloseはできない |
| #405 | Docs / Reviewで配置検討。物理実装は電流・単体確認後に人間 | B-2未実施。非通電の実装・向き・導通・絶縁・star pointの記録が完了証拠。通電は対象外 |
| #11 | Host Rustで既存実装を利用 | 実機Pi/Linuxでread/write・切断・再接続。#446（別電源＋UART）を利用。PC testを実port証拠にしない |
| #12 | Host Rust/ESP32 Buildでboot/ping/status/retry/重複処理の残実装 | #446・安全な実port、motion関連は#19。古いrelative motion非再実行を対象未実装のまま合格にしない |
| #13 | 上記LCD session（条件1〜5） | 条件6は#446と同時動作変更・実機証拠。blockedはこの理由で維持 |
| #14 | firmware担当が既存LCD/controller/bus仕様から実装準備 | #13のLCD/SPI単体証拠を利用し、touch配線・電圧・bus条件を別途確認。raw sample・debounce・LCD併用loss・誤検出を実機測定。#13条件6のcloseは準備の前提にしない |
| #15 | #445構成でsampling・変換・静止noiseの実装／実機確認 | EXP-015はID応答のみ。servo振動sampleは#17以降の安全な動作を待つ。これを単体samplingの開始条件にしない |
| #16 | #445構成でraw/変換/calibration/faultの実装／実機確認 | EXP-014のJ3は申告、EXP-015はID応答。reference比較・timeout等の証拠が必要。J3記述の整合は#16担当、#13を待たない |
| #17 | 人間とDocs / Reviewで初回駆動方法を判断 | 最新#474はbench実行禁止。servo識別・電源・全出力gateと校正循環の解消が必要。実測はその後の実機Linux/HIL |
| #18 | #34の機構案検討と測定準備 | #17の電気的根拠・#34の固定方法、安全な校正、可動域/速度/加速度/fail-safeの人間承認 |
| #19 | Host Rustで純粋limiter/trajectoryの境界test | #18の実測制限値と監視下試験。仮値testの成功を実動作の合格にしない |
| #20 | fault scenarioの準備 | #12/#13/#19と出力gate。実機で通信断/reset/同時動作/電源遮断の証拠 |
| #21 | Host Rustでeventから感情stateの純粋logicを準備 | #12/#13/#14/#20の成果。Piと実機で撫で→表情/motionとlatencyを確認 |
| #22 | Host Rustでtapから反応の純粋logicを準備 | #12/#13/#15/#20の成果。servo自己振動の誤検出を含む実機反応・latency |
| #23 | Host Rustでローカル発話候補・rate制限の準備 | `show_text`のwire/描画接続とPROTO-TBD-007（layout/memory計測）が未実装／未確定。既存#23で扱い、実体のない「display text機能」のclose待ちにしない |
| #24 | 既存counter・fault/enduranceの試験設計 | #21/#22/#23の統合成果と安全条件、目標時間の人間判断。実機を壊しうるfault injectionは別途監視 |
| #34 | 人間が実装方式・仮筐体・首固定案を選ぶ | #1全close待ちのblockedを外し、needs:decisionへ。現物servo/hornの適合・干渉と固定方法は本Issueの完了条件として残る。#18の校正完了を機構設計の前提にしない |
| #247 | Docs / Reviewで記録を整理する。案Aを最終の給電に採る場合の再試験は、[#405の見直しの候補として引き継いだ](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/405#issuecomment-6006010028) | Pi停止の原因は、ユーザーが「本体表記なしのUSBケーブル」と決めた（[決定コメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/247#issuecomment-5863026168)。機序は未測定）。案Aを採る場合の再試験の条件は、段階C全条件と人間監視、観測系の復元、Safety Policy §11の安全な再開判断。この条件は、別電源＋UARTの通信検証には掛からない |
| #446 | PR A/Bの成果からhost example→handle_boot接続（PR C）を準備 | 実機boot→ACKは未達。PR Cが要る。自己レビューは既に15巡で、PR Cの巡16開始前に有限の人間承認が必要 |
| #461 | close済みの通常接続判断を上記の限定構成に利用 | B-2/B-2b、段階C、servo gateの解除や実機成功を意味しない |

### 実装順・人間へ残す判断

1. 次の実機枠は#13条件1〜5へ使う。#15/#16の単体samplingは独立に実装準備できるが、
   同じ`main.rs`や共通I2C初期化への変更は担当を一人にして、#13の実験に使うcommitを固定する。
2. #446 PR B（#480）はmerge済み。PR Cはhost exampleを担当し、#12のcoordinator変更と重ねない。
   PR C統合後に#12の通信残件、次に#13条件6の同時動作変更へ進む。#451の3本merge待ちを再現しない。
3. #16がpower-budgetのJ3節を担当する。#13は既存追加接続手順を使い、#16の記述整理を待たない。
   #3/#405の電源設計・実装結果と#467の文書整理は、同じ節を並行編集せず、先に根拠を持つ変更を統合する。
   取得時open PRは0件、#467に着手コメントなし。未公開差分の不存在は保証しない。編集直前に再照合する。
4. B-2が現在の条件で実行できない問題は#3/#405で人間へ返す。通常接続の電流記録を代替にできるか、
   別構成にするかは安全条件の判断であり、#464では決めない。これを理由にLCD表示観測まで止めない。
5. #247のPi停止の原因は、ユーザーが「本体表記なしのUSBケーブル」と決めた（[決定コメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/247#issuecomment-5863026168)。機序は未測定）。
   本体表記なしのUSBケーブルは使わない。`E226000`を型番と断定しない。BOMへの識別情報の採用は現物照合後。
   この決定は段階Cの免除でも、案Aの再試験の結果でもない。#247の取り下げ済みTest 1/2を復活させない。
6. #17の最初の駆動方法、#34の機構、#23のlayout制限、各実機枠の日程は人間の判断を要する。
   #464は規則や電気的安全要件を変更せず、既存Issueへ判断と根拠を残す。

日付・Status・labelの変更前後、期限超過の扱い、自己レビュー通算、実行した検査と未実施範囲は
[#464](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/464)へ保存する。
計画の整理・Issueのclose数を製品進捗として数えず、上記の実機証拠を得た時点で結果を更新する。

## 2. 参照する基準資料

- [DeskCat マイコン開発技術ガイド](../DeskCat_Microcontroller_Development_Guide.md)
- 採用する各部品のメーカー公式データシート
- ESP32 開発ボードの公式回路図・ユーザーガイド
- 採用時点の Rust／ESP-IDF 公式資料

## 3. 運用ルール

- [x] フェーズを原則として番号順に進める
- [x] 各フェーズの完了条件を満たしてから次へ進む
- [x] 未確定値は推測で埋めず `TBD` とする
- [x] ハードウェア値にはデータシート、回路図、実測の根拠を付ける
- [x] 一つの Issue には一つの目的を設定する
- [x] 設計変更は ADR または決定ログへ記録する
- [x] 作業中に見つかった別問題は、原則として別 Issue に分ける
- [x] 各変更に対応する確認方法と結果を記録する
- [x] 公開、push、リリース、外部サービス変更は、計画作成とは別の操作として扱う

---

## Phase 0: リポジトリの安全確認

### 0.1 現状把握

- [x] `git status` で変更済み・未追跡ファイルを一覧化する
- [x] 各ファイルを「コミット対象」「ローカル専用」「要判断」に分類する
- [x] 既存のユーザー変更を特定し、基盤整備で上書きしない
- [x] 現在の既定ブランチとリモート設定を確認する
- [x] GitHub リポジトリの有無、所有者、公開範囲を確認する

### 0.2 秘密情報

- [x] `.env` が `.gitignore` の対象になっていることを確認する
- [x] `.env` が過去の Git 履歴に含まれていないことを確認する
- [x] API キー、トークン、パスワード、秘密鍵が管理対象に含まれていないことを確認する
- [x] ログ、設定例、ドキュメント内に秘密情報がないことを確認する
- [x] 基本的な秘密情報 marker scan と公開前の staged diff review を検査方法とする
- [x] サンプル設定は秘密を含まない例とし、必要な場合だけ `.env.example` を作る方針を決める

### 0.3 PDF・大容量ファイル

- [x] `DPMIF202608_Interface08release.pdf` は配布可否を確定せず、確認・情報抽出後に削除する
- [x] 配布可否が確認できない PDF はコミット対象外とする
- [x] ローカル参考資料の取扱いを README と repository safety baseline に記載する
- [x] その他の大容量バイナリがないことを確認する
- [x] 将来の画像、計測データ、ビルド成果物は Issue で出所、容量、保存場所を決める
- [x] 通常の Git review に適さない大きな versioned binary が必要な場合だけ Git LFS 等を検討する

### 0.4 公開・ライセンス

- [x] GitHub リポジトリは現在の public を維持する
- [x] 現在の `LICENSE` が意図した MIT License であると確認する
- [x] 外部ライブラリ、フォント、画像、回路資料は出所と再配布条件を公開前に確認する
- [x] 個人情報や内部パスを公開資料へ含めない規則を決める
- [x] 公開前に staged file list、diff、秘密情報 marker、ライセンスを確認する

### Phase 0 完了条件

- [x] コミットしてよいファイルが明確になっている
- [x] `.env` と秘密情報が Git 管理外である
- [x] PDF の扱いが決定済み、または安全側で保留されている
- [x] public／private とライセンスの方針が明確になっている
- [x] 意図しないファイルを `git add .` で追加しない状態になっている

---

## Phase 1: AI エージェント基盤設定

### 1.1 リポジトリ固有の Governance 文書

- [x] `docs/governance/README.md` を作成する
- [x] `docs/governance/ai-agent-policy.md` を作成する
- [x] `docs/governance/development-workflow.md` を作成する
- [x] `docs/governance/hardware-safety-policy.md` を作成する

- [x] プロジェクトの目的を記載する
- [x] MVP の対象機能を記載する
- [x] MVP に含めない機能を記載する
- [x] ESP32 と Raspberry Pi の責務を記載する
- [x] 人間、AI、実機確認の責任分界を記載する
- [x] 基準資料と情報源の優先順位を記載する
- [x] 資料間で矛盾した場合の解決方法を記載する
- [x] `確定`、`推奨`、`TBD` の扱いを記載する
- [x] GPIO、型番、しきい値、サーボ値を推測しない規則を記載する
- [x] 公式データシートを必須とする条件を記載する
- [x] 実測を必須とする条件を記載する

### 1.2 安全規則

- [x] サーボを ESP32 から給電しない規則を記載する
- [x] サーボ電源と ESP32 の GND 共通化を記載する
- [x] サーボ角度、速度、加速度の安全上限を迂回しない規則を記載する
- [x] 初回サーボ試験の安全手順を記載する
- [x] 回路変更、通電、書き込み、機械動作の確認手順を記載する
- [x] `unsafe` を原則禁止とし、例外承認条件を記載する
- [x] 秘密情報をログや AI 入力へ含めない規則を記載する
- [x] 削除、外部送信、公開、リリースの承認条件を記載する

### 1.3 開発ワークフロー

- [x] 一 Issue 一目的の規則を記載する
- [x] Issue 着手前に確認する資料を記載する
- [x] 変更可能範囲と変更禁止範囲の指定方法を記載する
- [x] 新規依存追加の判断・承認方法を記載する
- [x] 実装前に不明点を列挙する規則を記載する
- [x] 小さい変更単位を維持する規則を記載する
- [x] format、lint、test、build の実行方針を記載する
- [x] 実機確認が必要な変更の分類を記載する
- [x] 変更後に既存機能の回帰を確認する規則を記載する
- [x] 作業完了報告に含める内容を記載する

### 1.4 Git 運用

- [x] ブランチ命名方針を決める
- [x] コミット・メッセージ方針を決める
- [x] AI がコミットしてよい条件を決める
- [x] AI が push してよい条件を決める
- [x] force push を禁止または制限する
- [x] ユーザーの未コミット変更を保護する規則を記載する
- [x] 生成物とローカル設定をコミットしない規則を記載する

### 1.5 ルート `AGENTS.md`

- [x] Governance 文書から実行時必須ルールを抽出する
- [x] 作業開始時に読む資料を列挙する
- [x] リポジトリ内の正式な情報源を列挙する
- [x] 禁止事項を短く明確に記載する
- [x] 現時点で利用可能な検証範囲と未確定 command を記載する
- [x] 未確定のコマンドを捏造しないよう明記する
- [x] サブディレクトリ固有ルールの追加方法を記載する
- [x] 長文の背景説明は `docs/governance/` へリンクする

### 1.6 基盤設定レビュー

- [x] 人間だけが決める事項が明確である
- [x] AI が自律的に進めてよい事項が明確である
- [x] 危険操作の承認境界が明確である
- [x] 現在のツール・リポジトリ状態と矛盾していない
- [x] 将来のエージェントでも解釈できる具体性がある
- [x] 同じ規則が複数文書で食い違っていない

### Phase 1 完了条件

- [x] `docs/governance/` の永続方針がレビュー済みである
- [x] ルート `AGENTS.md` が存在する
- [x] AI の権限、安全規則、検証義務が明文化されている
- [x] 未確定値を推測して実装しない仕組みがある
- [x] 以後の AI 作業が同じ運用基準に従える

---

## Phase 2: リポジトリ・アーキテクチャの決定

### 2.1 ADR-0001 の作成

- [x] モノレポを採用するか決める
- [x] ESP32 ファームウェアの配置を決める
- [x] Raspberry Pi 側ソフトウェアの配置を決める
- [x] Pi 側の実装言語を Rust と決める
- [x] 共通プロトコル定義の配置を決める
- [ ] 共有型・スキーマの生成有無を #9 の互換性検証で決める
- [x] ハードウェア資料の配置を決める
- [ ] 実験ログと大容量計測データの正式な保存場所を最初の実験 Issue で決める
- [x] PC テストと HIL テストの配置を決める
- [x] 補助スクリプトの配置を決める
- [x] ESP32 と Pi のリリース単位を決める
- [x] firmware version と protocol major version の互換関係を決める

### 2.2 初期ディレクトリ案の確定

- [x] 次の構成案を採用し、`docs/toolchains/`とPages公開用`pages/`を追加する

```text
deskcat/
├─ AGENTS.md
├─ README.md
├─ docs/
│  ├─ architecture/
│  ├─ backlog/
│  ├─ decisions/
│  ├─ governance/
│  ├─ hardware/
│  ├─ protocol/
│  ├─ runbooks/
│  └─ toolchains/
├─ apps/
│  └─ deskcatd/
├─ crates/
├─ firmware/
│  └─ esp32/
├─ simulator/
│  └─ deskcat-sim/
├─ configs/
├─ deploy/
├─ hardware/
├─ pages/
├─ tests/
│  └─ hil/
├─ scripts/
└─ .github/
```

- [x] ディレクトリ名を英語、表記を一貫させる
- [x] 各ディレクトリの責務を一文で定義する
- [x] 同じ情報を複数箇所で重複管理しない
- [x] 既存資料は必要なものを現位置から参照し、一時資料は永続文書へ変換後に削除する
- [x] 文書を移動する場合は同じ変更で参照リンクを更新する

### 2.3 リポジトリ構成の作成

- [x] ADR-0001 に従ってディレクトリを作成する
- [x] Git が空ディレクトリを追跡しないことを考慮する
- [x] 必要なディレクトリには役割を説明する README を置く
- [x] ビルド成果物用ディレクトリを `.gitignore` へ追加する
- [x] IDE、OS の一時ファイルを `.gitignore` へ追加し、計測データは Issue ごとに追跡可否を決める
- [x] ローカル専用データと共有すべき実験データを区別する
- [x] ルートから主要資料へ辿れるようにする

### 2.4 ソース・オブ・トゥルース

- [x] プロジェクト要求の正式な定義元を決める
- [x] GPIO の正式な定義元を決める
- [x] 部品型番の正式な定義元を決める
- [x] 電源仕様の正式な定義元を決める
- [x] プロトコル仕様の正式な定義元を決める
- [x] サーボ安全範囲の正式な定義元を決める
- [x] 変更履歴を ADR、Git、文書内 revision log で追跡する

### Phase 2 完了条件

- [x] ADR-0001 が承認されている
- [x] 各トップレベル・ディレクトリの責務が明確である
- [x] リポジトリ構成が ADR と一致している
- [x] ESP32、Pi、プロトコル、ハードウェア資料の境界が明確である
- [x] 各設計値の正式な定義元が決まっている

---

## Phase 3: ハードウェア基準資料

### 3.1 `docs/hardware/hardware-bom.md`

- [x] 部品表のテンプレートを作成する
- [x] メーカー名を記載する欄を設ける
- [x] 正確な型番と末尾記号を記載する欄を設ける
- [x] 購入モジュール名を記載する欄を設ける
- [x] 公式データシート URL、版、日付を記載する欄を設ける
- [x] 供給電圧とロジック電圧を記載する欄を設ける
- [x] 最大・ピーク電流を記載する欄を設ける
- [x] 通信方式、アドレス、最大クロックを記載する欄を設ける
- [x] 供給性、代替品、数量を記載する欄を設ける
- [x] 確認状態と未確認事項を記載する欄を設ける
- [ ] 実際に使用する全モジュールを登録する

### 3.2 `docs/hardware/gpio-assignment.md`

- [ ] ESP32 開発ボードの正確なリビジョンを記載する
- [x] LCD の候補信号を列挙する
- [x] タッチの候補信号を列挙する
- [x] 加速度センサの候補信号を列挙する
- [x] 環境センサの候補信号を列挙する
- [x] サーボ PWM を列挙する
- [x] USB-UART、書き込み、デバッグ端子の確認欄を設ける
- [x] boot strap、予約、入力専用等の制約確認欄を設ける
- [x] GPIO、方向、起動時状態、pull、共有、根拠を記録する
- [ ] GPIO の二重割り当てがないことを確認する
- [ ] 起動モードを外部回路が妨げないことを確認する
- [x] 未確定 GPIO は `TBD` とする

### 3.3 `docs/hardware/power-budget.md`

- [x] 候補電源系統図を作成する
- [x] ESP32 系とサーボ系を分離する候補給電経路を記載する
- [x] ESP32 とサーボの共通 GND 方針を記載する
- [ ] 各部品の通常電流を記載する
- [ ] 各部品の最大・ピーク電流を記載する
- [ ] 同時発生するピークを見積もる
- [ ] 電源容量と設計余裕を記載する
- [ ] USB と外部電源の逆流可能性を確認する
- [x] デカップリングとバルク・コンデンサは公式値と実測から決める方針を記載する
- [ ] 配線、コネクタ、線材の電流条件を確認する
- [x] オシロスコープで確認すべき測定点を記載する

### 3.4 `docs/protocol/esp32-pi-protocol.md`

- [x] USB シリアルを初期通信経路として記載する
- [x] UART 候補設定を記載する
- [x] JSON Lines のフレーミング規則を記載する
- [x] 最大メッセージ長の候補と確定方法を記載する
- [x] protocol major version を定義する
- [x] message type を定義する
- [x] sequence ID を定義する
- [x] timestamp の基準を定義する
- [x] command、ACK、completion、event、status、log を定義する
- [x] エラー・コード体系を定義する
- [x] 重複コマンドと冪等性を定義する
- [x] timeout と再送方針を定義する
- [x] 再接続時の `boot`／status 同期を定義する
- [x] 不正入力、最大長超過、未知コマンドの扱いを定義する
- [x] MVP で必要な最小コマンド一覧を定義する

### 3.5 `docs/hardware/servo-safety-limits.md`

- [ ] 正確なサーボ型番を記載する
- [ ] 供給電圧、最大電流、拘束電流を記載する
- [ ] PWM 周期とパルス幅の公式条件を記載する
- [x] 機構を外した初回試験手順を記載する
- [x] ソフト最小、中立、最大値の校正方法を記載する
- [x] 最大速度と最大加速度の決定方法を記載する
- [ ] 通信断時の安全動作を決める
- [ ] 起動・リセット時の安全状態を決める
- [ ] emergency stop の方法を決める
- [x] 機械干渉を確認する試験手順を記載する
- [x] 実測前の値は `TBD` とする

### 3.6 `docs/hardware/sensor-datasheet-notes.md`

- [ ] LCD コントローラの仕様を記録する
- [ ] タッチ・コントローラの仕様を記録する
- [ ] 加速度センサの仕様を記録する
- [ ] 環境センサの仕様を記録する
- [ ] 各 I2C アドレスを確認する
- [ ] 各 I2C／SPI 速度と mode を確認する
- [ ] 起動時間、変換時間、応答時間を確認する
- [ ] CRC、校正係数、変換式を確認する
- [ ] モジュール上のレギュレータとプルアップを確認する
- [ ] 公式資料とモジュール回路図の差を記録する
- [x] 実機で確認すべき条件を列挙する

### 3.7 未確定事項管理

- [x] 全 `TBD` を一つの一覧から参照できるようにする
- [x] 各 `TBD` に決定担当を設定する
- [x] 各 `TBD` に必要な証拠を設定する
- [x] 各 `TBD` にブロックされる Issue を関連付ける
- [x] データシートで確定できる項目と実測が必要な項目を分ける

### Phase 3 完了条件

- [x] 6つの基準資料と `TBD` register が存在する
- [ ] 採用済み部品の正確な型番と一次資料が登録されている
- [ ] GPIO と電源の重大な競合が解消されている
- [x] プロトコル v1 の最小範囲が draft として定義されている
- [ ] サーボの実機試験を安全に開始できる手順がある
- [x] 未確定事項と、それにブロックされる作業が明確である

---

## Phase 4: README と GitHub 基盤

### 4.1 ルート `README.md`

- [x] DeskCat の概要を記載する
- [x] 現在の開発段階を記載する
- [x] MVP を記載する
- [x] ESP32 と Raspberry Pi の責務を記載する
- [x] システム構成を簡潔に記載する
- [x] リポジトリ構成を記載する
- [x] 開発環境の前提と端末 profile を記載する
- [ ] 現時点で有効なビルド・テスト手順を記載する
- [x] 未検証 command を draft として区別し、実行可能と断定しない
- [x] ハードウェア安全上の注意を記載する
- [x] 主要ドキュメントへのリンクを記載する
- [x] 貢献方法へのリンクを記載する
- [x] ライセンスを記載する
- [x] PDF 等のローカル専用資料の扱いを記載する

### 4.2 開発・セキュリティ文書

- [x] `CONTRIBUTING.md` を作成する
- [x] Issue の作り方を記載する
- [x] ブランチと PR の流れを記載する
- [x] 現時点の検証範囲と、正式 command が未確定であることを記載する
- [x] 実機確認結果の記載方法を定義する
- [x] `SECURITY.md` を作成する
- [x] 脆弱性報告方法を決める
- [x] ハードウェア安全問題の報告方法を決める
- [x] `CODE_OF_CONDUCT.md` は外部参加を積極募集する前に導入すると決める

### 4.3 Issue テンプレート

- [x] 不具合報告テンプレートを作成する
- [x] 機能提案テンプレートを作成する
- [x] ハードウェア実験テンプレートを作成する
- [x] 設計判断テンプレートを作成する
- [x] 不具合報告に期待値、実測値、再現手順を含める
- [x] ボード、回路、部品型番、firmware version を含める
- [x] 完全なログを添付する欄を設ける
- [x] 電源条件と測定結果を記載する欄を設ける
- [x] 受け入れ条件を必須にする
- [x] 実機確認の要否を記載する欄を設ける
- [x] 秘密情報を貼らない注意を記載する

### 4.4 Pull Request テンプレート

- [x] 目的と関連 Issue の欄を設ける
- [x] 変更内容の欄を設ける
- [x] 変更しなかった範囲の欄を設ける
- [x] format、lint、test、build の結果欄を設ける
- [x] 実機確認の構成と結果欄を設ける
- [x] GPIO、電源、プロトコル変更の有無を確認する
- [x] 新規依存とライセンス確認欄を設ける
- [x] 回帰確認の欄を設ける
- [x] 残存リスクと `TBD` の欄を設ける

### 4.5 ラベル

次のcheckは`.github/labels.yml`のローカル定義を示す。2026-07-28にGitHubへ適用し、2026-07-28の言語統一時に全25件のname、color、descriptionがローカル定義と一致することをread-back確認した。

- [x] GitHub標準label 9件は名称を維持し、descriptionを日本語化する
- [x] `area:firmware`
- [x] `area:raspberry-pi`
- [x] `area:protocol`
- [x] `area:hardware`
- [x] `area:docs`
- [x] `type:bug`
- [x] `type:feature`
- [x] `type:experiment`
- [x] `type:decision`
- [x] `type:maintenance`
- [x] `priority:critical`
- [x] `priority:high`
- [x] `priority:normal`
- [x] `status:blocked`
- [x] `needs:hardware-test`
- [x] `needs:decision`
- [x] DeskCat固有labelは初期backlogで使用する最小限の16件に限定する

### 4.6 マイルストーン

次のcheckは`.github/MILESTONES.md`のローカル定義を示す。2026-07-28にGitHubへ適用し、2026-07-28の言語統一時に全7件のtitleとdescriptionがローカル定義と一致することをread-back確認した。

- [x] `M0 Development Foundation`
- [x] `M1 ESP32 Bring-up`
- [x] `M2 ESP32–Pi Protocol`
- [x] `M3 Display and Input`
- [x] `M4 Servo Integration`
- [x] `M5 DeskCat MVP`
- [x] `M6 Reliability`
- [x] 各マイルストーンの完了条件を記載する
- [x] 依存順を記載する

### 4.7 GitHub 設定

- [x] default branch が `main` であることを確認する
- [x] [ADR-0004](../decisions/0004-main-develop-branch-strategy.md)で`main`を安定版、`develop`を通常開発の統合先と決める
- [x] GitHub Pagesのdeploy元を`main`だけに維持する
- [x] branch protection の段階的な導入条件を決める
- [x] solo bootstrap 中は 0、外部 contribution を受け入れる段階では 1 review と決める
- [x] status check 必須化の時期を決める
- [x] force push と branch delete の方針を決める
- [x] Issue／Discussions の利用範囲を決める
- [x] GitHub Actions の権限を最小化する
- [x] Actions で秘密情報を扱う場合の方針を決める
- [x] Dependabot 等は manifest 作成後に導入判断する
- [x] GitHub標準label 9件のdescription、DeskCat固有label 16件、M0–M6 milestoneをGitHubへ適用する
- [x] private vulnerability reporting を有効にする
- [x] `main` の force push と削除を禁止し、未実在 status check は要求しない

### 4.8 CI

- [x] 実際のプロジェクト生成前は、存在しないビルドを CI に書かない
- [ ] Markdown 検査を導入するか決める
- [ ] リンク検査を導入するか決める
- [ ] 秘密情報検査を導入するか決める
- [ ] Rust プロジェクト生成後に format check を追加する
- [ ] Rust プロジェクト生成後に lint を追加する
- [ ] Rust プロジェクト生成後に host unit test を追加する
- [ ] クロスビルド環境が安定後に ESP32 build を追加する
- [x] 実機試験を通常 CI と分離する
- [x] CI のツール・バージョンを固定する方針を記載する

### 4.9 文書言語

- [x] 説明、手順、ポリシー、Issue／PR templateの本文を日本語中心に統一する
- [x] コード、command、JSON field、label名、milestone名、型名、API名は英語の技術識別子として維持する
- [x] 文書言語の基準を`docs/governance/README.md`へ記載する
- [x] Governance、ADR、hardware、protocol、toolchain、runbook、backlogを基準に合わせる
- [x] Root文書、GitHub template、各directoryの責務READMEを基準に合わせる
- [x] 引継ぎ完了後に一時的な引継ぎ資料を削除し、技術ガイド内のcode、template、公式名称は原文との対応が必要な箇所として維持する

### Phase 4 完了条件

- [x] 新規参加者が README から主要資料へ辿れる
- [x] Issue と PR に必要な技術情報が残る
- [x] ラベルとマイルストーンが作業分類に利用できる
- [x] GitHub の公開・権限設定が Phase 0 の方針と一致する
- [x] 未検証 command を実行する CI が存在しない
- [x] 日本語本文と英語の技術識別子の使い分けがGovernanceに従っている

---

## Phase 4A: 公開ドキュメント基盤

この横断タスクは、公開文書の閲覧性を改善するfollow-upであり、M1のtoolchain検証や安全なソフトウェア作業をblockしない。`docs/`を正本とし、PagesとWikiへ同じ文書を手作業で複製しない。

### 現状

- [x] GitHub Pagesが`build_type: workflow`で有効であることを確認した
- [x] Pages workflowの初回成功runとdeploymentを確認した
- [x] Wikiが有効で、日本語の案内用`Home.md` 1件だけが存在することを確認した
- [x] 方針決定を[#25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)として作成した
- [x] Pages構築を[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)として作成した
- [x] Wiki入口整備を[#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)として作成した

### 方針決定と実装

- [x] [ADR-0003](../decisions/0003-public-documentation-publishing.md)で`docs/`とPagesの正本関係、公開範囲、navigation方針を承認する
- [x] Wikiを公開文書への入口ページに限定する
- [x] GitHub公式Jekyll Actionを採用し、追加依存、保守状況、license、代替をreviewする
- [x] Pull RequestでPages buildとlink checkだけを実行するworkflowを準備する
- [x] 最小権限かつcommit固定したActions workflowを準備する
- [x] `main`からPagesへdeployし、公開結果をread-back確認する
- [x] READMEから公開siteへlinkする
- [x] Wikiの既定Homeを日本語の入口ページへ置き換える

### Phase 4A 完了条件

- [x] #25の方針が永続文書へ記録されている
- [x] #26の受け入れ条件を満たしている
- [x] #27の受け入れ条件を満たしている
- [x] Wikiと`docs/`でlive文書を二重管理していない
- [x] Pagesの公開範囲にsecret、local専用資料、再配布不可資料が含まれていない

---

## Phase 5: 初期バックログ

この Phase の5.1–5.5にあるcheckは、`docs/backlog/initial-issues.md`に初期定義が揃っていることを示す。2026-07-28にGitHub基盤整備（直接完了、遡及Issue無し）を除く24件をGitHub Issueへ移行し、以後のlive statusはGitHubで管理する。機能実装の完了を意味しない。

### 5.1 `M0 Development Foundation`

- [x] Phase 0 のリモート残作業をGitHub基盤整備の直接完了作業に含める
- [x] 完了済みの AI エージェント基盤設定は遡及 Issue 化せず、永続文書を証拠とする
- [x] 完了済みの ADR-0001 とリポジトリ構成は遡及 Issue 化せず、ADR と Git 差分を証拠とする
- [x] ハードウェア基準資料の確定作業を #1～#3 に定義する
- [x] GitHub 基盤整備を直接完了の作業として定義する
- [x] 各 Issue に依存関係と受け入れ条件を記載する

### 5.2 `M1 ESP32 Bring-up`

- [x] Rust／ESP-IDF の対応バージョン調査
- [x] 最小プロジェクト生成
- [x] clean build
- [x] ESP32 への書き込み
- [x] 起動ログ
- [x] firmware version
- [x] reset reason
- [x] heartbeat
- [x] Debug／Release のサイズ確認

### 5.3 `M2 ESP32–Pi Protocol`

- [x] JSON Lines の host unit test
- [x] 最大長付き受信バッファ
- [x] `boot`
- [x] `ping` と ACK
- [x] `get_status`／`status`
- [x] ACK とエラー応答
- [x] 分割受信・複数行受信試験
- [x] 不正 JSON・最大長超過試験
- [x] 再接続・状態同期試験

### 5.4 後続マイルストーン

- [x] LCD 単体立ち上げ Issue
- [x] タッチ単体立ち上げ Issue
- [x] 加速度単体立ち上げ Issue
- [x] 環境センサ単体立ち上げ Issue
- [x] サーボ電源・単体試験 Issue
- [x] LCD と入力の統合 Issue
- [x] サーボ統合 Issue
- [x] Pi 感情ロジック統合 Issue
- [x] 長時間・異常試験 Issue

### 5.5 Issue 品質

- [x] 各 Issue の目的が一つである
- [x] 前提となる資料をリンクしている
- [x] 変更対象を記載している
- [x] 受け入れ条件が測定可能である
- [x] PC テストと実機試験を区別している
- [x] 実機試験に必要な機器または前提を記載している
- [x] `TBD` にブロックされる場合は明記している
- [x] 完了時に残す証拠を記載している

### Phase 5 完了条件

- [x] M0–M6 の目的と依存関係が明確である
- [x] 最初に着手する人間向け Issue と AI 向け Issue が決まっている
- [x] 最初の Issue に受け入れ条件がある
- [x] ハードウェア未確定の Issue を誤って実装開始しない
- [x] MVP までの作業をローカル backlog で追跡できる
- [x] 基盤文書の公開後、承認した初期Issue 24件をGitHubに作成してlive statusを移行する

---

## Phase 6: 実装開始ゲート

以下をすべて満たすまで、本格的な周辺デバイス実装を開始しない。

### 6.1 リポジトリ

- [x] コミット候補が repository safety baseline で整理されている
- [x] 秘密情報とローカル専用資料が除外されている
- [x] AI エージェント基盤設定が有効である
- [x] リポジトリ構成が ADR と一致している
- [x] README から必要な資料へ辿れる

### 6.2 ハードウェア

- [ ] 対象機能の正確な部品型番が確定している
- [ ] 公式データシートが登録されている
- [ ] 対象機能の GPIO が確定している
- [ ] ロジック電圧が確認されている
- [ ] 電源容量とピーク電流が確認されている
- [ ] サーボ電源が ESP32 から分離されている
- [ ] 安全な通電・測定手順がある

### 6.3 ソフトウェア

- [x] 対応する Rust／ESP-IDF の候補構成が公式資料で確認されている
- [x] ツールチェーンのバージョン固定方針がある
- [ ] build／flash／monitor の再現手順がある
- [x] プロトコル v1 の最小 draft がある
- [x] エラー・ログ・status の基本方針がある
- [x] host unit test を置ける構成がある

### 6.4 プロジェクト管理

- [x] GitHub上に着手Issue（[#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)）が存在する
- [x] #5の文書上の依存関係が完了している
- [x] #5の受け入れ条件が測定可能である
- [ ] 必要な実機と計測器が利用できる
- [x] 作業結果を記録する場所と template がある

### Phase 6 完了条件

- [ ] 最初の実装作業として「Rust／ESP-IDF 最小ビルドと起動確認」へ着手できる
- [x] 続く「ESP32–Pi `boot`／`ping`」の draft 仕様と受け入れ条件がある
- [x] ハードウェア固有値を推測せずに開発を進められる
- [x] 不具合発生時にログ、測定、Issue で追跡できる

---

## 7. 推奨する実行単位

基盤整備は、次の単位でレビュー可能な変更に分ける。チェックはローカル成果物の作成を示し、commit や push の完了を意味しない。

- [x] Change 1: リポジトリ安全確認と `.gitignore`
- [x] Change 2: `docs/governance/` のリポジトリ固有方針
- [x] Change 3: ルート `AGENTS.md`
- [x] Change 4: ADR-0001
- [x] Change 5: リポジトリ構成
- [x] Change 6: ハードウェア基準資料のテンプレート
- [x] Change 7: README と貢献・セキュリティ文書
- [x] Change 8: Issue／PR テンプレート
- [x] Change 9: ラベルとマイルストーンのローカル定義
- [x] Change 10: 初期 Issue のローカル draft
- [x] Change 11: 実装開始ゲートのレビュー
- [x] Change 12: 複数端末向け toolchain 情報と setup runbook
- [x] Change 13: 文書言語の日本語中心への統一

各 Change は、既存の未コミット変更と混在させないよう注意する。

## 8. この計画の対象外

以下は、この基盤整備計画が完了した後に個別 Issue で実施する。

- [ ] ESP32 ファームウェア本体の実装
- [ ] Raspberry Pi アプリケーション本体の実装
- [ ] LCD、タッチ、加速度、環境センサの実ドライバ
- [ ] サーボの実機駆動
- [ ] 回路製作・配線変更
- [ ] GitHub への公開、push、release
- [ ] OTA 実装
- [ ] 完成筐体での長時間試験
- [ ] Docs / Review profile 端末への不要な開発 tool 導入

## 9. 計画全体の完了条件

- [ ] Phase 0–5 の完了条件をすべて満たしている
- [ ] Phase 6 の実装開始ゲートを通過している
- [x] AI と人間の責務・承認境界が明確である
- [x] リポジトリ、ハードウェア資料、GitHub管理が相互にリンクしている
- [ ] 最初の実装 Issue へ安全に着手できる
- [x] 未確定事項を、推測せず追跡できる
