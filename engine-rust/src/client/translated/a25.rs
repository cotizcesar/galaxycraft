#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent25.rs
// ===========================================================================
// Translation batch 25.  Only this file is edited by the batch.
//
// Batch 25 owns `0x005a8000..=0x005a9fff`.  The ledger lists twelve rows in
// that window: nine `decompiled_uncited` entries and three
// `decompile_failed` entries whose cached bodies are empty.  All nine
// uncited entries are translated below, and the three failed entries were
// recovered from the shipped runtime image's disassembly (the cached `.c`
// files contain only the size header).
//
// Every routine in this window belongs to the executable's self-decrypting
// loader / anti-tamper stub, not to game logic.  The region is built from
// overlapping-instruction decoys and encrypted gaps, so the decompiler
// truncates most bodies with `halt_baddata()`; the coherent blocks that do
// survive are straight-line "operator ladders" (a four-way selector that
// picks add / subtract / xor / constant), bit rotations, 16-bit half-word
// folds and `rdtsc`/noreturn-abort tails.  None of them reads or writes the
// simulation's `World` or `CUnit` state, and retail never returns from the
// ones that end in the abort helper.
//
// Consequently the translations reproduce the recovered arithmetic as
// deterministic helpers attached to [`World`] (the inherent-method namespace
// used by the batch convention) and never add persistent state.  Each doc
// comment names the exact address, the ledger status and the recovered C /
// disassembly shape; no other routine's address is cited.

/// `0x005a8466`: low 16 bits of the embedded constant `0x5a496a08`, loaded
/// into the loader's first state word `A`.
pub const A25_LOADER_SEED: u32 = 0x6a08;

/// `0x005a8466`: the `imul ...,0x25` factor that derives both the rotate
/// count and the dispatch selector from the round counter.
pub const A25_LOADER_ROUND_STEP: u32 = 0x25;

/// `0x005a8fdf`: the increment applied to the watched 32-bit counter.
pub const A25_INTEGRITY_STEP: u32 = 0x3b_743;

/// `0x005a8fdf`: pre-increment value from which the addition wraps the
/// counter to zero (`0xfffc48bd + 0x3b743 == 0x1_0000_0000`).
pub const A25_INTEGRITY_WRAP_AT: u32 = 0xfffc_48bd;

/// The repeated four-way ladder: `op` 0 adds, 1 subtracts, 2 xors and every
/// other value keeps `lhs`.  Recovered from the forward blocks of
/// `0x005a903a`, which reuse the same compare ladder with register operands.
fn a25_pick(op: u32, lhs: u32, rhs: u32) -> u32 {
    match op {
        0 => lhs.wrapping_add(rhs),
        1 => lhs.wrapping_sub(rhs),
        2 => lhs ^ rhs,
        _ => lhs,
    }
}

/// The inverse ordering used by the second half of each ladder pair
/// (`0x005a903a`) and by the accumulator ladder that opens `0x005a98c0`:
/// `op` 0 subtracts, 1 adds, 2 xors, other keeps `lhs`.
fn a25_pick_inv(op: u32, lhs: u32, rhs: u32) -> u32 {
    match op {
        0 => lhs.wrapping_sub(rhs),
        1 => lhs.wrapping_add(rhs),
        2 => lhs ^ rhs,
        _ => lhs,
    }
}

/// The constant-first ladder (`0x005a8c02`, and the `800`/`0x18`/`900`/`100`
/// blocks of `0x005a903a`): `op` 0 returns `value + constant`, 1 returns
/// `constant - value`, 2 xors, and every other value returns the constant.
fn a25_pick_const(op: u32, value: u32, constant: u32) -> u32 {
    match op {
        0 => value.wrapping_add(constant),
        1 => constant.wrapping_sub(value),
        2 => value ^ constant,
        _ => constant,
    }
}

/// The identity/add/xor ladder that opens `0x005a98c0`: `op` 0 keeps `lhs`,
/// 1 adds, 2 xors and other values keep `lhs`.
fn a25_pick_identity(op: u32, lhs: u32, rhs: u32) -> u32 {
    match op {
        0 => lhs,
        1 => lhs.wrapping_add(rhs),
        2 => lhs ^ rhs,
        _ => lhs,
    }
}

