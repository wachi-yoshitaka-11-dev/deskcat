//! `SERVO-01`（SG90首振りサーボ）のPWM driver。
//!
//! [Issue #17](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/17)、
//! [Issue #19](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19)。
//! 首振り、fail-safe、拘束検知は別途進める。
//!
//! # 型が保証すること
//!
//! **この型が保証するのは、command の水準までである。**`bench-servo-test-17`付きのbuildを除くbuildで、このmoduleを通る
//! 出力について、次の2つを保証する（`bench-servo-test-17`付きのbuildは、下の例外段落のとおり、この保証の外にある）。
//!
//! - **出力の元になる`Setpoint`の列（量子化の前）が、自分で測った間隔で`Limiter`の制限**
//!   （位置、速度、加速度）**を満たす。**`Limiter`が加速度の
//!   上限を譲ったSetpoint（`acceleration_bound_conceded`）は出力せずにlatchする（下の「制御周期の外の間隔、加速度の譲り、出力の失敗ではlatchする」）
//!   **`Limiter`は出力するtick（N tickに1回）でだけ進むので、この`Setpoint`の列がそのままLEDCへ設定する列である**
//!   （下の「出力の間引き」。窓の間隔が許容の内側で揺れても、速度と加速度は窓の間隔で数えて上限を超えないことをhostで確かめている）
//! - **`tick`がLEDCへ設定するdutyは、承認値の位置範囲の両端を変換した値の内側に収まる。**dutyへの量子化は
//!   最も近い整数へ丸めたうえで、下端は切り上げ、上端は切り捨てた範囲へclampする。**位置だけは
//!   量子化の後も型で締める。**速度と加速度は量子化の刻みの分だけずれうる（下の「実機での実現は保証しない」）
//!
//! **`単一commandの最大変化量`は、この保証に入れていない。**`Limiter::step`がそれで制限するのは、
//! 加速度のclampより前の要求値だけであり、測った間隔が制御周期の許容幅の中で揺れると、1 stepの
//! 移動量はそれを超えうる（`deskcat_servo`のdocが保つと書くのも位置・速度・加速度の3つである）。
//! 1 stepあたりか1 commandあたりかも、正本がまだ決めていない（[#19の残件](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19#issuecomment-5892732352)）。
//!
//! **この保証はcodeを読んで立てたものである。**`LimitedServo`、dutyの量子化、latchを実行して
//! 確かめた試験は、hostにもtargetにも無い（testがあるのは`deskcat_servo`の`Limiter`だけである）。
//!
//! **保証は、[`LimitedServo::new`]または[`LimitedServo::resume`]から[`LimitedServo::stop`]までの
//! 区間ごとである。**`stop`はその境界であり、dutyを0にするだけで、軌道の制限を受けない（減速しない）。
//! `resume`は最後に出力した位置から速度0の`Limiter`を作るので、再開後の位置の列はその位置から
//! 制限を受ける。**`Limiter`が測っていないのは、止める直前の速度から0への不連続と、duty 0の間である。**
//!
//! **実機がdutyの列をどう実現するか（PWMのframe、servo内部の追従、resetの後）は保証しない。**
//! 下の「実機での実現は保証しない」に列挙し、段3の監視下の試験（#19の受け入れ条件7）で確かめる。
//!
//! # 角度を出せるのは、limiterを所有する[`LimitedServo`]だけである
//!
//! **`bench-servo-test-17`付きのbuildを除くbuildで、このmoduleの外から角度を指定してpulseを出す入口は
//! [`LimitedServo::tick`]だけである。**[`LimitedServo`]は[`Sg90`]と`deskcat_servo::Limiter`を
//! **1つずつ所有**し、外へ出す操作は[`LimitedServo::new`]、[`LimitedServo::resume`]、
//! [`LimitedServo::admit`]、[`LimitedServo::tick`]、[`LimitedServo::stop`]、
//! [`LimitedServo::counters`]、[`LimitedServo::fault_counters`]だけである（#19の受け入れ条件2・3、`AGENTS.md`
//! 「サーボ安全制限をデバッグ経路からも迂回させない」）。
//!
//! - **`Limiter`を外から受け取らない。**[`LimitedServo::new`]と[`LimitedServo::resume`]は、
//!   制限・制御周期・受理するmotion名を受け取り、**この型の中で速度0の`Limiter`を作る。**
//!   呼び出し側が進めておいた`Limiter`（位置を合わせたまま速度を持たせたもの等）を渡す経路が無い
//! - **`Setpoint`も`AdmittedTarget`も外から受け取らない。**`admit`は受理したtargetを内部に持ち、
//!   `tick`はそのtargetへ向けて**自分の**`Limiter::step`を呼び、結果を出力する。
//!   `Setpoint`は`Copy`であり、出力を受け取る入口があると、古い値を後で出し直して
//!   速度と加速度の制限を通らずに位置を跳ばせてしまうためである
//! - **制御周期の間隔を呼び出し側から受け取らない。**`tick`は前回の`tick`からの経過時間と、
//!   最後の出力からの経過時間（窓）を単調時計（`std::time::Instant`）で**自分で測り**、前者は毎回検査し、後者を
//!   `Limiter::step`へ渡す。
//!   間隔を申告させると、公称値を申告したまま短い間隔で呼ぶだけで、`Setpoint`の列が
//!   実時間で`Limiter`の制限を満たさなくなるためである。**測っているのは連続する`tick`の冒頭の
//!   時刻の間隔であり、`set_duty`を呼ぶ時刻の間隔ではない**（stepの計算やpreemptionによる遅れは
//!   測定に入らない）
//! - **servo 1つにつき`Limiter`は1つである。**[`LimitedServo::new`]は[`Sg90`]を値で受け取り、
//!   [`Sg90::new`]はLEDC timer・channel・pinを消費する。これらが`Peripherals`から1回だけ取れるのは、
//!   `unsafe`の`steal`／`reborrow`を使わない場合であり、製品buildは`forbid(unsafe_code)`で
//!   それを禁じている（`src/main.rs`先頭の`cfg_attr`。`mem-probe-23`のbuildは`deny`で、`mem_probe`だけが`allow`する）。止めた後は[`Stopped`]になり、
//!   [`Stopped`]から[`Sg90`]を取り出す手段は無い
//! - [`Sg90`]が外へ出すのは[`Sg90::new`]と[`Sg90::stop`]（dutyを0へ戻す。角度を指定しない）
//!   だけであり、角度を出す関数はこのmoduleのprivateである
//! - Pi由来でもdebug由来でも、`MotionRequest::source`で判定を分けない（`Limiter::admit`のdoc）
//!
//! **型が塞ぐのは、このmoduleを通る経路までである。**LEDC timer・channel・pinを[`Sg90::new`]へ
//! 渡さずに、`esp_idf_svc`の`LedcDriver`を直接作る経路は型では止められない。
//! `SERVO-PWM`へ触る経路をこのmoduleの外に作らないことは、呼び出し側（`main.rs`）の規則である。
//!
//! **例外は`bench-servo-test-17`の`Sg90::move_to_angle_once`である。**limiterを通らない。
//! 同featureのbuildでだけcompileされる。このbuildは#17の初回の駆動のためのものであり、
//! `servo-safety-limits.md`の`承認の状態`の項目6（2026-09-28、ユーザーの決定）と
//! `測定のための駆動`節が定める、狭い仮の制限で強制する単発のbuildである（1回の起動で1 command、
//! 動作角のclamp、露出時間の後にdutyを0、NVSのlatch。`main.rs`の`run_servo_bench_test`）。
//! **clampの幅は一次資料にも実測にも基づかない暫定値であり**（`crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`）、
//! 露出時間は安全な保持時間の主張ではない（`crate::config::SERVO_BENCH_TEST_EXPOSURE_MS`）。
//! limiterを通す駆動は、#18・#19のbuildで扱う（同節）。
//!
//! # 制御周期の外の間隔、加速度の譲り、出力の失敗ではlatchする
//!
//! 次のどれかが起きたら、**以後の`tick`は何も設定せず、`Limiter`も進めず、[`TickError::Faulted`]を
//! 返す。**解除の経路は、[`LimitedServo::stop`]の後に[`LimitedServo::resume`]で作り直すことだけである。
//!
//! - **測った間隔が制御周期（`ControlPeriod`）の外にある**（[`TickError::Rejected`]。各`tick`の間隔と、`Limiter`へ渡す窓の間隔の両方）。
//!   長すぎる場合（遅れ）にそのまま続けると、`Limiter`に残った速度で次の位置を設定することになり、
//!   その間の位置の列が`Limiter`の制限を満たさなくなる。短すぎる場合は、`Limiter`がその間隔では
//!   `最大加速度`を保証できない（`deskcat_servo::Limiter::step`のdocの「`dt_s`は
//!   `ControlPeriod`の範囲内でなければならない」）。**jitterが許容幅を超えると、そこで新しい位置を設定しなくなる**（LEDCは最後のdutyを出し続けうる）。
//!   新しい位置を設定しない側に倒れる既知の性質である
//!   **#18の段0で観察した超過は、jitterではなかった。**ESP32のLEDCでは`ledc_set_duty_and_update`が、前の更新が効くまで
//!   （`duty_start`を立ててから2回目のPWMの周期の頭。立てた位相に依って約20〜40 ms後）、critical sectionの中でbusy-waitする
//!   （ESP-IDF v5.5.3 `components/hal/esp32/include/hal/ledc_ll.h:483-489`、`components/esp_driver_ledc/src/ledc.c:1596-1611`）。
//!   制御周期20 ms・PWM 50 Hzでは、2回目の出力から`set_duty`が最大約23 ms止まり、間隔が約30〜40 msになる
//!   （系統的な伸びで、必ず起きる）。止まっている間は、そのcoreの割り込みも止まる。判断は[ADR-0027](../../../docs/decisions/0027-servo-output-decimation-for-ledc-update-latency.md)
//!   と[Issue #705](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/705)。**この実装は`set_duty`をN tickに1回へ間引く**（下の「出力の間引き」。[Issue #708](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/708)）。
//!   **間引いて`tick`の間隔が許容の内側に戻ることは、実機（#18の段0）で確かめるまで主張しない**
//! - **`Limiter::step`の後にPWMの出力が失敗した**（[`TickError::Output`]）。`Limiter`だけが1 step進み、
//!   LEDCに設定した位置は前の値のまま残る（esp-idf-hal 0.46.2の`LedcDriver::set_duty`が呼ぶ
//!   ESP-IDFの`ledc_set_duty_and_update`は、失敗をregisterへ書く前の検査で返す。source を読んだ結果）。
//!   そのまま続けると、次の`tick`で2 step分の変化を設定することになる
//! - **`Limiter::step`が加速度の上限を譲ったSetpointを返した**（[`TickError::AccelerationConceded`]）。
//!   **出力しない。**crateのdocはこのflagを「立ったら実装の前提が破れている」としている
//!
//! どれも[`ServoFaultCounters`]に数え、latch後の`tick`の回数も数える。このcounterは`stop`と`resume`を
//! 越えて残る（`Limiter`のcounterは`stop`で捨てる）。
//!
//! これは`servo-safety-limits.md`の`起動時とdriver故障時の動作`（`HW-TBD-019`、PWM driverの
//! 実行中の故障を検知したときの動作は未確定）の中で、「未検証の動作出力を行わない」側の
//! 既定として置いたものである。**値も復帰の条件も決めていない。**
//!
//! **latchは「それ以上動かさない」までであり、「出力を止める」ではない。**LEDCは最後に設定した
//! dutyを出し続けうる。**[`TickError`]のどれを受けても、呼び出し側は[`LimitedServo::stop`]を呼ぶ。**
//! その配線は`bench-servo-calib-18`のbuild（`crate::servo_calib`）と、製品buildの`crate::servo_path`（`product()`が`None`の間は呼ばれない）が持つ（`tick`のエラーの後に`stop`を呼ぶ）。
//!
//! # 間隔の基準と、tickを呼び続ける前提
//!
//! **間隔の基準を取るのは、[`LimitedServo::new`]または[`LimitedServo::resume`]の後で、targetが
//! あるときの最初の`tick`だけである。**その`tick`は何も設定しない。2回目以降の`admit`では基準を
//! 取り直さない。**したがって、一度動き始めたら、targetに着いた後も制御周期で`tick`を呼び続ける。**
//! 呼ぶのを止めてから再開すると、測った間隔が制御周期の外になり、latchする。
//!
//! # 出力の間引き（Issue #708、ADR-0027）
//!
//! **`Limiter::step`は出力する`tick`（[`crate::config::SERVO_OUTPUT_DECIMATION_TICKS`]（N）回の`tick`に1回）でだけ進め、
//! その`dt`には窓（最後の出力から今回の出力までに測った時間）を渡す。LEDCへの出力（`set_duty`）もその`tick`だけである。**
//! 出力する`tick`の判定は`deskcat_servo::OutputDecimator`が持つ（hostでtestできる）。数えるのは、間隔の検査を通した
//! `tick`だけで、`new`／`resume`の後の基準の`tick`の次からN回目が最初の出力である。Nは回ごとの実行承認の値ではなく、設計の定数である。
//! （ADR-0027の決定1は、はじめ「`Limiter`は毎`tick`進める」だったが、決定3を揺れる間隔でも満たすため、この形に改めた。
//! ADR-0027の「追記」参照。）
//!
//! - **各`tick`の間隔は、毎回検査する**（間引いた`tick`でも）。呼び出し側が渡した`ControlPeriod`の外ならlatchする
//!   （[`TickError::Rejected`]）。**`Limiter`が持つ`ControlPeriod`は、これをN倍した窓の許容**（公称 N×、許容 N×）である。
//!   各`tick`の間隔が許容の内側なら、N個の和は（f32の丸めの境界ぎりぎりを除き）窓の許容の内側に収まる。境界ぎりぎりで外れたときは、`Limiter::step`がrejectしてlatchする（安全側）。新しい値は決めない（`tolerance < nominal`もN倍で保たれる）
//! - **`Limiter`は出力する`tick`でしか進まないので、latchしない限り、`Limiter`の位置と最後に出力した位置は同じである。**（加速度の譲りと出力の失敗でlatchしたときだけ、`Limiter`が1 step先へ進んだ位置が残るが、再開は`resume_position`（最後に出力した位置）から作り直す。）
//!   `resume_position`（[`Stopped::resume_position`]）も、`stop`の後の位置の記録も、最後に出力した値である。
//!   間引いた`tick`は[`LimitedServo::tick`]が`Ok(None)`を返し、「出した位置」を作らない
//! - **出力の列は、`Limiter`が測った間隔（窓）で制限した列そのものである。**速度と加速度の上限は、`Limiter::step`の保証が
//!   そのまま効く（窓の間隔が許容の内側で揺れても、窓の間隔`T_k`で数えて上限を超えない。ADR-0027の決定3）。
//!   `deskcat_servo`の`tests/decimation.rs`が、N（1〜6）、各`tick`の間隔の揺れ、再targetの位相を総当たりして確かめている
//!   （窓の許容の内側で`acceleration_bound_conceded`も立たない）
//! - **代償。**出力（servoへ届く位置）の更新がN `tick`に1回になるのは、間引きそのものの代償で、旧設計と同じである。
//!   この変更で増える代償は、`Limiter`の速度の変化が窓の単位になる（1回の出力で変えられる速度は`最大加速度 × 窓`まで）ことと、
//!   窓の許容が広い分、境界への接近の見積もりが保守的になること（違反ではなく遅さ）
//! - **上限の判定は`Limiter`の内部状態（`Setpoint::velocity`）で数える。**出力した位置の差分から求めると、f32の
//!   分解能（100付近で約7.6e-6）の丸めで、公称の周期でも50.011のような値が出る
//! - **Nの下限。**N ≥ 1はこのmoduleがcompile時に検査する。`N × 制御周期 ≥ PWMの周期 × 2`は、
//!   [`crate::config::decimation_meets_pwm_floor`]を使って**校正build（`servo_calib`）だけ**がcompile時に検査する
//!   （製品の制御周期はまだ無い。#19の後続が自分で検査する）。**必要条件であり、十分条件ではない。**実際の更新の遅れ（約40.1〜40.2 ms）を
//!   超えるかは、PWMの周期がLEDCのtimer分周の丸めで決まる（約20.06〜20.12 ms）ことと、「2回目の周期の頭」で落ちる理由が
//!   TRMで未確認なことから、compile時には検査できない。**出力した`tick`の時間をlogに残して確かめる**
//!   （`servo_calib_18_phase_trace`の`after_tick_us − at_us`。出力した`tick`は`servo_calib_18_output_trace`）
//! - **STOPのCPUの停止は残る。**出力した`tick`の直後の`stop`は、前の更新が効くまで最大約40 ms busy-waitする
//!   （ADR-0027の「リスクと対策」。#19の設計を縛る）
//!
//! # 止めた後の再開
//!
//! [`LimitedServo::stop`]は自分を消費して[`Stopped`]を返す。[`Stopped`]は[`Sg90`]と**再開の位置**
//! （最後に**出力した**`Setpoint`の位置。間引いた`tick`には位置が作られない。量子化の前の値であり、LEDCへ書いたdutyそのものではない。
//! 一度も出力していなければ起動時の初期位置）をprivateに持つ。
//! [`LimitedServo::resume`]は**位置を受け取らず**、その位置から速度0の`Limiter`を作る。
//!
//! - **止める直前の速度を持ち越さない。**持ち越すと、再開の最初の位置が、止まっていた間を
//!   無視した速度で設定される
//! - **呼び出し側が選んだ位置から再開させない。**選べると、その位置へ`Limiter`の速度・加速度の
//!   制限を受けずに設定できてしまう
//!
//! # 既知の制限: 起動時の初期位置
//!
//! [`LimitedServo::new`]の`initial_position`は、呼び出し側が渡す値である。**呼び出し側が位置を
//! 選べる入口は、起動時のこの1つだけである。**`Limiter`の制限はこの位置から始まる列に効き、
//! この位置と実機の位置の食い違いには効かない（起動時の位置の扱いは`HW-TBD-019`／`HW-TBD-020`）。
//! **このmoduleは値も扱いも決めない。**
//!
//! # 実機での実現は保証しない
//!
//! 次は型でも、ESP-IDFのsourceを読むことでも確かめきれない。**量を主張しない。**
//! 段3の監視下の試験（#19の受け入れ条件7）の項目として扱う（[#19のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19#issuecomment-5891754488)に記録し、
//! 保証の範囲と再開の位置は[訂正のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19#issuecomment-5909603268)で置き換えた）。
//!
//! - **dutyへの量子化。**設定するdutyは`Setpoint`の位置を最も近い整数へ丸めた値であり、
//!   その刻みの分だけ、設定する列の速度と加速度は`Limiter`の制限からずれうる（位置は上の範囲に収まる）。
//!   変換は一般値による線形変換であり、calibration済みではない
//! - **PWMのframeで効くこと。**esp-idf-hal 0.46.2の`LedcDriver::set_duty`はESP-IDFの
//!   `ledc_set_duty_and_update`を呼び、それが内部で`_ledc_update_duty`を呼ぶ（`components/esp_driver_ledc/src/ledc.c`）。
//!   新しいdutyは次のPWM cycleから効く（`components/esp_driver_ledc/include/driver/ledc.h`の
//!   `ledc_update_duty`のnote）。dutyのregisterへの書き込みはbusy-waitの前に行われる（`ledc.c:1606`）。lowspeedでは`ledc_ls_channel_update`（para_up）が待ちの後に来る（`ledc.c:982`）ので、sourceの順序だけでは出力に出る時刻は決まらない。
//!   #18の段0（[EXP-058](../../../docs/hardware/experiment-log.md#exp-058-段0の4回目の窓書き直さない校正buildで動作1回ごとのパルスの本数をarduinoで数え同じ回のlogと並べた)）のパルス数（4本・4本・2本）は、**待ちの前に書いた値が、`duty_start`が立っている間、周期の頭ごとに出力に出る**と読むと合った（観察からの当てはめで、TRMは未確認）。
//!   この読みなら`stop`（`set_duty(0)`）が効くのは、呼んだ次の周期の頭（最大1周期）で、`stop`が戻るのは前の更新が効いた後になる。**STOPの同期した実測はまだ無い。**servoがpulseをどの刻みで読むかは確かめていない。制御周期とPWMの
//!   周期の関係によって、実機が受け取る列はLEDCへ設定した列と時間の刻みが違いうる。
//!   **制御周期は段3で決める**（このmoduleは値を持たない）
//! - **servoが受け取った位置へどう追従するか**（速さ、遅れ、行き過ぎ）は確かめていない
//! - **[`LimitedServo::stop`]の後にservoがどう止まるか。**信号を止めてもservo側が駆動を止める
//!   保証は無い（[`Sg90::stop`]のdoc）
//! - **再開の位置は、最後に出力した`Setpoint`の位置であり、実機に届いたとは限らない。**次のframeより前に
//!   `stop`のduty 0が上書きした場合や、duty 0の間にservoが押されて動いた場合は、実機の位置と食い違う
//! - **reset、panic、drop、`core::mem::forget`の後の出力は保証しない。**範囲外のpulseになりうる。
//!   **止めるのは、人が外部電源を切ることである。**panicの方式はabortである（target
//!   `xtensa-esp32-espidf`の`panic-strategy`が`abort`であり、`firmware/esp32/.cargo/config.toml`の
//!   `build-std`も`panic_abort`を使う）ため、panicでは`Drop`が走らない。ESP-IDF v5.5.3では、newlibの
//!   `abort()`が`esp_system_abort`を経て`panic_abort`へ進み（`components/newlib/src/abort.c`、
//!   `components/esp_system/port/esp_system_chip.c`、`components/esp_system/panic.c`）、panic handler
//!   （生成される`sdkconfig`の`CONFIG_ESP_SYSTEM_PANIC_PRINT_REBOOT=y`）は、cache errorのときは
//!   `esp_restart_noos_dig`へ、それ以外は`esp_restart`と同じ`esp_restart_noos`へ進む
//!   （`components/esp_system/port/panic_handler.c`の`panic_restart`）。`esp_restart_noos`は、reset
//!   するperipheralにLEDCを含まず（`components/esp_system/port/soc/esp32/system_internal.c`、
//!   `esp_system.h`の`esp_restart`のdoc）、reset前にCPUとAPBのclockを切り替える（同fileの
//!   `rtc_clk_cpu_set_to_default_config`）。LEDCの`Drop`は`ledc_stop`を呼ぶが、その失敗を`unwrap`する。
//!   **経路ごとの分析と、起動時にservoのLEDCを最初に止める要件は、[#19のコメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/19#issuecomment-5891754488)に置いた。**
//!
//! **`Setpoint`の位置の単位は、下の角度規約の度である**（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜
//! `_MAX_DEG`）。`Limiter`へ渡す可動域・速度・加速度もこの単位で与える。**値はこのmoduleが
//! 持たない。**どれも`HW-TBD-010`／`011`／`020`で未確定であり、呼び出し側が渡す。
//! 度からpulse幅への変換は、下の一般値による線形変換のままであり、**calibration済みの
//! 変換ではない**（`HW-TBD-026`／`010`）。
//!
//! **既定build（`bench-servo-test-17`も`bench-servo-calib-18`も付けないbuild）は`main()`からこのmoduleを
//! 呼ばない**（gate状態は[TBD台帳](../../../docs/hardware/tbd-register.md)、理由は
//! [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
//! `サーボ出力を有効化してよい条件`が正本）。`bench-servo-test-17` feature付きbuild
//! が[`run_servo_bench_test`](../../../firmware/esp32/src/main.rs)経由で、`bench-servo-calib-18` feature付きbuild
//! が[`servo_calib::run`](../../../firmware/esp32/src/servo_calib.rs)経由で呼ぶ
//! （承認の状態は同文書の`承認の状態`節が正本）。
//!
//! pin割り当ては[gpio-assignment.md](../../../docs/hardware/gpio-assignment.md)の
//! `信号inventory`が正本（`SERVO-PWM`＝GPIO27、[`crate::config::SERVO_PWM_GPIO`]）。
//! 電源は外部5 V系（`PSU-SERVO-01`）から取る（ESP32の電源pinからは給電しない）。
//! このmoduleは信号線だけを扱い、電源配線は対象外。
//!
//! PWM周期・pulse幅・角度規約は一次資料に基づかない一般値であり、確定は
//! `HW-TBD-026`／`010`の範囲。詳細は
//! [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の残余riskを参照。

