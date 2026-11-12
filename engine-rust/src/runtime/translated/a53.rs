#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent53.rs
// ===========================================================================
// Translation batch 53.  Only this file is edited by the batch.
//
// Batch 53 owns `0x005fc000..=0x005fdfff`.  The ledger lists 60 rows in the
// window, all `decompiled_uncited`; 30 are translated below (two of them
// share the generic release helper) and the remaining rows are skipped, see
// the end of this comment.
//
// The window is the numeric/locale half of the MSVC (Dinkumware) standard
// library: `num_get` sign parsing, `_Ffmt`, `numpunct`, the `_Hash` bucket
// vector, `vector` reallocation helpers, `std::string` append/assign and the
// block-release tails.  Recovered layout facts that recur across the rows:
//
// | offset                     | meaning                                                  |
// |----------------------------|----------------------------------------------------------|
// | `+0x0/+0x4`                | `vector` begin/end pointers (element step `4`, `0x18`, `0x1c`, `0x34`) |
// | `+0x8/+0xc`                | `std::string` data pointer and size                       |
// | `+0x14` (dword)            | `std::string` capacity (`0xf` = inline buffer)            |
// | `+0xc/+0x10/+0x14`         | `_Hash` bucket vector begin/end/cap (4-byte elements); `+0x4` is the sentinel bucket the fill copies |
// | `+0x18/+0x1c`              | `_Hash` mask (`buckets - 1`) and `_Maxidx` (`buckets`)    |
// | `+0x4` (byte)              | `_Iosb` state/`fmtflags` byte read by the eof probe       |
// | `+0x8/+0x10/+0x14`         | three owned `numpunct` blocks released by `FUN_005fd2e0`    |
//
// The allocation policy of every non-string container is the same three
// step sequence seen in `0x005fd6a0`, `0x005fd710`, `0x005fd780` and
// `0x005fd7f0`: reject `count >= 0xffffffff / stride`, ask for `count * 4
// bytes` while that is below `0x1000`, and otherwise ask for `bytes + 0x23`
// with a saturating-overflow guard.  The `std::string` paths grow with
// `(size + n) | 0xf`, fall back to `cap + cap / 2` when that is larger and
// clamp at `0x7fffffff`.  Release paths read the 4-byte cookie stored at
// `data - 4` and trap unless `data - cookie <= 4 + 0x1f`.
//
// None of the routines touches `CUnit`/`World` simulation state.  The
// `ECX`-based rows are modelled as [`World`] methods (prefix `a53_`), the
// plain argument rows as free functions; both operate on local value types
// that mirror the recovered shapes ([`A53Str`] for `_String_val`,
// [`A53Req`] for the `0x1c` requirement element).
//
// Skipped rows and why:
//
// * `FUN_005fc11d`, `FUN_005fc61b`, `FUN_005fcf7d` - two-instruction catch tails
//   (`ExceptionList` restore then the non-returning `FUN_00c86b6e`); the
//   decompiled body has no recoverable value behaviour.
// * `FUN_005fc150` - `num_get` sign/prefix scanner; interleaves out-of-window
//   vtable calls (`FUN_005f09e0`, `FUN_005f0860`, `FUN_005e9f20`) with a
//   partially recovered frame, so neither the arithmetic nor the buffer
//   writes can be reproduced exactly.
// * `FUN_005fc650`, `FUN_005fd060` - one-load bodies (`**(this + 0x1c)`,
//   `**(this + 0x20)`) with no observable effect beyond the load itself.
// * `FUN_005fc770` - `numpunct<char>` constructor; copies the locale's name
//   from the out-of-window `_Locinfo` and 5 bytes from `DAT_00d33860`,
//   whose contents are not part of any analysed body.
// * `FUN_005fc8e8`, `FUN_005fd189` - catch-all funclets that release a triple /
//   a `0x1c` array and then abort (`FUN_00aad4a0`); translation would only
//   restate the out-of-window abort.
// * `FUN_005fc900`, `FUN_005fca41`, `FUN_005fcaa0`, `FUN_005fcc89`, `FUN_005fcd63`,
//   `FUN_005fcd9e` - `num_get` bodies whose frames are split across several
//   funclets; the decompiler shows `unaff_EBP`-relative locals only.
// * `FUN_005fca25`, `FUN_005fd04b` - catch-all constants (`&DAT_005fca3e`,
//   `0x5fd039`); pure resume addresses with no recoverable computation.
// * `FUN_005fcd37` - duplicate of the `0x005fc5e9` string release tail.
// * `FUN_005fcfc0`, `FUN_005fdd20` - single-dword store/load helpers.
// * `FUN_005fd080`, `FUN_005fd260` - `sputn`/`sputc` loops over the
//   out-of-window `streambuf` vtable (`+0x8` end pointer, `+0xc` capacity,
//   `+0xc` slot `overflow`); the sink object is not part of the window.
// * `FUN_005fd2d0` - `RTTI_Type_Descriptor` address constant.
// * `FUN_005fd2e0` - releases three `numpunct` blocks by raw pointer; the
//   element sizes live in the allocating constructors outside the window.
// * `FUN_005fd4e0`, `FUN_005fd530` - thin forwards / duplicate of `0x005fd440`.
// * `FUN_005fdd30`, `FUN_005fdde0`, `FUN_005fde60` - `_Hash` bucket-list init and
//   destroy; they combine in-window arithmetic with out-of-window helpers
//   (`FUN_005fde60`, `FUN_005fdc10`) whose arguments are stack-swizzled.
// * `FUN_005fdfe0` - same `0x23`-header release as `0x005fdfa0` with a `0x1c`
//   stride; covered by [`a53_release_items`].

/// Inline `std::string` capacity (`cap == 0xf` keeps the bytes in the object).
pub const A53_SSO_CAPACITY: u32 = 0xf;

/// Largest representable string size; the growth policy clamps here
/// (`0x005fd860`, `0x005fd9c0`, `0x005fdb20`).
pub const A53_STRING_MAX: u32 = 0x7fff_ffff;

/// Block-size boundary: lengths above `0xfff` carry the 4-byte cookie and are
/// requested as `len + 0x23` (allocation) or freed as `len + 0x23/0x24`.
pub const A53_SMALL_BLOCK: u32 = 0xfff;

/// Extra bytes the allocator is asked for on a large block
/// (`uVar2 + 0x23` in `0x005fd640`, `0x005fd6a0`, `0x005fd710`, `0x005fd780`,
/// `0x005fd7f0`, `0x005fd1b0`, `0x005fd300`, `0x005fd380`, `0x005fdf60`,
/// `0x005fdfa0`).
pub const A53_ALLOC_HEADER: u32 = 0x23;

/// Extra bytes charged by the `std::string` destructor (`cap + 0x24` in the
/// tail of `FUN_005fc150` and in `0x005fcf3b`/`FUN_005fde60`).
pub const A53_STRING_HEADER: u32 = 0x24;

/// Allowed distance between the stored cookie and the data pointer:
/// `(data - cookie) - 4 <= 0x1f` is the `0x005fdf60` sanity test.
pub const A53_COOKIE_SLACK: u32 = 0x1f;

/// Element limit of the 4-byte `_Hash` bucket vector
/// (`0x005fd6a0`: `param_1 < 0x40000000`).
pub const A53_ALLOC4_LIMIT: u32 = 0x4000_0000;

