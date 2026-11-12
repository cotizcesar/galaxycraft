#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave12/agent05.rs
// ===========================================================================
// Wave 12 translation batch 05; temporary staging file.
//
// Window `0x0065a000..=0x0065bfff`.  The ledger lists 54 rows in the window,
// all `decompiled_uncited`; 13 are translated below and the rest are listed
// under "Skipped rows".
//
// Recovered context used by the translated routines:
//
// * The 0x20-byte game record at `0x011a0ec5` (`FUN_006e8130` copies an
//   eight-dword record there; a failed lookup writes `0xff` into its first
//   byte).  Its first byte is the game format (`0x0f` = single player) and
//   the low byte of its fifth dword, `DAT_011a0ed5` at `+0x10`, is the slot
//   count used by the UI.  `DAT_0119f7bc` is the client gate byte already
//   documented as `client::A79_KEY_GATE_BYTE`.
// * The 0x24-stride slot array behind `DAT_00f063cc ^ 0x10fae560`: eight
//   records (the range walks compare against `0x120` = 8 * 0x24).  A
//   per-player byte table starts at `+0xdb`.
// * `0x00f05970` is a flat u16 stream of requirement clauses,
//   `<unit id><opcode> <operand>... <0xffff>`, ending with a second
//   `0xffff` unit id.  `0x00f91e20` caches the 228 (`0xe4`) unit starts;
//   `0x00fdf8e8` holds the last requirement failure code.
// * `0x0065ba20` is a `qsort`-style introsort for `u16` keys:
//   median-of-nine pivot (`0x0065bc90`), a Bentley-McIlroy three-way
//   partition, insertion sort below 33 elements and a heap-sort fallback
//   (`0x0065bdd0`).  The ledger's second row at `FUN_0065ba29` is a
//   jump-table label inside the same body, not a separate routine.
//
// Skipped rows:
//
// | address | reason |
// |---|---|
// | `FUN_0065a120` | exception-frame setup followed by the non-returning `FUN_00538810(8)` (a fatal runtime error); no returning body |
// | `FUN_0065a520`, `FUN_0065a610`, `FUN_0065a6c0` | one tail call into the shared slot dispatch (the cached body is `FUN_00748d40(7)` plus the "does not return" warning); the callers pass a slot index, but no return value or local state survives in the decompilation |
// | `FUN_0065a320`, `FUN_0065a331`, `FUN_0065a46c` | fragments of one count loop: they read the caller's `unaff_EBP` frame and end in a non-returning dispatch, so they are not standalone functions |
// | `FUN_0065a7f0`..`FUN_0065b18a` | the basic blocks of one compiler-generated slot dispatcher; every block ends in a non-returning tail call and reads caller-frame (`unaff_*`) registers, so no block is a function on its own (`FUN_0065b0c0`/`FUN_0065b0e0`/`FUN_0065b18a` are its index drivers) |
// | `FUN_0065b2b0`, `FUN_0065b48e`, `FUN_0065b544` | the unit-requirement evaluator; it reads `CUnit` status/build-queue fields (`+0x4c`, `+0x64`, `+0x98`, `+0xc0`, `+0xc8`, `+0xc9`, `+0xd0`, `+0xdc`, `+0xec`, `+0x117`, `+0x119`, `+0x124`) and 12-column per-player tables, and the cached bodies lose the non-returning branches (the callees are value getters), so a faithful body would need a new `CUnit` view |
// | `FUN_0065ba29`, `FUN_0065bdea` | jump-target labels inside `0x0065ba20` and `0x0065bdd0`; the decompiler emitted their bodies a second time with a mid-function start and `unaff` registers |
// | `FUN_0065b920`, `FUN_0065b980` | `std::string`-like holder push/pop with SEH and CRT allocation, no rules state |
// | `FUN_0065bf20`, `FUN_0065bf33`, `FUN_0065bf90`, `FUN_0065bf99` | `TilePage` constructor/destructor: vtable store, base-class constructor and CRT array free, represented by fields outside this window |
//
// The 13 translated rows are pure functions: slot/player predicates, the
// requirement index and error word, and the three sort helpers.  None of
// them reads or writes `World`/`SimUnit`.

/// Number of `units.dat` rows, recovered twice in the window: the
/// `0x1c8`-byte (456 = 228 * 2) normalisation loop of `0x0065b230` and the
/// `0xe3 < param_1` bound of `FUN_0065b2b0`.
pub const W12A05_UNIT_TYPE_COUNT: usize = 0xe4;