// 既定buildは`main()`からこのmoduleを呼ばないためdead_codeになる（module doc参照）。
// `Sg90`の一部は`bench-servo-test-17` feature付きbuildで呼ばれる。
// `LimitedServo`、`Stopped`、`ServoFaultCounters`は、`bench-servo-calib-18`付きのbuild（`crate::servo_calib`）と`crate::servo_path`だけが呼ぶ。
#![allow(dead_code)]

use core::fmt;
use std::time::Instant;

use deskcat_servo::{
    AdmittedTarget, ControlPeriod, Limiter, LimiterCounters, LimitsError, MotionCatalog,
    MotionRequest, OutputDecimator, Rejection, ServoLimits, Setpoint,
};
use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

/// `SERVO-01`（SG90）のPWM driver。hardware LEDCでpulseを生成する（module doc参照）。
/// `Sg90::new`はLEDC channelを初期duty 0%で設定する。**同じ起動の中で`Sg90::new`の後は、**
/// [`LimitedServo::tick`]が出力するまで、このmoduleは有効なservo pulseを設定しない
/// （`bench-servo-test-17`付きbuildでは`move_to_angle_once`も同じ）。**resetやpanicの後、`Sg90::new`より
/// 前の出力は保証しない**（module docの「実機での実現は保証しない」）。**角度を出す関数はこのmoduleのprivateである**（module doc参照）。
pub struct Sg90<'d> {
    driver: LedcDriver<'d>,
}