/// Element limit of the `0x1c` requirement vector (`0x005fd710`,
/// `0xffffffff / 0x1c + 1`).
pub const A53_ALLOC1C_LIMIT: u32 = 0x0924_924a;

/// Element limit of the `0x18` name vector (`0x005fd780`).
pub const A53_ALLOC18_LIMIT: u32 = 0x0aaa_aaab;

/// Element limit of the `0x34` pair vector (`0x005fd7f0`).
pub const A53_ALLOC34_LIMIT: u32 = 0x04ec_4ec5;

/// Requirement element stride (`0x005fd440`/`FUN_005fd530`/`0x005fd100`).
pub const A53_REQ_STRIDE: u32 = 0x1c;

/// Name element stride (`0x005fd300`).
pub const A53_NAME_STRIDE: u32 = 0x18;

/// Pair element stride (`0x005fd380`, `0x005fd500`, `0x005fd5d0`).
pub const A53_PAIR_STRIDE: u32 = 0x34;

/// `_Ffmt` flag bit that inserts `'+'` (`0x005fc670`).
pub const A53_FMT_PLUS: u32 = 0x20;

/// `_Ffmt` flag bit that inserts `'#'`.
pub const A53_FMT_LEAD: u32 = 0x8;

/// `basefield` mask tested by `0x005fc670`.
pub const A53_FMT_BASEFIELD: u32 = 0xe00;

/// `basefield` value selecting the octal conversion (`'o'`).
pub const A53_FMT_OCT: u32 = 0x400;

/// `basefield` value selecting the hexadecimal conversion (`'x'`/`'X'`).
pub const A53_FMT_HEX: u32 = 0x800;

/// The `_Iosb` bits `0x005fdee0` masks with (`param_1 & 0x17`).
pub const A53_IOS_STATE_MASK: u32 = 0x17;

/// `badbit` value used by the `0x005fdee0` message selector.
pub const A53_IOS_BADBIT: u32 = 4;

/// `failbit` value used by the `0x005fdee0` message selector.
pub const A53_IOS_FAILBIT: u32 = 2;

/// `eofbit` value used by the `0x005fdee0` message selector.
pub const A53_IOS_EOFBIT: u32 = 1;

/// The throwing paths of the window (`0x005fd600`, `0x005fd610`,
/// `0x005fd620`, `0x005fd630`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A53Error {
    /// `"string too long"` (`FUN_00c841b1`, `0x005fd600`).
    StringTooLong,
    /// `"vector<T> too long"` (`FUN_00c841b1`, `0x005fd610`).
    VectorTooLong,
    /// `"invalid string position"` (`FUN_00c841d1`, `0x005fd620`).
    InvalidStringPosition,
    /// `"invalid vector<T> subscript"` (`FUN_00c841d1`, `0x005fd630`).
    InvalidVectorSubscript,
}

impl A53Error {
    /// The exact bytes handed to the CRT reporter by the four rows above.
    pub fn message(self) -> &'static str {
        match self {
            A53Error::StringTooLong => "string too long",
            A53Error::VectorTooLong => "vector<T> too long",
            A53Error::InvalidStringPosition => "invalid string position",
            A53Error::InvalidVectorSubscript => "invalid vector<T> subscript",
        }
    }
}

/// The two non-returning diagnostics of the release tails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A53Fault {
    /// `FUN_00c86b6e`: the string carries an inline buffer but the destructor
    /// only reaches this tail for heap strings (`cap < 0x10` at
    /// `0x005fc5e9`/`0x005fcf3b`).
    Terminate,
    /// `FUN_00c9474b`: the cookie at `data - 4` is more than `0x1f` bytes
    /// below the data pointer.
    CookieCorrupt,
}

/// `_String_val` as used by `0x005fd860`, `0x005fd9c0`, `0x005fdb20` and the
/// release tails: data bytes plus the capacity dword read at `+0x14`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A53Str {
    pub text: Vec<u8>,
    pub cap: u32,
}

impl A53Str {
    /// Empty string with the inline capacity (`FUN_005fc770` and every
    /// constructor in the window write `0`, `0xf`, `0`).
    pub fn empty() -> Self {
        A53Str {
            text: Vec::new(),
            cap: A53_SSO_CAPACITY,
        }
    }

    /// Builds a string whose capacity follows the recovered constructor
    /// policy: inline while `len <= 0xf`, otherwise `len | 0xf`.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        A53Str {
            text: bytes.to_vec(),
            cap: if len > A53_SSO_CAPACITY {
                len | A53_SSO_CAPACITY
            } else {
                A53_SSO_CAPACITY
            },
        }
    }

    /// Size dword at `+0x10`.
    pub fn size(&self) -> u32 {
        u32::try_from(self.text.len()).unwrap_or(u32::MAX)
    }
}

/// The `0x1c` requirement element copied by `0x005fd440`: kind byte at `+0`,
/// value byte at `+1` and the `std::string` at `+4`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A53Req {
    pub kind: u8,
    pub value: u8,
    pub text: A53Str,
}

/// Result of [`World::a53_hash_init`]: the optional reserve, the fill count
/// and the two index fields written at `+0x18`/`+0x1c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A53HashInit {
    pub reserve: Option<u32>,
    pub assign: u32,
    pub mask: u32,
    pub maxidx: u32,
}

/// Result of [`World::a53_realloc_1c`]: the allocation request computed by
/// `FUN_005fd710` and the element count `(end - begin) / 0x1c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A53Realloc1c {
    pub alloc: u32,
    pub moved: i32,
}

/// Result of [`World::a53_reserve4`].  The two branches are exclusive: a null
/// base pointer writes the three vector fields, a non-null one releases the
/// old block after the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A53Reserve4 {
    /// `begin == 0`: `begin = alloc`, `end = alloc + floor(len / 4) * 4`,
    /// `cap = alloc + n * 4`.
    Fresh {
        alloc: u32,
        moved: u32,
        end: u32,
        cap: u32,
    },
    /// `begin != 0`: elements were moved and the old block is released with
    /// `(cap - begin) & ~3` bytes.
    Reused {
        alloc: u32,
        moved: u32,
        release: u32,
    },
}

/// Result of [`World::a53_vec4_assign`]: whether `0x005fdc10` reallocated and
/// the values written back into the vector header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A53Vec4Assign {
    pub reallocated: bool,
    pub alloc: u32,
    pub capacity: u32,
    pub size: u32,
    /// Elements the in-place branch overwrites before appending the rest
    /// (`min(size, n)`); `0` when the buffer was reallocated.
    pub overwritten: u32,
}

/// Result of [`World::a53_copy_1c`]: the deep-copied elements and the
/// destination pointer after `count * 0x1c` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A53Copy1c {
    pub copied: Vec<A53Req>,
    pub dest: u32,
}

/// Result of [`World::a53_setstate`]: the masked state dword written at
/// `+0xc`, the bits that were new, and the diagnostic selected for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A53StateChange {
    pub state: u32,
    pub raised: u32,
    pub abort: bool,
    pub message: Option<&'static str>,
}