/// Clause terminator and table end marker (`0xffff`, compared as `-1` by the
/// signed `short` walks of `0x0065b230` and `0x0065b8a0`).
pub const W12A05_REQUIREMENT_END: u16 = 0xffff;

/// The requirement opcode `0x0065b8a0` returns the operand of (`0xff02`, the
/// decompiled `== -0xfe` test).
pub const W12A05_REQUIREMENT_HEAD_OPCODE: u16 = 0xff02;

/// Value `0x0065b8a0` returns when the unit has no `0xff02` clause head.
pub const W12A05_REQUIREMENT_FALLBACK: u16 = 0xe4;

/// Game-format byte (`0x011a0ec5`) that `0x0065a0e0` maps to a two-player
/// limit.
pub const W12A05_FORMAT_SINGLE_PLAYER: u8 = 0x0f;

/// Limit `0x0065a0e0` returns for [`W12A05_FORMAT_SINGLE_PLAYER`].
pub const W12A05_SINGLE_PLAYER_SLOTS: u8 = 2;

/// Offset of the per-player byte table inside the slot object.  `0x0065a100`
/// reads `*(u8 *)(base + 0xdb + slot)`.
pub const W12A05_SLOT_PLAYER_TABLE: usize = 0xdb;

/// Insertion-sort threshold of `0x0065ba20`: `iVar7 < 0x21` (32 elements),
/// and the byte-length guard `(len & ~1) < 0x41`.
pub const W12A05_SORT_ISORT_MAX: usize = 0x20;

/// `0x0065a100`: the per-player byte of one slot.
///
/// ```c
/// undefined1 FUN_0065a100(byte param_1)
/// {
///   return *(undefined1 *)(param_1 + 0xdb + (DAT_00f063cc ^ 0x10fae560));
/// }
/// ```
///
/// `table` is the byte array that begins at object offset
/// [`W12A05_SLOT_PLAYER_TABLE`]; `slot` is the raw `param_1` index.  The
/// shipped body has no bounds check, so neither does this indexing.
pub fn w12a05_slot_player_byte(table: &[u8], slot: u8) -> u8 {
    table[slot as usize]
}

/// `0x0065a0e0`: the slot limit implied by the game record.
///
/// ```c
/// if (FUN_006e7df0()) return 2;   // FUN_006e7df0: DAT_011a0ec5 == '\x0f'
/// return DAT_011a0ed5;
/// ```
///
/// `game_format` is the first byte of the record at `0x011a0ec5` and
/// `game_type` is `DAT_011a0ed5`, the low byte of its fifth dword (`+0x10`).
/// `FUN_00724e00` compares the result with a requested slot index, so the byte
/// is a one-past-the-end slot limit.
pub fn w12a05_player_limit(game_format: u8, game_type: u8) -> u8 {
    if game_format == W12A05_FORMAT_SINGLE_PLAYER {
        W12A05_SINGLE_PLAYER_SLOTS
    } else {
        game_type
    }
}

/// `0x0065a306`: the one-byte `jmp $` marker (`eb fe`).
///
/// The ledger body is a single `do { } while (true)` block with no exit, the
/// compiler's representation of a fatal `for (;;);`.  Modelled as a
/// diverging function so callers can use it in expression position.
pub fn w12a05_halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// `0x0065a7c0`: the empty function (`ret`, one byte).
///
/// No state is touched; kept as a named no-op because callers use it as a
/// default callback.
pub fn w12a05_retail_noop() {}

/// `0x0065a7d0`: the "a game is running" predicate.
///
/// ```c
/// if ((DAT_0119f7bc != '\0') && (DAT_011a0ed5 != '\0')) return 1;
/// return 0;
/// ```
///
/// `gate` is `DAT_0119f7bc` (the client gate byte) and `game_type` is
/// `DAT_011a0ed5` from the game record used by [`w12a05_player_limit`].
pub fn w12a05_game_session_active(gate: bool, game_type: u8) -> bool {
    gate && game_type != 0
}