impl<'d> Sg90<'d> {
    /// LEDC timer・channel・pinを受け取ってdriverを作る。周期は
    /// [`crate::config::SERVO_PWM_FREQUENCY_HZ`]を使う。
    pub fn new<T, C>(timer: T, channel: C, pin: impl OutputPin + 'd) -> Result<Self, EspError>
    where
        T: LedcTimer + 'd,
        C: LedcChannel<SpeedMode = T::SpeedMode> + 'd,
    {
        // esp-idf-halはpinをtype levelで選ぶため、config::SERVO_PWM_GPIOはpin選択には
        // 使えない（呼び出し側が手書きする）。一致を実行時に確認する。
        debug_assert_eq!(
            pin.pin(),
            crate::config::SERVO_PWM_GPIO,
            "呼び出し側が渡したpinがconfig::SERVO_PWM_GPIOと一致しない"
        );

        // 既定resolution（Bits8、max_duty=256）は20 msの周期に対し1 stepが約78 µsと粗く、
        // 1000〜2000 µsのpulse幅範囲を約13 stepでしか刻めない。Bits16（約0.31 µs/step）へ
        // 上げる。検討経緯はPR本文参照。
        let timer_driver = LedcTimerDriver::new(
            timer,
            &TimerConfig::new()
                .frequency(crate::config::SERVO_PWM_FREQUENCY_HZ.Hz())
                .resolution(esp_idf_svc::hal::ledc::config::Resolution::Bits16),
        )?;
        let driver = LedcDriver::new(channel, timer_driver, pin)?;
        Ok(Self { driver })
    }

    /// 角度規約の度を、LEDCのduty（量子化の前の小数）へ一般値の線形変換で写す。
    ///
    /// 角度規約の範囲（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）へclampするが、
    /// **これは安全制限ではない。**可動域を強制するのは`Limiter`と[`DutyBounds`]である。
    fn exact_duty(&self, angle_deg: f32) -> f64 {
        let min_deg = f64::from(crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG);
        let max_deg = f64::from(crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG);
        let min_us = f64::from(crate::config::SERVO_PULSE_WIDTH_MIN_US);
        let max_us = f64::from(crate::config::SERVO_PULSE_WIDTH_MAX_US);
        let period_us = 1_000_000.0 / f64::from(crate::config::SERVO_PWM_FREQUENCY_HZ);
        let angle_deg = f64::from(angle_deg).clamp(min_deg, max_deg);
        let pulse_us = min_us + (angle_deg - min_deg) / (max_deg - min_deg) * (max_us - min_us);
        pulse_us * f64::from(self.driver.get_max_duty()) / period_us
    }

    /// 承認値の位置範囲の両端を、dutyの整数の**内側**へ丸めた範囲。範囲の中にdutyの整数が
    /// 1つも無ければ`None`。
    fn duty_bounds(&self, approved_min_deg: f32, approved_max_deg: f32) -> Option<DutyBounds> {
        let low = self.exact_duty(approved_min_deg).ceil();
        let high = self.exact_duty(approved_max_deg).floor();
        if low > high {
            return None;
        }
        Some(DutyBounds {
            low: low as u32,
            high: high as u32,
        })
    }

    /// 角度規約の度でpulseを出す。**privateであり、[`LimitedServo::tick`]だけが呼ぶ**
    /// （module doc参照）。
    ///
    /// dutyへ直接量子化し（最も近い整数）、`bounds`の内側へclampする。**量子化しても、設定する
    /// dutyは承認値の位置範囲の両端を変換した値の内側に収まる。**
    fn output_angle(&mut self, angle_deg: f32, bounds: DutyBounds) -> Result<(), EspError> {
        let duty = (self.exact_duty(angle_deg).round() as u32).clamp(bounds.low, bounds.high);
        self.driver.set_duty(duty)
    }

    /// 指定した角度（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）へ**1回だけ**動かす
    /// （連続動作は行わない）。中央からの偏角を
    /// [`crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG`]でclampする
    /// （暫定値。同定数のdoc参照）。
    ///
    /// **limiterを通らない。**そのため`bench-servo-test-17`付きbuildでだけcompileする
    /// （module doc参照）。
    #[cfg(feature = "bench-servo-test-17")]
    pub fn move_to_angle_once(&mut self, angle_deg: f32) -> Result<(), EspError> {
        let neutral = crate::config::SERVO_ANGLE_CONVENTION_NEUTRAL_DEG;
        let max_deviation = crate::config::SERVO_FIRST_MOTION_MAX_DEVIATION_DEG;
        let clamped_angle_deg = angle_deg.clamp(neutral - max_deviation, neutral + max_deviation);

        let pulse_width_us = pulse_width_us_for_angle(clamped_angle_deg);
        let duty = self.duty_for_pulse_width_us(pulse_width_us);
        self.driver.set_duty(duty)
    }

    /// dutyを0へ戻し、pulseの生成を止める。**信号を止めてもservo側が駆動を止める
    /// 保証は無い**（詳細は
    /// [servo-safety-limits.md](../../../docs/hardware/servo-safety-limits.md)の
    /// 残余risk参照）。拘束を確実に止める手段は人間による外部電源の遮断である。
    ///
    /// `Self`が後でdropされると、`esp-idf-hal`の`LedcDriver::drop`が`ledc_stop`を
    /// idle level `0`で呼び、GPIO27をlowへ固定する（high-Zには戻らない）。
    pub fn stop(&mut self) -> Result<(), EspError> {
        self.driver.set_duty(0)
    }

    /// pulse幅（マイクロ秒）をLEDCのduty値（`set_duty`が受ける単位）へ変換する。
    ///
    /// `duty = pulse_width_us / period_us * max_duty`。`period_us`は
    /// [`crate::config::SERVO_PWM_FREQUENCY_HZ`]から求める。
    fn duty_for_pulse_width_us(&self, pulse_width_us: u32) -> u32 {
        let period_us = 1_000_000 / crate::config::SERVO_PWM_FREQUENCY_HZ;
        let max_duty = self.driver.get_max_duty();
        ((u64::from(pulse_width_us) * u64::from(max_duty)) / u64::from(period_us)) as u32
    }
}

/// 承認値の位置範囲の両端を、dutyの整数の内側へ丸めた範囲（[`Sg90::output_angle`]が使う）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DutyBounds {
    low: u32,
    high: u32,
}