impl World {
    /// `0x005a8175` (`FUN_005a8175`, 3 bytes, `decompiled_uncited`).
    ///
    /// The whole body is `jmp dword ptr [edi+ecx]`:
    ///
    /// ```c
    /// (**(code **)(unaff_EDI + in_ECX))();
    /// ```
    ///
    /// The routine is the loader's indexed dispatch thunk: the two registers
    /// are added to form a byte offset into a table of code pointers and the
    /// selected pointer is entered.  [`Self::a25_dispatch_slot`] reproduces the
    /// address formation with bounds and 4-byte alignment checks; a misaligned
    /// or out-of-range slot is the case retail would fault on, reported as
    /// `None` instead of an invalid call.  The thunk touches no world state.
    pub fn a25_dispatch_slot(&self, table: &[u32], byte_offset: u32) -> Option<u32> {
        let offset = byte_offset as usize;
        if !offset.is_multiple_of(4) {
            return None;
        }
        table.get(offset / 4).copied()
    }

    /// `0x005a8466` (`FUN_005a8466`, 232 bytes, ledger `decompile_failed`;
    /// recovered from the shipped runtime image's disassembly).
    ///
    /// The routine prepares one loader round.  The recovered instructions are:
    ///
    /// ```text
    /// 0x005a84c6  [ebp-0x9e0] = 0x5a496a08        ; constant, low word used
    /// 0x005a84d0  A           = movzx word [...]
    /// 0x005a84dd  round       = 2
    /// 0x005a84f3  lo          = word [state]
    /// 0x005a8506  hi          = state >> 16
    /// 0x005a851b  scaled      = round * 0x25
    /// 0x005a853a  selector    = (scaled + round) & 7
    /// 0x005a853c  compare selector with 3, jump to the combine switch above 3
    /// ```
    ///
    /// The four rotate blocks reached through the `0x005ab940` table use
    /// `rotate = ((scaled + round) & 0xf) + 1`.  Returns `(selector, rotate)`.
    /// The `A` word is [`A25_LOADER_SEED`]; the combine keys are the high and
    /// low 16-bit halves of the previous state word, which the caller supplies
    /// to [`Self::a25_loader_combine`].
    pub fn a25_loader_round_selector(&self, round: u32) -> (u32, u32) {
        let scaled = round.wrapping_mul(A25_LOADER_ROUND_STEP);
        let sum = scaled.wrapping_add(round);
        (sum & 7, (sum & 0xf) + 1)
    }

    /// `0x005a85c4` (`FUN_005a85c4`, 892 bytes, ledger `decompile_failed`;
    /// recovered from the shipped runtime image's disassembly).
    ///
    /// The entry is the eight-way combine switch whose jump table sits at
    /// `0x005ab950` (targets `0x005a862c`, `0x005a85d6`, `0x005a85de`,
    /// `0x005a85e6`, `0x005a85f5`, `0x005a8604`, `0x005a8614`,
    /// `0x005a8620`).  With `a = [ebp-0x7c]` and `b = [ebp-0x80]` the cases
    /// are exactly:
    ///
    /// | index | recovered body      |
    /// |-------|---------------------|
    /// | 0     | `a + b`             |
    /// | 1     | `a - b`             |
    /// | 2     | `a ^ b`             |
    /// | 3     | `a + (a - b)`       |
    /// | 4     | `b + (b - a)`       |
    /// | 5     | `b - a`             |
    /// | 6     | `b ^ !a`            |
    /// | 7     | `a ^ !b`            |
    /// | other | `a + b` (the `ja` target is the case-0 block) |
    ///
    /// Indices 6 and 7 are algebraically the same value because
    /// `(!a) ^ b == a ^ (!b)`; retail still keeps two separate blocks.
    /// After the switch the routine folds the result with the previous
    /// state's high half and rebuilds the word as
    /// `(result << 16) | state_low`, which is the caller's step and not part
    /// of this helper.  All arithmetic is 32-bit wrapping.
    pub fn a25_loader_combine(&self, mode: u32, a: u32, b: u32) -> u32 {
        match mode {
            0 => a.wrapping_add(b),
            1 => a.wrapping_sub(b),
            2 => a ^ b,
            3 => a.wrapping_add(a.wrapping_sub(b)),
            4 => b.wrapping_add(b.wrapping_sub(a)),
            5 => b.wrapping_sub(a),
            6 => b ^ !a,
            7 => a ^ !b,
            _ => a.wrapping_add(b),
        }
    }

