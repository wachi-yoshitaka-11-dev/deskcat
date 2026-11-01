# Issue #22: tap から表情描画までの実機確認

この手順は VM で用意した観測点を、#15 の実機 tap 検出が終わった後の実機窓で使うためのもの。**ここに測定結果はまだ無い。**`#22` の受け入れ条件は実測まで未達である。motion は扱わず、`play_motion` は送らない。

## 実施前

1. #15 の実機結果と使用した firmware commit、tap 設定、配線を記録する。しきい値、持続時間、最小間隔は [`deskcat-tap`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/crates/deskcat-tap/src/lib.rs) の暫定値であり、#15 の結果により評価する。この手順で変更しない。
2. [`firmware/esp32/README.md`](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/firmware/esp32/README.md#build構成と通電) の構成表と [`power-budget.md`](../hardware/power-budget.md) の `DISP-01` 追加接続の手順を確認する。既定の製品 build は現時点で通電不可と記録されている。`tap-21,face-21,bringup-display-13` のデモ構成を使う場合も、同手順の前提と条件(7)の当日の人間の承認を満たす。Pi–ESP32 の信号線と給電は [`gpio-assignment.md`](../hardware/gpio-assignment.md) の `Pi–ESP32間のtransport` 節の条件に従う。**同節「信号線をつないでよい条件」3の承認済み試験に #22 は含まれていない。#22 のための Pi 接続は別途人間の承認が得られるまで行わない。**この手順は接続の承認を代替しない。
3. ESP32 Flash / HIL 端末と Pi Runtime 端末を用意する。build の確定コマンドは [`verified-commands.md`](../toolchains/verified-commands.md) を参照する。Pi の実 serial port と HIL に検証済みコマンドはまだ無い。ESP32 の UART0 (USB) のログと Pi の `deskcatd` の stderr（systemd なら journal）を別々に、欠落しない形で保存する。UART1 は protocol に使う。servo 出力を使わない。
4. firmware と Pi の build commit、build 構成、各 device・配線 revision、給電条件、作業者、日時、ログの保存場所を記録する。実機の条件と停止基準は [`Hardware Safety Policy`](../governance/hardware-safety-policy.md) に従う。

## 1 回の tap の経路

session が確立し、LCD の bring-up と neutral 描画が終わった状態から、#15 で確認した方法で軽打を **1 回**行う。他の表情 command と独り言の表示を入れず、neutral に戻ってから次の試行をする。次を時系列で照合する。

| 場所 | 期待するログ | 読み方 |
|---|---|---|
| ESP32 UART0 | `tap_detected decision=Send ... sent=N`（**製品 build は保留するため、見つけたときは `tap_detected decision=Hold`、送るときは `tap_confirmed decision=Send ... sent=N`**）、`tap_event_poll_start uptime_ms=A`、`pi_tx frames=[("tapped", E)]` | filter を通過し、同じ `sid` の `id=E` を UART1 に書いたこと。`A` は `tapped` を送ると決めた `poll` の開始時刻（`tap-21` 構成は検出した `poll`。**製品 build は保留（`deskcat_tap::TAP_CONFIRM_MS`）の後なので、検出より保留（`deskcat_tap::TAP_CONFIRM_MS`）の分以上後**）。`pi_uart_write_failed`／`pi_uart_write_stalled` がないことも確認する |
| Pi stderr | `tapped_received sid=S id=E esp_ts_ms=... pi_uptime_ms=... total=N` | 現在の ESP32 session で event が受理されたこと。`total` は daemon 起動からの受理件数で、Mind の cooldown で抑えた分も含む |
| Pi stderr | `set_expression_queued id=C name=Surprised ...` | 感情 state が驚きを選び、表情 command を queue に入れたこと |
| ESP32 UART0 | `pi_rx_set_expression sid=... id=C name=Surprised accepted`、`face_show name=Surprised`、`face_drawn name=Some(Surprised) uptime_ms=B` | command の受理、描画開始、最後の帯の後の描画完了。ACK 受理だけでは描画完了を意味しない |

`E` は ESP32 の `pi_tx` と Pi の受信で一致させる。ESP32 の `pi_tx` ログには `sid` が無いため、`S` は Pi が現在の session の event として受理したことの確認に使う。`C` は Pi の送信と ESP32 の受信で一致させる。`face_draw_failed`、`pi_rx_set_expression_rejected`、session の再確立、別の `face_show`、文字表示が間に入った試行は latency の標本から除外し、除外理由と件数を残す。`face_drawn` が無ければ成功に数えない。

**End-to-end latency の標本は `B - A` ms。**両方 ESP32 の起動からの uptime なので、Pi と ESP32 の時計を同期する必要はない。`A` は `tapped` を送ると決めた `poll` の開始直前に採る（`tap-21` 構成ではそれが検出に至った `poll`。製品 build では保留が明けた `poll`）。`tap-21` 構成では、I2C の `INT_SOURCE` 読み出しと `tap_detected` log の時間も含むため、検出から描画までの**保守的な上限値**（`tap-21` 構成。製品 build は上のとおり `A` が保留の分だけ遅れるので、`B - A` は検出から描画までの時間より短い）であり、物理接触の開始時刻からの時間ではない。終点は最後の帯の転送後、次の main loop で完了を記録した時刻であり、画面の物理的な発光遅延は含まない。各試行の `A`、`B`、差、対応する `S/E/C`、失敗・除外の有無を表にし、標本数、最小、中央値、最大を報告する。目標遅延値は #22 に定義されていないため、合否値を推測で置かない。

## 件数と誤検出

意図した軽打の試行数と時刻を人間が記録する。触れない静止区間を分けて設け、開始・終了時刻と長さ、周囲の振動条件を記録する。この区間の `tap_event_poll_start` の増分（`tap_detected decision=Send`、製品 build は `tap_confirmed decision=Send` の増分と一致する）を **誤検出件数** とする。同じ区間の `tapped_received total` の増分は Pi に到達した件数である。抑制された検出は `tap_detected decision=DropTooSoon` 等の内訳として別に記録し、送信件数に混ぜない。`sent=N` は filter の送信判断の累積であり、UART 書き込み成功件数ではないため、`pi_tx` と Pi の受信も照合する。Pi 終了時の `daemon: ... tapped_received: N` は受理総数の補助確認になる。ログが欠けた区間は `0` 件とせず、欠測として扱う。

servo 自身の振動による再 trigger は servo を使える安全条件と #20 の実機成果が揃うまで **未確認** とする。この PR は servo を駆動しない。後続の実機試験では、人間が承認した servo 操作の前後を別区間にし、同じ件数の取り方で比較する。tap のしきい値や安全値をこの結果なしに確定しない。

結果は [実験ログ](../hardware/experiment-log.md) または #22 のコメントへ、試験条件、raw log の参照、試行別の表、誤検出件数、欠測と未確認事項を保存する。受け入れ条件の判定はその結果を読んで行う。
