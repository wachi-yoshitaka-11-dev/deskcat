//! #23 の `PROTO-TBD-007`（LCD layout と memory 測定）のための RAM 計測 log。
//!
//! **`mem-probe-23` feature を付けた試験 build だけが、この module を持つ。**既定 build（製品 build）は
//! この module をcompileせず、`unsafe`も持たない。この feature は log を足すだけで、LCD・Pi link・tap の
//! 挙動を変えない。ただし feature を1つでも指定すると製品経路の cfg（`deskcat_product_mode`）が外れる
//! （`build.rs`）ため、製品build と同じ構成の計測 build は `--features mem-probe-23,face-21,tap-21` になる（ただし、タッチの撫での判定（`crate::touch::StrokeDetector`）は製品 build だけが持ち、この構成には入らない）。
//!
//! # 出すもの
//!
//! health snapshot と同じ周期（`config::HEALTH_SNAPSHOT_PERIOD_MS`）で、main loop から1行出す。
//! 描画のたびには出さない。
//!
//! ```text
//! mem_probe uptime_ms=<..> free_heap=<byte> min_free_heap=<byte> main_stack_hwm=<byte>
//! ```
//!
//! - `free_heap`：その時点の空き heap（`esp_get_free_heap_size`）。
//! - `min_free_heap`：起動から今までの最小の空き heap（`esp_get_minimum_free_heap_size`）。
//!   ESP-IDF が内部で持つ最小値で、log の周期の間に落ちた値も含む。
//! - `main_stack_hwm`：main task の stack の high water mark、つまり起動から今までに一番深く使ったときの
//!   **残り**（`uxTaskGetStackHighWaterMark(NULL)`）。単位は byte（ESP-IDF の FreeRTOS は
//!   `StackType_t`が`uint8_t`）。値が小さいほど stack を使っている。
//!
//! **測れないもの。**firmware は main loop 1本で、thread を使わない。読むのは main task だけである。
//! ESP-IDF 内部の task の stack は読まない（`uxTaskGetSystemState`は
//! `CONFIG_FREERTOS_USE_TRACE_FACILITY`の sdkconfig 変更が要り、製品 build の設定に触れるため採らない）。
//! heap の `min_free_heap`は全体の最小であり、どの処理が下げたかは分からない。
//!
//! # `unsafe`
//!
//! この crate は`unsafe_code = "forbid"`だが、heap と stack の残量を読む ESP-IDF の API は
//! すべて`unsafe extern "C"`で、esp-idf-hal 0.46.2 と esp-idf-svc 0.52.1 に safe の wrapper が無い
//! （ソースを`free_heap`・`HighWaterMark`・`high_water`で grep して0件）。**この feature のときだけ**
//! `main.rs`の crate 属性が`forbid`を`deny`へ下げ、この module の下の3つの`unsafe`だけを`allow`する。
//! feature を付けない build は`forbid`のままである（`main.rs`の先頭の`cfg_attr`）。
//!
//! 3つの呼び出しに共通する不変条件は、各`unsafe`の直前の`SAFETY`に書く。
#![allow(unsafe_code)]

use esp_idf_svc::sys::{
    esp_get_free_heap_size, esp_get_minimum_free_heap_size, uxTaskGetStackHighWaterMark,
};

/// 1回分の読み。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSnapshot {
    /// その時点の空き heap（byte）。
    pub free_heap: u32,
    /// 起動から今までの最小の空き heap（byte）。
    pub min_free_heap: u32,
    /// main task の stack の high water mark（残りの最小値。byte）。
    pub main_stack_hwm: u32,
}

/// heap と main task の stack の残量を読む。
///
/// **main loop（task context）からだけ呼ぶ。ISR からは呼ばない。**
pub fn read() -> MemSnapshot {
    // SAFETY: 引数も前提条件も無い読み取り専用の関数である。ESP-IDF の heap の統計を読むだけで、
    // heap を書き換えも確保もしない。task context（main loop）から呼ぶ。ISR からは呼ばない。
    let free_heap = unsafe { esp_get_free_heap_size() };
    // SAFETY: 同上。引数も前提条件も無く、最小の空き heap の統計を読むだけである。
    let min_free_heap = unsafe { esp_get_minimum_free_heap_size() };
    // SAFETY: 引数は NULL（呼んだ task 自身、つまり main task）であり、他の task の handle を渡さない。
    // 読み取り専用で、stack を書き換えない。task context（main loop）から呼び、ISR からは呼ばない。
    let main_stack_hwm = unsafe { uxTaskGetStackHighWaterMark(core::ptr::null_mut()) };
    MemSnapshot {
        free_heap,
        min_free_heap,
        main_stack_hwm,
    }
}

/// `mem_probe`の1行をlogへ出す。
pub fn log(now_ms: u64) {
    let m = read();
    log::info!(
        "mem_probe uptime_ms={now_ms} free_heap={} min_free_heap={} main_stack_hwm={}",
        m.free_heap,
        m.min_free_heap,
        m.main_stack_hwm
    );
}