/// `_Ffmt` of `0x005fc670` (137 bytes): writes `'%'`, the `'+'`/`'#'` flags
/// selected by `flags & 0x20` and `flags & 8`, the length prefix (`"I64"` for
/// a leading `'L'`, otherwise `spec[0]`) and the conversion byte.
///
/// The conversion byte is `'o'` for `basefield == 0x400`, `'x'`/`'X'` for
/// `basefield == 0x800` (uppercase when flag bit `0x4` is set, computed by
/// the decompiler as `~((char)flags << 3) & 0x20 | 0x58`) and `spec[1]`
/// otherwise.  The returned bytes are the written characters; the body also
/// stores the `'\0'` terminator one byte past the last one.
pub fn a53_ffmt(spec: [u8; 2], flags: u32) -> Vec<u8> {
    let mut out = vec![b'%'];
    if flags & A53_FMT_PLUS != 0 {
        out.push(b'+');
    }
    if flags & A53_FMT_LEAD != 0 {
        out.push(b'#');
    }
    if spec[0] == b'L' {
        out.extend_from_slice(b"I64");
    } else {
        out.push(spec[0]);
    }
    if flags & A53_FMT_BASEFIELD == A53_FMT_OCT {
        out.push(b'o');
    } else if flags & A53_FMT_BASEFIELD != A53_FMT_HEX {
        out.push(spec[1]);
    } else if flags & 0x4 == 0 {
        out.push(b'x');
    } else {
        out.push(b'X');
    }
    out
}

/// The allocation request of `0x005fd640` (47 bytes):
///
/// ```text
/// if (0xfff < n) { want = n + 0x23; if (want <= n) want = 0xffffffff; alloc(want); }
/// if (n != 0)     alloc(n);
/// return 0;
/// ```
///
/// `0` means no allocation was requested, `0xffffffff` is the saturated
/// request that makes the CRT allocator fail.
pub fn a53_alloc_request(n: u32) -> u32 {
    if n > A53_SMALL_BLOCK {
        let want = n.wrapping_add(A53_ALLOC_HEADER);
        return if want <= n { u32::MAX } else { want };
    }
    n
}

/// Shared body of the four typed allocators `0x005fd6a0` (stride `4`),
/// `0x005fd710` (`0x1c`), `0x005fd780` (`0x18`) and `0x005fd7f0` (`0x34`):
///
/// ```text
/// bytes = count * stride;                       /* wrapping */
/// if (count >= limit) bytes = 0xffffffff;
/// if (count < limit && bytes < 0x1000) return bytes;   /* 0 = no allocation */
/// want = bytes + 0x23; if (want <= bytes) want = 0xffffffff;
/// ```
pub fn a53_alloc_items(count: u32, stride: u32, limit: u32) -> u32 {
    let bytes = if count < limit {
        count.wrapping_mul(stride)
    } else {
        u32::MAX
    };
    if count < limit && bytes < 0x1000 {
        return bytes;
    }
    let want = bytes.wrapping_add(A53_ALLOC_HEADER);
    if want <= bytes { u32::MAX } else { want }
}

/// The cookie sanity test shared by every release tail:
/// `(data - stored) - 4 <= 0x1f` (the trap is
/// `if (0x1f < (data - stored) - 4)`).
pub fn a53_cookie_ok(distance: u32) -> bool {
    distance.wrapping_sub(4) <= A53_COOKIE_SLACK
}

/// Byte count handed to the deleter by `0x005fdf60` (47 bytes) and by the
/// release tails: below or at `0xfff` the length is used verbatim, above it
/// the block carried the `0x23` header `a53_alloc_items` requested and the
/// cookie must sit within [`A53_COOKIE_SLACK`] bytes of the data pointer.
pub fn a53_release_request(len: u32, cookie_distance: u32) -> Result<u32, A53Fault> {
    if len > A53_SMALL_BLOCK {
        if !a53_cookie_ok(cookie_distance) {
            return Err(A53Fault::CookieCorrupt);
        }
        Ok(len.wrapping_add(A53_ALLOC_HEADER))
    } else {
        Ok(len)
    }
}

/// `0x005fdfa0` (50 bytes): releases a block of `count * 0x20` bytes, with the
/// `+0x23` header and cookie test above `0xfff`.  `FUN_005fdfe0` is the same
/// body with a `0x1c` stride; both feed [`a53_release_request`].
pub fn a53_release_items(count: u32, stride: u32, cookie_distance: u32) -> Result<u32, A53Fault> {
    a53_release_request(count.wrapping_mul(stride), cookie_distance)
}

/// The string capacity policy of `0x005fd860`, `0x005fd9c0` and
/// `0x005fdb20`, factored out because all three use the identical sequence:
///
/// ```text
/// needed = (size + add) | 0xf;              /* or n | 0xf when assigning */
/// if (needed >= 0x80000000)             capacity = 0x7fffffff;
/// else if (0x7fffffff - (cap >> 1) < cap) capacity = 0x7fffffff;
/// else                                  capacity = max(needed, cap + cap / 2);
/// ```
fn a53_capacity(needed: u32, cap: u32) -> u32 {
    if needed >= 0x8000_0000 {
        return A53_STRING_MAX;
    }
    if A53_STRING_MAX - (cap >> 1) < cap {
        return A53_STRING_MAX;
    }
    needed.max((cap >> 1) + cap)
}

/// The append capacity of `0x005fd860`/`0x005fd9c0`: `None`-equivalent
/// `Err(StringTooLong)` is the `param_1 <= 0x7fffffff - size` failure.
pub fn a53_string_grow(size: u32, add: u32, cap: u32) -> Result<u32, A53Error> {
    if add > A53_STRING_MAX - size {
        return Err(A53Error::StringTooLong);
    }
    Ok(a53_capacity((size + add) | A53_SSO_CAPACITY, cap))
}

/// The assign capacity of `0x005fdb20`: the growth input is the new size
/// alone, not `size + add`, and `n >= 0x80000000` is the length error.
pub fn a53_string_assign_capacity(n: u32, cap: u32) -> Result<u32, A53Error> {
    if n >= 0x8000_0000 {
        return Err(A53Error::StringTooLong);
    }
    Ok(a53_capacity(n | A53_SSO_CAPACITY, cap))
}

/// `0x005fd400` (44 bytes): the overlap-safe range copy.  The body builds a
/// scoped guard and calls `FUN_00c89b00(dest, src, end - begin)`, i.e.
/// `memmove(dest, src, len)`.
///
/// Returns `false` (and copies nothing) when either range leaves `buf`; the
/// retail body trusts its caller and has no bounds test.
pub fn a53_move_bytes(buf: &mut [u8], src: usize, dest: usize, len: usize) -> bool {
    if src > buf.len() || dest > buf.len() || len > buf.len() - src || len > buf.len() - dest {
        return false;
    }
    buf.copy_within(src..src + len, dest);
    true
}

/// `0x005fd440` (153 bytes): copy-constructs a `0x1c` element range.
///
/// Per element it copies the kind byte at `+0`, the value byte at `+1` and
/// copy-constructs the `std::string` at `+4` (a deep copy of the payload),
/// then advances the destination by `0x1c` and returns it.  The trailing
/// `FUN_005ed240(dest, dest, ...)` writes the sentinel element and does not
/// contribute to the returned pointer.
pub fn a53_copy_1c(src: &[A53Req], dest: u32) -> A53Copy1c {
    let mut copied = Vec::with_capacity(src.len());
    let mut out = dest;
    for req in src {
        copied.push(A53Req {
            kind: req.kind,
            value: req.value,
            text: req.text.clone(),
        });
        out = out.wrapping_add(A53_REQ_STRIDE);
    }
    A53Copy1c { copied, dest: out }
}

