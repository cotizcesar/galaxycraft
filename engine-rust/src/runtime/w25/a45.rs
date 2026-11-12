#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent45.rs
// ===========================================================================
// Wave 25 agent 45: fmt::v6 dynamic width/precision argument extraction.

/// Integer argument as stored in a fmt::v6 `basic_format_arg` (type tag at +8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W25a45Arg {
    /// tag 2: `int`
    Int(i32),
    /// tag 3: `unsigned`
    UInt(u32),
    /// tag 4: `long long`
    LongLong(i64),
    /// tag 5: `unsigned long long`
    ULongLong(u64),
    /// tag 8: `bool`/`char` read as a byte
    Byte(u8),
    /// tag 9: `signed char`
    SChar(i8),
    /// tags 10..=15 and everything else: not an integer
    NotInteger,
}

/// `0x00893920`: dynamic width.  Negative signed values are negated and set
/// the "left aligned" flag (returned as `.1`); values above `i32::MAX` fail
/// with "number is too big"; non-integers fail with "width is not integer".
pub fn w25a45_get_dynamic_width(arg: W25a45Arg) -> Result<(u32, bool), &'static str> {
    const BIG: &str = "number is too big";
    match arg {
        W25a45Arg::Int(v) => {
            let (u, neg) = if v < 0 {
                ((v as u32).wrapping_neg(), true)
            } else {
                (v as u32, false)
            };
            if u > 0x7fff_ffff {
                Err(BIG)
            } else {
                Ok((u, neg))
            }
        }
        W25a45Arg::UInt(u) => {
            if u > 0x7fff_ffff {
                Err(BIG)
            } else {
                Ok((u, false))
            }
        }
        W25a45Arg::LongLong(v) => {
            let neg = v < 0;
            let m = v.unsigned_abs();
            if m < 0x8000_0000 {
                Ok((m as u32, neg))
            } else {
                Err(BIG)
            }
        }
        W25a45Arg::ULongLong(u) => {
            if u > 0x7fff_ffff {
                Err(BIG)
            } else {
                Ok((u as u32, false))
            }
        }
        W25a45Arg::Byte(b) => Ok((b as u32, false)),
        W25a45Arg::SChar(c) => {
            let neg = c < 0;
            Ok(((c as i32).unsigned_abs(), neg))
        }
        W25a45Arg::NotInteger => Err("width is not integer"),
    }
}

/// `0x00894080`: dynamic precision.  Negative signed values clamp to 0;
/// too-large values fail with "number is too big"; non-integers fail with
/// "precision is not integer".
pub fn w25a45_get_dynamic_precision(arg: W25a45Arg) -> Result<u32, &'static str> {
    const BIG: &str = "number is too big";
    match arg {
        W25a45Arg::Int(v) => Ok(v.max(0) as u32),
        W25a45Arg::UInt(u) => {
            if u > 0x7fff_ffff {
                Err(BIG)
            } else {
                Ok(u)
            }
        }
        W25a45Arg::LongLong(v) => {
            if v > i32::MAX as i64 {
                Err(BIG)
            } else {
                Ok(v.max(0) as u32)
            }
        }
        W25a45Arg::ULongLong(u) => {
            if u > 0x7fff_ffff {
                Err(BIG)
            } else {
                Ok(u as u32)
            }
        }
        W25a45Arg::Byte(b) => Ok(b as u32),
        W25a45Arg::SChar(c) => Ok(c.max(0) as u32),
        W25a45Arg::NotInteger => Err("precision is not integer"),
    }
}

#[cfg(test)]
mod tests_wave25_agent45 {
    use super::*;

    #[test]
    fn width() {
        assert_eq!(w25a45_get_dynamic_width(W25a45Arg::Int(-5)), Ok((5, true)));
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::Int(i32::MIN)),
            Err("number is too big")
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::UInt(0x8000_0000)),
            Err("number is too big")
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::LongLong(-7)),
            Ok((7, true))
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::LongLong(1 << 31)),
            Err("number is too big")
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::ULongLong(1 << 32)),
            Err("number is too big")
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::SChar(-3)),
            Ok((3, true))
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::Byte(200)),
            Ok((200, false))
        );
        assert_eq!(
            w25a45_get_dynamic_width(W25a45Arg::NotInteger),
            Err("width is not integer")
        );
    }

    #[test]
    fn precision() {
        assert_eq!(w25a45_get_dynamic_precision(W25a45Arg::Int(-5)), Ok(0));
        assert_eq!(w25a45_get_dynamic_precision(W25a45Arg::Int(9)), Ok(9));
        assert_eq!(
            w25a45_get_dynamic_precision(W25a45Arg::UInt(0x8000_0000)),
            Err("number is too big")
        );
        assert_eq!(w25a45_get_dynamic_precision(W25a45Arg::LongLong(-1)), Ok(0));
        assert_eq!(
            w25a45_get_dynamic_precision(W25a45Arg::LongLong(1 << 31)),
            Err("number is too big")
        );
        assert_eq!(w25a45_get_dynamic_precision(W25a45Arg::SChar(-1)), Ok(0));
        assert_eq!(
            w25a45_get_dynamic_precision(W25a45Arg::NotInteger),
            Err("precision is not integer")
        );
    }
}
