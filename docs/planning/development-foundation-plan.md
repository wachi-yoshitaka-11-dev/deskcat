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

| Phase | 状態 | 証拠／残作業 |
|---|---|---|
| 0: リポジトリ安全確認 | 完了 | `docs/runbooks/repository-safety-baseline.md` |
| 1: AI エージェント基盤 | 完了 | `AGENTS.md`、`docs/governance/` |
| 2: リポジトリ構成 | 完了 | ADR-0001、責務 README |
| 3: ハードウェア基準資料 | 進行中 | 資料作成済み。実部品の型番・回路・実測待ち |
| 4: README／GitHub 基盤 | 完了 | ラベル、マイルストーン、脆弱性報告、`main`最小保護を適用済み。文書言語を日本語中心に統一し、commit `19853d0`で`origin/main`へ公開済み |
| 4A: 公開ドキュメント基盤 | 方針確定・実装待ち | [ADR-0003](../decisions/0003-public-documentation-publishing.md)を承認済み。[GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)でPages、[GH-004 #27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)でWiki入口を構築する。M1開始のblockerにはしない |
| 5: 初期バックログ | 完了 | 初期Issue 24件を[#1](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/1)〜[#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24)として作成し、GitHubへlive statusを移行済み |
| 6: 実装開始ゲート | レビュー済み・未通過 | `docs/runbooks/implementation-readiness-review.md` |
| M1-001a: 開発環境の情報整理 | 完了 | `docs/toolchains/`、役割別 setup runbook、ADR-0002 |
| M1-001b: 開発端末での検証 | 未着手 | ESP32 Build profile 端末での生成・clean build 待ち |

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
- [ ] 共有型・スキーマの生成有無を M2-001 の互換性検証で決める
- [x] ハードウェア資料の配置を決める
- [ ] 実験ログと大容量計測データの正式な保存場所を最初の実験 Issue で決める
- [x] PC テストと HIL テストの配置を決める
- [x] 補助スクリプトの配置を決める
- [x] ESP32 と Pi のリリース単位を決める
- [x] firmware version と protocol major version の互換関係を決める

### 2.2 初期ディレクトリ案の確定

- [x] 次の構成案を採用し、`docs/toolchains/` を追加する

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
- [x] Pages workflowとActions実行履歴が存在しないことを確認した
- [x] Wikiが有効で、既定の英語`Home.md`だけが存在することを確認した
- [x] 方針決定を[GH-002 #25](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/25)として作成した
- [x] Pages構築を[GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)として作成した
- [x] Wiki入口整備を[GH-004 #27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)として作成した

### 方針決定と実装

- [x] [ADR-0003](../decisions/0003-public-documentation-publishing.md)で`docs/`とPagesの正本関係、公開範囲、navigation方針を承認する
- [x] Wikiを公開文書への入口ページに限定する
- [ ] Pages生成方式と追加依存をreviewする
- [ ] Markdown／link checkをPull Requestで実行する
- [ ] 最小権限かつversion固定したActions workflowを導入する
- [ ] `main`からPagesへdeployし、公開結果をread-back確認する
- [ ] READMEから公開siteへlinkする
- [ ] Wikiの既定Homeを日本語の入口ページへ置き換える

### Phase 4A 完了条件

- [x] GH-002の方針が永続文書へ記録されている
- [ ] GH-003の受け入れ条件を満たしている
- [ ] GH-004の受け入れ条件を満たしている
- [ ] Wikiと`docs/`でlive文書を二重管理していない
- [ ] Pagesの公開範囲にsecret、local専用資料、再配布不可資料が含まれていない

---

## Phase 5: 初期バックログ

この Phase の5.1–5.5にあるcheckは、`docs/backlog/initial-issues.md`に初期定義が揃っていることを示す。2026-07-28にGH-001を除く24件をGitHub Issueへ移行し、以後のlive statusはGitHubで管理する。機能実装の完了を意味しない。

### 5.1 `M0 Development Foundation`

- [x] Phase 0 のリモート残作業を GH-001 に含める
- [x] 完了済みの AI エージェント基盤設定は遡及 Issue 化せず、永続文書を証拠とする
- [x] 完了済みの ADR-0001 とリポジトリ構成は遡及 Issue 化せず、ADR と Git 差分を証拠とする
- [x] ハードウェア基準資料の確定作業を FND-001～FND-003 に定義する
- [x] GitHub 基盤整備を GH-001 に定義する
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

- [x] GitHub上に着手Issue（[M1-001 #5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)）が存在する
- [x] M1-001の文書上の依存関係が完了している
- [x] M1-001の受け入れ条件が測定可能である
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
