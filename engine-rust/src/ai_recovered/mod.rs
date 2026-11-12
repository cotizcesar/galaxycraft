//! Recovered AI build-queue, card and script-runtime helpers.
//!
//! These pure/self-contained translations were merged out of the per-agent
//! staging files; they have no direct rules call site but document the AI
//! queue, card and script allocation behaviour.

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

// Wave blocks, one file each (scripts/split_merged.py).
mod translated;
mod w13;
mod w15;
mod w16;
mod w17;

pub use translated::*;
pub use w13::*;
pub use w15::*;
pub use w16::*;
pub use w17::*;
