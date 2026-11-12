#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent50.rs
// ===========================================================================
// Translation batch 50.  Only this file is edited by the batch.
//
// Batch 50 owns `0x005f6000..=0x005f7fff`.  The ledger lists 31 rows in the
// window, all `decompiled_uncited`; 30 are translated below and one is
// skipped:
//
// * `FUN_005f6445` (1 byte) is the decoder's `halt_baddata()` marker.  The
//   cached body contains no instruction stream at all, so there is nothing
//   to reproduce.
//
// The window is the requirement/description plumbing of one C++ class
// (MSVC, allocator-aware `std::string`/`std::vector`).  Recovered member
// offsets that recur across the rows are:
//
// | offset          | meaning (as used by the bodies)                         |
// |-----------------|---------------------------------------------------------|
// | `+0xc`  (`+0x1c` size, `+0x20` cap) | member string copied into records     |
// | `+0x24..+0x28`  | intrusive child list; nodes expose a flags byte at `+4` and requirement data at `+8` |
// | `+0x70`/`+0x88` | two stem strings selected by the dword at `+0x98` (`+0x26` dwords) |
// | `+0x100..+0x104`| 0x34-stride list of `{key, string, string}` records      |
// | `+0xc4` (`+0xd4` size, `+0xd8` cap) | member string returned by `0x005f6460` |
// | `+0xfc`         | singly linked provider used by `0x005f6340`/`0x005f7a20` |
//
// None of the routines reads or writes `CUnit`/`World` simulation state, so
// the translations are deterministic [`World`] helpers over four local value
// types that mirror the recovered shapes: [`A50String`] (16-byte inline
// buffer, size at `+0x10`, capacity at `+0x14`), [`A50Requirement`] (the
// 0x1c vector element: kind byte, value byte, string at `+4`),
// [`A50Pair`] (the 0x34 list element: key, string at `+4`, string at
// `+0x1c`) and [`A50Child`] (a list node with a flags byte and vtable
// results).  Virtual calls whose targets live outside the window are
// abstracted as value inputs; every recovered numeric constant - capacity
// policy, strides, table offsets, the `0x9249249` vector limit and the
// signed `/28` multiply - is reproduced exactly and named in the doc
// comment of the routine that carries it.

/// Inline (`std::string`) capacity recovered at every `std::string`
/// construction site in the window, e.g. `0x005f6570`'s
/// `*(+0x14) = 0xf` and `0x005f6460`'s `0xf < *(+0xd8)` heap test.
pub const A50_SSO_CAPACITY: u32 = 0xf;

/// Largest representable string size: `0x7fffffff` clamps the growth policy
/// in the append/replace paths of `0x005f66a0`, `0x005f6ac4` and
/// `0x005f75ab` (`0x7fffffff - (cap >> 1) < cap`).
pub const A50_STRING_MAX: u32 = 0x7fff_ffff;

/// Allocation class boundary: capacities whose `cap + 1` byte count exceeds
/// `0xfff` take the `cap + 0x24` block-header path.
pub const A50_LARGE_BLOCK: u32 = 0xfff;

/// Header bytes charged for a large allocation (`uVar9 = cap + 0x24`).
pub const A50_BLOCK_HEADER: u32 = 0x24;

/// Allowed slack between a large block's stored pointer and its data
/// (`0x1f < (data - stored) - 4` is the `0x005f73f0` corruption trap).
pub const A50_ALIGN_SLACK: u32 = 0x1f;

/// `std::string` record size: 16 inline bytes + size + capacity.
pub const A50_STRING_SIZE: usize = 0x18;

/// Requirement vector element size (`0x005f7260` writes `+0x1c` per
/// `push_back`, and `0x005f7380` reads kind/value from its first dword).
pub const A50_REQUIREMENT_STRIDE: usize = 0x1c;

/// Pair-list element size: `0x005f66a0`/`0x005f6ac4` advance by `0xd`
/// dwords (`0x34` bytes), with strings at `+4` and `+0x1c`.
pub const A50_PAIR_STRIDE: usize = 0x34;

/// Name-list element size used by `0x005f66a0` (`0x18`, string only).
pub const A50_NAME_STRIDE: usize = 0x18;

/// `0x005f7260`'s growth guard: `0x9249249 < count` is the
/// `0xffffffff / 0x1c` element limit of the requirement vector.
pub const A50_MAX_ELEMENTS_1C: u64 = 0x0092_4924_9;

/// Two-byte separator the builder appends between entries
/// (`0x005f73f0`: `FUN_005fd9c0(&DAT_00d38418, 2)`).  The bytes live in a
/// data section that is not part of the analysed function bodies; the length
/// is exact and the value is the conventional ", ".
pub const A50_SEPARATOR: [u8; 2] = *b", ";

/// Character kind byte: `0` selects the named form (string payload and the
/// `0x3c`/`0x6c` table entries), any other value selects the single-byte
/// form (payload is the value byte and the table entries are `0x24`/`0x54`).
pub const A50_KIND_NAMED: u8 = 0;

/// Character kind byte for the single-byte form (`*puVar3 = 1` in
/// `0x005f7260`/`0x005f7380`).
pub const A50_KIND_CHAR: u8 = 1;

/// Value used when no requirement exists at all: the pair `0x2001` written by
/// `0x005f7380`, i.e. kind `1` and value `0x20` (a space placeholder).
pub const A50_DEFAULT_CHAR: u8 = 0x20;

/// Offset of the named label inside the display object (`0x005f73f0`:
/// `iVar5 = 0x24; if (*pcVar11 == 0) iVar5 = 0x3c;`).
pub const A50_LABEL_NAMED: usize = 0x3c;

/// Offset of the single-byte label inside the display object.
pub const A50_LABEL_CHAR: usize = 0x24;

/// Offset of the named suffix label (`iVar5 = 0x54; if (*pcVar11 == 0)
/// iVar5 = 0x6c;`).
pub const A50_LABEL_NAMED_SUFFIX: usize = 0x6c;

/// Offset of the single-byte suffix label.
pub const A50_LABEL_CHAR_SUFFIX: usize = 0x54;

/// Offset of the optional second-line table row when the selection bit is
/// clear (`param_2 + 0x100` and `+0x118`).
pub const A50_PAIR_ROW_LOW: usize = 0x100;

/// Offset of the optional second-line table row when the selection bit is
/// set (`param_2 + 0x130` and `+0x148`, one 0x30 row after the low row).
pub const A50_PAIR_ROW_HIGH: usize = 0x130;

/// Distance between the two second-line rows (`0x130 - 0x100`).
pub const A50_PAIR_ROW_STRIDE: usize = 0x30;

/// The flags bit skipped by the builder loops
/// (`(*(byte *)(node + 4) & 8) == 0`): nodes without it participate.
pub const A50_FLAG_SKIP: u8 = 0x8;

/// The flags bit that gates the linked-child scan of `0x005f7a20`
/// (`& 0x10`).
pub const A50_FLAG_LINKED: u8 = 0x10;

/// Magic multiplier of the signed `/28` sequence in `0x005f75ab`
/// (`0x92492493`, i.e. `-1840700269` as a 32-bit signed value).
pub const A50_DIV28_MAGIC: i32 = 0x9249_2493u32 as i32;

/// The shared `std::string` value model.  `cap == 0xf` (or less) means the
/// inline buffer is live; a larger capacity means the bytes were heap
/// allocated with the recovered `size | 0xf` / `cap + cap / 2` policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50String {
    pub text: String,
    pub cap: u32,
}

/// A list node as seen by the builders: the flags byte tested at `+4`, the
/// string a vtable probe returns, and the integer code probed by
/// `0x005f7a20`/`0x005f7aa0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Child {
    pub flags: u8,
    pub text: A50String,
    pub code: i32,
}

/// The 0x1c requirement vector element: kind/value in the first dword and the
/// string at `+4` (see `0x005f7260`, `0x005f7380`, `0x005f73f0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Requirement {
    pub kind: u8,
    pub value: u8,
    pub text: A50String,
}

