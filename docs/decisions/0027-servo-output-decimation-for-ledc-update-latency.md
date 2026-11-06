# ADR-0027: servo の出力を、LEDC の更新の遅れに合わせて間引く

> 状態: Accepted
> 日付: 2026-11-06

## 背景

[#18](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/18)の段0（servo を接続しない）で、校正 build が`tick`の間隔の判定で必ず fault した（`limiter`が間隔を reject して latch する）。#703・#704 の診断 log と [EXP-056](../hardware/experiment-log.md#exp-056-段0の2回目の窓703入りの校正buildでmotion-calib-103を3回出し3回とも約101-msで止まった)〜[EXP-058](../hardware/experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)（段0の窓）の観察で、原因が分かった。判断の経緯と選択肢の全体は[#705](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/705)にある。

- ESP32 の LEDC では、`ledc_set_duty_and_update`が、前の更新が効くまで **critical section の中で busy-wait** する（ESP-IDF v5.5.3 `components/esp_driver_ledc/src/ledc.c:1596-1611`。待ちの本体は`components/hal/esp32/include/hal/ledc_ll.h:483-489`の`while (conf1.duty_start);`）。
- 待ちは、前に立てた`duty_start`が落ちるまで。実測では、**立ててから2回目の PWM の周期の頭（立てた位相に依って約 20〜40.2 ms 後。周期の頭の直後に立てた場合は約 40.1〜40.2 ms。EXP-057 の 1回目は約 32.3 ms、後続は約 40.1〜40.2 ms）に落ちる。**この「2回目」の理由は ESP32 の TRM で確認していない**（log とパルス数への当てはめ）。
- **duty のレジスタへの書き込みは、busy-wait の前に行われる**（`ledc.c:1606`の`ledc_duty_config`）。一方、low speed では`ledc_ls_channel_update`（para_up）が待ちの後に来る（`ledc.c:982`）ので、**source の順序だけでは、出力に出る時刻は決まらない。**[EXP-058](../hardware/experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)のパルス数（4本・4本・2本）は、**待ちの前に書いた値が、`duty_start`が立っている間、周期の頭ごとに出力に出る**と読むと合った（値が引き金の後に見えると読むと 6本・6本・4本になり、合わなかった）。**観察からの当てはめで、TRM は未確認。**
- 結果として、50 Hz の PWM（周期 約 20.1 ms）で 20 ms の制御周期をとると、2回目の出力から CPU が最大で約 23 ms 止まり、`tick`の間隔が約 30〜40 ms になって許容を超える。止まっている間は、**その core の割り込みも止まる**。
- **この問題は校正 build だけのものではない。**製品の servo task（[#19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)の後続）も、同じ`Sg90::output_angle`（`firmware/esp32/src/servo.rs`）を通る。

制約：出力は必ず`Limiter`を通す（デバッグ経路でも安全制限を迂回しない）。制御周期・許容幅・PWM 周波数などの値は、この ADR で変えない（値は回ごとの実行承認が決める）。

## 判断要因

- **制御の間隔**：`tick`の間隔が`ControlPeriod`（公称±許容）に収まること。
- **STOP が効くまで**：`stop()`を呼んでから PWM のパルス幅が 0 になるまで（`stop()`が戻る時刻とは別）。
- **割り込みの停止**：どの core の割り込みが、どれだけ止まるか。
- **`unsafe`・依存の要否**：AGENTS.md は`unsafe`を原則禁止とし、新しい依存には必要性・公式性・保守状況・ライセンス・代替の確認を求める。firmware の crate は`forbid(unsafe_code)`（`main.rs`。`mem-probe-23`だけ例外）。
- **波形の測り直し**が要るか。
- **通常の build への影響**。

## 検討した選択肢

**選択肢の記号は[#705](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/705)と違う。**#705 の H が本 ADR の A、#705 の A が本 ADR の B に当たる（C・D は同じ）。#705 の F（`ledc_set_duty`と`ledc_update_duty`を分ける案）は、確度が低いので本 ADR には載せていない。

### 選択肢A：同じ thread のまま、LEDC への出力を間引く（出力は N tick に1回、limiter は毎 tick）

`LimitedServo::tick`は 20 ms ごとに`Limiter::step`を進めるが、`set_duty`は**2〜3 tick に1回**だけ呼ぶ。前の更新が効くまで約 40.2 ms なので、**呼ぶ間隔が 40.2 ms 以上なら busy-wait がほぼ起きない**。

利点：`unsafe`・新しい依存・thread・構造変更が要らない。`Limiter`を通る経路のまま。host の`Limiter`（approved 90〜105、速度 10、加速度 50、step 2、20±10 ms）で、目標 8回の 1 つの軌道を 20 ms の全 step で計算し、2 tick・3 tick に間引いて見た：位置は範囲の内（90.000〜105.000）、速度は最大 10.000（上限と同じ）、加速度は最大 50.001（2 tick）・50.002（3 tick）で、上限 50 を**わずかに超えた**。この超過は、位置（約 100）の f32 の分解能（約 7.6e-6）から差分で速度・加速度を求める丸めの範囲で、**20 ms の全 step でも 50.011 が出る**（`crates/deskcat-servo/tests/trajectory.rs:61-62`の許容`max_acc + 1e-4/dt`は、この差分の求め方では 20 ms の全 step も超える）。したがって、**この確認だけでは上限の合否を出せない。**網羅は実装の Issue の`tests/trajectory.rs`相当（`Limiter`の内部状態で数える）で行う。

コスト：2 tick（40 ms）だと、呼ぶ間隔が 40.2 ms をわずかに下回る周に busy-wait が入りうる。tick の量子化（`CONFIG_FREERTOS_HZ`＝100、1 tick ＝ 10 ms）で間隔が 1 tick 短くなる周があれば、その分（最大 10 ms 前後）まで延びうる（段0の log に 30 ms の間隔が現れた）。3 tick（60 ms）なら 20 ms の余裕があるが、出力の更新は約 16.7 Hz になる。STOP の CPU の停止は残る。間引いた tick の setpoint を「出した位置」として扱わない実装が要る。

### 選択肢B：LEDC を呼ぶ処理を、別の core の専用 thread に移す

利点：main の`tick`の間隔が 20 ms に戻る。`unsafe`は要らない。

コスト：busy-wait と割り込み停止が別の core に移るだけで、消えない。thread の`set_duty`も約 40 ms に1回しか進めないので、**出力の間引きは A と同程度に起きる**。thread 間の共有と`stop`の同期が増え、`stop()`の意味（返り値の「duty を 0 に戻せたか」）が変わる。`Sg90`／`LimitedServo`の構造が変わる。

### 選択肢C：MCPWM に替える

ESP-IDF 公式の例`examples/peripherals/mcpwm/mcpwm_servo_control`。compare 値は shadow への書き込みで busy-wait せず（`mcpwm_cmpr.h`）、STOP は`mcpwm_generator_set_force_level`が即座に適用される（`mcpwm_gen.h`）。

利点：busy-wait も割り込み停止も無い。根本対策。

コスト：esp-idf-hal 0.46.2 に MCPWM の wrapper が無く、`unsafe`の FFI か新しい依存が要る。GPIO27 の信号源が変わるので、波形（周期・分解能・起動時の出力・`Drop`の挙動）を測り直す。通常 build にも広く影響する。

### 選択肢D：`stop`だけ`ledc_stop`（idle level 0）を使う

`ledc_stop`（`ledc.c:996-1007`）はループが無く CPU を止めない。

利点：`stop`の CPU の停止が消える。

コスト：`LedcDriver::stop`は private（`Drop`からだけ）で、呼ぶには`unsafe`の FFI が要る。出力の更新（`tick`）の問題は残る。

### 除外した選択肢

- **PWM 周波数を上げる**（例：100 Hz）：更新が効くまでの時間は半分になるが、`SERVO_PWM_FREQUENCY_HZ`は値で、SG90 は 50 Hz 前提（一般値）。この ADR で値を変えない。
- **制御周期を 40 ms 以上にする**：`CONTROL_PERIOD_NOMINAL_MS`は値で、回ごとの実行承認が決める。ただし、A でも B でも出力の更新が 40 ms 前後になることは、後の回の値の決定に効く。

## 決定

**選択肢Aを採る。3 tick（60 ms）から始め、2 tick にするかは`tick`の間隔の log で決める。選択肢B・C・Dは、いまは採らない。**

1. **`Limiter`は毎 tick（制御周期ごと）進める。LEDC への出力（`set_duty`）は、N tick に1回だけにする。**N は 3 から始める。
2. **間引いた tick（`set_duty`を呼ばない tick）の setpoint を「出した位置」として扱わない。**`resume_position`と、`stop`の後の位置の記録は、**最後に出力した値**に合わせる。
3. **間引いた出力の列（実際に`set_duty`へ渡す列）で、速度・加速度の上限を、出力の間隔で数えて超えない設計にする。**超える位相が残る設計のままでは、#18 の段1へ進まない。
4. **間引いた後に busy-wait が消えたことを、測って確かめる。**出力した tick の時間（`after_tick_us − at_us`）を log に残し、段0で全部が小さいことを確かめる（閾値は実装の Issue で決める）。`tick`の間隔の条件だけだと、busy-wait が残っていても許容の内側に入る場合を見落とす。
5. **D は採らない。**`unsafe`を足さないため、また出力は`stop`を呼んだ次の周期の頭（最大1周期）で 0 になると読めるため（[EXP-058](../hardware/experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)のパルス数からの当てはめ。STOP の同期した実測で確かめる）。
6. **C（MCPWM）は、#19 の製品の servo task を作る前に、別の判断として検討する。**A は C への道を塞がない（`Sg90::output_angle`の呼び方を変えるだけ）。

## 影響

### 利点

- `unsafe`・新しい依存・thread の追加が要らず、`Limiter`を通る経路のまま、校正 build の`tick`の間隔を制御周期に戻せる見込みがある。
- 出力の更新が約 40 ms ごと以下であることを、LEDC のハードウェアの制約として設計に出せる。

### 欠点

- 出力の更新が約 16.7 Hz（3 tick）に落ちる。
- 出力しない tick の setpoint を「出した位置」として扱わない、という実装上の約束が増える。
- 「約 40 ms（`duty_start`が 2回目の周期の頭で落ちる）」の理由は TRM で確認しておらず、log とパルス数への当てはめである。

### リスクと対策

| リスク | 対策 |
|---|---|
| **STOP の CPU の停止と、その間の割り込み禁止。**出力した tick の直後に`stop`が来ると、`set_duty(0)`が前の更新が効くまで最大約 40 ms busy-wait する（`portENTER_CRITICAL`、`ledc.c:1605`）。出力は`stop`を呼んだ次の周期の頭（最大1周期）で 0 になると読める（[EXP-058](../hardware/experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)のパルス数からの当てはめで、TRM と同期した実測は未確認）ので、止まるのは CPU と、その core の割り込みである | 残存リスクとして記録する。D（`ledc_stop`）は`unsafe`を足さないため採らない。Arduino での STOP の同期測定は実機側が持つ |
| **割り込み禁止の間の UART の受信で、byte を落としうる。**FIFO は 128 byte（ESP-IDF v5.5.3 `components/soc/esp32/include/soc/soc_caps.h:360`の`SOC_UART_FIFO_LEN`）、driver は 120 byte で割り込みを上げる（`components/esp_driver_uart/src/uart.c:69`、2035）。driver の ring buffer（`PI_PROTOCOL_UART_RX_BUFFER_BYTES`＝512 byte）は、ISR が走らないと使われない。115200 bps・8N1（[esp32-pi-protocol.md](../protocol/esp32-pi-protocol.md) §2 の`Baud`・`UART framing`、いずれも`Candidate`）では 1 byte 約 86.8 µs で、**FIFO が空から 128 byte 満ちるのに約 11.1 ms**（連続して送られ続けた場合）。ただし**受信が続いている最中は、割り込み禁止に入る時点で FIFO に最大で約 119 byte 入っていることがあり**（120 byte で割り込みが上がるため）、その場合は残り 約 9 byte（約 0.8 ms）で溢れる。**したがって、byte を落とさない十分条件は「割り込み禁止の間に受信が続かない」ことで、「11 ms を超えない」ではない。** Pi link の実際の送信量は根拠として持っていない。UART1 の割り込みをどの core に確保したかは未確認 | **#19 の設計を縛る**（次項）。console の`STOP`のような短い行（数 byte）は、FIFO がほぼ空のときに限って収まる |
| **#19 の製品の servo task への制約。** | `stop`と出力の busy-wait が、**受信の途中に入らない**設計にする（受信が静かな区間で出力する、または busy-wait を無くす案Cを別に判断する）。`stop`を出力した tick の直後に呼ばないか、呼ぶ前後の UART の受信量を見積もる。UART1 の割り込みの core を確かめる。出力の間引き（本 ADR の決定 1〜4）を守る |
| 出力の間引きで、速度・加速度の上限を破る位相が残る | 決定 3 を受け入れ条件にする（実装の Issue）。網羅は`tests/trajectory.rs`相当で行う |
| busy-wait が残っていても、`tick`の間隔が許容の内側に入って見落とす | 決定 4（出力した tick の時間の log） |

## 検証

- 実装の Issue で、`tick`の間隔が許容に収まること、出力した tick の時間が小さいこと、間引いた出力の列で位置・速度・加速度が上限の内にあることを確かめる。
- 実機側が、STOP が効くまで（`command_accepted`／`stop_post_us`と信号が止まる時刻）と、波形（パルス数・周期）を Arduino で測る。方法は実機側が決める。
- 見直しの条件：A で`tick`の間隔が許容に収まらない（2 tick・3 tick のどちらでも）とき、または、STOP の CPU の停止と UART の受信の影響が許容できないとき。C（MCPWM）を別の判断として扱う。

## 置き換える決定

なし。
