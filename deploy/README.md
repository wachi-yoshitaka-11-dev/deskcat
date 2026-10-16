# 配置・運用

Raspberry Pi用serviceとinstall用artifactをこのディレクトリに置く。

## #522 クラウド独り言の有効化条件（未実施）

現行のunitはクラウド独り言を起動しない。`--cloud-speech`を追加する前に、
Pi Zero W上で`deskcatd`とHTTPSを確認し、#23のローカル独り言を表示できること、
非DefaultのAnthropic Workspaceに月額`$2`の上限が設定され、専用キーがその
Workspaceだけで使えることを人間が確認する（[ADR-0026](../docs/decisions/0026-cloud-speech-generation.md)）。
元のキーはPiの`/etc/deskcatd/anthropic-api-key`へroot所有・mode `0600`で置く。
unitの`LoadCredential=`がサービスへ渡し、fileが無い場合は`SetCredential=`の既定値（キーではない印の`deskcatd-no-api-key`）へ戻る。`deskcatd`はこの印をキーとして使わず、クラウドを使わずにローカル独り言だけで動く。既定値を空にすると、systemd 257.9 は空の値を捨て、fileが無いと起動しない（#522 の Pi での観察）。
キーの値をunit、環境file、CLI引数、log、repositoryへ書かない。
予算の記録は`StateDirectory=deskcatd`が作る`/var/lib/deskcatd`へ保存する。
予算fileの読書きに失敗した場合はクラウドへ送らず、ローカル独り言を使う。

既定の`ExecStart`は独り言そのものもoffのままである。表示の実機確認を終えた後、
ローカル独り言の2引数を先に有効化する。上のクラウドの有効化条件をすべて
確認した後だけ`--cloud-speech`を追加する。サービスの詳細logでもキーを
出さないよう、`deskcatd`はHTTPライブラリのlogを抑止する。
キーの漏洩が疑われる場合はクラウド経路を無効にし、その専用キーをAnthropic側で
失効させてWorkspaceの使用量を確認する。再発行後も、Workspaceのscopeと月額上限を
確認してからPiへ置き直す。キーの値はIssueやPRの報告にも載せない。

置いたもの（Issue #491 段階2b-iii）:

- `systemd/deskcatd.service`（port と baud を除く値は暫定で、根拠は無い）、`systemd/deskcatd.env.example`