/// The 0x34 pair-list element: key dword at `+0`, string at `+4`, string at
/// `+0x1c` (the strides used by `0x005f66a0`, `0x005f6ac4`, `0x005f6ba1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Pair {
    pub key: u32,
    pub text: A50String,
    pub second: A50String,
}

/// Unit-requirement node consumed by `0x005f7260`/`0x005f7380`; the value
/// byte is read at node offset `+8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A50UnitNode {
    pub value: u8,
}

/// Tech-requirement node consumed by `0x005f7260`/`0x005f7380`; the string
/// starts at node offset `+8`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50TechNode {
    pub text: A50String,
}

/// `0x005f7260`'s `std::length_error` path
/// (`if (0x9249249 < count) FUN_005fd610()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A50LengthError;

/// The non-returning tail of the catch-all funclets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A50Tail {
    /// Restore the exception list, then rethrow: no value reaches the caller.
    Rethrow,
}

/// Result of [`World::a50_scan_words`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Scan {
    /// Words flushed before the stop byte.
    pub words: Vec<A50String>,
    /// Index of the byte that stopped the scan (`'\n'`, or `bytes.len()`).
    pub next: usize,
    /// True when the stop byte was `'\n'`; false means the range ended and
    /// retail enters its unwind tail.
    pub newline: bool,
}

/// The four label strings of the `0x005f73f0` display object, in table order
/// (`0x24`, `0x3c`, `0x54`, `0x6c`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Labels {
    pub character: A50String,
    pub named: A50String,
    pub character_suffix: A50String,
    pub named_suffix: A50String,
}

/// The extra fields `0x005f73f0` appends when its `local_4c` is non-zero:
/// the count at `+0x1a8`, the name at `+0x1b0` and the tail at `+0x1c0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50Extended {
    pub count: u32,
    pub name: A50String,
    pub tail: A50String,
}

/// The four strings of the `0x005f7b20` second-line table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A50RenderTexts {
    pub low_first: A50String,
    pub low_second: A50String,
    pub high_first: A50String,
    pub high_second: A50String,
}

/// Builds a string with the recovered append policy: start empty (`size 0`,
/// capacity `0xf`), then extend.  Mirrors the `std::string` constructors seen
/// at `0x005f6460`, `0x005f6570` and the inline append at `0x005f66a0`.
pub fn a50_string_from_bytes(bytes: &[u8]) -> A50String {
    let mut s = a50_string_empty();
    let ok = a50_string_append(&mut s, bytes);
    debug_assert!(ok, "string length overflow is not representable");
    s
}

/// The empty string written by `0x005f6570`: size `0`, capacity `0xf`, first
/// byte `0`.
pub fn a50_string_empty() -> A50String {
    A50String {
        text: String::new(),
        cap: A50_SSO_CAPACITY,
    }
}

/// The capacity the append path of `0x005f66a0`/`0x005f75ab` computes for
/// `size + add` bytes when the current capacity `cap` is too small:
///
/// ```text
/// if (size + add > 0x7fffffff)            length_error
/// needed = (size + add) | 0xf
/// if (needed >= 0x80000000)               capacity = 0x7fffffff
/// else if (0x7fffffff - (cap >> 1) < cap) capacity = 0x7fffffff
/// else                                    capacity = max(needed, cap + cap / 2)
/// ```
///
/// Returns `None` for the length-error case.
pub fn a50_string_grow(size: u32, add: u32, cap: u32) -> Option<u32> {
    if add > A50_STRING_MAX - size {
        return None;
    }
    let needed = (size + add) | A50_SSO_CAPACITY;
    if needed >= 0x8000_0000 {
        return Some(A50_STRING_MAX);
    }
    if A50_STRING_MAX - (cap >> 1) < cap {
        return Some(A50_STRING_MAX);
    }
    let doubled = (cap >> 1) + cap;
    Some(needed.max(doubled))
}

/// Appends `bytes` with the policy of [`a50_string_grow`].  Returns `false`
/// for the length-error path instead of throwing.
pub fn a50_string_append(s: &mut A50String, bytes: &[u8]) -> bool {
    let Ok(add) = u32::try_from(bytes.len()) else {
        return false;
    };
    let size = u32::try_from(s.text.len()).unwrap_or(u32::MAX);
    if add <= s.cap.saturating_sub(size) {
        s.text.push_str(&String::from_utf8_lossy(bytes));
        return true;
    }
    let Some(cap) = a50_string_grow(size, add, s.cap) else {
        return false;
    };
    s.cap = cap;
    s.text.push_str(&String::from_utf8_lossy(bytes));
    true
}

/// Byte count handed to the CRT deleter for a string with capacity `cap`
/// (`0x005f6ac4`, `0x005f6af8`, `0x005f73f0`, `0x005f75ab`):
///
/// ```text
/// if (0xf < cap) {
///     n = cap + 1;
///     if (0xfff < n) { n = cap + 0x24;  /* header-bearing block */ }
/// }
/// ```
///
/// `None` means the bytes are inline and there is nothing to free.
pub fn a50_string_dealloc_bytes(cap: u32) -> Option<u32> {
    if cap <= A50_SSO_CAPACITY {
        return None;
    }
    let plain = cap + 1;
    if plain > A50_LARGE_BLOCK {
        Some(cap + A50_BLOCK_HEADER)
    } else {
        Some(plain)
    }
}

/// The alignment sanity test of the large-block release path: the stored
/// pointer must be within `0x1f` bytes below the data pointer, i.e.
/// `(data - stored) - 4 <= 0x1f`.
pub fn a50_large_block_header_ok(distance: u32) -> bool {
    distance.wrapping_sub(4) <= A50_ALIGN_SLACK
}

/// The `0x005f7260` element-limit test: `count <= 0xffffffff / 0x1c`.
pub fn a50_vector_count_fits_1c(count: usize) -> bool {
    count as u64 <= A50_MAX_ELEMENTS_1C
}

/// Signed divide-by-28 performed by `0x005f75ab`:
///
/// ```text
/// hi = (int)(((long long)magic * (long long)n) >> 32)
/// q  = hi + n
/// q  = (q >> 4) - (q >> 31)
/// ```
///
/// It is the element count of a `0x1c`-byte vector of length `n`; the
/// recovered `== 1` compare selects the single-element display mode.
pub fn a50_count_1c(byte_len: i32) -> i32 {
    let hi = ((i64::from(byte_len) * i64::from(A50_DIV28_MAGIC)) >> 32) as i32;
    let q = hi.wrapping_add(byte_len);
    (q >> 4).wrapping_sub(q >> 31)
}

/// `0x005f7b20` reads the selection bit as `(in_ECX[1] >> 1) & 1`; the set
/// state selects the second-line row at `+0x130`/`+0x148` instead of
/// `+0x100`/`+0x118`.
pub fn a50_second_bit(word: u32) -> bool {
    (word >> 1) & 1 == 1
}

impl World {
    /// `0x005f6028` (`FUN_005f6028`, 107 bytes, `decompiled_uncited`).
    ///
    /// The body is a word scanner over a byte range.  It pre-increments the
    /// cursor, so the byte at the cursor itself is never examined:
    ///
    /// ```c
    /// do {
    ///   while (true) {
    ///     unaff_EDI = unaff_EDI + 1;
    ///     if (unaff_EDI == end) {           /* flush pending word, then unwind */
    ///     }
    ///     local = *unaff_EDI;
    ///     if (FUN_00c983ad(local) != 0) break;  /* separator */
    ///     FUN_006024d0();                       /* append byte */
    ///   }
    ///   if (pending != 0) { push_back(pending); pending = 0; }
    /// } while (local != '\n');
    /// ```
    ///
    /// Every separator flushes the accumulated word; only `'\n'` ends the
    /// scan (other separators are dropped and scanning continues).  Running
    /// past the end flushes and enters the non-returning tail, reported here
    /// as `newline == false` with `next == bytes.len()`.  The separator
    /// predicate is the out-of-window call `FUN_00c983ad`, so it is supplied
    /// by the caller.
    pub fn a50_scan_words<F>(&self, bytes: &[u8], cursor: usize, mut is_separator: F) -> A50Scan
    where
        F: FnMut(u8) -> bool,
    {
        let mut words = Vec::new();
        let mut pending: Option<A50String> = None;
        let mut i = cursor;
        loop {
            i += 1;
            if i >= bytes.len() {
                if let Some(word) = pending.take() {
                    words.push(word);
                }
                return A50Scan {
                    words,
                    next: i,
                    newline: false,
                };
            }
            let byte = bytes[i];
            if is_separator(byte) {
                if let Some(word) = pending.take() {
                    words.push(word);
                }
                if byte == b'\n' {
                    return A50Scan {
                        words,
                        next: i,
                        newline: true,
                    };
                }
            } else {
                let word = pending.get_or_insert_with(a50_string_empty);
                let _ = a50_string_append(word, &[byte]);
            }
        }
    }

