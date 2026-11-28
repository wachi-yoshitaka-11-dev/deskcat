# ADR-0028: Pi の単一 runner と ESP32 の単一 main loop を、いまは分けない

> 状態: Proposed
> 日付: 2026-11-22（**2026-11-28 に状態を Accepted から Proposed へ戻した。**採用の承認の記録が見つからないためである。[#820](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/820)）
>
> **Proposed の間は、この ADR の全体（決定・影響・検証を含む）を作業の根拠にしない。**今のコードはそのまま残るが、それはこの ADR が決めたからではない。変える方向も変えない方向も、この ADR を根拠にしない。**Accepted にするときは、この段落を消す。**

## 背景

[#683](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/683)（[#546](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/546)の計画の全体（段0〜5）を受け持つ）は、Pi の serial 境界（`deskcatd` の `Daemon::step` と read／write／flush）と、ESP32 の main loop が、実機でどこで・どれだけ待つかを測り、両側の実行方式（単一 runner／単一 loop のままか、thread 等に分けるか）を**別々に**判断することを求めている。

#546 の結論は、**thread を使っていないことや allocation があることだけを欠陥としない**である。判断は、実測した待ち時間・期限超過・取りこぼしだけを根拠にする。

材料は次の3つである。

- ESP32 の `health` の `overrun_ticks`、`pi_uart_rx_buffer_full`、`pi_uart_rx_fifo_overflow`、`max_read_gap_ms`（既存の観測）。
- Pi の `link_wait`（PR #737。`step`、serial の read／write／flush の最大の待ち）。
- [EXP-066](../hardware/experiment-log.md#exp-066-受信の間隔の-log-を足した製品-buildlink-gap-20no-touch-487でpi-が送る-frame-の到着の間隔をlcd-の描画センサーの読み出しpi-の負荷と同時に測った)。**servo を動かさない**製品 build（`link-gap-20,no-touch-487`）を Pi とつなぎ、5つの条件（基線、`show_text`、叩く・揺らす、Pi の負荷、それらの同時）を各4分47秒〜6分46秒で測った窓。

EXP-066 は servo を駆動しない製品 build で測った（[#683](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/683)の安全の前提条件の「サーボ動作範囲を意図的に制限した、または機構から切り離した」の側）。servo を駆動する製品 build での確認は、下の`リスク`の行のとおり #20 の servo を動かす窓で行う。この ADR の判断は、それを待たない。

## 判断要因

- 期限に対する待ちの大きさ。比べる期限は次のとおり。いずれも値の根拠はそれぞれの正本が持つ。ここへ新しい値は置かない。
  - ESP32：`HEARTBEAT_PERIOD_MS`（1000 ms）、`HEALTH_SNAPSHOT_PERIOD_MS`（10000 ms）、`LINK_LOSS_TIMEOUT_MS`（値は[HW-TBD-017](../hardware/tbd-register.md)。#784 で置いた値であり、実機の確認は済んでいない）。
  - Pi：ESP32 の `LINK_LOSS_TIMEOUT_MS` に対する、Pi からの frame の到着の間隔。
- 期限超過・取りこぼしの有無（`overrun_ticks`、rx の取りこぼし、`pi_link_lost`、task WDT、`panic`）。
- 待ちの所在が分かっているか。**所在が分からないまま分けても、直るとは言えない。**
- 分けた場合に増えるもの（stack、優先度、停止手順、channel の上限、所有権）。

## 検討した選択肢

### 選択肢A：両側とも今の方式を維持する

利点：変更が無い。測った範囲では、期限超過と、ESP32 の rx の取りこぼしが無い（Pi 側の取りこぼしは測っていない）。thread 化で増える所有権・停止手順・stack の判断を、必要が確かめられるまで持ち込まない。

コスト：測っていない条件（EXP-066 の「確かめていないこと」）では、維持の根拠が無い。

### 選択肢B：Pi だけ、serial の I/O を別 thread と bounded channel に分ける

利点：`step` のうち serial 以外の処理が長く止まるなら、その間も受信を読み続けられる。

コスト：Pi の待ちの長い窓（約 1.1〜2.5 s）で、serial の read（data の無い poll を除く。`read_data`）と write／flush の呼び出しは最大でも 59 ms（read）、21 ms（write）、50 ms（flush）だった。data の無い poll の待ち（`read_idle`）も含めて、`step` が 1 s を超えたどの窓でも、serial の1回の呼び出しの最大は 82.5 ms 以下だった（EXP-066 の表）。**1回の呼び出しが 2.5 s の `step` を占めた形跡は無い。**呼び出しの合計時間は log に無いので、合計が占めたかどうかは分からない。止まりが `step` の serial 以外の処理にあるのか、process が動けていないのかは、まだ分けていない。後者なら、I/O を別 thread にしても同じ負荷で同じ止まりが起きうる。

### 選択肢C：ESP32 を、`std::thread` で task に分ける

利点：LCD 等の長い処理が、受信や heartbeat を遅らせる経路を断てる。

コスト：測った範囲で、ESP32 の遅れの指標はどれも期限に遠い（下の数値）。分けると、stack・優先度・core・停止手順・所有権の判断が要る（#546 の未確認事項）。

## 決定

**Pi の単一 runner と ESP32 の単一 main loop を、どちらも今は分けない（選択肢A）。**両側を別々に判断した。

### ESP32：維持

EXP-066 の5つの条件で、ESP32 の `health` の行（174行）の `overrun_ticks`、`pi_uart_rx_buffer_full`、`pi_uart_rx_fifo_overflow` は**すべて 0** だった。`max_read_gap_ms` の最大は、条件0から順に 86、90、91、107、109 ms で、`HEARTBEAT_PERIOD_MS`（1000 ms）の約 1/9 に当たる。`heartbeat_overrun`、`health_snapshot_overrun`、`task_watchdog`、`panic` の行は出なかった。LCD の描画、センサーの読み出し、B3 を叩く・揺らす操作、Pi の負荷を同時に掛けた条件4でも、`max_read_gap_ms` は 109 ms で、他の数は 0 のままだった。

この条件では、main loop が heartbeat・health snapshot・受信を読みに行く間隔の期限を脅かす待ちは測れなかった。分ける根拠が無い。

### Pi：維持（待ちの所在は未分離）

- 負荷の無い条件0〜2で、`step_max_us` の最大は 80503〜82762 µs、serial の `read_data_max_us` は最大 46814 µs だった。
- Pi に CPU の負荷（`yes`）とディスクの同期書き込み（`dd ... conv=fdatasync` のくり返し）を掛けた条件3・4では、`step_max_us` が 2505263 µs（条件3）、1745795 µs（条件4）まで伸びた。条件3・4を通した serial の呼び出しの最大は、read（`read_data`）58944 µs、write 20798 µs、flush 49639 µs だった。
- ESP32 側で `window_max_ms` が 2000 ms を超えた行（条件3で9行、条件4で5行）は、**すべて**、約 1.1〜2.5 秒の `step_max_us` を持つ Pi の窓と重なった（10秒の窓の粒度。逆は成り立たない）。
- Pi からの frame の到着の間隔の定常の最大は 3380 ms で、`LINK_LOSS_TIMEOUT_MS`（値は HW-TBD-017）に収まる。EXP-066 の build（`befc02d`）にはこの見張りがまだ無く（値は `None`）、`pi_link_lost` を出す経路も無かったので、検知そのものは確かめていない。起動からの累計の最大（4104〜4893 ms）は、4回とも ESP32 の `boot` の送り直しと Pi の `hello` の位置関係で決まる1件で、定常の値ではない。
- `deskcatd` は5回とも、止める SIGTERM まで動き続けた（EXP-066 の`deskcatd の止め方`）。再試行の上限を超えたときの終了（75。`apps/deskcatd/src/daemon.rs` の module doc）は起きていない。再試行の回数は数えていない。

負荷は人為的で、製品が通常出会う負荷ではない。条件を掛けても、測った範囲で期限を超えなかった。分ける構造（独立した I/O と bounded channel）を決めるには、止まりが `step` のどこにあるかが要る。いまは、それを分けた材料が無い。

### 計画の4（直す必要がある側の変更）

**行わない。**両側とも、測った範囲で期限超過が無く（取りこぼしは ESP32 の rx だけを測り、0 だった）、Pi の止まりの所在が分からないまま構造を変える根拠が無い。ESP32 の firmware、`deskcatd`、共有 crate は変えない。

### 計画の5（不要な依存・古い説明・allocation）

- **`embassy-time`（`generic-queue-8`）と `esp-idf-svc` の `embassy-time-driver`・`embassy-sync` feature：外さない。**`firmware/esp32/src` に `embassy` を使う呼び出しは見つからない（grep）。ただし、`esp-idf-svc` の feature が time driver を登録しうるため、外して挙動が変わらないことは、ESP32 build と実機の起動で確かめる必要がある。この変更では build も実機起動もしていない。**不要と確かめたものではないので、整理した扱いにしない。**外すなら、別の変更として、ESP32 build と実機起動の記録を添える。
- **allocation（`PiLink`、`Daemon::step` の受信 frame の `Vec`）：変えない。**EXP-066 で `overrun_ticks` は 0、rx の取りこぼしは 0 で、allocation が期限の超過に結び付く観測は無い。heap と stack は EXP-066 の対象外である。
- **古い説明：#683 の判断待ちを前提にした文言は、リポジトリ内に見つからなかった。**変えない。

## 影響

### 利点

- 測った根拠のある範囲だけを判断し、測っていない範囲を維持の根拠にしない。
- 実行方式を変えずに済むので、#20 の安全の検証（servo、fail-safe）と #24（耐久試験）の前のコードを動かさない。

### 欠点

- Pi の止まり（最大 2.5 s）の所在を分けていない。
- 維持の根拠は、servo を動かさない、touch を読まない構成の、1回 5〜7 分ほどの窓 5つに限る。

### リスクと対策

| リスク | 対策 |
|---|---|
| servo を動かす製品 build で、main loop の遅れが期限に近づく | [#20](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/20) の servo を動かす窓で、`overrun_ticks`・`max_read_gap_ms`・`heartbeat_overrun` を同じ形で記録し、この ADR の維持を見直す |
| touch を読む製品 build（`no-touch-487` でない build）は、読み出しの時間が loop に加わる | touch を読む build で同じ窓を測る。測るまで、ESP32 の維持は `no-touch-487` の範囲 |
| Pi の止まりが、製品の実際の同居の負荷（`yes`・`dd` 以外）で `LINK_LOSS_TIMEOUT_MS` を超える | `step` の内訳（serial 以外の処理と、process が動けていない時間）を分ける観測を足し、`HW-TBD-017` の値の再評価（#20）の材料にする |
| 5〜7 分ほどの窓の最大は、長時間の最悪を上限として示さない | [#24](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/24)（耐久試験）の記録で、`overrun_ticks`・`max_read_gap_ms`・`link_wait` を同じ形で取る |
| 観測の log 自体が、測った待ちを変えている | EXP-066 の「確かめていないこと」に残してある。影響の大きさは未確認 |

## 検証

次のどれかが起きたら、この ADR を見直す（見直しは、該当する側だけ）。

- ESP32：`overrun_ticks` が 0 でなくなる、`max_read_gap_ms` が heartbeat の期限を超える、`heartbeat_overrun`／`health_snapshot_overrun`／`task_watchdog` の行が出る、rx の取りこぼしが出る。
- Pi：Pi からの frame の到着の間隔が `LINK_LOSS_TIMEOUT_MS` に近づく、または `pi_link_lost` が製品の条件で出る。その場合は先に `step` の内訳を分け、serial の呼び出しが止まりの主因なら独立した I/O と bounded channel を、そうでなければ止まりの所在を直す。

この判断の測定は実機の 1 日の窓（2026-11-20、EXP-066）に限る。他の構成へそのまま当てはめない。

## 置き換える決定

なし。