/// [`LimitedServo`]が自分で数えるcounter。**[`Stopped`]へ引き継ぎ、[`LimitedServo::resume`]の後も残る。**
///
/// `Limiter`のcounter（[`LimitedServo::counters`]）は`stop`で`Limiter`ごと捨てるため、latchの原因は
/// こちらで数える。値（しきい値）は持たない。どれも`u32::MAX`で飽和し、wrapしない。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServoFaultCounters {
    /// 測った間隔（各`tick`の間隔、または`Limiter::step`へ渡す窓の間隔）が許容の外でrejectされてlatchした回数
    /// （[`TickError::Rejected`]）。**各`tick`の間隔の検査は`LimitedServo`自身が行うので、`Limiter`のcounterには入らない。**
    pub interval_rejections: u32,
    /// PWMの出力に失敗してlatchした回数（[`TickError::Output`]）。
    pub output_failures: u32,
    /// `Limiter`が加速度の上限を譲ったSetpointを返し、出力せずにlatchした回数
    /// （[`TickError::AccelerationConceded`]）。
    pub acceleration_conceded: u32,
    /// latchの後に`tick`が呼ばれ、[`TickError::Faulted`]を返した回数。
    pub faulted_ticks: u32,
}

/// [`Sg90`]と`deskcat_servo::Limiter`を1つずつ所有し、limiterを通った出力だけを出す
/// （module doc参照）。
///
/// **値を持たない。**制限（`ServoLimits`）、制御周期（`ControlPeriod`）、受理するmotion名
/// （`MotionCatalog`）、起動時の初期位置は呼び出し側が渡す。**`Limiter`そのものは受け取らず、
/// この型の中で作る**（速度0から始まる`Limiter`だけを持つため）。
pub struct LimitedServo<'d> {
    servo: Sg90<'d>,
    limiter: Limiter,
    target: Option<AdmittedTarget>,
    /// 承認値の位置範囲をdutyの内側へ丸めた範囲。`new`／`resume`で1回だけ求める。
    bounds: DutyBounds,
    /// 自分で数えるcounter。
    faults: ServoFaultCounters,
    /// 前回の`tick`の時刻。`new`／`resume`の後で、targetがあるときの最初の`tick`で基準として取る
    /// （module docの「間隔の基準と、tickを呼び続ける前提」）。
    last_tick: Option<Instant>,
    /// 各`tick`の間隔の許容（呼び出し側が渡した`ControlPeriod`）。**`Limiter`が持つのは、これをN倍した窓の許容である。**
    tick_period: ControlPeriod,
    /// 最後に出力した時刻（出力が無ければ基準の`tick`の時刻）。窓（`Limiter::step`へ渡す間隔）の始まり。
    window_start: Option<Instant>,
    /// 再開の位置（角度規約の度）。**最後に出力した**`Setpoint`の位置（量子化の前）、一度も出力して
    /// いなければ起動時の初期位置。`Limiter`は出力するtickでしか進めないので、latchしない限り`Limiter`の位置と同じになる（加速度の譲りと出力の失敗でlatchしたときは`Limiter`が先へ進んでいるが、再開はこの位置から作り直す）。
    resume_position: f32,
    /// LEDCへ出力し、`Limiter`を1 step進めるtickの判定（N tickに1回。[`crate::config::SERVO_OUTPUT_DECIMATION_TICKS`]）。
    decimator: OutputDecimator,
    /// latchしているか（module docの「制御周期の外の間隔、加速度の譲り、出力の失敗ではlatchする」）。
    faulted: bool,
}