    /// `0x005f609c` (`FUN_005f609c`, 28 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// ExceptionList = *(void **)(unaff_EBP + -0xc);
    /// puStack00000010 = &UNK_005f60b8;
    /// FUN_00c86b6e();
    /// ```
    ///
    /// A catch-all funclet: restore the saved exception list, then tail-call
    /// the CRT rethrow stub.  It never returns to its caller; the translation
    /// exposes that contract as [`A50Tail::Rethrow`] and touches no state.
    pub fn a50_catch_all_tail(&self) -> A50Tail {
        A50Tail::Rethrow
    }

    /// `0x005f60d0` (`FUN_005f60d0`, 334 bytes, `decompiled_uncited`).
    ///
    /// Collects requirement strings either from the object's own provider or
    /// from its child list:
    ///
    /// ```c
    /// if (in_ECX[0x3f] == 0) {              /* this+0xfc */
    ///   if (!vtable_plus_4()) { empty vector }
    ///   else for (p = in_ECX[9]; p != in_ECX[10]; ++p)
    ///          push(vtable_plus_0x10(*p));
    /// } else {
    ///   vtable_plus_0x10(in_ECX[0x3f]);     /* single provider */
    /// }
    /// ```
    ///
    /// The two virtual probes are modelled as the supplied values: `own` is
    /// the single provider, `enabled` is the `vtable+4` predicate and
    /// `children` are the `+0x24..+0x28` nodes.
    pub fn a50_collect_requirement_texts(
        &self,
        own: Option<&A50Child>,
        enabled: bool,
        children: &[A50Child],
    ) -> Vec<A50String> {
        if let Some(own) = own {
            return vec![own.text.clone()];
        }
        if !enabled {
            return Vec::new();
        }
        children.iter().map(|c| c.text.clone()).collect()
    }

    /// `0x005f6240` (`FUN_005f6240`, 94 bytes, `decompiled_uncited`).
    ///
    /// The child-list-only variant of the same loop:
    ///
    /// ```c
    /// push = *(undefined4 **)(in_ECX + 0x24);
    /// end  = *(undefined4 **)(in_ECX + 0x28);
    /// while (push != end) { (*push)->vtable_0x10(&tmp); push_string(tmp); ++push; }
    /// ```
    ///
    /// The 0x18-byte string push and the large-block release path are the
    /// same plumbing translated by [`World::a50_extend_requirement_texts`].
    pub fn a50_child_requirement_texts(&self, children: &[A50Child]) -> Vec<A50String> {
        children.iter().map(|c| c.text.clone()).collect()
    }

    /// `0x005f629f` (`FUN_005f629f`, 123 bytes, `decompiled_uncited`).
    ///
    /// The exception-path copy of the child loop: it appends into an existing
    /// vector (`*(unaff_EDI + 4)` is the vector end) and returns after the
    /// range is consumed.  Returns how many strings were appended.
    pub fn a50_extend_requirement_texts(
        &self,
        out: &mut Vec<A50String>,
        children: &[A50Child],
    ) -> usize {
        let before = out.len();
        out.extend(children.iter().map(|c| c.text.clone()));
        out.len() - before
    }

    /// `0x005f6340` (`FUN_005f6340`, 24 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// do { iVar1 = in_ECX; in_ECX = *(int *)(iVar1 + 0xfc); } while (in_ECX != 0);
    /// return iVar1 + 0xd0;
    /// ```
    ///
    /// Follows the provider chain held at `+0xfc` until a null link and
    /// returns the last node (retail returns `node + 0xd0`, the string member
    /// inside that node).  Links are given as indices into `links`; a null
    /// link is `None`.  A `start` outside the table, or a link pointing
    /// outside it, is the case retail would fault on and is reported as
    /// `None`.
    pub fn a50_chain_tail(&self, links: &[Option<usize>], start: usize) -> Option<usize> {
        let mut node = start;
        loop {
            match *links.get(node)? {
                Some(next) => {
                    if next >= links.len() {
                        return None;
                    }
                    node = next;
                }
                None => return Some(node),
            }
        }
    }

    /// `0x005f6460` (`FUN_005f6460`, 271 bytes, `decompiled_uncited`).
    ///
    /// Streams the member string at `+0xc4` into an `ostringstream` and
    /// extracts it again:
    ///
    /// ```c
    /// if (0xf < *(uint *)(in_ECX + 0xd8)) piVar1 = (int *)*piVar1;  /* heap */
    /// FUN_005ef9b0(stream, piVar1, *(undefined4 *)(in_ECX + 0xd4));
    /// ```
    ///
    /// The copy starts from an empty stream buffer, so the result gets the
    /// append policy's capacity (`len | 0xf` once the text no longer fits
    /// inline) rather than the source capacity.
    pub fn a50_member_string_text(&self, member: &A50String) -> A50String {
        a50_string_from_bytes(member.text.as_bytes())
    }

    /// `0x005f6570` (`FUN_005f6570`, 27 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// *(undefined4 *)(param_1 + 0x10) = 0;    /* size   */
    /// *(undefined4 *)(param_1 + 0x14) = 0xf;  /* inline */
    /// *param_1 = 0;                           /* first byte */
    /// ```
    ///
    /// The canonical empty-string writer: size `0`, capacity `0xf`, zeroed
    /// first byte.
    pub fn a50_empty_string(&self) -> A50String {
        a50_string_empty()
    }

    /// `0x005f6590` (`FUN_005f6590`, 224 bytes, `decompiled_uncited`).
    ///
    /// Seeds the 0x34 record handed to the vector builder.  Two locals start
    /// as empty strings (`0`, `0xf`); the member string at `+0xc` is copied
    /// into the record's first string (guarded by the usual self-assignment
    /// test `local_44 != this + 0xc`); the caller's 32-bit key (`param_3`) is
    /// stored in the record header and rendered into the second string by the
    /// out-of-window formatter.  The record is then moved with the two-string
    /// move helper and torn down by the vector destructor.
    ///
    /// The key's rendered form is the only piece that cannot be recovered
    /// from this body; the translation renders it as its decimal text.
    pub fn a50_seed_pair(&self, member: &A50String, key: u32) -> A50Pair {
        A50Pair {
            key,
            text: a50_string_from_bytes(member.text.as_bytes()),
            second: a50_string_from_bytes(key.to_string().as_bytes()),
        }
    }

    /// `0x005f66a0` (`FUN_005f66a0`, 81 bytes, `decompiled_uncited`).
    ///
    /// The name list formatter of the builder.  It walks the 0x18-stride
    /// string list and joins the entries with a single `0x20` separator:
    ///
    /// ```c
    /// bVar2 = true;
    /// for (puVar12 = first; puVar12 != last; puVar12 += 6) {
    ///   if (!bVar2) FUN_005e99b0(stream, 0x20);   /* one space */
    ///   bVar2 = false;
    ///   if (0xf < puVar12[5]) puVar7 = (undefined4 *)*puVar12;
    ///   FUN_005ef9b0(stream, puVar7, puVar12[4]); /* append text */
    /// }
    /// ```
    ///
    /// The flags-`0x8` child filter that surrounds this loop in the same body
    /// is translated separately at [`World::a50_buildable_texts`].
    pub fn a50_join_names(&self, names: &[A50String]) -> A50String {
        let mut out = a50_string_empty();
        for (i, name) in names.iter().enumerate() {
            if i != 0 {
                let _ = a50_string_append(&mut out, &[0x20]);
            }
            let _ = a50_string_append(&mut out, name.text.as_bytes());
        }
        out
    }