**この文書の手順は、2026-10-12にPi Zero Wで1回行った内容の記録であり、検証済みコマンドではない**
（観察は[EXP-024](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/docs/hardware/experiment-log.md)）。
**確かめていないことは、「未確認」と書く。**
下のcommandは、実施した文面である（作業セッションの実行記録〔sshの入出力〕と1行ずつ照らした。`<…>`は、実施では具体的なpathだった位置）。[#446のcomment 6123783146](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446#issuecomment-6123783146)は操作の要約で、commandの文面は含まない。

## 導入（Pi上。2026-10-12に実施）

Piの給電は[hardware-bom.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/docs/hardware/hardware-bom.md)の`PSU-PI-A-02`である（極性と出力電圧は実測していない）。
**ESP32側の信号線の端を外した状態で行う**（[gpio-assignment.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/docs/hardware/gpio-assignment.md)の`信号線をつないでよい条件`の条件2が正本。ここへ写さない）。

1. 対象のcommitの`apps/`、`crates/`、`Cargo.toml`、`Cargo.lock`を、Piの新しいdirへ展開する（実施：開発端末の`git archive`をsshでPiへ流した。85 file）
2. そのdirで`cargo build --locked -p deskcatd`（debug）。非対話のsshでは`cargo`がPATHに無く、`. ~/.cargo/env`が要った。実施：cargoの表示は19分32秒（開始と終了の時刻の記録は19分34秒）、peak RSS 248,644 kB（sampling）。`--workspace`は試していない（[検証済みコマンド](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/toolchains/verified-commands.md)の`Raspberry Pi`節）
3. 専用ユーザーを作る（実施では、存在しなければ）：`sudo useradd --system --no-create-home --shell /usr/sbin/nologin deskcatd`
4. `sudo install -m 0755 <build先>/target/debug/deskcatd /usr/local/bin/deskcatd`
5. `/etc/deskcatd/deskcatd.env`を作る（`sudo install -d -m 0755 /etc/deskcatd`の後）。実施した中身は`DESKCATD_PORT=/dev/serial0`と`DESKCATD_BAUD=115200`。**この値は、実機で通信できた値であり、確定値ではない**（baudの正本は`PROTO-TBD-001`）。リポジトリの`deskcatd.env.example`は空のままにする
6. unitを開発端末から`scp`でPiの`/tmp/deskcatd.service`へ送り、`sudo install -m 0644 /tmp/deskcatd.service /etc/systemd/system/deskcatd.service`（実施。送ったのはdevelopの`deploy/systemd/deskcatd.service`、sha256 `032541225e9ec62d02391dab6bea9b0ea5bb544b85aef4bc1fc4c3a3873ca8de`）
7. `sudo systemctl daemon-reload`と`sudo systemctl enable deskcatd`。**startはしない**（実施：`systemd-analyze verify`は警告なし。`enable`の後`is-active`は`inactive`）

unitが渡す引数は`--port`、`--baud`、`--duplicate-capacity 8`、`--duplicate-retention-ms 5000`、`--startup-deadline-ms 10000`だけである。
**speechの引数（`--speech-min-interval-ms`、`--speech-duration-ms`）は渡さないので、`show_text`は送られない**（`apps/deskcatd/src/cli.rs`）。

## 電源を入れる順序と、failedからの戻し

- 信号線は、PiとESP32の両方に電源が入っている間だけつなぐ。入れるときは両方を入れてからつなぎ、切るときは先にESP32側の端を外す（`信号線をつないでよい条件`の条件2）
- **`enable`したので、Piの起動で`deskcatd`が始まる。**ESP32とつながらない間は、`HelloRetryExhausted`（終了コード75）で終わり、systemdが再起動する（実施：1回の実行は約2秒、周期は約5〜12秒）。実施では、ESP32側の端を外したPiの起動から約4分で、`StartLimitBurst`（30回）に達して`failed`になった（journal：`Start request repeated too quickly`）
- `failed`からの戻し（実施）：ESP32側の端を挿した後、`sudo systemctl reset-failed deskcatd`と`sudo systemctl restart deskcatd`で`active`になった。`hello`へのESP32のACKは届いたが、`deskcatd`は「応答送信側のsessionが未承認のため受理しない」というWARNを出した（`active`は、sessionの確立を示さない）
- 止めるとき：`sudo systemctl stop deskcatd`の後、ESP32側の端を外してから、電源を切る。`stop`しても`enabled`のままである。止めたままにするなら`disable`が要る（未実施）

## 未確認

- PiとESP32を同時に入れて、`deskcatd`が自動でESP32とつながること（実施では、Piが上がった時点でESP32側の端は外れていて、端を挿した後に手で`reset-failed`と`restart`を実行した）
- update（新しいbuildへの入れ替え）の手順
- logとrestartの方針（unitの値以外）、`failed`の自動の復帰が要るか
- `deskcatd`の長時間の運転。unitの値（`RestartSec`、`StartLimitBurst`、`--startup-deadline-ms`など）が適切であること（どれも根拠は無い。選び直しの回数の上限は`PROTO-TBD-011`が未確定で、`--startup-deadline-ms`と`RestartSec`に対応するTBDの登録は無い）

実行file、path、実行user、service動作が確定する前に、動作するdeployment scriptを作成しない。
（`systemd/deskcatd.service`が固定するpath・user・動作は暫定で、確定していない。この文が禁じるのは、install・update用のscriptである）