/// `0x005fd500` (46 bytes): walks a `0x34` element range, destroying each
/// element with `FUN_005f1550`, and advances the destination by `0x34` per
/// element.  The `while (param_1 != param_2)` form is preserved, so a range
/// whose length is not a multiple of `0x34` never terminates.
pub fn a53_destroy_34_into(begin: u32, end: u32, dest: u32) -> u32 {
    let mut p = begin;
    let mut d = dest;
    while p != end {
        p = p.wrapping_add(A53_PAIR_STRIDE);
        d = d.wrapping_add(A53_PAIR_STRIDE);
    }
    d
}

/// `0x005fd5d0` (44 bytes): the same `0x34` destroy loop without a
/// destination pointer.  Returns the number of elements destroyed.
pub fn a53_destroy_34(begin: u32, end: u32) -> u32 {
    let mut p = begin;
    let mut count = 0u32;
    while p != end {
        p = p.wrapping_add(A53_PAIR_STRIDE);
        count = count.wrapping_add(1);
    }
    count
}

impl World {
    /// `0x005fc5e9` (55 bytes, `decompiled_uncited`).
    ///
    /// The `std::string` release tail.  It is only reached for a heap string,
    /// so `cap < 0x10` enters the non-returning terminate path:
    ///
    /// ```c
    /// if (cap < 0x10) terminate();
    /// if (0xfff < cap + 1) {
    ///     if (0x1f < (data - *(int *)(data - 4)) - 4) trap();
    /// }
    /// free(data, cap + 1);            /* cap + 0x24 for the large block */
    /// ```
    ///
    /// Returns the byte count handed to the deleter; `FUN_005fc150`'s tail
    /// shows the large-block variant as `cap + 0x24`.
    pub fn a53_string_release_tail(&self, cap: u32, cookie_distance: u32) -> Result<u32, A53Fault> {
        if cap < A53_SSO_CAPACITY + 1 {
            return Err(A53Fault::Terminate);
        }
        let len = cap.wrapping_add(1);
        if len > A53_SMALL_BLOCK {
            if !a53_cookie_ok(cookie_distance) {
                return Err(A53Fault::CookieCorrupt);
            }
            return Ok(cap.wrapping_add(A53_STRING_HEADER));
        }
        Ok(len)
    }

    /// `0x005fc700` (5 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// LOCK(); *(int *)(this + 4) = *(int *)(this + 4) + 1; UNLOCK();
    /// ```
    ///
    /// A full-barrier atomic increment; returns the previous value like the
    /// `LOCK XADD` the compiler emits.
    pub fn a53_atomic_increment(&self, counter: &AtomicU32) -> u32 {
        counter.fetch_add(1, Ordering::SeqCst)
    }

    /// `0x005fc710` (89 bytes, `decompiled_uncited`).
    ///
    /// `_Hash` bucket grow.  `capacity` is the byte count at
    /// `*(+0x14) - *(+0xc)`; the body doubles the requested bucket count:
    ///
    /// ```c
    /// want = buckets * 2;
    /// if (capacity >> 2 < want) {
    ///     if (0x3fffffff < want) throw vector<T> too long;
    ///     reserve4(want);
    /// }
    /// assign4(want, *(this + 4));       /* 4-byte elements */
    /// *(this + 0x1c) = buckets;         /* _Maxidx */
    /// *(this + 0x18) = buckets - 1;     /* _Mask   */
    /// ```
    pub fn a53_hash_init(
        &self,
        buckets: u32,
        capacity_bytes: u32,
    ) -> Result<A53HashInit, A53Error> {
        let want = buckets.wrapping_mul(2);
        let capacity = capacity_bytes >> 2;
        let reserve = if capacity < want {
            if want > 0x3fff_ffff {
                return Err(A53Error::VectorTooLong);
            }
            Some(want)
        } else {
            None
        };
        Ok(A53HashInit {
            reserve,
            assign: want,
            mask: buckets.wrapping_sub(1),
            maxidx: buckets,
        })
    }

    /// `0x005fcf3b` (71 bytes, `decompiled_uncited`).
    ///
    /// Resets the string header (`size = 0`, `cap = 0xf`, first byte `0`) and
    /// then runs the release tail on the old capacity:
    ///
    /// ```c
    /// *(+0x10) = 0; *(+0x14) = 0xf; *data = 0;
    /// if (old_cap < 0x10) terminate();
    /// if (0xfff < old_cap + 1) { 0x1f < (data - cookie) - 4 and trap; }
    /// free(data, old_cap + 1);   /* old_cap + 0x24 when large */
    /// ```
    ///
    /// The reset happens before the diagnostic, so `Err` still leaves the
    /// string empty.
    pub fn a53_reset_and_release(
        &self,
        s: &mut A53Str,
        cookie_distance: u32,
    ) -> Result<u32, A53Fault> {
        let old_cap = s.cap;
        s.text.clear();
        s.cap = A53_SSO_CAPACITY;
        let len = old_cap.wrapping_add(1);
        if old_cap < A53_SSO_CAPACITY + 1 {
            return Err(A53Fault::Terminate);
        }
        if len > A53_SMALL_BLOCK {
            if !a53_cookie_ok(cookie_distance) {
                return Err(A53Fault::CookieCorrupt);
            }
            return Ok(old_cap.wrapping_add(A53_STRING_HEADER));
        }
        Ok(len)
    }

    /// `0x005fcfd0` (123 bytes, `decompiled_uncited`).
    ///
    /// The `eof` guard of the input path:
    ///
    /// ```c
    /// if (*(+0xc) == 0 && (*(byte *)(+0x14) & 2) != 0) {
    ///     if (probe() == -1) setstate(*(+0xc) | 4, 0);   /* badbit, no throw */
    /// }
    /// ```
    ///
    /// `state` is the `+0xc` dword, `flags` the byte at `+0x14`, and `probe`
    /// is the out-of-window `vtable + 0x34` call on `*(+0x38)`.  Returns the
    /// new state when `badbit` was added.
    pub fn a53_eof_guard<F>(&self, state: u32, flags: u32, probe: F) -> Option<u32>
    where
        F: FnOnce() -> i32,
    {
        if state == 0 && flags & 0x2 != 0 && probe() == -1 {
            return Some(state | A53_IOS_BADBIT);
        }
        None
    }

    /// `0x005fd100` (137 bytes, `decompiled_uncited`).
    ///
    /// The `0x1c` element reallocation preparation:
    ///
    /// ```c
    /// begin = *(+0); end = *(+4);
    /// new_block = alloc_1c(n);          /* 0x005fd710, translated below */
    /// copy_1c(begin, end, new_block);   /* FUN_005fd530, the void twin of
    ///                                      0x005fd440 */
    /// FUN_005f9c90(new_block, (end - begin) / 0x1c, n);
    /// ```
    ///
    /// `moved` is the signed `/ 0x1c` (truncating) element count; `alloc` is
    /// the byte request, where `0xffffffff` marks the out-of-range request.
    pub fn a53_realloc_1c(&self, begin: u32, end: u32, n: u32) -> A53Realloc1c {
        A53Realloc1c {
            alloc: a53_alloc_items(n, A53_REQ_STRIDE, A53_ALLOC1C_LIMIT),
            moved: (end as i32).wrapping_sub(begin as i32) / A53_REQ_STRIDE as i32,
        }
    }