    /// `0x005f66f3` (`FUN_005f66f3`, 442 bytes, `decompiled_uncited`).
    ///
    /// The inlined clone of the same builder; the recovered per-node test is
    /// identical in both copies:
    ///
    /// ```c
    /// for (piVar14 = in_ECX[9]; piVar14 != in_ECX[10]; ++piVar14) {
    ///   if ((*(byte *)((int *)*piVar14 + 1) & 8) == 0) { /* include */ }
    /// }
    /// ```
    ///
    /// Returns the texts of the participating nodes in list order.
    pub fn a50_buildable_texts(&self, children: &[A50Child]) -> Vec<A50String> {
        children
            .iter()
            .filter(|c| c.flags & A50_FLAG_SKIP == 0)
            .map(|c| c.text.clone())
            .collect()
    }

    /// `0x005f6895` (`FUN_005f6895`, 97 bytes, `decompiled_uncited`).
    ///
    /// The builder's success tail.  It copies the three vector words into the
    /// caller's out-parameter, then clears the locals, runs the vector
    /// destructor and enters the non-returning unwind tail:
    ///
    /// ```c
    /// puVar1 = *(undefined4 **)(unaff_EBP + -0x80);
    /// *puVar1 = *(undefined4 *)(unaff_EBP + -0x68);
    /// puVar1[1] = piVar9;
    /// puVar1[2] = uVar2;
    /// ... FUN_005fd380();  /* destructor */
    /// FUN_00c86b6e();
    /// ```
    ///
    /// The move is `std::mem::take`; the destructor has nothing to release in
    /// the Rust value model.
    pub fn a50_take_texts(&self, out: &mut Vec<A50String>) -> Vec<A50String> {
        std::mem::take(out)
    }

    /// `0x005f69e4` (`FUN_005f69e4`, 224 bytes, `decompiled_uncited`).
    ///
    /// The builder's child loop, entered after the current node has already
    /// been processed.  It advances with a pre-increment and skips every node
    /// whose flags byte has bit `0x8` set:
    ///
    /// ```c
    /// do {
    ///   (*in_EDX)(&tmp, ...); push(tmp);      /* process node at EDI */
    ///   do {
    ///     unaff_EDI = unaff_EDI + 1;
    ///     if (unaff_EDI == unaff_EBX) goto tail;
    ///   } while ((*(byte *)((int *)*unaff_EDI + 1) & 8) != 0);
    ///   in_EDX = (*unaff_EDI)->vtable + 0xc;
    /// } while (true);
    /// ```
    ///
    /// Returns the new cursor and the next participating node's text, or
    /// `None` at the end of the list.
    pub fn a50_append_texts_from(
        &self,
        children: &[A50Child],
        start: usize,
    ) -> (usize, Option<A50String>) {
        let mut i = start;
        loop {
            i += 1;
            if i >= children.len() {
                return (i, None);
            }
            if children[i].flags & A50_FLAG_SKIP == 0 {
                return (i, Some(children[i].text.clone()));
            }
        }
    }

    /// `0x005f6ac4` (`FUN_005f6ac4`, 52 bytes, `decompiled_uncited`).
    ///
    /// The 0x34-stride pair loop of the builder.  The recoverable arithmetic
    /// is the key adjustment performed for every record
    /// (`*(int *)(unaff_EBP + -0x5c) = key + base`), with the two strings at
    /// `+4`/`+0x1c` copied and released per iteration; the loop's release
    /// path also implements the large-block check reproduced by
    /// [`a50_string_dealloc_bytes`].  Keys are signed 32-bit additions.
    pub fn a50_pair_keys(&self, pairs: &[A50Pair], base: i32) -> Vec<i32> {
        pairs
            .iter()
            .map(|p| (p.key as i32).wrapping_add(base))
            .collect()
    }

    /// `0x005f6af8` (`FUN_005f6af8`, 173 bytes, `decompiled_uncited`).
    ///
    /// The same 0x34-stride key loop, but this funclet's loop guard tests the
    /// two copied strings' capacities:
    ///
    /// ```c
    /// uVar1 = *(uint *)(unaff_EBP + -0x2c);
    /// ...
    /// if (0xf < uVar1) { release(...); }
    /// uVar1 = *(uint *)(unaff_EBP + -0x44);
    /// unaff_EDI = puVar5;
    /// if (0xf < uVar1) { release(...); }
    /// ```
    ///
    /// The loop therefore continues only while both strings still use the
    /// inline buffer (`cap <= 0xf`); the first heap-backed string exits into
    /// the release path.  Keys are `key + base`.
    pub fn a50_pair_keys_while_inline(&self, pairs: &[A50Pair], base: i32) -> Vec<i32> {
        let mut out = Vec::new();
        for pair in pairs {
            if pair.text.cap > A50_SSO_CAPACITY || pair.second.cap > A50_SSO_CAPACITY {
                break;
            }
            out.push((pair.key as i32).wrapping_add(base));
        }
        out
    }

    /// `0x005f6ba1` (`FUN_005f6ba1`, 148 bytes, `decompiled_uncited`).
    ///
    /// The terminal `push_back` fragment: the vector end is advanced by one
    /// 0x34 record (`iVar3 = unaff_ESI + 0x34`), the record at the old end is
    /// constructed, the three vector words are stored to the out-parameter,
    /// the locals are zeroed and the destructor runs before the unwind tail.
    /// The translation pushes and hands the vector to the caller.
    pub fn a50_push_pair_and_take(&self, pairs: &mut Vec<A50Pair>, pair: A50Pair) -> Vec<A50Pair> {
        pairs.push(pair);
        std::mem::take(pairs)
    }

    /// `0x005f6c60` (`FUN_005f6c60`, 150 bytes, `decompiled_uncited`).
    ///
    /// Collects the member string followed by the participating children:
    ///
    /// ```c
    /// bVar4 = *(int *)(in_ECX + 0x1c) != 0;   /* member size != 0 */
    /// local_18 = 1;
    /// if (bVar4) push(in_ECX + 0xc);
    /// local_14 = (uint)bVar4;
    /// for (piVar3 = in_ECX[9]; piVar3 != in_ECX[10]; ++piVar3) {
    ///   if ((*(byte *)((int *)*piVar3 + 1) & 8) == 0) {
    ///     (*piVar3)->vtable_0xc(&tmp, param_2, local_14 + param_3);
    ///     push(tmp);
    ///   }
    /// }
    /// ```
    ///
    /// `local_14` (zero or one) is added to the caller's byte offset before
    /// the child probe; the offset itself is only observable through that
    /// out-of-window call, so the translation returns the text order.
    pub fn a50_collect_texts_with_member(
        &self,
        member: &A50String,
        children: &[A50Child],
    ) -> Vec<A50String> {
        let mut out = Vec::new();
        if !member.text.is_empty() {
            out.push(member.clone());
        }
        for child in children {
            if child.flags & A50_FLAG_SKIP == 0 {
                out.push(child.text.clone());
            }
        }
        out
    }

    /// `0x005f6cf6` (`FUN_005f6cf6`, 93 bytes, `decompiled_uncited`).
    ///
    /// The other child loop shape: the flags test is performed on the entry
    /// *before* advancing, and a participating node is probed immediately:
    ///
    /// ```c
    /// while (true) {
    ///   if (ZF) { (*in_ECX)->vtable_0xc(&tmp, ...); push(tmp); unaff_EDI = ...; }
    ///   unaff_ESI = unaff_ESI + 1;
    ///   if (unaff_ESI == unaff_EBX) break;
    ///   in_ECX = (int *)*unaff_ESI;
    ///   ZF = (*(byte *)(in_ECX + 1) & 8) == 0;
    /// }
    /// return unaff_EDI;
    /// ```
    ///
    /// Returns the first participating node at or after `start`.
    pub fn a50_first_buildable_from(
        &self,
        children: &[A50Child],
        start: usize,
    ) -> Option<(usize, A50String)> {
        for (i, child) in children.iter().enumerate().skip(start) {
            if child.flags & A50_FLAG_SKIP == 0 {
                return Some((i, child.text.clone()));
            }
        }
        None
    }