    /// `0x005a8c02` (`FUN_005a8c02`, 525 bytes, ledger `decompile_failed`;
    /// recovered from the shipped runtime image's disassembly).
    ///
    /// The recovered head is the constant-`4` ladder on the operation index
    /// `[ebp+0x14]` and the value in `EDX`:
    ///
    /// ```c
    /// if (op == 0)      value = value + 4;
    /// else if (op == 1) value = 4 - value;
    /// else if (op == 2) value = value ^ 4;
    /// else              value = 4;
    /// ```
    ///
    /// The remainder of the body repeats the same add/sub/xor ladders with the
    /// `EBX`, `ESI` and `EDI` operands and folds the result into the loader
    /// state word; only the fully recovered head is reproduced here.
    pub fn a25_relative_add4(&self, op: u32, value: u32) -> u32 {
        a25_pick_const(op, value, 4)
    }

    /// `0x005a8fdf` (`FUN_005a8fdf`, 15 bytes, `decompiled_uncited`).
    ///
    /// The coherent body is the loader's timer/overflow gate.  The decompiled
    /// control flow is:
    ///
    /// ```c
    /// uVar2 = *counter;              // value before the increment
    /// *counter = *counter + 0x3b743; // wraps exactly at 0xfffc48bd
    /// if (uVar2 < 0xfffc48bd)
    ///     flag = (klass == 5 && count > 1);
    /// else
    ///     flag = 1;
    /// ```
    ///
    /// `0xfffc48bd + 0x3b743 == 0x1_0000_0000`, so the compare is the carry
    /// flag of the increment: `pre_add >= 0xfffc48bd` means the counter wrapped.
    /// The second clause is the recovered `[ebp-0x3b8] == 5 && [ebp-0x3bc] > 1`
    /// test.  Returns the flag byte written to `[ebp-0x109]`; `true` is the
    /// branch that falls into the tamper-report chain at `0x005a903a`.
    pub fn a25_counter_wrap_gate(&self, pre_add: u32, klass: u32, count: u32) -> bool {
        pre_add >= A25_INTEGRITY_WRAP_AT || (klass == 5 && count > 1)
    }

    /// `0x005a903a` (`FUN_005a903a`, 245 bytes, `decompiled_uncited`).
    ///
    /// This is the gate-true branch: the loader builds an argument from six
    /// ladder steps.  With `op_first = [ebp+0x14]`, `op_pair = [ebp+0x1c]`
    /// (reused for `[ebp+0x24]`), `seed = [ebp+0x18]`, `X = [ebp+0x20]` and
    /// `Y = [ebp+0x28]` the decompiled C is:
    ///
    /// 1. `const` ladder with constant `800`: 0 `seed+800`, 1 `800-seed`,
    ///    2 `seed^800`, other `800`;
    /// 2. `op_pair` add/sub/xor with `X`;
    /// 3. `op_pair` add/sub/xor with `Y`;
    /// 4. inverse `op_pair` with `Y`;
    /// 5. inverse `op_pair` with `X`;
    /// 6. inverse `op_first` with `seed`.
    ///
    /// Every operation is paired with its inverse, so the chain always yields
    /// the obfuscated constant `800` regardless of the selectors or operands.
    /// After this the routine indexes a table with the result, calls through
    /// `FUN_00c86d80`, and continues with `0x18`, `900` and `100` versions of
    /// the same chain before `FUN_00c874a0` and the noreturn abort; only the
    /// fully recovered chain is reproduced.
    pub fn a25_tamper_branch_value(
        &self,
        op_first: u32,
        op_pair: u32,
        seed: u32,
        first: u32,
        second: u32,
    ) -> u32 {
        let mut value = a25_pick_const(op_first, seed, 800);
        value = a25_pick(op_pair, value, first);
        value = a25_pick(op_pair, value, second);
        value = a25_pick_inv(op_pair, value, second);
        value = a25_pick_inv(op_pair, value, first);
        a25_pick_inv(op_first, value, seed)
    }