    /// `0x005fd1b0` (161 bytes, `decompiled_uncited`).
    ///
    /// Reserve for the 4-byte bucket vector:
    ///
    /// ```c
    /// new_block = alloc4(n);                         /* 0x005fd6a0 */
    /// moved = end >= begin ? (end - begin + 3) >> 2 : 0;
    /// copy moved dwords from begin to new_block;
    /// if (begin == 0) { *(+0) = new_block;
    ///                   *(+4) = new_block + ((end - begin) >> 2) * 4;
    ///                   *(+8) = new_block + n * 4; return; }
    /// release_size = (*(+8) - begin) & 0xfffffffc;
    /// if (0xfff < release_size) { cookie test; release_size += 0x23; }
    /// free(begin, release_size);
    /// ```
    ///
    /// `cookie_distance` is `begin - *(int *)(begin - 4)`.  The returned
    /// [`A53Reserve4`] keeps the two exclusive outcomes separate.
    pub fn a53_reserve4(
        &self,
        begin: u32,
        end: u32,
        cap: u32,
        n: u32,
        cookie_distance: u32,
    ) -> Result<A53Reserve4, A53Fault> {
        let alloc = a53_alloc_items(n, 4, A53_ALLOC4_LIMIT);
        let moved = if end >= begin {
            end.wrapping_sub(begin).wrapping_add(3) >> 2
        } else {
            0
        };
        if begin == 0 {
            Ok(A53Reserve4::Fresh {
                alloc,
                moved,
                end: (end.wrapping_sub(begin) >> 2) << 2,
                cap: n.wrapping_mul(4),
            })
        } else {
            let release_len = cap.wrapping_sub(begin) & 0xffff_fffc;
            let release = a53_release_request(release_len, cookie_distance)?;
            Ok(A53Reserve4::Reused {
                alloc,
                moved,
                release,
            })
        }
    }

    /// `0x005fd300` (93 bytes, `decompiled_uncited`).
    ///
    /// Releases the `0x18` name vector:
    ///
    /// ```c
    /// if (*(+0) == 0) return;
    /// destroy_range(*(+0), *(+4));                       /* FUN_005ed2b0,
    ///                                                       out of window */
    /// bytes = ((*(+8) - *(+0)) / 0x18) * 0x18;           /* signed division */
    /// if (0xfff < bytes) { cookie test; bytes += 0x23; }
    /// free(*(+0), bytes);
    /// ```
    ///
    /// `capacity_bytes` is `*(+8) - *(+0)` and `first_present` is
    /// `*(+0) != 0`; `Ok(None)` is the early return.
    pub fn a53_release_18(
        &self,
        first_present: bool,
        capacity_bytes: i32,
        cookie_distance: u32,
    ) -> Result<Option<u32>, A53Fault> {
        if !first_present {
            return Ok(None);
        }
        let bytes = ((capacity_bytes / A53_NAME_STRIDE as i32) * A53_NAME_STRIDE as i32) as u32;
        Ok(Some(a53_release_request(bytes, cookie_distance)?))
    }

    /// `0x005fd380` (100 bytes, `decompiled_uncited`).
    ///
    /// The `0x34` pair-vector variant of [`Self::a53_release_18`]: every
    /// element is destroyed with `FUN_005f34c0`, then
    /// `bytes = ((cap - first) / 0x34) * 0x34` is released with the same
    /// `0x23` header policy.
    pub fn a53_release_34(
        &self,
        first_present: bool,
        capacity_bytes: i32,
        cookie_distance: u32,
    ) -> Result<Option<u32>, A53Fault> {
        if !first_present {
            return Ok(None);
        }
        let bytes = ((capacity_bytes / A53_PAIR_STRIDE as i32) * A53_PAIR_STRIDE as i32) as u32;
        Ok(Some(a53_release_request(bytes, cookie_distance)?))
    }

    /// `0x005fdc10` (269 bytes, `decompiled_uncited`).
    ///
    /// `assign(n, value)` for the 4-byte vector:
    ///
    /// ```c
    /// if (capacity < n) {
    ///     if (0x3fffffff < n) throw vector<T> too long;
    ///     newcap = n;
    ///     if (capacity <= 0x3fffffff - (capacity >> 1)) {
    ///         newcap = capacity + (capacity >> 1);
    ///         if (newcap < n) newcap = n;
    ///     }
    ///     free(begin, capacity);              /* FUN_005fe030 */
    ///     begin = end = cap = 0;
    ///     if (newcap != 0) { begin = end = alloc4(newcap);
    ///                        cap = begin + newcap * 4; }
    ///     *end = value; end += 4;  x n;
    /// } else if (n <= size) {
    ///     fill [begin, begin + n * 4);  end = begin + n * 4;
    /// } else {
    ///     fill [begin, end);  append value (n - size) times;
    /// }
    /// ```
    pub fn a53_vec4_assign(
        &self,
        capacity: u32,
        size: u32,
        n: u32,
    ) -> Result<A53Vec4Assign, A53Error> {
        if capacity < n {
            if n > 0x3fff_ffff {
                return Err(A53Error::VectorTooLong);
            }
            let mut newcap = n;
            if capacity <= 0x3fff_ffff - (capacity >> 1) {
                newcap = capacity + (capacity >> 1);
                if newcap < n {
                    newcap = n;
                }
            }
            if newcap > 0x3fff_ffff {
                return Err(A53Error::VectorTooLong);
            }
            return Ok(A53Vec4Assign {
                reallocated: true,
                alloc: a53_alloc_items(newcap, 4, A53_ALLOC4_LIMIT),
                capacity: newcap,
                size: n,
                overwritten: 0,
            });
        }
        Ok(A53Vec4Assign {
            reallocated: false,
            alloc: 0,
            capacity,
            size: n,
            overwritten: size.min(n),
        })
    }

    /// `0x005fd860` (213 bytes, `decompiled_uncited`).
    ///
    /// `append(n, ch)`:
    ///
    /// ```c
    /// if (n <= cap - size) { size += n; memset(data + old_size, ch, n); return; }
    /// if (n > 0x7fffffff - size) throw string too long;
    /// cap = grow(size, n, cap);      /* (size + n) | 0xf, ... */
    /// new_block = alloc(cap + 1);    /* FUN_005fd640 */
    /// size = old_size + n;  memcpy(new_block, old_data, old_size);
    /// memset(new_block + old_size, ch, n);
    /// ```
    pub fn a53_append_fill(&self, s: &mut A53Str, byte: u8, n: u32) -> Result<(), A53Error> {
        let size = s.size();
        if n <= s.cap.wrapping_sub(size) {
            s.text.extend(std::iter::repeat(byte).take(n as usize));
            return Ok(());
        }
        s.cap = a53_string_grow(size, n, s.cap)?;
        s.text.extend(std::iter::repeat(byte).take(n as usize));
        Ok(())
    }

    /// `0x005fd9c0` (204 bytes, `decompiled_uncited`).
    ///
    /// `append(first, n)`, identical to [`Self::a53_append_fill`] except the
    /// payload is `memmove`d from `first` instead of a repeated byte:
    ///
    /// ```c
    /// if (n <= cap - size) { size += n; memmove(data + old_size, first, n); return; }
    /// if (n > 0x7fffffff - size) throw string too long;
    /// cap = (size + n) | 0xf;  ... cap + cap / 2 ...
    /// new_block = alloc(cap + 1);  size = old_size + n;
    /// copy old payload then the n bytes.
    /// ```
    pub fn a53_append_bytes(&self, s: &mut A53Str, bytes: &[u8]) -> Result<(), A53Error> {
        let add = u32::try_from(bytes.len()).map_err(|_| A53Error::StringTooLong)?;
        let size = s.size();
        if add <= s.cap.wrapping_sub(size) {
            s.text.extend_from_slice(bytes);
            return Ok(());
        }
        s.cap = a53_string_grow(size, add, s.cap)?;
        s.text.extend_from_slice(bytes);
        Ok(())
    }