    /// `0x005f6d60` (`FUN_005f6d60`, 45 bytes, `decompiled_uncited`).
    ///
    /// Builds the separator string.  The body first moves the 0x34 record
    /// returned by the `vtable+0x40` probe (`puVar1[4] = 0; puVar1[5] = 0xf;
    /// *puVar1 = 0` after the copy), copies the member string at `+0xc` into a
    /// local (capacity `+0x20` decides the heap pointer), and finishes by
    /// assigning the two-byte constant at `DAT_00d38418`:
    ///
    /// ```c
    /// FUN_005fdb20(&DAT_00d38418, 2);
    /// ```
    ///
    /// The record move and member copy are pure value movement; the
    /// reproducible terminal effect is the two-byte string with the append
    /// policy's capacity.
    pub fn a50_separator_string(&self) -> A50String {
        a50_string_from_bytes(&A50_SEPARATOR)
    }

    /// `0x005f6d8d` (`FUN_005f6d8d`, 286 bytes, `decompiled_uncited`).
    ///
    /// The sibling of `0x005f6d60`: same 0x34 record move, same member copy
    /// (`piVar8 = in_ECX + 3`, member string at `+0xc`, capacity at `+0x20`)
    /// and same two-byte separator assignment, plus one extra dword stored at
    /// frame `-0x5c` from the third argument.  Returns the moved record, the
    /// member copy and that hint dword.
    pub fn a50_move_pair_with_hint(
        &self,
        member: &A50String,
        produced: A50Pair,
        hint: u32,
    ) -> (A50Pair, A50String, u32) {
        (
            produced,
            a50_string_from_bytes(member.text.as_bytes()),
            hint,
        )
    }