    /// `0x005a954e` (`FUN_005a954e`, 10 bytes, `decompiled_uncited`).
    ///
    /// The cached body is the 32-bit counter decrement followed by the bad
    /// instruction that truncates control flow:
    ///
    /// ```c
    /// *(int *)(unaff_EBX + -0x7a74e38b) += -1;
    /// halt_baddata();          // no continuation
    /// ```
    ///
    /// The helper applies the wrapping decrement and reports `None` for the
    /// trap, so callers can observe the decremented counter without modelling
    /// a return that retail never performs.
    pub fn a25_decrement_and_trap(&self, counter: &mut i32) -> Option<i32> {
        *counter = counter.wrapping_sub(1);
        None
    }

    /// `0x005a98c0` (`FUN_005a98c0`, 1061 bytes, `decompiled_uncited`).
    ///
    /// The recovered head folds an incoming pair and the loader key:
    ///
    /// ```c
    /// // unaff_ESI is the selector: 1 adds, 2 xors, 0/other keeps EAX.
    /// v = select(eax, edx);
    /// // [ebp+0x14] is the operation: 0 subtracts, 1 adds, 2 xors,
    /// // every other value keeps the accumulator.
    /// v = op(v, in_ECX);
    /// ```
    ///
    /// The result is stored as the argument at `[ebp-0x29c]`; the rest of the
    /// routine builds the sibling abort arguments from the `900`, `[ebp+0x20]`
    /// and `[ebp+0x28]` ladders and finishes in `FUN_00c874a0` plus the
    /// noreturn abort.  Only the two fully recovered ladders are reproduced.
    pub fn a25_loader_state_fold(&self, select: u32, op: u32, lhs: u32, rhs: u32, key: u32) -> u32 {
        let folded = a25_pick_identity(select, lhs, rhs);
        a25_pick_inv(op, folded, key)
    }

    /// `0x005a9d80` (`FUN_005a9d80`, 756 bytes, `decompiled_uncited`).
    ///
    /// The recovered prologue is four instructions:
    ///
    /// ```text
    /// 0x005a9d81  ecx = [ebp-0x198]
    /// 0x005a9d87  eax = eax + ecx
    /// 0x005a9d89  ecx = [ebp-0x29c]
    /// 0x005a9d8f  ecx = ecx + eax
    /// 0x005a9d91  [ebp-0x29c] = ecx
    /// ```
    ///
    /// Returns `(seed + addend, accumulator + seed + addend)`: the first value
    /// is the updated `EAX`, the second the updated accumulator.  The remainder
    /// of the routine is an encrypted gap followed by `rdtsc` and repeated
    /// ladder chains, so only the recovered prologue is reproduced.
    pub fn a25_loader_accumulate(&self, seed: u32, addend: u32, accumulator: u32) -> (u32, u32) {
        let sum = seed.wrapping_add(addend);
        (sum, accumulator.wrapping_add(sum))
    }
}

impl crate::sim::SimUnit {
    /// `0x005a85ab` (`FUN_005a85ab`, 1 byte), `0x005a87b6`
    /// (`FUN_005a87b6`, 1 byte) and `0x005a9d4c` (`FUN_005a9d4c`, 15 bytes)
    /// are byte-identical `halt_baddata()` stubs in the ledger: the cached
    /// bodies contain only the truncation warning, and the bytes at each entry
    /// are an overlapping-instruction decoy (`0x90 0xfe 0xff`, `0xff 0xff`,
    /// and the tail of an overlapping `test`).  Retail has no continuation.
    ///
    /// One method cites all three because the recovered body is identical;
    /// `None` is the "no return" result.  It only borrows the unit so the
    /// caller can keep using it, mirroring the retail `ECX`-only entry.
    pub fn a25_bad_instruction_trap(&self) -> Option<()> {
        None
    }
}