/// `0x0065b230`: build the unit -> requirement-clause index.
///
/// ```c
/// i = 1;
/// if (*(short *)(&DAT_00f05970 + 2) != -1) {
///   do {
///     (&DAT_00f91e20)[*(ushort *)(&DAT_00f05970 + i * 2)] = (short)i + 1;
///     s = *(short *)(&DAT_00f05970 + i * 2);       // the unit id
///     while (s != -1) { i++; s = *(short *)(&DAT_00f05970 + i * 2); }
///     next = *(ushort *)(&DAT_00f05972 + i * 2);
///     i++;
///   } while (next != 0xffff);
/// }
/// for (j = 0; j < 0x1c8; j += 2)
///   if (*(short *)((int)&DAT_00f91e20 + j) == -1)
///     *(ushort *)((int)&DAT_00f91e20 + j) = 0;
/// ```
///
/// The stream layout is `<unit id><first opcode> <operands> <0xffff>` repeated,
/// closed by a `0xffff` unit id; the stored value is the index of the first
/// opcode after the unit id.  Every entry left at `0xffff` becomes 0.  The
/// shipped routine has no bound checks on the unit id or the stream; this
/// version ignores ids outside [`W12A05_UNIT_TYPE_COUNT`] and stops at the
/// end of `stream`.
pub fn w12a05_build_requirement_index(stream: &[u16]) -> [u16; W12A05_UNIT_TYPE_COUNT] {
    let mut index = [W12A05_REQUIREMENT_END; W12A05_UNIT_TYPE_COUNT];
    if stream.len() > 1 && stream[1] != W12A05_REQUIREMENT_END {
        let mut i = 1usize;
        loop {
            let unit = match stream.get(i) {
                Some(&unit) => unit,
                None => break,
            };
            if (unit as usize) < W12A05_UNIT_TYPE_COUNT {
                index[unit as usize] = (i + 1) as u16;
            }
            // `while (s != -1) { i++; s = stream[i]; }` starts after the unit
            // id, so the first step is unconditional.
            loop {
                i += 1;
                match stream.get(i) {
                    Some(&W12A05_REQUIREMENT_END) => break,
                    Some(_) => {}
                    None => return w12a05_normalise_requirement_index(index),
                }
            }
            let next = match stream.get(i + 1) {
                Some(&next) => next,
                None => break,
            };
            i += 1;
            if next == W12A05_REQUIREMENT_END {
                break;
            }
        }
    }
    w12a05_normalise_requirement_index(index)
}

/// The second loop of `0x0065b230`: every untouched (`0xffff`) entry becomes
/// 0 so that index 0 doubles as "no requirement clause".
fn w12a05_normalise_requirement_index(
    mut index: [u16; W12A05_UNIT_TYPE_COUNT],
) -> [u16; W12A05_UNIT_TYPE_COUNT] {
    for entry in &mut index {
        if *entry == W12A05_REQUIREMENT_END {
            *entry = 0;
        }
    }
    index
}

/// `0x0065b71e`: the sentinel index of "no requirement" (`or eax, -1`).
pub fn w12a05_requirement_none() -> u32 {
    u32::MAX
}

/// `0x0065b900`: read the last requirement failure code.
///
/// ```c
/// undefined4 FUN_0065b900(void) { return DAT_00fdf8e8; }
/// ```
///
/// `state` is the recovered `DAT_00fdf8e8` word.
pub fn w12a05_requirement_error(state: &u32) -> u32 {
    *state
}

/// `0x0065b910`: store a requirement failure code.
///
/// ```c
/// void FUN_0065b910(undefined4 param_1) { DAT_00fdf8e8 = param_1; }
/// ```
pub fn w12a05_set_requirement_error(state: &mut u32, code: u32) {
    *state = code;
}

/// `0x0065b8a0`: first operand of a unit's `0xff02` clause head.
///
/// ```c
/// DAT_00fdf8e8 = 0;
/// u = (&DAT_00f91e20)[param_1];
/// if (u == 0xffff) { FUN_0065b230(); u = (&DAT_00f91e20)[param_1]; }
/// if (u != 0 && *(short *)(&DAT_00f05970 + u * 2) == -0xfe)
///   return *(ushort *)((int)&DAT_00f05972 + u * 2);
/// return 0xe4;
/// ```
///
/// `index` is the cache built by [`w12a05_build_requirement_index`]; the
/// `0xffff` entry triggers the lazy rebuild exactly like the shipped body.
/// `error` mirrors `DAT_00fdf8e8` (cleared on entry).  The two table reads use
/// the fallback value when the cached index or the stream is truncated, which
/// the shipped body would read past its arrays.
pub fn w12a05_unit_requirement_head(
    unit: u16,
    stream: &[u16],
    index: &mut [u16; W12A05_UNIT_TYPE_COUNT],
    error: &mut u32,
) -> u16 {
    *error = 0;
    let unit_index = unit as usize;
    if unit_index >= W12A05_UNIT_TYPE_COUNT {
        return W12A05_REQUIREMENT_FALLBACK;
    }
    let mut start = index[unit_index];
    if start == W12A05_REQUIREMENT_END {
        *index = w12a05_build_requirement_index(stream);
        start = index[unit_index];
    }
    if start != 0 {
        let start = start as usize;
        if stream.get(start) == Some(&W12A05_REQUIREMENT_HEAD_OPCODE) {
            return stream
                .get(start + 1)
                .copied()
                .unwrap_or(W12A05_REQUIREMENT_FALLBACK);
        }
    }
    W12A05_REQUIREMENT_FALLBACK
}