    /// `0x005f7260` (`FUN_005f7260`, 280 bytes, `decompiled_uncited`).
    ///
    /// Merges the two intrusive requirement lists into one vector.  The size
    /// guard is the first thing the body does:
    ///
    /// ```c
    /// uVar6 = *(int *)(in_ECX + 0x28) + *(int *)(in_ECX + 8);
    /// if (uVar6 != 0) {
    ///   if (0x9249249 < uVar6) FUN_005fd610();   /* length_error */
    ///   FUN_005fd100(uVar6);                     /* reserve */
    /// }
    /// ```
    ///
    /// The first list (`this+4`) contributes single-byte requirements: the
    /// record gets kind `1` and the node's byte at `+8`.  The second list
    /// (`this+0x24`) contributes named requirements: kind `0` and the node's
    /// string at `+8` (`FUN_005f12e0(puVar2 + 2)`).  Order is units first,
    /// then techs, exactly as the two loops appear.
    pub fn a50_merge_requirements(
        &self,
        units: &[A50UnitNode],
        techs: &[A50TechNode],
    ) -> Result<Vec<A50Requirement>, A50LengthError> {
        let count = units.len().saturating_add(techs.len());
        if !a50_vector_count_fits_1c(count) {
            return Err(A50LengthError);
        }
        let mut out = Vec::with_capacity(count);
        for unit in units {
            out.push(A50Requirement {
                kind: A50_KIND_CHAR,
                value: unit.value,
                text: a50_string_empty(),
            });
        }
        for tech in techs {
            out.push(A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: tech.text.clone(),
            });
        }
        Ok(out)
    }

    /// `0x005f7380` (`FUN_005f7380`, 97 bytes, `decompiled_uncited`).
    ///
    /// Produces the object's first requirement:
    ///
    /// ```c
    /// if (*(int *)(in_ECX + 0x28) != 0) {       /* secondary list present */
    ///   *param_1 = 0;                           /* kind = named */
    ///   FUN_005f12e0(*(int *)(in_ECX + 0x24) + 8);
    /// } else if (*(int *)(in_ECX + 8) == 0) {
    ///   *param_1 = 0x2001;                      /* kind 1, value 0x20 */
    /// } else {
    ///   *(byte *)param_1 = 1;
    ///   *((byte *)param_1 + 1) = *(byte *)(**(int **)(in_ECX + 4) + 8);
    /// }
    /// ```
    ///
    /// The empty case is the `0x2001` pair, i.e. kind `1` with value `0x20`
    /// (the space placeholder).  The named case carries the first tech node's
    /// string; the unit case carries the first unit node's value byte.
    pub fn a50_default_requirement(
        &self,
        units: &[A50UnitNode],
        techs: &[A50TechNode],
    ) -> A50Requirement {
        if let Some(tech) = techs.first() {
            return A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: tech.text.clone(),
            };
        }
        let value = units.first().map_or(A50_DEFAULT_CHAR, |unit| unit.value);
        A50Requirement {
            kind: A50_KIND_CHAR,
            value,
            text: a50_string_empty(),
        }
    }

    /// `0x005f73f0` (`FUN_005f73f0`, 430 bytes, `decompiled_uncited`).
    ///
    /// Formats a requirement vector.  The recovered loop is:
    ///
    /// ```c
    /// if ((int)local_a8 - (int)local_ac) / 0x1c == 1) cStack_91 = '\0';
    /// else cStack_91 = *(char *)(param_2 + 0x194);   /* collapse */
    /// for each entry {
    ///   if (not first) FUN_005fd9c0(&DAT_00d38418, 2);          /* ", "  */
    ///   iVar5 = (*entry == 0) ? 0x3c : 0x24;
    ///   append(display + iVar5);                                /* label */
    ///   if (*entry == 0) append(entry->string); else append(char entry[1]);
    ///   if (extra != 0 && (!collapse || last entry)) {
    ///     iVar5 = (*entry == 0) ? 0x6c : 0x54;
    ///     append(display + iVar5);                              /* suffix */
    ///     append(count); append(name); append(tail);
    ///   }
    /// }
    /// ```
    ///
    /// A single-entry vector forces `collapse` off, so its extra block is
    /// always emitted.  The count is the `+0x1a8` value plus the `local_4c`
    /// base; the translation takes the already-adjusted count.
    pub fn a50_format_requirements(
        &self,
        labels: &A50Labels,
        requirements: &[A50Requirement],
        collapse: bool,
        extended: Option<&A50Extended>,
    ) -> A50String {
        let collapse = collapse && requirements.len() != 1;
        let mut out = a50_string_empty();
        let last = requirements.len().saturating_sub(1);
        for (i, requirement) in requirements.iter().enumerate() {
            if i != 0 {
                let _ = a50_string_append(&mut out, &A50_SEPARATOR);
            }
            let (primary, suffix) = if requirement.kind == A50_KIND_NAMED {
                (&labels.named, &labels.named_suffix)
            } else {
                (&labels.character, &labels.character_suffix)
            };
            let _ = a50_string_append(&mut out, primary.text.as_bytes());
            if requirement.kind == A50_KIND_NAMED {
                let _ = a50_string_append(&mut out, requirement.text.text.as_bytes());
            } else {
                let _ = a50_string_append(&mut out, &[requirement.value]);
            }
            if let Some(extra) = extended {
                if !collapse || i == last {
                    let _ = a50_string_append(&mut out, suffix.text.as_bytes());
                    let _ = a50_string_append(&mut out, extra.count.to_string().as_bytes());
                    let _ = a50_string_append(&mut out, extra.name.text.as_bytes());
                    let _ = a50_string_append(&mut out, extra.tail.text.as_bytes());
                }
            }
        }
        out
    }

    /// `0x005f75ab` (`FUN_005f75ab`, 1060 bytes, `decompiled_uncited`).
    ///
    /// The inlined copy of the formatter opens with the signed `/28`
    /// element-count sequence:
    ///
    /// ```c
    /// iVar13 = (int)((ulonglong)((longlong)in_EAX * (longlong)in_ECX) >> 0x20)
    ///          + in_ECX;
    /// if ((iVar13 >> 4) - (iVar13 >> 0x1f) == 1) uVar5 = 0;
    /// else uVar5 = *(undefined1 *)(*(int *)(unaff_EBP + -0x98) + 0x194);
    /// ```
    ///
    /// The multiply uses `0x92492493` (= [`A50_DIV28_MAGIC`]) and yields the
    /// number of `0x1c` elements in `in_ECX` bytes; the `== 1` compare selects
    /// the single-element display mode.  The rest of the body repeats the
    /// label/payload/suffix loop of [`World::a50_format_requirements`].
    pub fn a50_count_1c(&self, byte_len: i32) -> i32 {
        a50_count_1c(byte_len)
    }

    /// `0x005f79e6` (`FUN_005f79e6`, 6 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// void FUN_005f79e6(void) { return; }
    /// ```
    ///
    /// An empty virtual stub: no state, no result.
    pub fn a50_noop(&self) {}

    /// `0x005f7a00` (`FUN_005f7a00`, 18 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// (**(code **)(*in_ECX + 0x44))(param_1);
    /// return param_1;
    /// ```
    ///
    /// A forwarding shim for the `vtable+0x44` pair producer: the callee
    /// fills the caller's 0x34 result slot and the same slot is returned.  The
    /// callee lives outside the window, so only the forwarding contract is
    /// reproduced.
    pub fn a50_forward_pair(&self, pair: A50Pair) -> A50Pair {
        pair
    }

    /// `0x005f7a20` (`FUN_005f7a20`, 125 bytes, `decompiled_uncited`).
    ///
    /// The first-error search.  When the linked provider at `+0xfc` exists it
    /// is asked first, then the child list is scanned with the `0x10` flags
    /// gate; otherwise the direct provider at `+0x11c` is asked, then (only
    /// when `vtable+4` is true) every child is scanned without a flags gate:
    ///
    /// ```c
    /// if (in_ECX[0x3f] == 0) {
    ///   if (in_ECX[0x47] != 0) return probe(in_ECX[0x47]);
    ///   if (!vtable_plus_4()) return 0;
    ///   for (p = in_ECX[9]; p != in_ECX[10]; ++p)
    ///     if ((code = probe(*p)) != 0) return code;
    /// } else {
    ///   if ((code = probe(in_ECX[0x3f])) != 0) return code;
    ///   for (p = in_ECX[9]; p != in_ECX[10]; ++p)
    ///     if ((flags(*p) & 0x10) && (code = probe(*p)) != 0) return code;
    /// }
    /// return 0;
    /// ```
    ///
    /// `probe` is the abstracted `vtable+0x1c`; [`A50Child::code`] carries its
    /// result.
    pub fn a50_first_error(
        &self,
        linked: Option<&A50Child>,
        direct: Option<&A50Child>,
        enabled: bool,
        children: &[A50Child],
    ) -> i32 {
        if let Some(linked) = linked {
            if linked.code != 0 {
                return linked.code;
            }
            for child in children {
                if child.flags & A50_FLAG_LINKED != 0 && child.code != 0 {
                    return child.code;
                }
            }
            return 0;
        }
        if let Some(direct) = direct {
            return direct.code;
        }
        if enabled {
            for child in children {
                if child.code != 0 {
                    return child.code;
                }
            }
        }
        0
    }

    /// `0x005f7aa0` (`FUN_005f7aa0`, 39 bytes, `decompiled_uncited`).
    ///
    /// The unfiltered first-error search over the `+0x24..+0x28` list:
    ///
    /// ```c
    /// while (true) {
    ///   if (puVar3 == puVar1) return 0;
    ///   iVar2 = (*puVar3)->vtable_0x1c();
    ///   if (iVar2 != 0) break;
    ///   puVar3 = puVar3 + 1;
    /// }
    /// return iVar2;
    /// ```
    pub fn a50_first_error_any(&self, children: &[A50Child]) -> i32 {
        children
            .iter()
            .map(|c| c.code)
            .find(|code| *code != 0)
            .unwrap_or(0)
    }

    /// `0x005f7ad0` (`FUN_005f7ad0`, 66 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// cVar1 = (**(code **)(*in_ECX + 4))();
    /// if (cVar1 == '\0') { *param_1 = 0; param_1[1] = 0; param_1[2] = 0; }
    /// else FUN_005f5a00(param_1, param_2);
    /// return param_1;
    /// ```
    ///
    /// Returns the 0x18-stride name list when the `vtable+4` predicate is
    /// true, otherwise an empty vector.
    pub fn a50_requirement_texts_if_enabled(
        &self,
        enabled: bool,
        children: &[A50Child],
    ) -> Vec<A50String> {
        if !enabled {
            return Vec::new();
        }
        children.iter().map(|c| c.text.clone()).collect()
    }

    /// `0x005f7b20` (`FUN_005f7b20`, 212 bytes, `decompiled_uncited`).
    ///
    /// The last stage of the pair builder.  The recovered flow is:
    ///
    /// ```c
    /// if (*(char *)(param_2 + 0x160) == '\0') { empty result; unwind; }
    /// ... produce the stem record via vtable+0x48 / vtable+0x44 ...
    /// if (*(char *)(param_2 + 0x161) == '\0') req = REQUIRED_FIRST(param_2);
    /// else                                    req = OTHER_FIRST(param_2);
    /// if (((uint)in_ECX[1] >> 1 & 1) == 0)
    ///   pair = combine(param_2 + 0x130, param_2 + 0x148);
    /// else
    ///   pair = combine(param_2 + 0x100, param_2 + 0x118);
    /// ```
    ///
    /// The two `combine` inputs are 0x30 apart, the stride of
    /// [`A50_PAIR_ROW_STRIDE`].  The producers and combiners are outside the
    /// window; the translation takes their results as [`A50RenderTexts`] and
    /// [`A50Requirement`] values and reproduces the visibility gate, the row
    /// selection and the record assembly.
    pub fn a50_render_pair(
        &self,
        visible: bool,
        producer: &A50Requirement,
        second_flag: bool,
        key: u32,
        texts: &A50RenderTexts,
    ) -> Option<(A50Requirement, A50Pair)> {
        if !visible {
            return None;
        }
        let (first, second) = if second_flag {
            (&texts.high_first, &texts.high_second)
        } else {
            (&texts.low_first, &texts.low_second)
        };
        let pair = A50Pair {
            key,
            text: a50_string_from_bytes(first.text.as_bytes()),
            second: a50_string_from_bytes(second.text.as_bytes()),
        };
        Some((producer.clone(), pair))
    }
}

