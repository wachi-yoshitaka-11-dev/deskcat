//! 動作中のconsoleの取り出しの上限（Issue #751の7）。
//!
//! 入力が続いてqueueが空にならなくても、1周あたりの取り出しを[`drain_bounded`]の`max`件で打ち切り、
//! 露出とsessionの期限の確認とwatchdogの給電へ戻る。残りはqueueに残り、次の周で取り出す
//! （`STOP`もqueueの順に届く。捨てない）。servoの出力はここを通らない（`Limiter`だけが出す）。

use std::ops::ControlFlow;

/// 取り出しの結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drained {
    /// 取り出して`handle`へ渡した件数。
    pub taken: usize,
    /// `handle`が`Break`を返して止めたか（`false`なら、空になったか上限に達した）。
    pub stopped: bool,
}

/// `try_recv`が`None`を返すか、`handle`が`Break`を返すか、`max`件に達したところで止める。
pub fn drain_bounded<T>(
    max: usize,
    mut try_recv: impl FnMut() -> Option<T>,
    mut handle: impl FnMut(T) -> ControlFlow<()>,
) -> Drained {
    let mut taken = 0;
    while taken < max {
        let Some(event) = try_recv() else {
            break;
        };
        taken += 1;
        if handle(event).is_break() {
            return Drained {
                taken,
                stopped: true,
            };
        }
    }
    Drained {
        taken,
        stopped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_never_ending_stream_is_cut_at_the_limit() {
        let mut handled = 0;
        let d = drain_bounded(
            16,
            || Some(()),
            |()| {
                handled += 1;
                ControlFlow::Continue(())
            },
        );
        assert_eq!(handled, 16);
        assert_eq!(
            d,
            Drained {
                taken: 16,
                stopped: false
            }
        );
    }

    #[test]
    fn an_empty_queue_ends_the_drain_early() {
        let mut q = vec![1, 2];
        let d = drain_bounded(16, || q.pop(), |_| ControlFlow::Continue(()));
        assert_eq!(
            d,
            Drained {
                taken: 2,
                stopped: false
            }
        );
    }

    #[test]
    fn stop_within_the_limit_ends_the_drain_and_leaves_the_rest_queued() {
        let mut q: std::collections::VecDeque<&str> = ["a", "STOP", "b"].into();
        let d = drain_bounded(
            16,
            || q.pop_front(),
            |e| {
                if e == "STOP" {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        );
        assert_eq!(
            d,
            Drained {
                taken: 2,
                stopped: true
            }
        );
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn stop_beyond_the_limit_is_reached_on_the_next_call() {
        // 上限の後ろにいる`STOP`は捨てられず、次の周で届く。
        let mut q: std::collections::VecDeque<u32> = (0..10).collect();
        let is_stop = |e: u32| {
            if e == 9 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let first = drain_bounded(4, || q.pop_front(), is_stop);
        assert!(!first.stopped);
        let second = drain_bounded(4, || q.pop_front(), is_stop);
        assert!(!second.stopped);
        let third = drain_bounded(4, || q.pop_front(), is_stop);
        assert!(third.stopped);
    }
}