    /// `0x005fdb20` (143 bytes, `decompiled_uncited`).
    ///
    /// `assign(first, n)` - replaces the payload entirely:
    ///
    /// ```c
    /// if (n <= cap) { size = n; memmove(data, first, n); return; }
    /// if (n >= 0x80000000) throw string too long;
    /// needed = n | 0xf;
    /// if (needed < 0x80000000) {
    ///     if (0x7fffffff - (cap >> 1) < cap) needed = 0x7fffffff;
    ///     else { needed = max(needed, cap + cap / 2); }
    /// }
    /// new_block = alloc(needed + 1);
    /// cap = needed; size = n; memcpy(new_block, first, n);
    /// ```
    pub fn a53_assign_bytes(&self, s: &mut A53Str, bytes: &[u8]) -> Result<(), A53Error> {
        let n = u32::try_from(bytes.len()).map_err(|_| A53Error::StringTooLong)?;
        if n <= s.cap {
            s.text.clear();
            s.text.extend_from_slice(bytes);
            return Ok(());
        }
        s.cap = a53_string_assign_capacity(n, s.cap)?;
        s.text.clear();
        s.text.extend_from_slice(bytes);
        Ok(())
    }

    /// `0x005fdee0` (113 bytes, `decompiled_uncited`).
    ///
    /// The `ios_base` state writer:
    ///
    /// ```c
    /// *(+0xc) = bits & 0x17;
    /// raised  = *(+0x10) & bits & 0x17;
    /// if (raised == 0) return;
    /// if (rethrow) abort();                      /* FUN_00aad4a0(0, 0) */
    /// msg = (raised & 4) ? "ios_base::badbit set"
    ///     : (raised & 2) ? "ios_base::failbit set"
    ///                     : "ios_base::eofbit set";
    /// throw failure(msg);
    /// ```
    ///
    /// The message order is the recovered nested test: `badbit` wins over
    /// `failbit`, which wins over `eofbit`.
    pub fn a53_setstate(&self, bits: u32, mask: u32, rethrow: bool) -> A53StateChange {
        let state = bits & A53_IOS_STATE_MASK;
        let raised = mask & bits & A53_IOS_STATE_MASK;
        let abort = raised != 0 && rethrow;
        let message = if raised == 0 || abort {
            None
        } else if raised & A53_IOS_BADBIT != 0 {
            Some("ios_base::badbit set")
        } else if raised & A53_IOS_FAILBIT != 0 {
            Some("ios_base::failbit set")
        } else {
            Some("ios_base::eofbit set")
        };
        A53StateChange {
            state,
            raised,
            abort,
            message,
        }
    }
}

/// `0x005fd600` (10 bytes, `decompiled_uncited`): reports
/// `"string too long"` through `FUN_00c841b1` and never returns.
pub fn a53_string_too_long() -> A53Error {
    A53Error::StringTooLong
}

/// `0x005fd610` (10 bytes, `decompiled_uncited`): reports
/// `"vector<T> too long"` through `FUN_00c841b1`.
pub fn a53_vector_too_long() -> A53Error {
    A53Error::VectorTooLong
}

/// `0x005fd620` (11 bytes, `decompiled_uncited`): reports
/// `"invalid string position"` through `FUN_00c841d1`, then executes the
/// `int3` the decompiler shows as `swi(3)`.
pub fn a53_invalid_string_position() -> A53Error {
    A53Error::InvalidStringPosition
}

/// `0x005fd630` (11 bytes, `decompiled_uncited`): reports
/// `"invalid vector<T> subscript"` through `FUN_00c841d1`, then `int3`.
pub fn a53_invalid_vector_subscript() -> A53Error {
    A53Error::InvalidVectorSubscript
}

/// The four typed allocation rows:
///
/// * `0x005fd6a0` (65 bytes): 4-byte elements, limit `0x40000000`.
/// * `0x005fd710` (71 bytes): `0x1c` elements, limit `0x924924a`.
/// * `0x005fd780` (65 bytes): `0x18` elements, limit `0xaaaaaab`.
/// * `0x005fd7f0` (62 bytes): `0x34` elements, limit `0x4ec4ec5`.
impl World {
    /// `0x005fd6a0` (65 bytes, `decompiled_uncited`).
    ///
    /// ```c
    /// bytes = param_1 * 4;
    /// if (param_1 < 0x40000000) { if (bytes < 0x1000) { alloc(bytes); return 0; } }
    /// else bytes = 0xffffffff;
    /// want = bytes + 0x23; if (want <= bytes) want = 0xffffffff;
    /// alloc(want);
    /// ```
    pub fn a53_alloc4(&self, n: u32) -> u32 {
        a53_alloc_items(n, 4, A53_ALLOC4_LIMIT)
    }

    /// `0x005fd710` (71 bytes, `decompiled_uncited`): `0x1c` elements.
    pub fn a53_alloc1c(&self, n: u32) -> u32 {
        a53_alloc_items(n, A53_REQ_STRIDE, A53_ALLOC1C_LIMIT)
    }

    /// `0x005fd780` (65 bytes, `decompiled_uncited`): `0x18` elements.
    pub fn a53_alloc18(&self, n: u32) -> u32 {
        a53_alloc_items(n, A53_NAME_STRIDE, A53_ALLOC18_LIMIT)
    }

    /// `0x005fd7f0` (62 bytes, `decompiled_uncited`): `0x34` elements.
    pub fn a53_alloc34(&self, n: u32) -> u32 {
        a53_alloc_items(n, A53_PAIR_STRIDE, A53_ALLOC34_LIMIT)
    }

    /// `0x005fdf60` (47 bytes, `decompiled_uncited`).
    ///
    /// The generic block release:
    ///
    /// ```c
    /// stored = param_1;
    /// if (0xfff < param_2) { stored = *(int *)(param_1 - 4); param_2 += 0x23;
    ///                        if (0x1f < (param_1 - stored) - 4) trap(); }
    /// free(stored, param_2);
    /// ```
    ///
    /// `len` is the requested byte length and `cookie_distance` is
    /// `data - stored`.
    pub fn a53_release_block(&self, len: u32, cookie_distance: u32) -> Result<u32, A53Fault> {
        a53_release_request(len, cookie_distance)
    }
}

#[cfg(test)]
mod tests_agent53 {
    use super::*;