/// `0x0065bdd0`: sift one value into a max-heap.
///
/// ```c
/// hole = param_2;
/// while (true) {
///   right = hole * 2 + 2;
///   if (len <= right) break;
///   left = right - 1;
///   pick = (a[left] <= a[right]) ? right : left;
///   a[hole] = a[pick];
///   hole = pick;
/// }
/// if (right == len) {                  // only the left child exists
///   a[hole] = a[right - 1];
///   hole = right - 1;
/// }
/// if (hole <= param_2) { a[hole] = value; return; }
/// do {
///   parent = (hole - 1) >> 1;
///   if (value <= a[parent]) break;
///   a[hole] = a[parent];
///   hole = parent;
/// } while (param_2 < parent);
/// a[hole] = value;
/// ```
///
/// `hole` is the 0-based element index, `len` the element count of the heap
/// prefix, and `value` the element the caller removed from (or pushes into)
/// the heap.  The comparison `a[left] <= a[right]` selects the right child on
/// ties, exactly like the shipped `<=`.
pub fn w12a05_adjust_heap(a: &mut [u16], hole: usize, len: usize, value: u16) {
    let top = hole;
    let mut hole = hole;
    loop {
        let left = hole * 2 + 1;
        let right = left + 1;
        if right >= len {
            if right == len {
                // Only the left child exists (`iVar2 == param_3`).
                a[hole] = a[left];
                hole = left;
            }
            break;
        }
        let child = if a[left] <= a[right] { right } else { left };
        a[hole] = a[child];
        hole = child;
    }
    if hole > top {
        loop {
            let parent = (hole - 1) / 2;
            if value <= a[parent] {
                break;
            }
            a[hole] = a[parent];
            hole = parent;
            if parent <= top {
                break;
            }
        }
    }
    a[hole] = value;
}

/// `0x0065bc90`: index of the median of the nine sampled elements.
///
/// The body is three median-of-three picks followed by a median of those
/// three.  With `n = end - first`, `step = n / 8`, the samples are
/// `{first, first+step, first+2*step}`, `{mid-step, mid, mid+step}` and
/// `{last-2*step, last-step, last}` where `mid = first + n/2` and
/// `last = end-1`.  The returned index is the pointer the caller swaps to the
/// end before partitioning; equal keys may select any of the equal samples,
/// exactly like the shipped pointer comparisons.
///
/// The caller (`0x0065ba20`) only enters with `n > 32`, so `step >= 4` and all
/// nine indices are distinct and in range.  Shorter slices still return a
/// valid index (`step == 0` degenerates to `first`).
pub fn w12a05_median_of_nine(a: &[u16], first: usize, end: usize) -> usize {
    let n = end - first;
    if n < 2 {
        return first;
    }
    let step = n / 8;
    let mid = first + n / 2;
    let last = end - 1;
    let low = w12a05_median_index(a, first, first + step, first + 2 * step);
    let middle = w12a05_median_index(a, mid - step, mid, mid + step);
    let high = w12a05_median_index(a, last - 2 * step, last - step, last);
    w12a05_median_index(a, low, middle, high)
}

/// The `_Med3`-style selection the three groups of `0x0065bc90` use: returns
/// the index of the middle value of `a[i]`, `a[j]`, `a[k]`.
fn w12a05_median_index(a: &[u16], i: usize, j: usize, k: usize) -> usize {
    let (x, y, z) = (a[i], a[j], a[k]);
    if x < y {
        if y < z {
            j
        } else if x < z {
            k
        } else {
            i
        }
    } else if z <= x {
        if z <= y { j } else { k }
    } else {
        i
    }
}