/// [`LimitedServo::stop`]の後の状態。[`Sg90`]と再開の位置を持つ。
///
/// **[`Sg90`]を取り出す手段は無い。**再開は[`LimitedServo::resume`]だけであり、
/// 再開の位置を呼び出し側から受け取らない（module docの「止めた後の再開」）。
pub struct Stopped<'d> {
    servo: Sg90<'d>,
    resume_position: f32,
    faults: ServoFaultCounters,
}

impl Stopped<'_> {
    /// 再開の位置（角度規約の度。log用）。最後に出力した`Setpoint`の位置（量子化の前。間引いた`tick`には位置が作られない）、
    /// 一度も出力していなければ起動時の初期位置である。**これを渡す入口は無い。**
    #[must_use]
    pub fn resume_position(&self) -> f32 {
        self.resume_position
    }

    /// 引き継いだcounter（[`ServoFaultCounters`]）。
    #[must_use]
    pub fn fault_counters(&self) -> ServoFaultCounters {
        self.faults
    }
}

/// [`LimitedServo::new`]と[`LimitedServo::resume`]が拒否した理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitedServoError {
    /// `Limiter::new`が拒否した（初期位置が有限でない、または承認値の位置範囲の外）。
    Limits(LimitsError),
    /// 承認値の位置範囲が、角度規約（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）の外へ
    /// はみ出している。
    ///
    /// はみ出した分はpulse幅の変換で黙って飽和し、`Limiter`が認めた位置と
    /// 実際に出す位置が食い違う。**黙って受けずに構築を拒否する。**
    ApprovedRangeOutsideAngleConvention,
    /// 承認値の位置範囲の中に、LEDCのdutyの整数が1つも無い（内側へ丸めると範囲が空になる）。
    ApprovedRangeNarrowerThanDutyStep,
}