    fn world() -> World {
        use crate::data::GameData;
        use crate::map::Terrain;
        use std::sync::Arc;
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(16, 16),
            2,
        )
    }

    #[test]
    fn ffmt_writes_flags_prefix_and_conversion() {
        // 0x005fc670
        assert_eq!(a53_ffmt([b'l', b'd'], 0), b"%ld");
        assert_eq!(a53_ffmt([b'l', b'd'], A53_FMT_PLUS), b"%+ld");
        assert_eq!(a53_ffmt([b'l', b'd'], A53_FMT_LEAD), b"%#ld");
        assert_eq!(
            a53_ffmt([b'l', b'd'], A53_FMT_PLUS | A53_FMT_LEAD),
            b"%+#ld"
        );
        assert_eq!(a53_ffmt([b'L', b'f'], 0), b"%I64f");
        assert_eq!(
            a53_ffmt([b'l', b'd'], A53_FMT_BASEFIELD),
            b"%ld",
            "basefield zero keeps spec[1]"
        );
        assert_eq!(a53_ffmt([b'l', b'd'], A53_FMT_OCT), b"%lo");
        assert_eq!(a53_ffmt([b'l', b'd'], A53_FMT_HEX), b"%lx");
        assert_eq!(
            a53_ffmt([b'l', b'd'], A53_FMT_HEX | 0x4),
            b"%lX",
            "flag bit 2 selects the uppercase X"
        );
    }

    #[test]
    fn atomic_increment_returns_the_previous_value() {
        // 0x005fc700
        let w = world();
        let counter = AtomicU32::new(41);
        assert_eq!(w.a53_atomic_increment(&counter), 41);
        assert_eq!(w.a53_atomic_increment(&counter), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 43);
    }

    #[test]
    fn hash_init_doubles_the_bucket_request() {
        // 0x005fc710
        let w = world();
        assert_eq!(
            w.a53_hash_init(4, 0).unwrap(),
            A53HashInit {
                reserve: Some(8),
                assign: 8,
                mask: 3,
                maxidx: 4,
            }
        );
        assert_eq!(
            w.a53_hash_init(4, 8 * 4).unwrap(),
            A53HashInit {
                reserve: None,
                assign: 8,
                mask: 3,
                maxidx: 4,
            },
            "capacity of 8 dwords is enough for 8 buckets"
        );
        assert_eq!(w.a53_hash_init(0, 0).unwrap().mask, u32::MAX);
        assert_eq!(
            w.a53_hash_init(0x2000_0000, 0),
            Err(A53Error::VectorTooLong),
            "2 * buckets above 0x3fffffff throws"
        );
    }

    #[test]
    fn string_release_tail_uses_the_inline_limit_and_cookie() {
        // 0x005fc5e9
        let w = world();
        assert_eq!(w.a53_string_release_tail(0xf, 0), Err(A53Fault::Terminate));
        assert_eq!(w.a53_string_release_tail(0x10, 0), Ok(0x11));
        assert_eq!(w.a53_string_release_tail(0x1000, 4), Ok(0x1024));
        assert_eq!(
            w.a53_string_release_tail(0x1000, 0x24),
            Err(A53Fault::CookieCorrupt)
        );
        assert_eq!(
            w.a53_string_release_tail(0x1000, 0x23),
            Ok(0x1024),
            "data - cookie == 0x23 is still aligned"
        );
    }

    #[test]
    fn reset_and_release_clears_the_header_before_the_diagnostic() {
        // 0x005fcf3b
        let w = world();
        let mut s = A53Str::from_bytes(b"0123456789abcdefXY");
        assert_eq!(s.cap, 0x1f);
        assert_eq!(w.a53_reset_and_release(&mut s, 4), Ok(0x20));
        assert_eq!(s, A53Str::empty());

        let mut inline = A53Str::from_bytes(b"tiny");
        assert_eq!(
            w.a53_reset_and_release(&mut inline, 0),
            Err(A53Fault::Terminate)
        );
        assert_eq!(inline, A53Str::empty(), "the reset already happened");
    }

    #[test]
    fn eof_guard_only_probes_when_the_state_and_flag_allow_it() {
        // 0x005fcfd0
        let w = world();
        let calls = std::cell::Cell::new(0);
        let probe = || {
            calls.set(calls.get() + 1);
            -1
        };
        assert_eq!(w.a53_eof_guard(0, 2, probe), Some(4));
        assert_eq!(calls.get(), 1);

        let untouched = || {
            calls.set(calls.get() + 1);
            -1
        };
        assert_eq!(w.a53_eof_guard(1, 2, untouched), None);
        assert_eq!(calls.get(), 1, "a non-zero state skips the probe");

        let ok = || 0;
        assert_eq!(w.a53_eof_guard(0, 2, ok), None, "a non -1 result is fine");
        assert_eq!(w.a53_eof_guard(0, 0, || -1), None);
    }

    #[test]
    fn realloc_1c_reports_the_allocation_and_signed_element_count() {
        // 0x005fd100
        let w = world();
        let plan = w.a53_realloc_1c(0x1000, 0x1058, 4);
        assert_eq!(plan.alloc, 4 * 0x1c);
        assert_eq!(plan.moved, 3, "0x58 / 0x1c truncates to 3");
        assert_eq!(w.a53_realloc_1c(0x1000, 0x1000, 1).moved, 0);
        assert_eq!(
            w.a53_realloc_1c(0x1000, 0x1004, A53_ALLOC1C_LIMIT).alloc,
            0xffff_ffff
        );
    }

    #[test]
    fn reserve4_models_both_header_paths() {
        // 0x005fd1b0
        let w = world();
        assert_eq!(
            w.a53_reserve4(0, 0x20, 0x20, 8, 0).unwrap(),
            A53Reserve4::Fresh {
                alloc: 0x20,
                moved: 8,
                end: 0x20,
                cap: 0x20,
            }
        );
        assert_eq!(
            w.a53_reserve4(0x1000, 0x1008, 0x1010, 4, 0).unwrap(),
            A53Reserve4::Reused {
                alloc: 0x10,
                moved: 2,
                release: 0x10,
            }
        );
        let large = w.a53_reserve4(0x1000, 0x1004, 0x2000, 1, 4).unwrap();
        assert_eq!(
            large,
            A53Reserve4::Reused {
                alloc: 4,
                moved: 1,
                release: 0x1023,
            },
            "(0x2000 - 0x1000) & ~3 == 0x1000 above 0xfff takes the header"
        );
    }

    #[test]
    fn vector_release_helpers_use_signed_stride_division() {
        // 0x005fd300, 0x005fd380
        let w = world();
        assert_eq!(w.a53_release_18(false, 0, 0), Ok(None));
        assert_eq!(w.a53_release_18(true, 0x30, 0), Ok(Some(0x30)));
        assert_eq!(
            w.a53_release_18(true, 0x31, 0),
            Ok(Some(0x30)),
            "the element count truncates"
        );
        assert_eq!(w.a53_release_34(true, 0x68, 0), Ok(Some(0x68)));
        assert_eq!(
            w.a53_release_34(true, 0x1800, 4),
            Ok(Some(0x181b)),
            "0x1800/0x34 == 118 elements, 0x17f8 bytes, plus the 0x23 header"
        );
        assert_eq!(
            w.a53_release_34(true, 0x1800, 0x24),
            Err(A53Fault::CookieCorrupt)
        );
    }

    #[test]
    fn move_bytes_is_overlap_safe() {
        // 0x005fd400
        let mut buf = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        assert!(a53_move_bytes(&mut buf, 0, 2, 4));
        assert_eq!(buf, vec![0, 1, 0, 1, 2, 3, 6, 7]);
        let mut back = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        assert!(a53_move_bytes(&mut back, 2, 0, 4));
        assert_eq!(back, vec![2, 3, 4, 5, 4, 5, 6, 7]);
        assert!(!a53_move_bytes(&mut back, 6, 0, 4));
    }

    fn req(kind: u8, value: u8, text: &[u8]) -> A53Req {
        A53Req {
            kind,
            value,
            text: A53Str::from_bytes(text),
        }
    }

    #[test]
    fn copy_1c_deep_copies_and_advances_by_stride() {
        // 0x005fd440
        let src = vec![req(1, 2, b"tech"), req(0, 9, b"unit")];
        let copy = a53_copy_1c(&src, 0x1000);
        assert_eq!(copy.dest, 0x1000 + 2 * 0x1c);
        assert_eq!(copy.copied, src);
        assert_ne!(
            copy.copied[0].text.text.as_ptr(),
            src[0].text.text.as_ptr(),
            "the +4 string is copy-constructed"
        );
    }

    #[test]
    fn destroy_34_loops_follow_the_pair_stride() {
        // 0x005fd500, 0x005fd5d0
        assert_eq!(a53_destroy_34_into(0x1000, 0x1068, 0x2000), 0x2068);
        assert_eq!(a53_destroy_34(0x1000, 0x1068), 2);
        assert_eq!(a53_destroy_34(0x1000, 0x1000), 0);
    }

    #[test]
    fn throw_rows_report_the_exact_messages() {
        // 0x005fd600, 0x005fd610, 0x005fd620, 0x005fd630
        assert_eq!(a53_string_too_long().message(), "string too long");
        assert_eq!(a53_vector_too_long().message(), "vector<T> too long");
        assert_eq!(
            a53_invalid_string_position().message(),
            "invalid string position"
        );
        assert_eq!(
            a53_invalid_vector_subscript().message(),
            "invalid vector<T> subscript"
        );
    }

    #[test]
    fn alloc_request_applies_the_0x23_header() {
        // 0x005fd640
        assert_eq!(a53_alloc_request(0), 0);
        assert_eq!(a53_alloc_request(1), 1);
        assert_eq!(a53_alloc_request(0xfff), 0xfff);
        assert_eq!(a53_alloc_request(0x1000), 0x1023);
        assert_eq!(
            a53_alloc_request(0xffff_ffff),
            0xffff_ffff,
            "the overflow guard saturates"
        );
    }

    #[test]
    fn typed_allocators_pick_the_small_or_headed_path() {
        // 0x005fd6a0, 0x005fd710, 0x005fd780, 0x005fd7f0
        let w = world();
        assert_eq!(w.a53_alloc4(1), 4);
        assert_eq!(w.a53_alloc4(0x400), 0x1023);
        assert_eq!(w.a53_alloc4(A53_ALLOC4_LIMIT), 0xffff_ffff);
        assert_eq!(w.a53_alloc1c(1), 0x1c);
        assert_eq!(w.a53_alloc1c(0x100), 0x1c23);
        assert_eq!(w.a53_alloc18(0x100), 0x1823);
        assert_eq!(w.a53_alloc34(0x100), 0x3423);
        assert_eq!(w.a53_alloc18(A53_ALLOC18_LIMIT), 0xffff_ffff);
    }

    #[test]
    fn append_fill_repeats_the_byte_within_or_above_the_capacity() {
        // 0x005fd860
        let w = world();
        let mut s = A53Str::empty();
        w.a53_append_fill(&mut s, b'a', 5).unwrap();
        assert_eq!(s.text, b"aaaaa");
        assert_eq!(s.cap, 0xf, "5 bytes stay inline");
        w.a53_append_fill(&mut s, b'b', 0x10).unwrap();
        assert_eq!(s.text.len(), 21);
        assert_eq!(s.cap, 0x1f, "(5 + 16) | 0xf == 0x1f beats cap + cap/2");
        assert_eq!(s.text[5..], *b"bbbbbbbbbbbbbbbb");
    }

    #[test]
    fn append_bytes_grows_through_the_shared_policy() {
        // 0x005fd9c0
        let w = world();
        let mut s = A53Str::from_bytes(b"hello");
        assert_eq!(s.cap, 0xf);
        w.a53_append_bytes(&mut s, b"0123456789abcdef").unwrap();
        assert_eq!(s.text.len(), 21);
        assert_eq!(s.cap, 0x1f);
        assert_eq!(&s.text[..5], b"hello");
        assert_eq!(w.a53_append_bytes(&mut s, &[0u8; 0x10]), Ok(()));
        assert_eq!(s.text.len(), 37);
    }

    #[test]
    fn assign_bytes_replaces_the_payload() {
        // 0x005fdb20
        let w = world();
        let mut s = A53Str::from_bytes(b"old");
        w.a53_assign_bytes(&mut s, b"new").unwrap();
        assert_eq!(s.text, b"new");
        assert_eq!(s.cap, 0xf, "3 bytes fit the inline buffer");
        w.a53_assign_bytes(&mut s, b"0123456789abcdef").unwrap();
        assert_eq!(s.text, b"0123456789abcdef");
        assert_eq!(s.cap, 0x1f);
        assert_eq!(
            a53_string_assign_capacity(0x8000_0000, 0x1f),
            Err(A53Error::StringTooLong)
        );
    }

    #[test]
    fn vec4_assign_grows_or_reuses_the_buffer() {
        // 0x005fdc10
        let w = world();
        assert_eq!(
            w.a53_vec4_assign(4, 2, 6).unwrap(),
            A53Vec4Assign {
                reallocated: true,
                alloc: 0x18,
                capacity: 6,
                size: 6,
                overwritten: 0,
            }
        );
        assert_eq!(
            w.a53_vec4_assign(8, 8, 3).unwrap(),
            A53Vec4Assign {
                reallocated: false,
                alloc: 0,
                capacity: 8,
                size: 3,
                overwritten: 3,
            }
        );
        assert_eq!(
            w.a53_vec4_assign(0, 0, A53_ALLOC4_LIMIT),
            Err(A53Error::VectorTooLong),
            "capacity + capacity/2 cannot exceed 0x3fffffff"
        );
        assert_eq!(
            w.a53_vec4_assign(0x3fff_ffff, 0, 8).unwrap(),
            A53Vec4Assign {
                reallocated: false,
                alloc: 0,
                capacity: 0x3fff_ffff,
                size: 8,
                overwritten: 0,
            }
        );
    }

    #[test]
    fn setstate_reports_the_first_raised_bit() {
        // 0x005fdee0
        let w = world();
        assert_eq!(
            w.a53_setstate(1, 0x17, false),
            A53StateChange {
                state: 1,
                raised: 1,
                abort: false,
                message: Some("ios_base::eofbit set"),
            }
        );
        assert_eq!(
            w.a53_setstate(2, 0x17, false).message,
            Some("ios_base::failbit set")
        );
        assert_eq!(
            w.a53_setstate(6, 0x17, false).message,
            Some("ios_base::badbit set"),
            "badbit wins over failbit"
        );
        assert_eq!(
            w.a53_setstate(2, 0, false),
            A53StateChange {
                state: 2,
                raised: 0,
                abort: false,
                message: None,
            }
        );
        assert!(w.a53_setstate(4, 0x17, true).abort);
        assert_eq!(w.a53_setstate(0x20, 0x17, false).state, 0, "0x17 mask");
    }

    #[test]
    fn release_block_and_items_apply_the_cookie_test() {
        // 0x005fdf60, 0x005fdfa0
        let w = world();
        assert_eq!(w.a53_release_block(0x10, 0), Ok(0x10));
        assert_eq!(w.a53_release_block(0x2000, 4), Ok(0x2023));
        assert_eq!(
            w.a53_release_block(0x2000, 0x24),
            Err(A53Fault::CookieCorrupt)
        );
        assert_eq!(a53_release_items(2, 0x20, 0), Ok(0x40));
        assert_eq!(a53_release_items(0x100, 0x20, 4), Ok(0x2023));
    }
}