/// `0x0065ba20`: sort a `u16` slice.
///
/// The recovered introsort:
///
/// * a range of at most [`W12A05_SORT_ISORT_MAX`] elements is insertion
///   sorted;
/// * otherwise, with no depth budget left (`depth < 1`), it is heap sorted
///   through [`w12a05_adjust_heap`] (build the heap top-down from `n/2 - 1`
///   to 0, then pop `n - 1` times);
/// * otherwise the median-of-nine sample of [`w12a05_median_of_nine`] is
///   swapped to the last element, the body partitions it three ways with the
///   inlined Bentley-McIlroy equal-element blocks, `depth` is halved, the
///   left part recurses and the loop continues on the right part.
///
/// `depth` is the caller-supplied recursion budget (`0` therefore forces the
/// heap fallback for long ranges); callers in the binary pass their own
/// value, so it is an argument here instead of being invented.
pub fn w12a05_sort_u16(a: &mut [u16], depth: i32) {
    let end = a.len();
    let mut first = 0usize;
    let mut depth = depth;
    loop {
        let n = end - first;
        if n <= W12A05_SORT_ISORT_MAX || depth < 1 {
            if n <= W12A05_SORT_ISORT_MAX {
                w12a05_insertion_sort(&mut a[first..end]);
            } else {
                w12a05_heap_sort(&mut a[first..end]);
            }
            return;
        }
        let (left_end, right_start) = w12a05_partition(a, first, end);
        depth >>= 1;
        w12a05_sort_u16(&mut a[first..left_end], depth);
        first = right_start;
    }
}

/// The inlined `shortsort` of `0x0065ba20`: stable insertion sort.
fn w12a05_insertion_sort(a: &mut [u16]) {
    for i in 1..a.len() {
        let value = a[i];
        let mut hole = i;
        while hole > 0 && value < a[hole - 1] {
            a[hole] = a[hole - 1];
            hole -= 1;
        }
        a[hole] = value;
    }
}

/// The `make_heap` + `sort_heap` fallback of `0x0065ba20`.
fn w12a05_heap_sort(a: &mut [u16]) {
    let n = a.len();
    if n < 2 {
        return;
    }
    for hole in (0..n / 2).rev() {
        let value = a[hole];
        w12a05_adjust_heap(a, hole, n, value);
    }
    let mut len = n;
    while len > 1 {
        let value = a[len - 1];
        a[len - 1] = a[0];
        w12a05_adjust_heap(a, 0, len - 1, value);
        len -= 1;
    }
}

/// The partition of `0x0065ba20`: `partition(first, end) -> (left_end,
/// right_start)`.
///
/// The pivot is the median sampled by [`w12a05_median_of_nine`], swapped into
/// `end - 1`.  `pb`/`pc` are the crossing scans, `pa` the front equal block
/// and `pd` the back equal block.  The two recovered `vecswap` copies are
/// expanded to index arithmetic:
///
/// * the front equal block is `[first, pa)`; the recovered loop exchanges its
///   first `pa - first - 1` entries with the tail of the less-than region,
///   so the left recursion ends at `pb + 1 - max(pa - first, 1)`;
/// * the back block starts at `pb + 1` and swaps at most until `pd` reaches
///   the source cursor, so the right recursion starts after those swaps.
///
/// Everything between `left_end` and `right_start` is equal to the pivot and
/// is skipped.
fn w12a05_partition(a: &mut [u16], first: usize, end: usize) -> (usize, usize) {
    let last = end - 1;
    let median = w12a05_median_of_nine(a, first, end);
    a.swap(median, last);
    let pivot = a[last];
    let mut pb = first;
    let mut pc = last;
    let mut pa = first;
    let mut pd = last;
    let left_value = loop {
        while a[pb] < pivot {
            pb += 1;
        }
        loop {
            pc -= 1;
            if a[pc] <= pivot {
                break;
            }
            if pc == first {
                break;
            }
        }
        let left = a[pb];
        if pc <= pb {
            break left;
        }
        a[pb] = a[pc];
        a[pc] = left;
        let moved_from_right = a[pb];
        if moved_from_right == pivot {
            let displaced = a[pa];
            a[pa] = moved_from_right;
            a[pb] = displaced;
            pa += 1;
        }
        let moved_from_left = a[pc];
        if moved_from_left == pivot {
            pd -= 1;
            a[pc] = a[pd];
            a[pd] = moved_from_left;
        }
        pb += 1;
    };
    a[pb] = pivot;
    a[last] = left_value;

    let equal_front = pa - first;
    for i in 0..equal_front.saturating_sub(1) {
        a.swap(first + i, pb - 1 - i);
    }
    let left_end = pb + 1 - equal_front.max(1);

    let mut right_start = pb;
    let mut source = end - 2;
    loop {
        right_start += 1;
        if pd >= source {
            break;
        }
        a.swap(right_start, source);
        source -= 1;
    }
    (left_end, right_start)
}

#[cfg(test)]
mod tests_wave12_agent05 {
    use super::*;