impl fmt::Display for LimitedServoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limits(err) => write!(f, "limiter rejected the configuration: {err}"),
            Self::ApprovedRangeOutsideAngleConvention => {
                f.write_str("approved position range lies outside the servo angle convention")
            }
            Self::ApprovedRangeNarrowerThanDutyStep => {
                f.write_str("approved position range is narrower than one LEDC duty step")
            }
        }
    }
}

/// [`LimitedServo::tick`]の失敗。**どれを受けても、呼び出し側は[`LimitedServo::stop`]を呼ぶ。**
#[derive(Debug)]
pub enum TickError {
    /// 測った間隔（各`tick`の間隔は`LimitedServo`が、窓の間隔は`Limiter::step`が検査する）が制御周期の範囲外、または0で、rejectされた。
    /// **何も出力せず、`Limiter`も進めていない。**ここでlatchする。
    Rejected(Rejection),
    /// `Limiter::step`は通ったが、PWMの出力に失敗した。
    ///
    /// **`Limiter`の位置と速度はこのstep分だけ進んでいる。**戻さずにここでlatchする。
    Output(EspError),
    /// `Limiter::step`が加速度の上限を譲ったSetpoint（`ClampReport::acceleration_bound_conceded`）を
    /// 返した。**出力せず**、ここでlatchする。crateのdocはこのflagを「立ったら実装の前提が
    /// 破れている」としている。`Limiter`はこのstep分だけ進んでいる。
    AccelerationConceded,
    /// 以前の`tick`でlatchしている。**何も出力せず、`Limiter`も進めていない。**
    Faulted,
}

