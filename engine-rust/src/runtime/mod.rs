//! Runtime/library translations recovered from the shipped image.
//!
//! These are the `args`/iostream/STL/hash/string helpers that do not
//! belong to the rules simulation.  They were merged out of the
//! per-agent staging files so the client/runtime layer has a natural
//! home in the crate.

#![allow(
    clippy::unusual_byte_groupings,
    clippy::unnecessary_cast,
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::result_unit_err,
    clippy::byte_char_slices,
    clippy::while_let_loop,
    clippy::redundant_closure,
    clippy::manual_is_multiple_of,
    clippy::manual_repeat_n,
    clippy::assign_op_pattern,
    clippy::implicit_saturating_sub,
    clippy::if_same_then_else,
    clippy::doc_lazy_continuation,
    clippy::field_reassign_with_default,
    clippy::identity_op,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_patterns,
    clippy::needless_range_loop,
    clippy::erasing_op,
    clippy::bool_assert_comparison,
    clippy::chunks_exact_to_as_chunks,
    clippy::unnecessary_map_or,
    clippy::precedence,
    clippy::derivable_impls,
    clippy::manual_div_ceil
)]

use crate::sim::World;
use std::sync::atomic::{AtomicU32, Ordering};

// Wave blocks, one file each (scripts/split_merged.py).
mod translated;
mod w13;
mod w14;
mod w17;
mod w18;
mod w19;
mod w20;
mod w21;
mod w22;
mod w23;
mod w24;
mod w25;
mod w26;
mod w27;
mod w28;
mod w29;
mod w30;
mod w31;
mod w32;
mod w33;
mod w34;

pub use translated::*;
pub use w13::*;
pub use w14::*;
pub use w17::*;
pub use w18::*;
pub use w19::*;
pub use w20::*;
pub use w21::*;
pub use w22::*;
pub use w23::*;
pub use w24::*;
pub use w25::*;
pub use w26::*;
pub use w27::*;
pub use w28::*;
pub use w29::*;
pub use w30::*;
pub use w31::*;
pub use w32::*;
pub use w33::*;
pub use w34::*;