    /// Deterministic LCG so the fuzz cases do not need a dependency.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
    }

    fn is_max_heap(a: &[u16], len: usize) -> bool {
        (0..len).all(|i| {
            let left = 2 * i + 1;
            let right = left + 1;
            (left >= len || a[i] >= a[left]) && (right >= len || a[i] >= a[right])
        })
    }

    fn triple_median(values: [u16; 3]) -> u16 {
        let mut values = values;
        values.sort();
        values[1]
    }

    fn expected_median_of_nine(a: &[u16], first: usize, end: usize) -> u16 {
        let n = end - first;
        let step = n / 8;
        let mid = first + n / 2;
        let last = end - 1;
        let medians = [
            triple_median([a[first], a[first + step], a[first + 2 * step]]),
            triple_median([a[mid - step], a[mid], a[mid + step]]),
            triple_median([a[last - 2 * step], a[last - step], a[last]]),
        ];
        triple_median(medians)
    }

    fn linspace(len: usize) -> Vec<u16> {
        (0..len).map(|i| i as u16).collect()
    }

    #[test]
    fn slot_player_byte_indexes_the_table_at_offset_db() {
        // 0x0065a100
        assert_eq!(W12A05_SLOT_PLAYER_TABLE, 0xdb);
        let table = [7u8, 4, 1, 0];
        assert_eq!(w12a05_slot_player_byte(&table, 0), 7);
        assert_eq!(w12a05_slot_player_byte(&table, 2), 1);
        assert_eq!(w12a05_slot_player_byte(&table, 3), 0);
    }

    #[test]
    fn player_limit_maps_the_single_player_format_to_two() {
        // 0x0065a0e0
        assert_eq!(w12a05_player_limit(W12A05_FORMAT_SINGLE_PLAYER, 6), 2);
        assert_eq!(w12a05_player_limit(0x02, 4), 4);
        assert_eq!(w12a05_player_limit(0x00, 0), 0);
    }

    #[test]
    fn session_active_needs_both_the_gate_and_a_game_type() {
        // 0x0065a7d0
        assert!(w12a05_game_session_active(true, 4));
        assert!(!w12a05_game_session_active(false, 4));
        assert!(!w12a05_game_session_active(true, 0));
        assert!(!w12a05_game_session_active(false, 0));
    }

    #[test]
    fn noop_touches_nothing() {
        // 0x0065a7c0
        w12a05_retail_noop();
    }

    #[test]
    fn requirement_none_is_the_all_ones_sentinel() {
        // 0x0065b71e
        assert_eq!(w12a05_requirement_none(), 0xffff_ffff);
    }

    #[test]
    fn requirement_error_round_trips() {
        // 0x0065b900 / 0x0065b910
        let mut state = 0u32;
        assert_eq!(w12a05_requirement_error(&state), 0);
        w12a05_set_requirement_error(&mut state, 0x17);
        assert_eq!(w12a05_requirement_error(&state), 0x17);
        w12a05_set_requirement_error(&mut state, 0);
        assert_eq!(w12a05_requirement_error(&state), 0);
    }

    /// `<unit 5: 0xff02 0x0007, 0x0010><unit 9: 0xff24>` from `0x0065b230`.
    fn sample_stream() -> Vec<u16> {
        vec![
            0xff02, // 0: the clause marker `0x0065b8a0` looks for
            0x0005, // 1: unit id 5
            0xff02, // 2: first opcode of unit 5 (the clause head)
            0x0007, // 3: the head operand
            0x0010, // 4
            0xffff, // 5: terminator
            0x0009, // 6: unit id 9
            0xff24, // 7: first opcode of unit 9
            0xffff, // 8: terminator
            0xffff, // 9: end marker
        ]
    }

    #[test]
    fn requirement_index_maps_each_clause_to_its_first_opcode() {
        // 0x0065b230
        let index = w12a05_build_requirement_index(&sample_stream());
        assert_eq!(index[5], 2);
        assert_eq!(index[9], 7);
        assert_eq!(index[0], 0, "untouched rows normalise to 0");
        assert_eq!(index[8], 0);
        assert_eq!(index[W12A05_UNIT_TYPE_COUNT - 1], 0);
    }

    #[test]
    fn requirement_index_defaults_when_the_stream_is_empty_or_closed() {
        // 0x0065b230
        assert_eq!(
            w12a05_build_requirement_index(&[]),
            [0; W12A05_UNIT_TYPE_COUNT]
        );
        assert_eq!(
            w12a05_build_requirement_index(&[0xff02, 0xffff, 0xffff]),
            [0; W12A05_UNIT_TYPE_COUNT],
            "a 0xffff first unit id skips the build"
        );
        let truncated = w12a05_build_requirement_index(&[0x0000, 0x0004, 0xff02]);
        assert_eq!(truncated[4], 2, "the entry is stored before the walk ends");
    }

    #[test]
    fn requirement_index_ignores_out_of_range_unit_ids() {
        // 0x0065b230 (guarded: the shipped store is unchecked)
        let stream = [
            0x0000, 0x0100, 0xff02, 0xffff, 0x0005, 0xff02, 0xffff, 0xffff,
        ];
        let index = w12a05_build_requirement_index(&stream);
        assert_eq!(index[5], 5);
    }

    #[test]
    fn unit_requirement_head_returns_the_clause_operand() {
        // 0x0065b8a0
        let stream = sample_stream();
        let mut index = w12a05_build_requirement_index(&stream);
        let mut error = 0xaa;
        assert_eq!(
            w12a05_unit_requirement_head(5, &stream, &mut index, &mut error),
            0x0007
        );
        assert_eq!(error, 0, "the entry clears DAT_00fdf8e8");
        assert_eq!(
            w12a05_unit_requirement_head(9, &stream, &mut index, &mut error),
            W12A05_REQUIREMENT_FALLBACK,
            "0xff24 is not the 0xff02 clause head"
        );
    }

    #[test]
    fn unit_requirement_head_rebuilds_a_stale_index() {
        // 0x0065b8a0
        let stream = sample_stream();
        let mut index = [W12A05_REQUIREMENT_END; W12A05_UNIT_TYPE_COUNT];
        let mut error = 1;
        assert_eq!(
            w12a05_unit_requirement_head(5, &stream, &mut index, &mut error),
            0x0007,
            "0xffff triggers the 0x0065b230 rebuild"
        );
        assert_eq!(index[5], 2);
        assert_eq!(error, 0);
    }

    #[test]
    fn unit_requirement_head_falls_back_without_a_clause() {
        // 0x0065b8a0
        let stream = sample_stream();
        let mut index = w12a05_build_requirement_index(&stream);
        let mut error = 0;
        assert_eq!(
            w12a05_unit_requirement_head(0, &stream, &mut index, &mut error),
            W12A05_REQUIREMENT_FALLBACK,
            "index 0 means no clause"
        );
        assert_eq!(
            w12a05_unit_requirement_head(0xe4, &stream, &mut index, &mut error),
            W12A05_REQUIREMENT_FALLBACK,
            "unit ids outside the table are guarded"
        );
    }

    #[test]
    fn adjust_heap_pushes_a_value_into_a_heap() {
        // 0x0065bdd0
        let mut heap = [10u16, 9, 8, 7, 6];
        assert!(is_max_heap(&heap, heap.len()));
        w12a05_adjust_heap(&mut heap, 0, 5, 5);
        assert_eq!(heap, [9, 7, 8, 5, 6]);
        assert!(is_max_heap(&heap, heap.len()));
    }

    #[test]
    fn adjust_heap_fills_the_last_even_slot() {
        // 0x0065bdd0: the `child == len` branch (only a left child exists).
        let mut heap = [9u16, 2, 8, 5];
        w12a05_adjust_heap(&mut heap, 1, 4, 1);
        assert_eq!(heap, [9, 5, 8, 1]);
    }

    #[test]
    fn adjust_heap_keeps_the_heap_property_for_random_values() {
        // 0x0065bdd0
        let mut rng = Lcg(0x5eed);
        for _ in 0..64 {
            let len = 2 + rng.below(32) as usize;
            let mut heap: Vec<u16> = (0..len).map(|_| rng.below(1000) as u16).collect();
            for hole in (0..len / 2).rev() {
                let value = heap[hole];
                w12a05_adjust_heap(&mut heap, hole, len, value);
            }
            assert!(
                is_max_heap(&heap, heap.len()),
                "make_heap produced {heap:?}"
            );
            let value = rng.below(1000) as u16;
            w12a05_adjust_heap(&mut heap, 0, len, value);
            assert!(is_max_heap(&heap, heap.len()), "push produced {heap:?}");
        }
    }

    #[test]
    fn median_of_nine_selects_the_median_of_the_three_groups() {
        // 0x0065bc90
        let mut rng = Lcg(0x1234_5678);
        for _ in 0..128 {
            let len = 33 + rng.below(200) as usize;
            let values: Vec<u16> = (0..len).map(|_| rng.below(10_000) as u16).collect();
            let picked = w12a05_median_of_nine(&values, 0, len);
            assert_eq!(values[picked], expected_median_of_nine(&values, 0, len));
        }
    }

    #[test]
    fn median_of_nine_accepts_duplicate_heavy_input() {
        // 0x0065bc90
        let values: Vec<u16> = (0..64).map(|i| (i % 3) as u16).collect();
        let picked = w12a05_median_of_nine(&values, 0, values.len());
        assert_eq!(
            values[picked],
            expected_median_of_nine(&values, 0, values.len())
        );
        assert_eq!(w12a05_median_of_nine(&[7], 0, 1), 0);
    }

    #[test]
    fn sort_matches_the_standard_sort_for_random_inputs() {
        // 0x0065ba20
        let mut rng = Lcg(0xdead_beef);
        for &len in &[0usize, 1, 2, 3, 7, 31, 32, 33, 34, 40, 100, 257, 1000] {
            for &depth in &[0i32, 1, 2, 4, 32] {
                let mut values: Vec<u16> = (0..len).map(|_| rng.below(50) as u16).collect();
                let mut expected = values.clone();
                expected.sort();
                w12a05_sort_u16(&mut values, depth);
                assert_eq!(values, expected, "len {len} depth {depth} did not sort");
            }
        }
    }

    #[test]
    fn sort_handles_extremes_and_long_runs() {
        // 0x0065ba20
        let mut values: Vec<u16> = (0..500).map(|_| 0).collect();
        values[0] = u16::MAX;
        values[499] = u16::MIN;
        values[250] = u16::MAX;
        let mut expected = values.clone();
        expected.sort();
        w12a05_sort_u16(&mut values, 64);
        assert_eq!(values, expected);

        let mut descending = linspace(2048);
        descending.reverse();
        let mut expected = descending.clone();
        expected.sort();
        w12a05_sort_u16(&mut descending, 64);
        assert_eq!(descending, expected);
    }

    #[test]
    fn sort_with_no_budget_still_sorts_long_ranges() {
        // 0x0065ba20's heap fallback
        let mut rng = Lcg(0xabcd_ef01);
        let mut values: Vec<u16> = (0..300).map(|_| rng.below(4096) as u16).collect();
        let mut expected = values.clone();
        expected.sort();
        w12a05_sort_u16(&mut values, 0);
        assert_eq!(values, expected);
    }

    #[test]
    fn sort_handles_adversarial_patterns() {
        // 0x0065ba20: ascending, descending, organ pipe, two values, sawtooth
        // and an all-equal range exercise every partition branch.
        let ascending = linspace(300);
        let mut descending = ascending.clone();
        descending.reverse();
        let organ_pipe: Vec<u16> = (0..300)
            .map(|i| if i < 150 { i as u16 } else { (299 - i) as u16 })
            .collect();
        let two_values: Vec<u16> = (0..301).map(|i| (i % 2) as u16).collect();
        let sawtooth: Vec<u16> = (0..301).map(|i| (i % 7) as u16).collect();
        let all_equal = vec![0x1234u16; 1000];
        for (name, pattern) in [
            ("ascending", ascending),
            ("descending", descending),
            ("organ pipe", organ_pipe),
            ("two values", two_values),
            ("sawtooth", sawtooth),
            ("all equal", all_equal),
        ] {
            for depth in [1i32, 2, 64] {
                let mut values = pattern.clone();
                let mut expected = pattern.clone();
                expected.sort();
                w12a05_sort_u16(&mut values, depth);
                assert_eq!(values, expected, "{name} at depth {depth}");
            }
        }
    }

    #[test]
    fn sort_matches_the_standard_sort_for_random_lengths() {
        // 0x0065ba20
        let mut rng = Lcg(0x0bad_f00d);
        for _ in 0..64 {
            let len = rng.below(600) as usize;
            let depth = rng.below(65) as i32;
            let mut values: Vec<u16> = (0..len)
                .map(|_| match rng.below(4) {
                    0 => 0,
                    1 => u16::MAX,
                    _ => rng.below(700) as u16,
                })
                .collect();
            let mut expected = values.clone();
            expected.sort();
            w12a05_sort_u16(&mut values, depth);
            assert_eq!(values, expected, "len {len} depth {depth}");
        }
    }

    #[test]
    fn sort_of_short_ranges_uses_insertion_sort() {
        // 0x0065ba20's `iVar7 < 0x21` path
        let mut short = vec![9u16, 1, 8, 2, 7, 3, 6, 4, 5];
        w12a05_sort_u16(&mut short, 0);
        assert_eq!(short, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let mut empty: Vec<u16> = Vec::new();
        w12a05_sort_u16(&mut empty, 3);
        assert!(empty.is_empty());
    }
}