impl fmt::Display for TickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(rejection) => {
                write!(f, "the measured interval was rejected: {rejection}")
            }
            Self::Output(err) => write!(f, "servo output failed: {err}"),
            Self::AccelerationConceded => {
                f.write_str("limiter conceded the acceleration bound; output withheld")
            }
            Self::Faulted => f.write_str("servo output is latched after an earlier failure"),
        }
    }
}

/// 出力を間引く判定の初期状態。Nは[`crate::config::SERVO_OUTPUT_DECIMATION_TICKS`]。
/// **定数として評価するので、Nが0ならこのmoduleのcompileが止まる**（`mod servo`はfeatureによらずcompileされる）。
const NEW_DECIMATOR: OutputDecimator =
    match OutputDecimator::new(crate::config::SERVO_OUTPUT_DECIMATION_TICKS) {
        Some(decimator) => decimator,
        None => panic!("SERVO_OUTPUT_DECIMATION_TICKSは1以上にする"),
    };

/// 制限がservoの角度規約とdutyの刻みに収まるかを確かめ、速度0の`Limiter`と、
/// 承認値の位置範囲をdutyの内側へ丸めた範囲を作る。
fn build_limiter(
    servo: &Sg90<'_>,
    limits: ServoLimits,
    period: ControlPeriod,
    catalog: MotionCatalog,
    start_position: f32,
) -> Result<(Limiter, DutyBounds), LimitedServoError> {
    let range = limits.position();
    if range.approved_min() < crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG
        || range.approved_max() > crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG
    {
        return Err(LimitedServoError::ApprovedRangeOutsideAngleConvention);
    }
    let bounds = servo
        .duty_bounds(range.approved_min(), range.approved_max())
        .ok_or(LimitedServoError::ApprovedRangeNarrowerThanDutyStep)?;
    // `Limiter`は出力するtick（N tickに1回）でしか進めないので、`Limiter::step`へ渡す間隔は窓（Nつの`tick`の間隔の和）である。
    // 各`tick`の間隔が許容の内側なら和は`N × 公称 ± N × 許容`の内側なので、窓の許容は呼び出し側が渡した値から導く
    // （新しい値を決めない。`tolerance < nominal`はN倍しても保たれる）。
    let n = crate::config::SERVO_OUTPUT_DECIMATION_TICKS as f32;
    let window_period = ControlPeriod::new(period.nominal_s() * n, period.tolerance_s() * n)
        .map_err(LimitedServoError::Limits)?;
    let limiter = Limiter::new(limits, window_period, catalog, start_position)
        .map_err(LimitedServoError::Limits)?;
    Ok((limiter, bounds))
}

impl<'d> LimitedServo<'d> {
    /// 起動時に[`Sg90`]を所有し、速度0の`Limiter`を作る。**この時点では何も出力しない。**
    ///
    /// `initial_position`は実機の位置と一致する保証が無い（module docの「既知の制限: 起動時の初期位置」）。
    ///
    /// # Errors
    ///
    /// [`LimitedServoError`]のいずれかに当たる場合に拒否し、[`Sg90`]を返す。
    pub fn new(
        servo: Sg90<'d>,
        limits: ServoLimits,
        period: ControlPeriod,
        catalog: MotionCatalog,
        initial_position: f32,
    ) -> Result<Self, (Sg90<'d>, LimitedServoError)> {
        match build_limiter(&servo, limits, period, catalog, initial_position) {
            Ok((limiter, bounds)) => Ok(Self {
                servo,
                limiter,
                target: None,
                bounds,
                faults: ServoFaultCounters::default(),
                last_tick: None,
                tick_period: period,
                window_start: None,
                resume_position: initial_position,
                decimator: NEW_DECIMATOR,
                faulted: false,
            }),
            Err(err) => Err((servo, err)),
        }
    }

    /// [`LimitedServo::stop`]の後に、[`Stopped`]が持つ再開の位置から、速度0の`Limiter`で
    /// 作り直す。**位置は受け取らない。この時点では何も出力しない。**[`ServoFaultCounters`]は引き継ぐ。
    ///
    /// # Errors
    ///
    /// [`LimitedServoError`]のいずれかに当たる場合（再開の位置が新しい制限の承認値の範囲外に
    /// ある場合を含む）に拒否し、[`Stopped`]を返す。
    pub fn resume(
        stopped: Stopped<'d>,
        limits: ServoLimits,
        period: ControlPeriod,
        catalog: MotionCatalog,
    ) -> Result<Self, (Stopped<'d>, LimitedServoError)> {
        match build_limiter(
            &stopped.servo,
            limits,
            period,
            catalog,
            stopped.resume_position,
        ) {
            Ok((limiter, bounds)) => Ok(Self {
                servo: stopped.servo,
                limiter,
                target: None,
                bounds,
                faults: stopped.faults,
                last_tick: None,
                tick_period: period,
                window_start: None,
                resume_position: stopped.resume_position,
                decimator: NEW_DECIMATOR,
                faulted: false,
            }),
            Err(err) => Err((stopped, err)),
        }
    }

    /// motion commandを自分の`Limiter::admit`へ通し、受理したtargetを内部に持つ。
    ///
    /// **Pi由来でもdebug由来でも同じ判定を通る。**直前のtargetは置き換える。
    /// **このcallは出力しない。**出力は[`LimitedServo::tick`]で行う。
    ///
    /// # Errors
    ///
    /// `Limiter::admit`のrejectをそのまま返す。rejectしたときは直前のtargetを保つ。
    pub fn admit(&mut self, request: &MotionRequest<'_>) -> Result<(), Rejection> {
        let admitted = self.limiter.admit(request)?;
        self.target = Some(admitted);
        Ok(())
    }

