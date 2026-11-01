//! debug consoleの1行を、motion指示へ読む（Issue #19の受け入れ条件3）。
//!
//! 形式は`MOTION <name> <target>`（空白区切り）である。**この層は読むだけで、判定しない。**
//! 名前がcatalogにあるか、targetが有限か、範囲の内か、clampが要るかは、
//! [`MotionDispatch::handle_debug`]と`Limiter::admit`が判定する。ここで先に弾くと、
//! Pi由来の指示と違う基準でrejectすることになり、counterにも数えられない。
//! **`target`には`nan`や`inf`も読める。**それを有限でないとしてrejectするのは`Limiter::admit`である。
//!
//! `speed`と`repeat`は扱わない（`PROTO-TBD-008`の上限が決まるまで、debugの指示に足さない）。
//!
//! [`MAX_DEBUG_LINE_BYTES`]は読む側のbufferの上限であり、安全要件の値ではない。
//!
//! [`MotionDispatch::handle_debug`]: crate::MotionDispatch::handle_debug

/// 1行の最大長（byte。改行を含まない）。呼び出し側のbufferの大きさを揃えるための一般値で、
/// 安全要件の5項目に効かない。
pub const MAX_DEBUG_LINE_BYTES: usize = 96;

/// 行の先頭の語。
pub const DEBUG_MOTION_TOKEN: &str = "MOTION";

/// 読んだdebugのmotion指示。`name`は行から借りる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugMotion<'a> {
    /// Motion名。catalogの検査は`MotionDispatch::handle_debug`が行う。
    pub name: &'a str,
    /// 目標位置。有限かどうかの検査は`Limiter::admit`が行う。
    pub target: f32,
}

/// 行を読めなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DebugParseError {
    /// 空の行。
    Empty,
    /// [`MAX_DEBUG_LINE_BYTES`]を超える。
    TooLong,
    /// 先頭の語が[`DEBUG_MOTION_TOKEN`]でない。
    NotMotionCommand,
    /// motion名が無い。
    MissingName,
    /// targetが無い。
    MissingTarget,
    /// targetを数として読めない。
    TargetNotANumber,
    /// target の後ろに余分な語がある。
    TrailingToken,
}

/// 改行を除いた1行を読む。
///
/// # Errors
///
/// 形式に合わない行は[`DebugParseError`]を返す。
pub fn parse_debug_line(line: &str) -> Result<DebugMotion<'_>, DebugParseError> {
    if line.len() > MAX_DEBUG_LINE_BYTES {
        return Err(DebugParseError::TooLong);
    }
    let mut tokens = line.split_ascii_whitespace();
    let Some(first) = tokens.next() else {
        return Err(DebugParseError::Empty);
    };
    if first != DEBUG_MOTION_TOKEN {
        return Err(DebugParseError::NotMotionCommand);
    }
    let name = tokens.next().ok_or(DebugParseError::MissingName)?;
    let target_text = tokens.next().ok_or(DebugParseError::MissingTarget)?;
    let target = target_text
        .parse::<f32>()
        .map_err(|_| DebugParseError::TargetNotANumber)?;
    if tokens.next().is_some() {
        return Err(DebugParseError::TrailingToken);
    }
    Ok(DebugMotion { name, target })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_line_is_read() {
        assert_eq!(
            parse_debug_line("MOTION nod 12.5"),
            Ok(DebugMotion {
                name: "nod",
                target: 12.5
            })
        );
        // 余分な空白は区切りとして扱う。
        assert_eq!(
            parse_debug_line("  MOTION   nod\t-3  "),
            Ok(DebugMotion {
                name: "nod",
                target: -3.0
            })
        );
    }

    #[test]
    fn malformed_lines_are_refused_with_their_reason() {
        assert_eq!(parse_debug_line(""), Err(DebugParseError::Empty));
        assert_eq!(parse_debug_line("   "), Err(DebugParseError::Empty));
        assert_eq!(
            parse_debug_line("motion nod 1"),
            Err(DebugParseError::NotMotionCommand)
        );
        assert_eq!(
            parse_debug_line("GO nod 1"),
            Err(DebugParseError::NotMotionCommand)
        );
        assert_eq!(
            parse_debug_line("MOTION"),
            Err(DebugParseError::MissingName)
        );
        assert_eq!(
            parse_debug_line("MOTION nod"),
            Err(DebugParseError::MissingTarget)
        );
        assert_eq!(
            parse_debug_line("MOTION nod up"),
            Err(DebugParseError::TargetNotANumber)
        );
        assert_eq!(
            parse_debug_line("MOTION nod 1 2"),
            Err(DebugParseError::TrailingToken)
        );
    }

    #[test]
    fn the_line_length_limit_is_inclusive() {
        let name = "n".repeat(MAX_DEBUG_LINE_BYTES - "MOTION  1".len());
        let at_limit = format!("MOTION {name} 1");
        assert_eq!(at_limit.len(), MAX_DEBUG_LINE_BYTES);
        assert!(parse_debug_line(&at_limit).is_ok());
        let over = format!("MOTION {name}x 1");
        assert_eq!(parse_debug_line(&over), Err(DebugParseError::TooLong));
    }

    /// 有限でない値は読める。rejectするのはlimiterであり、この層ではない。
    #[test]
    fn non_finite_targets_are_read_not_judged() {
        let nan = parse_debug_line("MOTION nod nan").expect("read");
        assert!(nan.target.is_nan());
        let inf = parse_debug_line("MOTION nod inf").expect("read");
        assert!(inf.target.is_infinite());
    }
}
