# Issue #24: サーボを使わない fault の窓と recovery runbook

> Status: Draft（2026-11-22）。**この文書の手順は実機でまだ実行していない。**`link_stress.py` と
> `stress.rs` は host で通した。実機の窓で得た結果は [experiment-log.md](../hardware/experiment-log.md) の
> 新しい EXP へ書き、この文書の「既知の制限」を更新する。**推測と観測を混ぜない。表の「予想」は現版の
> コードを読んだものであり、測るのが目的である。**

[Issue #24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24) の受け入れ条件のうち、**サーボを
使わずに確かめられる分**を、実機の窓で実行できる形にする。

| 受け入れ条件 | この文書 |
|---|---|
| Pi restart と ESP32 restart から復旧する | 対象（手順 2、3） |
| USB disconnect／reconnect から復旧する | 対象（手順 4。**今の transport では 2 つの意味があり、両方を行う**） |
| Sensor disconnect の影響を分離できる | 対象（手順 5。**電源を切ってから抜く**） |
| 不正・高 rate の protocol 入力を上限内で処理する | 対象（手順 6。**「上限」の値は未決のため、観測する。合否は下の基準**） |
| Memory、queue、parser、bus、sensor、reset counter を記録した | 対象（手順 7。**memory は記録できない。下の表**） |
| 既知の制限と recovery runbook を更新した | この文書の最後の 2 節 |
| 目標動作時間を定義し、試験に合格する | **対象外**（人間の判断が要る） |
| Servo 動作で brownout が発生しない | **対象外**（完成形の電源が要る。#525） |

## 0. 読んだもの

[AGENTS.md](../../AGENTS.md)、[Hardware Safety Policy](../governance/hardware-safety-policy.md)（§1 の 5 項目、§7、§8）、
[Machine Profiles](../toolchains/machine-profiles.md)、[Protocol](../protocol/esp32-pi-protocol.md)（§2、§4.6、§8〜§10）、
[Power Budget](../hardware/power-budget.md) の `touchを読まない製品build（no-touch-487）の通電の条件`、
[GPIO Assignment](../hardware/gpio-assignment.md) の `信号線をつないでよい条件`、`deploy/README.md`、
`firmware/esp32/README.md` の「build構成と通電」、EXP-065、EXP-066（記録）。**基点は develop の `addf99e0`（2026-11-22）。**#20 が `ping` の間隔（1000 ms）と ESP32 の loss timeout（7000 ms）を入れた版である。

## 1. 前提（満たさないなら窓を始めない）

1. **build は `no-touch-487` を単独で付けた製品 build**（`link-gap-20` との併用は EXP-066 の窓の構成であり、
   この窓は `no-touch-487` 単独）。ESP32 Flash／HIL profile の**実機 Linux**で書き込む（[Machine Profiles](../toolchains/machine-profiles.md)。
   コマンドは[検証済みコマンド](../toolchains/verified-commands.md)）。起動 log に `servo=not_driven` が出ることを確かめる。
2. **サーボは接続しない**（plug を抜く。EXP-065 は「針に挿したまま」だったが、この窓は抜く）。`PSU-BENCH-01` は
   電源プラグを抜いたままにする。ESP32 は端末の USB から給電する（EXP-065 と同じ。電流は測らない）。
3. **通電の承認は実行する日に 1 件、人間から得る**（Power Budget の同節。条件(7)と同じ形）。承認は PM が訊く。
   この窓で足す操作（EN ボタン、信号線の抜き差し、USB の抜き差し、センサーの線の抜き差し）は、承認の問いに
   含める。**意図的な fault injection と回路変更後の試験は、人間が立ち会い、電源を遮断できる状態で行う**（Hardware Safety Policy §7）。
4. **配線は EXP-065 と同じ構成から変えない。**Pi との UART と GND の線を変えたら、通電前の確認をやり直す（`信号線をつないでよい条件` の 6）。センサーの信号線の抜き差し（手順 5）は、挿し先の目視と記録した位置との照合を行う。**それがセンサーの線の通電前の確認として足りるかを示す正本は確かめていない**ため、手順 5 を承認の問いに含め、人間が決める（立ち会いは Hardware Safety Policy §7）。
5. **Pi の `deskcatd` は 1 つだけ動かす。**Pi を再起動すると本番の `deskcatd` service が自動で起動し、serial を
   取り合う（EXP-065 で起きた）。手順 6 の `link_stress.py` を使う前に `sudo systemctl stop deskcatd` する。
6. **証拠は repository の外に置く**（ESP32 の log は MAC を含みうる）。記録に device 名、個人 path、SSID を残さない。

**危険、異音、発熱、異臭、LCD の異常を認めたら、試験を止める**（AGENTS.md）。止めるときは電源を切る。
**ESP32 の USB を抜くときは、先に信号線を ESP32 側で外す**（次節 A）。同じ操作を繰り返さず、log と reset reason を保存し、
変更の前に配線を記録する（Hardware Safety Policy §11 の 2〜4。この窓はサーボを接続しないので、§11 の 1 の actuator 電源は無い）。

## 2. 操作の安全の判断（5 項目に当たるか）

| 操作 | 5 項目との関係 | 扱い |
|---|---|---|
| A. ESP32 の USB を抜く、Pi の電源を切る（cable を抜く） | **電圧違い**（片方の電源が切れた状態で信号線がつながると、ESP32 の入力の上限 VDD＋0.3 V を超えうる。`HW-TBD-036`）に当たる | **先に信号線を ESP32 側の端で外す。**入れるときは両方の電源を入れてから信号線をつなぐ（`信号線をつないでよい条件` の 2）。外す端は **ESP32 側**で、Pi の header 側ではない（同 6 の末尾） |
| B. 信号線の ESP32 側の端の抜き差し（両電源が入ったまま） | 同条件 2 が許す操作（両方に電源が入っている間だけつなぐ）。差し直す前に挿し先を目視する | 行ってよい。Pi の header 側は抜かない |
| C. `sudo reboot`（Pi の OS の再起動）、`systemctl restart deskcatd`、ESP32 の EN ボタン | 電源は切れない。経路も変わらない | 行ってよい。**これは私の読みである。**Pi の再起動の間の Pi の TX の電位は確かめていない。不安なら A の扱いにする（人間の判断） |
| D. センサー（`ACCEL-01`、`ENV-01`）の線を**通電したまま**抜き差しする | **短絡**（`3V3` と GND の線が並ぶ header で、抜き差しの途中に隣の pin へ触れうる。過電流保護 `PROT-OC-01` は入っていない）と**電圧違い**に当たりうる。**通電中の抜き差しが 5 項目に当たらないと言える文書が無い** | **しない。**USB を抜いて（A の順序で）、電源を切ってから抜く。再通電は挿し先を目視してから（EXP の先行例：「通電中の抜き挿しはしていない。USB を抜いた状態で抜いた」） |
| E. センサーの `VCC` と GND の線を抜く | 電源経路そのもの | **この窓では抜かない。**抜くのは信号線（`SDA`／`SCL`）だけ |

**D により、この窓が確かめるのは「センサーが接続されていない状態での起動と運転」である。運転中にセンサーが外れる
ことは確かめない。**これは「Sensor disconnect の影響を分離できる」の一部しか満たさない。残りは人間が
通電中の抜き差しを承認するかの判断であり、AI が決めない（未確認に残す）。

## 3. 何を見れば合否が決まるか

ESP32 の `status.protocol` の counter は、**今の firmware では増えない（常に 0）。**拒否は UART0 の log にだけ出る
（`firmware/esp32/src/pi_link.rs` の module doc、`health.rs`）。Pi 側の counter は `deskcatd` が**終了するときだけ**
1 行ずつ出す（`終了する:`、`session:`、`peer:`、`daemon:`）。

### 3.1 counter と log の対応

| 知りたいこと | 見る場所 | 備考 |
|---|---|---|
| reset の回数と理由 | ESP32 UART0 の `reset_reason=…` の行（起動ごとに 1 行）と、health 行の `uptime_ms` が 0 付近から始まること | 値は `power_on`／`external_pin`／`software`／`panic`／`task_watchdog`／`brownout` など（`reset_reason_str`）。**予想する値は `power_on`／`external_pin`／`software` だけ。`panic`／`task_watchdog`／`interrupt_watchdog`／`watchdog`／`brownout`／`power_glitch`／`cpu_lockup` など、それ以外が出たら異常として窓を止める**（`brownout`／`power_glitch` は電源の異常の表示で、AGENTS.md の「電圧降下を認めたら試験を停止する」に当たる） |
| main loop の遅れ | health 行の `overrun_ticks`、`snapshot_errors` | 増え続けたら記録して PM へ |
| Pi link の UART の溢れ | health 行の `pi_uart_rx_buffer_full`、`pi_uart_rx_fifo_overflow`、`max_read_gap_ms`、`pi_uart_event kind=… count=…` | **溢れの数は「上限」の測定値**（下の 3.3） |
| parser の拒否 | `pi_rx_invalid_utf8`、`pi_rx_decode_rejected code=<code>`、`pi_rx_oversize_line` の行を数える | 行の数と `link_stress.py` の「送った数」を突き合わせる |
| bus（I2C） | `tap_read_failed … consecutive=`、`tap_disabled`、`env_fault kind=i2c_timeout／i2c_other`（`count=`） | `ACCEL-01` と `ENV-01` は同じ bus（EXP-065） |
| sensor | `env_reading`、`env_event_stale_dropped`、`env_event_superseded`、`env_fault kind=…`、`tap_detected`／`tap_confirmed` | 欠測は fault として数える |
| Pi からの受信の途切れ（loss timeout 7000 ms） | **ESP32 の log には出ない。**`pi_link_lost` は servo の経路がある build だけが出し（`check_link` は servo が無いと戻る）、この build は servo を作らない。表示で見えるのは、白 LED が付く `no-touch-487,led-no-touch-514`（Power Budget は、**`touch` の `T_CS` を挿した配線に限り**可。挿さない配線では不可で、この窓の配線が `T_CS` を挿したものかは EXP-065 の記録に無く、確かめていない。実機では動かしていない。窓の承認の問いに含めるなら PM が決める） | Pi 側は `deskcatd` の終了行と `ping` の拒否の warn |
| queue（Pi 側の送信） | `deskcatd` の終了行 `session: SessionCounters { dropped_out, discarded_on_disconnect, disconnects, reconnect_attempts, timeouts, retries, … }` | `peer:` に `session_switches`、`stale_sessions`、`unapproved_hello_acks` など |
| queue（ESP32 側） | **観測できない。**firmware に queue の深さの counter が無い | 未確認に残す |
| memory | **記録できない。**空き heap と stack の残量は `mem-probe-23` の build だけが出し、その構成は通電不可（Power Budget。承認が要る）で、`no-touch-487` と併用できない（`compile_error!`） | 製品 build に出すには `unsafe`（製品は `forbid(unsafe_code)`）が要り、この作業では足さない。**#23 の窓の記録（mem-probe 付き）を指すだけで、この窓の根拠にしない** |

### 3.2 復旧の合否（案。PM が確認する）

**合格 = 障害を取り除いた後、人の追加操作（`reset-failed`、`restart`、再 flash）なしに、次の 3 つがそろう。**

1. `deskcatd` が `active` のまま、または終了コード 75（`EXIT_SESSION_REDO`）で終わって systemd が再起動して `active` に戻る。
2. ESP32 の `reset_reason` が、予想した値（`power_on`／`external_pin`／`software`）だけである（3.1）。
3. **端から端まで届く**：B3 を叩くと、`deskcatd` の log に `tapped_received … total=` が増える（EXP-062・065 と同じ観察。service のままで info の行として出る）。または手順 6 の `link_stress.py` の `recovered=yes`。

**手動の復旧が要った場合は「不合格」ではなく「手動復旧が要る」と記録する。**`StartLimitBurst=30`（300 秒）に達した
`failed` は既知（EXP-024）。**復旧までの時間の上限は置かない**（`PROTO-TBD-011`、`HW-TBD-017` が未決。測った時間を記録する）。

### 3.3 不正・高 rate の入力の合否（案）

**「上限」の値は未決である**（`PROTO-TBD-012`。Protocol §8.1・§8.2 の流量制限は ESP32 に未実装で、§2 の既知の逸脱(iii)）。
**したがって数の上限で合否を決めない。**合格 = 次の 3 つ。

1. ESP32 が止まらない・再起動しない（`reset_reason` の行が増えない。`hb` の行が続く）。
2. 入力を流し終えた後に、確認の `get_status` に `ack: ok` が返る（`link_stress.py` が `recovered=yes`。確認の `id` への応答だけで判定する）。
3. 拒否の行の数が、流した不正な行の数と一致する（`corpus`、`noise`、`oversize`）。**ただし UART の受信が溢れた場合は一致しない**
   （`pi_uart_rx_buffer_full` などが増える）。溢れた rate と回数を**測定値として記録する**。溢れ自体は不合格にしない
   （流量制限が未実装で、溢れは予想される）。溢れの後に 1・2 が成り立つことを見る。

host の `stress.rs` は、**同じ種類の入力**（分類済みの不正な行は同じ fixture、ランダムな不正 UTF-8 の行、上限を超える行、改行のない入力）の**parser だけ**を確かめる。
`noise`・`oversize`・`newline-free` の byte 列は script と host test で別である（組み立て中の byte 数が上限を超えない、流した行数と結果の数が
一致する、拒否が counter の field に分類できる、流し終えた後に復帰する）。**UART、ring buffer、main loop の読み取り間隔は
試さない。**

## 4. 窓の手順

準備：UART0 の log を端末で取り続ける（pty を割り当てる。[検証済みコマンド](../toolchains/verified-commands.md)の
`flash と serial monitor`）。Pi では service のまま `journalctl -u deskcatd -f` を取る（`tapped_received` は info の行で、`--verbose` は要らない）。
**各手順の前後の時刻を記録し、log と突き合わせる。**

### 手順 1: 基線

1. 承認を確認し、サーボの plug が抜けていることを目視する。
2. 両方の電源を入れてから信号線をつなぐ（`信号線をつないでよい条件` の 2、6）。
3. ESP32 が `reset_reason=power_on` と `servo=not_driven` を出し、`deskcatd` が session を確立し、B3 を叩いて
   `tapped_received` が出ることを確かめる。**ここで 5 分、log を取る**（基線の health 行：`overrun_ticks`、
   `pi_uart_*`、`env_reading` の間隔）。

### 手順 2: Pi の再起動

`deskcatd` が動いていることから始める。**各回の後に 3.2 の復旧を確かめ、次へ進む前に 1 分以上置く。**

- 2a. `sudo systemctl restart deskcatd`（process の再起動。新しい `sid`。Protocol §10.2）。
- 2b. `sudo reboot`（OS の再起動。操作 C の扱い）。
- 2c. Pi の電源を切って入れ直す：**先に信号線を ESP32 側の端で外し**、Pi の電源 cable を抜く。入れ直して Pi が起動した後、
  信号線を挿し直す（操作 A、B）。挿す前に挿し先を目視する。

見るもの：ESP32 の log の `pi_rx_*`（`hello` の受理、`pi_rx_reverse_boot` などの異常）、`deskcatd` の終了行の counter、
`boot` の再送、復旧までの時間（記録のみ）。

### 手順 3: ESP32 の再起動

- 3a. EN ボタンを押す（電源は切れない）。**`reset_reason` は `power_on` を予想する**（experiment-log の記録では、RTS で EN を駆動した reset が `power_on` だった。EN ボタンの値は未確認で、`external_pin` が出てもよい。EN と電源の入れ直しは `reset_reason` で区別できない場合がある）。
- 3b. 繰り返し 3 回（`deskcatd` が `EspRebooted` で終了コード 75 → systemd が再起動 → `StartLimitBurst=30` に近づかないこと。
  **回数は 3 回に限る。**増やす判断は PM）。

見るもの：`deskcatd` が `EspRebooted` で終わり再起動して復帰すること（Protocol §10.1）、新しい `sid`、`reset_reason`。

### 手順 4: link と USB の切断

- 4a（信号線）：両電源が入ったまま、信号線の **ESP32 側の端**を抜き、10 秒おいて、挿し先を目視してから挿し直す
  （操作 B）。見るもの：`deskcatd` の終了（`StatusUnanswered`／`LinkLost`。75）か継続、挿し直した後の復旧（3.2）、
  ESP32 側の `hb` が止まらないこと。ESP32 は 7000 ms で受信の途切れと判定する（`LINK_LOSS_TIMEOUT_MS`。`boot` の確立から見張る。根拠の水準は下限側だけ）が、
  **servo を持たないこの build は判定の結果を log にも出さない**（3.1）。通信断の後の動作（`HW-TBD-018`）は未決で、
  サーボは動かさない構成なので影響は表示と log だけである。
- 4b（ESP32 の USB）：**先に信号線を ESP32 側で外す。**USB を抜き、10 秒おき、挿し直す。ESP32 が起動した後
  （`reset_reason=power_on`）に、信号線をつなぐ。見るもの：3.2。

### 手順 5: センサーの切断（電源を切ってから）

1. **先に信号線を外し、USB を抜く**（操作 A）。
2. `ENV-01` の `SDA` の線（信号線だけ。`VCC`・GND は抜かない）を**1 本**、**両端とも外して取り除き、他の線から離して置く**（片端だけ外した線を挿したまま通電しない。浮いた端が他の pin や rail に触れうる。EXP の先行例：「抜いて置いた」）。外した位置と線の色を記録する。
3. USB を挿し、ESP32 が起動した（`reset_reason=power_on`）後に、信号線を ESP32 側の端でつなぐ（`信号線をつないでよい条件` の 2。
   挿す前に挿し先を目視する）。ESP32 の起動 log を取る。見るもの：`env_fault kind=i2c_*`（`count=`）が出ること、**`tap_detected`
   と `tapped` の経路が止まらないこと**（`ACCEL-01` は同じ bus を使う。bus を引きずるかどうかが「影響を分離できる」の中身）、
   LCD の表情が出ること、`deskcatd` の session が続くこと。**5 分取る。**
4. 戻す：**先に信号線を ESP32 側で外し**、USB を抜き、センサーの線を挿し直し、挿し先を目視する。**再通電の前に 2 の位置の記録と照合する。**
   USB を挿し、ESP32 が起動した後に信号線をつなぐ。
5. `ACCEL-01` の `SDA` について、1〜4 を繰り返す（`ENV-01` 側は元のまま）。
6. 戻した後に手順 1 の基線の確認をやり直す。

**2〜4 の間に ESP32 が 予想外の `reset_reason`（3.1）で再起動する、`tap_disabled` が出る、LCD が止まる、のどれかが出たら**、
「影響が分離できていない」として記録する。結論を急がず、どの組み合わせで出たかを残す。

### 手順 6: 不正・高 rate の入力

**`deskcatd` を止めてから行う**（`sudo systemctl stop deskcatd`。Pi の再起動後は本番の service が自動で起動していないか
`systemctl is-active deskcatd` で確かめる）。`link_stress.py` と、それが読む fixture（`crates/deskcat-protocol/tests/fixtures/`）を
**Pi に置き、Pi の UART（`/dev/serial0`）を使う。**PC の USB serial は ESP32 の UART0（debug）であり、Pi link ではない
（PC から流しても Pi link の受信は試さない）。

```bash
python3 scripts/link_stress.py --mode corpus --port <device> --baud 115200
python3 scripts/link_stress.py --mode noise --count 200 --port <device> --baud 115200
python3 scripts/link_stress.py --mode oversize --count 50 --port <device> --baud 115200
python3 scripts/link_stress.py --mode newline-free --count 64 --port <device> --baud 115200
python3 scripts/link_stress.py --mode flood --count 2000 --port <device> --baud 115200
```

`--sid` は省く（実行ごとにランダム。ESP32 は使った sid を retired として覚え、同じ sid の再実行を `stale_session` で拒否する）。`--count` は出発点であり、確定値ではない。**1 回ごとに ESP32 の log の拒否の行の数を数え、script が表示する `expected_rejections`
と突き合わせる（`corpus`・`noise`・`oversize` は送った行の数、`newline-free` は 1、`flood` は 0）。**3.3 の 1〜3 を判定する。`flood` は baud が上限の速さで送る。`recovered=no` が溢れと同時に出たら、確認だけを送り直して判定する（`--mode flood --count 0`。溢れで改行が落ち、確認の行が前の行と連結された可能性がある）。溢れが出た場合は、`--count` を半分にして
同じ手順を繰り返し、溢れない最大を記録する（測定値であり、上限の決定ではない）。

終わったら `sudo systemctl start deskcatd` し、手順 1 の基線の確認をやり直す。

### 手順 7: counter の記録

各手順の後に、次を記録する（1 つの表にまとめる。値は log から写す）。**取れなかった項目は「取れなかった」と書く。**

- ESP32：`reset_reason` の列、最後の health 行（`overrun_ticks`、`snapshot_errors`、`pi_uart_*`、`max_read_gap_ms`）、
  `tap_read_failed`／`env_fault` の件数、`pi_rx_*` の行数。
- Pi：`deskcatd` の終了行（`session:`、`peer:`、`daemon:`）。**SIGINT で止まらない場合がある**（EXP-065）。終了行が
  出なかったら、そう書く。

## 5. recovery runbook

**操作の順序（A）を守る。**どの症状でも、**サーボは接続しない**。

| 症状 | 確認 | 対処 |
|---|---|---|
| `deskcatd` が `failed`（`Start request repeated too quickly`） | ESP32 の電源と信号線、ESP32 の `reset_reason` | 原因を取り除いた後、`sudo systemctl reset-failed deskcatd` → `sudo systemctl restart deskcatd`（EXP-024） |
| `deskcatd` が起動するが ESP32 から 1 byte も届かない | 信号線の 2 本が Pi の pin 8・10 に挿さっているか（目視。EXP-065 で抜けていた）、GPIO15 を pull-down で読んで High か | ESP32 側の端は、電源の入った状態で挿し直してよい（操作 B）。Pi の header 側が抜けていた場合は、**先に信号線を ESP32 側で外し**、Pi を止めて（`poweroff`）から直す。直した後は `信号線をつないでよい条件` の 6（GND の導通測定と目視）をやり直し、両方の電源を入れてから信号線をつなぐ |
| ESP32 が `pi_rx_ack_sid_mismatch` を出す | 本番の service と手で起動した `deskcatd` が同じ port を開いていないか | 一方を止める（`systemctl stop deskcatd`） |
| ESP32 が応答しない | UART0 の log が続いているか（`hb`）。止まっていれば ESP32 の USB、`reset_reason` | 信号線を ESP32 側で外し、EN ボタン。それで戻らなければ USB を抜き挿し（先に信号線を外す）。**原因が分からない停止は EXP-030 にもある。**log を残して PM へ |
| `reset_reason` が予想した値（`power_on`／`external_pin`／`software`）以外 | その直前の log | **その窓を止める。**再通電しない。log と配線を記録して PM へ（brownout は電源の問題として人間の判断） |
| センサーの `env_fault`／`tap_read_failed` が続く | どのセンサーか、bus が共有（`accel=0x53 env=0x76`）か | USB を抜いて（先に信号線を外す）、線の挿し先を目視し、挿し直す。通電中に直さない |
| 温度・異臭・火花・LCD の異常 | — | **すぐ止める。**USB を抜く前に信号線を ESP32 側で外せるなら外す。間に合わなければ USB を先に抜き、その事実を記録する。log と reset reason を保存し、配線を記録する（§11 の 3・4） |

## 6. 既知の制限と未確認

**これは窓の前の状態である。**窓の後に、結果で更新する。

- **ESP32 の `status.protocol` の counter は常に 0 である。**拒否の分類は UART0 の log だけで、`get_status` で Pi へ返らない
  （`pi_link.rs`、`health.rs`）。Pi 側で ESP32 の拒否を数える経路は無い。**足すなら firmware の変更（ESP32 build と実機試験）が要り、
  この作業では足していない。**
- **流量制限（Protocol §8.1・§8.2）は ESP32 に未実装である**（`PROTO-TBD-012`）。「上限内で処理」の上限の値は無い。
- **memory（空き heap・stack の残量）は、通電してよい build では観測できない。**
- **ESP32 側の queue の深さは観測できない。**
- **運転中のセンサーの抜けは確かめない**（手順 5。通電中の抜き差しを 5 項目に当たらないと言える文書が無い）。
- **通信断の判定（7000 ms。#20、EXP-066）は入ったが、servo を持たない build では判定の結果が観測できない。**
  通信断の後の動作は未決（`HW-TBD-018`。Protocol §10.3）。上限側（`HW-TBD-020` の連続動作時間との比較）は servo の有効化の前に残る。
- **復旧までの時間の上限は決まっていない**（`PROTO-TBD-011`。`deploy/systemd/deskcatd.service` の値は暫定）。
- **`StartLimitBurst=30`（300 秒）に達すると `failed` になる**（EXP-024）。長い断のあとの自動復帰は確かめていない。
- **Pi の再起動の間の Pi の TX の電位**（操作 C の読みの根拠）は確かめていない。
- **長時間動作（目標動作時間）と Servo 動作の brownout は、この文書の対象外**であり、#24 の該当項目は未達のままである。