    /// 前回の`tick`からの経過時間を自分で測って毎回検査し（許容の外ならlatchする）、**N回に1回だけ**
    /// （[`crate::config::SERVO_OUTPUT_DECIMATION_TICKS`]）、内部のtargetへ向けて自分の`Limiter::step`を
    /// **窓（最後の出力から今回の出力までに測った時間）**だけ進め、結果をLEDCへ設定する
    /// （実機でいつ・どう効くかはmodule docの「実機での実現は保証しない」）。
    ///
    /// **何も設定せず`Ok(None)`を返す場合が3つある。**targetが無い（まだ`admit`していない）とき、
    /// `new`／`resume`の後でtargetがあるときの最初の`tick`（時刻の基準を取るだけ。
    /// module docの「間隔の基準と、tickを呼び続ける前提」）、**間引いて出力しない`tick`**（このとき`Limiter`は進めない）
    /// である。最後は`Setpoint`を返さない（呼び出し側が「出した位置」として扱わないため）。起動時の出力は
    /// `HW-TBD-019`で未確定であり、`servo-safety-limits.md`の`起動時とdriver故障時の動作`が
    /// 「承認されるまで、安全状態は『未検証の動作出力を行わない』とする」としているためである。
    ///
    /// 返す`Setpoint`は**出力した値の報告**（log用。量子化の前の値）である。これを受け取る入口は無い。
    /// `Some`は「このtickでLEDCへ設定した」ことを表す。
    ///
    /// # Errors
    ///
    /// 測った間隔（各`tick`の間隔、または出力する`tick`で`Limiter::step`へ渡す窓の間隔）が許容の外でrejectされた場合は
    /// [`TickError::Rejected`]、加速度の上限を譲った
    /// Setpointが返った場合は[`TickError::AccelerationConceded`]（どちらも何も設定しない）、
    /// PWMの出力に失敗した場合は[`TickError::Output`]を返し、どれもここでlatchする。
    /// latchの後は[`TickError::Faulted`]を返す。どれも[`ServoFaultCounters`]に数える。
    pub fn tick(&mut self) -> Result<Option<Setpoint>, TickError> {
        if self.faulted {
            self.faults.faulted_ticks = self.faults.faulted_ticks.saturating_add(1);
            return Err(TickError::Faulted);
        }
        let Some(target) = self.target else {
            return Ok(None);
        };
        let now = Instant::now();
        let Some(previous) = self.last_tick.replace(now) else {
            self.window_start = Some(now);
            return Ok(None);
        };
        // 各`tick`の間隔は毎回検査する（間引いた`tick`でも）。外れたらlatchする。
        let tick_s = now.duration_since(previous).as_secs_f32();
        if !self.tick_period.accepts(tick_s) {
            self.faulted = true;
            self.faults.interval_rejections = self.faults.interval_rejections.saturating_add(1);
            return Err(TickError::Rejected(Rejection::IntervalOutsideControlPeriod));
        }
        // `Limiter`が進むのは出力するtick（N tickに1回）だけである。間引いた`tick`では何も進めない。
        if !self.decimator.advance() {
            return Ok(None);
        }
        let window_start = self.window_start.replace(now).unwrap_or(previous);
        let dt_s = now.duration_since(window_start).as_secs_f32();
        let setpoint = match self.limiter.step(target, dt_s) {
            Ok(setpoint) => setpoint,
            Err(rejection) => {
                self.faulted = true;
                self.faults.interval_rejections = self.faults.interval_rejections.saturating_add(1);
                return Err(TickError::Rejected(rejection));
            }
        };
        if setpoint.clamps().acceleration_bound_conceded {
            self.faulted = true;
            self.faults.acceleration_conceded = self.faults.acceleration_conceded.saturating_add(1);
            return Err(TickError::AccelerationConceded);
        }
        if let Err(err) = self.servo.output_angle(setpoint.position(), self.bounds) {
            self.faulted = true;
            self.faults.output_failures = self.faults.output_failures.saturating_add(1);
            return Err(TickError::Output(err));
        }
        self.resume_position = setpoint.position();
        Ok(Some(setpoint))
    }

    /// dutyを0へ戻し（[`Sg90::stop`]）、**自分を消費して**[`Stopped`]を返す。
    ///
    /// `Limiter`とtargetはここで捨てる。[`ServoFaultCounters`]は[`Stopped`]へ引き継ぐ。
    /// 再開は[`LimitedServo::resume`]だけである（module docの「止めた後の再開」）。
    ///
    /// **dutyを0にするだけで、軌道の制限を受けない**（減速しない。module docの「型が保証すること」）。
    ///
    /// 返す`Result`は[`Sg90::stop`]の結果である。**失敗を握りつぶさない。失敗したら、人が外部電源を
    /// 切る。**dropやpanicの後の出力は保証しない（module docの「実機での実現は保証しない」）。
    #[must_use = "dutyを0へ戻せたかをResultで確かめること"]
    pub fn stop(self) -> (Stopped<'d>, Result<(), EspError>) {
        let Self {
            mut servo,
            resume_position,
            faults,
            ..
        } = self;
        let result = servo.stop();
        (
            Stopped {
                servo,
                resume_position,
                faults,
            },
            result,
        )
    }

    /// 自分の`Limiter`の累計counter（#19の受け入れ条件5）。
    ///
    /// **[`LimitedServo::stop`]で`Limiter`ごと捨てるため、`stop`と`resume`を越えて残らない。**
    /// latchの原因は[`LimitedServo::fault_counters`]が数え、そちらは残る。
    #[must_use]
    pub fn counters(&self) -> LimiterCounters {
        self.limiter.counters()
    }

    /// 自分で数えるcounter（[`ServoFaultCounters`]）。`stop`と`resume`を越えて残る。
    #[must_use]
    pub fn fault_counters(&self) -> ServoFaultCounters {
        self.faults
    }
}

/// 角度（`SERVO_ANGLE_CONVENTION_MIN_DEG`〜`_MAX_DEG`）をpulse幅（マイクロ秒）へ
/// 線形変換する（module doc参照）。
fn pulse_width_us_for_angle(angle_deg: f32) -> u32 {
    exact_pulse_width_us_for_angle(angle_deg).round() as u32
}

/// [`pulse_width_us_for_angle`]の、丸める前の値（マイクロ秒）。**LEDCの量子化とclampの前の、一般値の
/// 線形変換の値である**（`bench-servo-calib-18`がlogへ出す`pulse_us`。`crate::servo_calib`）。
pub(crate) fn exact_pulse_width_us_for_angle(angle_deg: f32) -> f32 {
    let min_deg = crate::config::SERVO_ANGLE_CONVENTION_MIN_DEG;
    let max_deg = crate::config::SERVO_ANGLE_CONVENTION_MAX_DEG;
    let min_us = crate::config::SERVO_PULSE_WIDTH_MIN_US as f32;
    let max_us = crate::config::SERVO_PULSE_WIDTH_MAX_US as f32;

    let angle_deg = angle_deg.clamp(min_deg, max_deg);
    let ratio = (angle_deg - min_deg) / (max_deg - min_deg);
    min_us + ratio * (max_us - min_us)
}
