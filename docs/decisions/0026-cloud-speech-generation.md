# ADR-0026: 独り言のクラウド生成を表示可能な文字と費用上限の内側で行う

> 状態: Accepted
> 日付: 2026-10-14

## 背景

[#522](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/522)は、Pi が独り言をクラウドで生成する経路を求める。
現在の[#23の表示用glyph](../fonts/k8x12-speech.md)は固定文の22字だけであり、
生成文の大半を表示できない。Pi Zero WはArmv6で、
[Pi toolchain](../toolchains/raspberry-pi-rust-toolchain.md)はPi上の直接buildを採る。
`deskcatd`のPi上buildとsystemd導入は[配置記録](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/deploy/README.md)にあるが、
新しいHTTPS clientを含むbuildと実行は未検証である。

## 判断要因

- AIの出力は独り言の表示文だけに使い、表情・サーボ・安全判断へ渡さない。
- APIキーとネットワークの識別子をrepository、公開記録、logへ残さない。
- [protocol §5.4](../protocol/esp32-pi-protocol.md)の`show_text`上限と、firmwareが実際に持つglyph・layoutを守る。
- Pi Zero W上でbuild・常駐でき、デモに足りる頻度で利用できる。
- 課金を月額のサービス側上限でも止められる。

## 検討した選択肢

- **Anthropic Claude APIのClaude Haiku 5.5:** [モデル仕様](https://platform.claude.com/docs/en/models/haiku-5-5/overview)は`claude-haiku-5-5`を短時間・多数のrequest向けとし、[料金表](https://platform.claude.com/docs/en/about-claude/pricing)は短いpromptの標準料金を入力$0.10、出力$0.50／100万tokenとする。
- **Gemini 2.5 Flash-Lite、OpenAIの小型モデル:** 他社の候補。モデルや料金が変わるため、切り替えるときは各社の公式資料と送信先を改めて確認する。
- **常駐型クラウドエージェント:** 会話履歴、session、toolsを保持する形は一文生成の要件を増やす。常駐そのものは既存の`deskcatd`が担う。
- **#23のローカル独り言だけ:** 外部通信・課金は不要だが、#522の生成機能は得られない。障害時のfallbackとして残す。
- **HTTPS client:** `ureq`の`native-tls`を第一候補とする。`rustls`構成と別clientも候補であり、Pi上でのbuild・実行結果で比較する。[ureqの公開repository](https://github.com/algesten/ureq)はMIT／Apache-2.0のdual licenseである。

## 決定

本決定はユーザーが承認し、[#522の承認内容の記録](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/522#issuecomment-6158331864)に残した。

1. `deskcatd`からAnthropicのMessages APIへ、`claude-haiku-5-5`で一文を依頼する。履歴とtoolsは送らず、`thinking: {"type":"disabled"}`と`max_tokens: 128`を指定する。[Haiku 5.5の公式ガイド](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-haiku-5-5)はthinkingの無効化を認める。入力は1行20文字以内を指定する固定の発話指示、感情enum（`neutral`・`happy`・`surprised`）、時計が有効な場合の大まかな時刻帯に限る。名前、識別子、センサ生値、log、会話履歴、SSID、credentialを送らない。
2. PiからのHTTPS clientには`ureq 3.4.1`の`json`・`native-tls`を第一候補とし、OSのTLS証明書検証を使う。同期HTTP処理はUART処理を止めない別workerでtimeoutつきとし、自動再試行しない。[ureqの資料](https://docs.rs/ureq/3.4.1/ureq/)と[native-tlsの資料](https://docs.rs/native-tls/latest/native_tls/)を根拠とする。**Pi Zero W上での依存込みの直接build、HTTPS接続、常駐時のmemory・時間を確認するまではcrateを確定せず、クラウド経路を有効化しない。**
3. 専用APIキーは、決定5の月額`$2`上限を設定する非Default Workspaceだけにscopeして発行し、複数Workspace用のキーは使わない。[AnthropicのWorkspace仕様](https://platform.claude.com/docs/en/manage-claude/workspaces)では、単一Workspace用のキーによるrequestは発行時に選んだWorkspaceで実行される。キーはPiの`/etc/deskcatd/anthropic-api-key`にroot所有・mode `0600`で置き、systemdの`LoadCredential=`で`deskcatd`へ渡す。[systemdのcredential仕様](https://systemd.io/CREDENTIALS/)による。キーの値をrepository、unit、環境変数、起動引数、log、公開記録へ書かない。キー発行とPiへの設置は別途、人間の管理下で行う。
4. 生成は最短1分間隔、UTCの1日で最大300回とする。事前の[無料のtoken計数API](https://platform.claude.com/docs/en/build-with-claude/token-counting)を各生成前に最大1回使う場合、HTTP requestは合計最大600回／日となる。入力は計数値512 token以下、出力は`max_tokens: 128`とし、回数と見積費用を再起動後も維持する。入力512・出力128 tokenで300回使った標準料金の計算値は`300 × (512 × $0.10 + 128 × $0.50) / 1,000,000 = $0.03456/日`、31日で`$1.07136`。アプリ側の日額見積上限は`$0.04`とし、次のrequestに最大見積額を予約できないときは送らない。計数は見積もりであり、実際の請求額を保証しない。
5. 専用の非Default Workspaceに月額`$2`のspend limitを設定する。[AnthropicのWorkspace仕様](https://platform.claude.com/docs/en/manage-claude/workspaces)による。**実accountで`$2`を設定でき、決定3のキーがそのWorkspaceだけにscopeされ、上限が有効であると確認するまでクラウド経路を有効化しない。設定できない場合は代わりの具体額を人間へ提示して再承認を得る。**
6. クラウド接続に先立ち、#23の既存22字を残したうえで、k8x12からひらがな・カタカナ・記号・JIS第一水準のglyphを生成する方針とする。[作者の配布ページ](https://littlelimit.net/k8x12.htm)は第一・第二水準への対応を示し、[許諾](https://littlelimit.net/font.htm)は商用を含む改変・再配布を認める。現在の[生成script](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/scripts/generate_speech_glyphs.py)は22字専用なので変更を要する。文字表が数十KBという見積もりは、firmwareに収まる実測ではない。対象構成のbuildでimageとpartitionを比較し、実LCDの字形・描画時間を確認する。
7. 生成文は空文字、制御文字、UTF-8の884byte（暫定）、実装中の1行20文字、未収録glyphを送信前に検査する。満たさない文は捨てる。未接続、timeout、HTTP error、予算超過、無効な応答でも、その回は[#23](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23)のローカル独り言へ戻す。失敗理由は秘密情報を含めず分類・計数する。現在のservice unitではローカル独り言も無効なので、クラウド経路の有効化前にローカル独り言を有効化して表示を検証する。
8. Piのネットワーク設定はOS標準の手段を使い、デモ会場ではスマホのテザリングを第一候補とする。接続情報をrepositoryや記録に残さない。

## 影響

表示可能文字の拡張がクラウド生成の前提となる。文字数・byte数・glyphの検査は拡張後も必要である。生成文は表情・サーボの動作を変えない。クラウドが使えない間もローカル独り言を使う。

## 検証と未確定事項

- 本ADRは対象範囲1の方針であり、firmware変更、API接続、キー設置、ネットワーク設定の実施記録ではない。
- glyph拡張後のfirmware image、partition内への収まり、RAM、実LCDの可読性・描画時間は未確認。表示できた生成文の割合を後続の試験で確認する。
- `ureq`とTLSのPi Zero W上のbuild・HTTPS実行、systemd credentialの対象Piでの適合、月額`$2`の設定可否は未確認。
- PiのWi-Fi利用時の電源条件と、クラウド障害時のfallbackの実機確認は後続範囲で行う。
- 料金・API仕様が変わった場合は公式資料を再確認し、上限と送信先の変更には人間の承認を得る。

## 置き換える決定

なし。