#[cfg(test)]
mod tests_agent50 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn s(text: &str) -> A50String {
        a50_string_from_bytes(text.as_bytes())
    }

    fn child(flags: u8, text: &str, code: i32) -> A50Child {
        A50Child {
            flags,
            text: s(text),
            code,
        }
    }

    fn pair(key: u32, text: &str, second: &str) -> A50Pair {
        A50Pair {
            key,
            text: s(text),
            second: s(second),
        }
    }

    #[test]
    fn scan_words_skips_the_cursor_byte_and_skips_separators() {
        // 0x005f6028
        let w = world();
        let scan = w.a50_scan_words(b"ab cd\nxy", 0, |b| b.is_ascii_whitespace());
        assert_eq!(
            scan.words,
            vec![s("b"), s("cd")],
            "the byte at the cursor is not examined"
        );
        assert_eq!(scan.next, 5);
        assert!(scan.newline);
    }

    #[test]
    fn scan_words_flushes_at_end_without_newline() {
        // 0x005f6028
        let w = world();
        let scan = w.a50_scan_words(b"ab", 0, |b| b == b' ');
        assert_eq!(scan.words, vec![s("b")]);
        assert_eq!(scan.next, 2);
        assert!(!scan.newline, "range end enters the non-returning tail");
    }

    #[test]
    fn catch_all_tail_is_marked_non_returning() {
        // 0x005f609c
        let w = world();
        assert_eq!(w.a50_catch_all_tail(), A50Tail::Rethrow);
    }

    #[test]
    fn collect_requirement_texts_selects_own_or_children() {
        // 0x005f60d0
        let w = world();
        let own = child(0, "own", 0);
        let kids = [child(0, "a", 0), child(A50_FLAG_SKIP, "b", 0)];
        assert_eq!(
            w.a50_collect_requirement_texts(Some(&own), false, &kids),
            vec![s("own")],
            "own provider wins over the enabled flag"
        );
        assert_eq!(
            w.a50_collect_requirement_texts(None, true, &kids),
            vec![s("a"), s("b")],
            "the child loop has no flags filter"
        );
        assert!(
            w.a50_collect_requirement_texts(None, false, &kids)
                .is_empty()
        );
    }

    #[test]
    fn child_requirement_texts_and_extend_agree() {
        // 0x005f6240, 0x005f629f
        let w = world();
        let kids = [child(0, "a", 0), child(0, "b", 0)];
        assert_eq!(w.a50_child_requirement_texts(&kids), vec![s("a"), s("b")]);
        let mut out = vec![s("head")];
        assert_eq!(w.a50_extend_requirement_texts(&mut out, &kids), 2);
        assert_eq!(out, vec![s("head"), s("a"), s("b")]);
    }

    #[test]
    fn chain_tail_follows_links_to_the_last_node() {
        // 0x005f6340
        let w = world();
        let links = [Some(1), Some(2), None];
        assert_eq!(w.a50_chain_tail(&links, 0), Some(2));
        assert_eq!(w.a50_chain_tail(&links, 2), Some(2));
        assert_eq!(w.a50_chain_tail(&[None], 0), Some(0));
        assert_eq!(w.a50_chain_tail(&links, 9), None);
        assert_eq!(w.a50_chain_tail(&[Some(7)], 0), None);
    }

    #[test]
    fn member_string_copy_uses_the_append_capacity() {
        // 0x005f6460
        let w = world();
        let short = s("hello");
        assert_eq!(w.a50_member_string_text(&short), short);
        let long = s("0123456789abcdefXY");
        assert_eq!(long.text.len(), 18);
        assert_eq!(long.cap, 0x1f, "len | 0xf once inline is exhausted");
        assert_eq!(w.a50_member_string_text(&long), long);
    }

    #[test]
    fn empty_string_is_size_zero_capacity_fifteen() {
        // 0x005f6570
        let w = world();
        let empty = w.a50_empty_string();
        assert!(empty.text.is_empty());
        assert_eq!(empty.cap, A50_SSO_CAPACITY);
    }

    #[test]
    fn dealloc_sizes_follow_the_recovered_block_classes() {
        // 0x005f6ac4 / 0x005f6af8 / 0x005f73f0 / 0x005f75ab release paths
        assert_eq!(a50_string_dealloc_bytes(0xf), None);
        assert_eq!(a50_string_dealloc_bytes(0x10), Some(0x11));
        assert_eq!(a50_string_dealloc_bytes(0xffe), Some(0xfff));
        assert_eq!(a50_string_dealloc_bytes(0xfff), Some(0x1023));
        assert!(a50_large_block_header_ok(4));
        assert!(a50_large_block_header_ok(0x23));
        assert!(!a50_large_block_header_ok(0x24));
    }

    #[test]
    fn seed_pair_copies_the_member_and_renders_the_key() {
        // 0x005f6590
        let w = world();
        let pair = w.a50_seed_pair(&s("Mem"), 300);
        assert_eq!(pair.key, 300);
        assert_eq!(pair.text, s("Mem"));
        assert_eq!(pair.second, s("300"));
    }

    #[test]
    fn join_names_uses_one_space_between_entries() {
        // 0x005f66a0
        let w = world();
        let names = [s("Alpha"), s("Beta"), s("Gamma")];
        let joined = w.a50_join_names(&names);
        assert_eq!(joined.text, "Alpha Beta Gamma");
        assert_eq!(
            joined.cap, 0x1f,
            "the last append outgrows the 0xf inline buffer, so the policy takes len | 0xf"
        );
        assert!(w.a50_join_names(&[]).text.is_empty());
    }

    #[test]
    fn buildable_texts_skip_the_flags_bit() {
        // 0x005f66f3
        let w = world();
        let kids = [
            child(0, "keep", 0),
            child(A50_FLAG_SKIP, "skip", 0),
            child(0x10, "keep2", 0),
        ];
        assert_eq!(w.a50_buildable_texts(&kids), vec![s("keep"), s("keep2")]);
    }

    #[test]
    fn take_texts_moves_the_vector_out() {
        // 0x005f6895
        let w = world();
        let mut out = vec![s("x"), s("y")];
        assert_eq!(w.a50_take_texts(&mut out), vec![s("x"), s("y")]);
        assert!(out.is_empty(), "the source vector is left empty");
    }

    #[test]
    fn append_texts_from_advances_past_flagged_nodes() {
        // 0x005f69e4
        let w = world();
        let kids = [
            child(0, "0", 0),
            child(A50_FLAG_SKIP, "1", 0),
            child(0, "2", 0),
            child(A50_FLAG_SKIP, "3", 0),
        ];
        assert_eq!(w.a50_append_texts_from(&kids, 0), (2, Some(s("2"))));
        assert_eq!(w.a50_append_texts_from(&kids, 2), (4, None));
    }

    #[test]
    fn pair_keys_add_the_recovered_base_offset() {
        // 0x005f6ac4
        let w = world();
        let pairs = [pair(0x10, "a", "b"), pair(1, "c", "d")];
        assert_eq!(w.a50_pair_keys(&pairs, 0x100), vec![0x110, 0x101]);
        assert_eq!(w.a50_pair_keys(&pairs, -0x10), vec![0, -0xf]);
    }

    #[test]
    fn pair_keys_stop_at_the_first_heap_string() {
        // 0x005f6af8
        let w = world();
        let inline = pair(1, "a", "b");
        let mut heap = pair(2, "c", "d");
        heap.second = s("0123456789abcdef");
        assert_eq!(heap.second.cap, 0x1f);
        let pairs = [inline.clone(), heap, pair(3, "e", "f")];
        assert_eq!(w.a50_pair_keys_while_inline(&pairs, 1), vec![2]);
    }

    #[test]
    fn push_pair_and_take_returns_the_extended_vector() {
        // 0x005f6ba1
        let w = world();
        let mut pairs = vec![pair(1, "a", "b")];
        let out = w.a50_push_pair_and_take(&mut pairs, pair(2, "c", "d"));
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].key, 2);
        assert!(pairs.is_empty());
    }

    #[test]
    fn collect_texts_with_member_orders_member_first() {
        // 0x005f6c60
        let w = world();
        let kids = [child(0, "k1", 0), child(A50_FLAG_SKIP, "k2", 0)];
        assert_eq!(
            w.a50_collect_texts_with_member(&s("mem"), &kids),
            vec![s("mem"), s("k1")]
        );
        assert_eq!(
            w.a50_collect_texts_with_member(&a50_string_empty(), &kids),
            vec![s("k1")],
            "an empty member contributes nothing"
        );
    }

    #[test]
    fn first_buildable_from_tests_the_entry_before_advancing() {
        // 0x005f6cf6
        let w = world();
        let kids = [
            child(A50_FLAG_SKIP, "0", 0),
            child(0, "1", 0),
            child(0, "2", 0),
        ];
        assert_eq!(w.a50_first_buildable_from(&kids, 0), Some((1, s("1"))));
        assert_eq!(w.a50_first_buildable_from(&kids, 2), Some((2, s("2"))));
        assert_eq!(w.a50_first_buildable_from(&kids, 3), None);
    }

    #[test]
    fn separator_string_is_the_two_byte_constant() {
        // 0x005f6d60
        let w = world();
        let sep = w.a50_separator_string();
        assert_eq!(sep.text.as_bytes(), b", ");
        assert_eq!(sep.cap, A50_SSO_CAPACITY);
    }

    #[test]
    fn move_pair_with_hint_copies_member_and_keeps_the_hint() {
        // 0x005f6d8d
        let w = world();
        let produced = pair(9, "p", "q");
        let (moved, member, hint) = w.a50_move_pair_with_hint(&s("Mem"), produced.clone(), 0x2a);
        assert_eq!(moved, produced);
        assert_eq!(member, s("Mem"));
        assert_eq!(hint, 0x2a);
    }

    #[test]
    fn merge_requirements_orders_units_then_techs() {
        // 0x005f7260
        let w = world();
        let units = [A50UnitNode { value: 3 }, A50UnitNode { value: 4 }];
        let techs = [A50TechNode { text: s("T") }];
        let merged = w.a50_merge_requirements(&units, &techs).unwrap();
        assert_eq!(
            merged,
            vec![
                A50Requirement {
                    kind: A50_KIND_CHAR,
                    value: 3,
                    text: a50_string_empty(),
                },
                A50Requirement {
                    kind: A50_KIND_CHAR,
                    value: 4,
                    text: a50_string_empty(),
                },
                A50Requirement {
                    kind: A50_KIND_NAMED,
                    value: 0,
                    text: s("T"),
                },
            ]
        );
    }

    #[test]
    fn merge_requirements_enforces_the_1c_element_limit() {
        // 0x005f7260, 0xffffffff / 0x1c
        assert!(a50_vector_count_fits_1c(0x0092_4924_9));
        assert!(!a50_vector_count_fits_1c(0x0092_4925_0));
    }

    #[test]
    fn default_requirement_prefers_tech_then_unit_then_space() {
        // 0x005f7380
        let w = world();
        let tech = [A50TechNode { text: s("Tech") }];
        let units = [A50UnitNode { value: 7 }];
        assert_eq!(
            w.a50_default_requirement(&units, &tech),
            A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: s("Tech"),
            }
        );
        assert_eq!(
            w.a50_default_requirement(&units, &[]),
            A50Requirement {
                kind: A50_KIND_CHAR,
                value: 7,
                text: a50_string_empty(),
            }
        );
        assert_eq!(
            w.a50_default_requirement(&[], &[]),
            A50Requirement {
                kind: A50_KIND_CHAR,
                value: A50_DEFAULT_CHAR,
                text: a50_string_empty(),
            },
            "the 0x2001 pair is kind 1 with value 0x20"
        );
    }

    #[test]
    fn format_requirements_uses_the_two_label_rows() {
        // 0x005f73f0
        let w = world();
        let labels = A50Labels {
            character: s("<c>"),
            named: s("<n>"),
            character_suffix: s("~"),
            named_suffix: s("/"),
        };
        let reqs = [
            A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: s("Alpha"),
            },
            A50Requirement {
                kind: A50_KIND_CHAR,
                value: b'X',
                text: a50_string_empty(),
            },
        ];
        let extra = A50Extended {
            count: 2,
            name: s("N"),
            tail: s("T"),
        };
        let text = w.a50_format_requirements(&labels, &reqs, false, Some(&extra));
        assert_eq!(text.text, "<n>Alpha/2NT, <c>X~2NT");
    }

    #[test]
    fn format_requirements_collapses_the_extra_block_for_multi_entry_vectors() {
        // 0x005f73f0
        let w = world();
        let labels = A50Labels {
            character: s("c"),
            named: s("n"),
            character_suffix: s("~"),
            named_suffix: s("/"),
        };
        let reqs = [
            A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: s("A"),
            },
            A50Requirement {
                kind: A50_KIND_NAMED,
                value: 0,
                text: s("B"),
            },
        ];
        let extra = A50Extended {
            count: 5,
            name: s(""),
            tail: s(""),
        };
        assert_eq!(
            w.a50_format_requirements(&labels, &reqs, true, Some(&extra))
                .text,
            "nA, nB/5",
            "the extra block is held back until the last entry"
        );
        assert_eq!(
            w.a50_format_requirements(&labels, &reqs[..1], true, Some(&extra))
                .text,
            "nA/5",
            "a one-element vector forces the collapse flag off"
        );
    }

    #[test]
    fn label_rows_match_the_recovered_table_offsets() {
        assert_eq!(A50_LABEL_CHAR, 0x24);
        assert_eq!(A50_LABEL_NAMED, 0x3c);
        assert_eq!(A50_LABEL_CHAR_SUFFIX, 0x54);
        assert_eq!(A50_LABEL_NAMED_SUFFIX, 0x6c);
    }

    #[test]
    fn count_1c_reproduces_the_signed_magic_multiply() {
        // 0x005f75ab
        let w = world();
        assert_eq!(w.a50_count_1c(0), 0);
        assert_eq!(w.a50_count_1c(27), 0);
        assert_eq!(w.a50_count_1c(28), 1);
        assert_eq!(w.a50_count_1c(29), 1);
        assert_eq!(w.a50_count_1c(0x1c * 1000), 1000);
        assert_eq!(w.a50_count_1c(-1), 0);
        assert_eq!(w.a50_count_1c(-28), -1);
    }

    #[test]
    fn noop_and_forward_pair_have_no_side_effects() {
        // 0x005f79e6, 0x005f7a00
        let w = world();
        w.a50_noop();
        let original = pair(0x11, "a", "b");
        assert_eq!(w.a50_forward_pair(original.clone()), original);
    }

    #[test]
    fn first_error_prefers_the_linked_provider() {
        // 0x005f7a20
        let w = world();
        let linked = child(0, "", 5);
        let direct = child(0, "", 9);
        let kids = [child(A50_FLAG_LINKED, "", 7), child(0, "", 8)];
        assert_eq!(
            w.a50_first_error(Some(&linked), Some(&direct), false, &kids),
            5,
            "the linked provider is asked first"
        );
        assert_eq!(
            w.a50_first_error(None, Some(&direct), true, &kids),
            9,
            "the direct provider wins over the enabled child scan"
        );
        assert_eq!(
            w.a50_first_error(None, None, true, &kids),
            7,
            "the enabled scan ignores the 0x10 flag"
        );
        assert_eq!(
            w.a50_first_error(None, None, false, &kids),
            0,
            "a disabled scan reports no error"
        );
        let linked_clean = child(0, "", 0);
        assert_eq!(
            w.a50_first_error(Some(&linked_clean), None, false, &kids),
            7,
            "the linked branch gates children with the 0x10 flag"
        );
    }

    #[test]
    fn first_error_any_returns_the_first_nonzero_code() {
        // 0x005f7aa0
        let w = world();
        let kids = [child(0, "", 0), child(0, "", 4), child(0, "", 6)];
        assert_eq!(w.a50_first_error_any(&kids), 4);
        assert_eq!(w.a50_first_error_any(&[]), 0);
    }

    #[test]
    fn requirement_texts_if_enabled_is_empty_when_disabled() {
        // 0x005f7ad0
        let w = world();
        let kids = [child(0, "a", 0)];
        assert_eq!(
            w.a50_requirement_texts_if_enabled(true, &kids),
            vec![s("a")]
        );
        assert!(w.a50_requirement_texts_if_enabled(false, &kids).is_empty());
    }

    #[test]
    fn render_pair_selects_the_row_spaced_thirty_bytes_apart() {
        // 0x005f7b20
        let w = world();
        let texts = A50RenderTexts {
            low_first: s("lf"),
            low_second: s("ls"),
            high_first: s("hf"),
            high_second: s("hs"),
        };
        let producer = A50Requirement {
            kind: A50_KIND_CHAR,
            value: 1,
            text: a50_string_empty(),
        };
        assert_eq!(
            w.a50_render_pair(true, &producer, false, 7, &texts),
            Some((producer.clone(), pair(7, "lf", "ls")))
        );
        assert_eq!(
            w.a50_render_pair(true, &producer, true, 8, &texts),
            Some((producer.clone(), pair(8, "hf", "hs")))
        );
        assert!(
            w.a50_render_pair(false, &producer, false, 9, &texts)
                .is_none()
        );
        assert_eq!(A50_PAIR_ROW_STRIDE, 0x30);
        assert_eq!(A50_PAIR_ROW_HIGH - A50_PAIR_ROW_LOW, A50_PAIR_ROW_STRIDE);
        assert!(a50_second_bit(2));
        assert!(!a50_second_bit(4));
    }
}