#[cfg(test)]
mod tests_agent25 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn dispatch_slot_walks_the_recovered_indirect_jump() {
        // 0x005a8175
        let w = world();
        let table = [0xdead_beef, 0x1234_5678, 0xcafe_babe];
        assert_eq!(w.a25_dispatch_slot(&table, 0), Some(0xdead_beef));
        assert_eq!(w.a25_dispatch_slot(&table, 4), Some(0x1234_5678));
        assert_eq!(w.a25_dispatch_slot(&table, 8), Some(0xcafe_babe));
        assert_eq!(w.a25_dispatch_slot(&table, 12), None);
        assert_eq!(w.a25_dispatch_slot(&table, 2), None);
        assert_eq!(w.a25_dispatch_slot(&[], 0), None);
    }

    #[test]
    fn loader_round_selector_matches_the_recovered_arithmetic() {
        // 0x005a8466
        let w = world();
        assert_eq!(w.a25_loader_round_selector(0), (0, 1));
        assert_eq!(w.a25_loader_round_selector(2), (4, 13));
        assert_eq!(w.a25_loader_round_selector(3), (2, 3));
        assert_eq!(w.a25_loader_round_selector(8), (0, 1));
        for round in 0..64u32 {
            let sum = round * A25_LOADER_ROUND_STEP + round;
            assert_eq!(
                w.a25_loader_round_selector(round),
                (sum & 7, (sum & 0xf) + 1),
                "round {round}"
            );
        }
        assert_eq!(A25_LOADER_SEED, 0x5a49_6a08u32 & 0xffff);
        assert_eq!(0x5a49_6a08u32 >> 16, 0x5a49);
    }

    #[test]
    fn loader_combine_matches_all_eight_jump_table_cases() {
        // 0x005a85c4
        let w = world();
        let a = 0x1234_5678u32;
        let b = 0x0fed_cba9u32;
        assert_eq!(w.a25_loader_combine(0, a, b), 0x2222_2221);
        assert_eq!(w.a25_loader_combine(1, a, b), 0x0246_8acf);
        assert_eq!(w.a25_loader_combine(2, a, b), 0x1dd9_9dd1);
        assert_eq!(w.a25_loader_combine(3, a, b), 0x147a_e147);
        assert_eq!(w.a25_loader_combine(4, a, b), 0x0da7_40da);
        assert_eq!(w.a25_loader_combine(5, a, b), 0xfdb9_7531);
        // Cases 6 and 7 are two blocks with the same algebra: b ^ !a == a ^ !b.
        assert_eq!(w.a25_loader_combine(6, a, b), 0xe226_622e);
        assert_eq!(w.a25_loader_combine(7, a, b), 0xe226_622e);
        assert_eq!(w.a25_loader_combine(8, a, b), 0x2222_2221);
        assert_eq!(w.a25_loader_combine(u32::MAX, 5, 7), 12);
    }

    #[test]
    fn relative_add4_matches_the_constant_ladder() {
        // 0x005a8c02
        let w = world();
        assert_eq!(w.a25_relative_add4(0, 0x1234), 0x1238);
        assert_eq!(w.a25_relative_add4(1, 0x1234), -0x1230i32 as u32);
        assert_eq!(w.a25_relative_add4(2, 0x1234), 0x1230);
        assert_eq!(w.a25_relative_add4(3, 0x1234), 4);
        assert_eq!(w.a25_relative_add4(u32::MAX, 0), 4);
        assert_eq!(w.a25_relative_add4(1, 0), 4);
    }

    #[test]
    fn counter_wrap_gate_detects_the_wrapping_addition() {
        // 0x005a8fdf
        let w = world();
        assert_eq!(A25_INTEGRITY_STEP, 0x3b_743);
        assert_eq!(A25_INTEGRITY_WRAP_AT, 0xfffc_48bd);
        assert_eq!(A25_INTEGRITY_WRAP_AT.wrapping_add(A25_INTEGRITY_STEP), 0);
        assert!(!w.a25_counter_wrap_gate(0xfffc_48bc, 0, 0));
        assert!(!w.a25_counter_wrap_gate(0xfffc_48bc, 5, 1));
        assert!(w.a25_counter_wrap_gate(0xfffc_48bc, 5, 2));
        assert!(w.a25_counter_wrap_gate(0xfffc_48bd, 0, 0));
        assert!(w.a25_counter_wrap_gate(u32::MAX, 0, 0));
        assert!(w.a25_counter_wrap_gate(0, 5, 2));
        assert!(!w.a25_counter_wrap_gate(0, 4, 100));
    }

    #[test]
    fn tamper_branch_chain_collapses_to_the_obfuscated_constant() {
        // 0x005a903a: each ladder step is paired with its inverse, so every
        // selector and operand combination yields 800 (0x320).
        let w = world();
        for op_first in 0..5u32 {
            for op_pair in 0..5u32 {
                for &(seed, first, second) in &[
                    (0u32, 0u32, 0u32),
                    (0x1122_3344, 0x5566_7788, 0x99aa_bbcc),
                    (u32::MAX, 1, 2),
                ] {
                    assert_eq!(
                        w.a25_tamper_branch_value(op_first, op_pair, seed, first, second),
                        800,
                        "first {op_first} pair {op_pair}"
                    );
                }
            }
        }
    }

    #[test]
    fn decrement_and_trap_reports_the_decremented_counter() {
        // 0x005a954e
        let w = world();
        let mut counter = 5;
        assert_eq!(w.a25_decrement_and_trap(&mut counter), None);
        assert_eq!(counter, 4);
        counter = 0;
        assert_eq!(w.a25_decrement_and_trap(&mut counter), None);
        assert_eq!(counter, -1);
        counter = i32::MIN;
        assert_eq!(w.a25_decrement_and_trap(&mut counter), None);
        assert_eq!(counter, i32::MAX);
    }

    #[test]
    fn loader_state_fold_matches_the_two_ladders() {
        // 0x005a98c0
        let w = world();
        // select 0 keeps lhs, select 1 adds, select 2 xors.
        assert_eq!(w.a25_loader_state_fold(0, 1, 0x1000, 0x234, 0x0f0f), 0x1f0f);
        assert_eq!(w.a25_loader_state_fold(1, 1, 0x1000, 0x234, 0x0f0f), 0x2143);
        assert_eq!(w.a25_loader_state_fold(2, 1, 0x1000, 0x234, 0x0f0f), 0x2143);
        // op 0 subtracts, op 1 adds, op 2 xors, other keeps the accumulator.
        assert_eq!(w.a25_loader_state_fold(1, 0, 0x1000, 0x234, 0x0f0f), 0x0325);
        assert_eq!(w.a25_loader_state_fold(1, 2, 0x1000, 0x234, 0xffff), 0xedcb);
        assert_eq!(w.a25_loader_state_fold(1, 9, 0x1000, 0x234, 0x0f0f), 0x1234);
        // 0x1234 - 0x0f0f == 0x0325 and 0x1234 ^ 0xffff == 0xedcb hold above.
        assert_eq!(w.a25_loader_state_fold(1, 0, 0, 0, 1), u32::MAX);
    }

    #[test]
    fn loader_accumulate_adds_seed_then_the_accumulator() {
        // 0x005a9d80
        let w = world();
        assert_eq!(w.a25_loader_accumulate(0x10, 0x20, 0x30), (0x30, 0x60));
        assert_eq!(w.a25_loader_accumulate(u32::MAX, 2, 0x40), (1, 0x41));
        assert_eq!(w.a25_loader_accumulate(0, 0, 0), (0, 0));
    }

    #[test]
    fn bad_instruction_stubs_never_continue() {
        // 0x005a85ab, 0x005a87b6, 0x005a9d4c
        let mut w = world();
        let index = w.spawn(0, uid::MARINE, 512, 512).unwrap();
        assert_eq!(w.units[index].a25_bad_instruction_trap(), None);
        assert!(w.units[index].a25_bad_instruction_trap().is_none());
    }

    #[test]
    fn loader_helpers_leave_the_world_untouched() {
        let mut w = world();
        let index = w.spawn(0, uid::MARINE, 512, 512).unwrap();
        let before = (
            w.units.len(),
            w.tick,
            w.units[index].id,
            w.units[index].hp,
            w.units[index].x,
            w.units[index].y,
        );
        let table = [1u32, 2, 3];
        let mut counter = 1;
        let _ = w.a25_dispatch_slot(&table, 4);
        let _ = w.a25_loader_round_selector(7);
        let _ = w.a25_loader_combine(6, 0x1234, 0x5678);
        let _ = w.a25_relative_add4(2, 0x99);
        let _ = w.a25_counter_wrap_gate(0xfffc_48bd, 5, 2);
        let _ = w.a25_tamper_branch_value(2, 0, 1, 2, 3);
        let _ = w.a25_decrement_and_trap(&mut counter);
        let _ = w.a25_loader_state_fold(2, 1, 3, 4, 5);
        let _ = w.a25_loader_accumulate(1, 2, 3);
        assert_eq!(
            (
                w.units.len(),
                w.tick,
                w.units[index].id,
                w.units[index].hp,
                w.units[index].x,
                w.units[index].y,
            ),
            before
        );
        assert_eq!(counter, 0);
    }
}
