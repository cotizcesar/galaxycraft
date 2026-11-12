#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent73.rs
// ===========================================================================
// Translation batch 73.  Only this file is edited by the batch.
//
// Batch 73 owns `0x00624000..=0x00625fff`.  The ledger lists 68
// `decompiled_uncited` rows plus one thunk in that window; 22 of them are
// translated below and the remaining rows are listed at the end with the
// reason they are not.
//
// The window is two unrelated things:
//
// * `0x00624080..=0x006245f0` is a small `std::streambuf`-shaped string
//   buffer: an `0x24`-byte object with a vtable word, two cursor triples, a
//   storage pointer and a used length, plus the seek/copy primitives that
//   operate on it.
// * `0x00624e90..=0x00625ee0` is the pixel/texture cache: two `0x48`-byte
//   block descriptors, an open-addressing table keyed by texture id, the two
//   8x8 box/kernel blurs and the small-object release helpers shared with
//   `blz::thread::context`.
//
// Recovered layouts:
//
// | layout                    | words                                                      |
// |---------------------------|------------------------------------------------------------|
// | stream buffer (`0x24`)    | `+0x00` vtable, `+0x04` in_beg, `+0x08` in_cur, `+0x0c` in_end, `+0x10` out_beg, `+0x14` out_cur, `+0x18` out_end, `+0x1c` storage, `+0x20` used |
// | hash node                 | `+0x00` next, `+0x04` key record, `+0x08` data, `+0x0c` size, `+0x10` capacity+tag, `+0x20` payload |
// | key table (`0x006253e0`)  | `+0x00` bucket count, `+0x04` bucket array                 |
// | pixel cache (`0x00625c70`)| `+0x0c` bucket count, `+0x10` bucket array, `+0x1c` cache id |
// | block descriptor          | four dword words copied by `0x00624e90`/`0x00624f60`       |
// | tagged slot               | `+0x10` reference word (low bit = out of line), `+0x30`/`+0x4c` length words |
//
// Every recovered body in this window addresses its object through
// `ECX`/`EAX` (there is no stack argument for the object), so the
// translations are value types plus free functions; no `sim` record gains a
// field.
//
// Address to function:
//
// * `0x00624080` [`a73_invalid_position`]
// * `0x006240a0` [`A73StreamBuf::a73_seek_target`]
// * `0x00624170` [`a73_invalid_position_alt`]
// * `0x00624190` [`A73StreamBuf::a73_seek_commit`]
// * `0x00624200` [`A73StreamBuf::a73_sync_output`]
// * `0x00624220` [`a73_sbumpc`]
// * `0x00624280` [`a73_sputn_probe`]
// * `0x006242e0` [`a73_sgetn`]
// * `0x00624330` [`a73_xsputn`]
// * `0x00624430` [`a73_capacity_plan`]
// * `0x006245f0` [`a73_descriptor_probe`]
// * `0x00624e90` [`a73_descriptor_take`]
// * `0x00624f60` [`a73_descriptor_borrow`]
// * `0x00625300` [`a73_context_release`]
// * `0x00625343` [`a73_slot_release_checked`]
// * `0x00625399` [`a73_slot_release_ref`]
// * `0x006253e0` [`a73_hash_find`]
// * `0x00625550` [`a73_thread_context_ctor`]
// * `0x00625870` [`a73_cache_id`]
// * `0x00625c70` [`a73_pixel_fetch`]
// * `0x00625d00` [`a73_box_blur_8x8`]
// * `0x00625ee0` [`a73_kernel_blur_8x8`]
//
// The two blurs and the two pixel-cache helpers are the numerically
// interesting part of the window: `0x00625ee0` carries a literal 8x8 weight
// kernel (sum 128) and both blurs clamp through `0.0`/`1.0` with the shipped
// branch order.

// The batch template imports the simulation types; no recovered body in
// `0x00624000..=0x00625fff` touches simulation state, so the import is kept
// but unused.
// ---------------------------------------------------------------------------
// Faults shared by the window
// ---------------------------------------------------------------------------

/// Failure modes that the shipped bodies report by entering the noreturn
/// helper at `FUN_0093ec70` or by writing the `-1, -1` position pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A73Fault {
    /// `0x00624190`: the requested cursor would leave the buffer.
    PastEnd,
    /// `0x00624430`: the doubling loop cannot reach the requested count.
    UnrepresentableBuckets(u32),
    /// `0x006245f0`: the probed record starts with a zero byte.
    EmptyProbe,
    /// `0x006245f0`: the probed record's major version is above `0x14`.
    MajorTooNew(u32),
    /// `0x006245f0`: the probed record's minor version is above `4`.
    MinorTooNew(u32),
    /// `0x00625300`/`0x00625343`: a length word is not negative.
    LengthNotSmall,
    /// `0x006253e0`: the bucket array pointer is null.
    NoBuckets,
    /// `0x006253e0`: no node carries the key.
    NotFound,
}

// ---------------------------------------------------------------------------
// 0x00624080 / 0x00624170 - the "no position" pair
// ---------------------------------------------------------------------------

/// `0x00624080` (`FUN_00624080`, 23 bytes): the "no position" pair.
///
/// ```c
/// *param_1 = 0xffffffff;
/// param_1[1] = 0xffffffff;
/// ```
///
/// The whole body is the two stores (the 23 bytes are the stores plus the
/// `ret`/prologue padding); the pair is what the seek routines return for an
/// invalid request.
pub fn a73_invalid_position() -> (u32, u32) {
    (u32::MAX, u32::MAX)
}

/// `0x00624170` (`FUN_00624170`, 23 bytes): the second byte-identical copy of
/// [`a73_invalid_position`].
///
/// The retail image contains two separate copies of the store pair at
/// `0x00624080` and `0x00624170`; they are distinguished here only so that
/// both ledger rows have a translation.
pub fn a73_invalid_position_alt() -> (u32, u32) {
    (u32::MAX, u32::MAX)
}

// ---------------------------------------------------------------------------
// 0x006240a0 - seekoff-style target computation
// ---------------------------------------------------------------------------

/// The `0x24`-byte stream buffer addressed through `ECX` by the six routines
/// below.
///
/// Slot order is the decompiled `in_ECX[n]` indexing: `in_ECX[1]` is
/// [`A73StreamBuf::in_beg`] and so on, with slot 0 the vtable word that the
/// virtual calls use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A73StreamBuf {
    /// `+0x04` (slot 1): first readable byte.
    pub in_beg: u32,
    /// `+0x08` (slot 2): next readable byte.
    pub in_cur: u32,
    /// `+0x0c` (slot 3): one past the last readable byte.
    pub in_end: u32,
    /// `+0x10` (slot 4): first writable byte.
    pub out_beg: u32,
    /// `+0x14` (slot 5): next writable byte.
    pub out_cur: u32,
    /// `+0x18` (slot 6): one past the last writable byte.
    pub out_end: u32,
    /// `+0x1c` (slot 7): storage base.
    pub storage: u32,
    /// `+0x20` (slot 8): used length of `storage`.
    pub used: u32,
}

/// What `0x006240a0` asks the virtual `seekoff` at vtable `+0x0c` to seek to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A73SeekPlan {
    /// The `(lo, hi)` pair handed to the virtual call, as one 64-bit value.
    At(i64),
    /// `which == 1` with neither direction flag: the shipped routine falls
    /// through with `local_14`/`iStack_10` still uninitialised, so there is no
    /// defined target.
    Undefined,
    /// The `-1, -1` pair from [`a73_invalid_position`].
    Invalid,
}

/// Result of [`A73StreamBuf::a73_seek_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73SeekResult {
    /// Address that received the NUL terminator in the prologue, if the
    /// prologue ran and the used length had to grow.
    pub terminator: Option<u32>,
    /// The target the virtual call receives.
    pub plan: A73SeekPlan,
}

impl A73StreamBuf {
    /// Shared prologue of `0x006240a0`, `0x00624190` and `0x00624200`:
    /// resynchronise `used` with the output cursor and NUL-terminate.
    ///
    /// ```c
    /// uVar1 = in_ECX[5] - in_ECX[4];
    /// if (in_ECX[8] < uVar1) {
    ///   *(undefined1 *)(uVar1 + in_ECX[7]) = 0;
    ///   in_ECX[8] = uVar1;
    /// }
    /// ```
    ///
    /// Returns the address of the terminator byte when the store happened.
    pub fn a73_close_output(&mut self) -> Option<u32> {
        let used = self.out_cur.wrapping_sub(self.out_beg);
        if self.used < used {
            self.used = used;
            Some(self.storage.wrapping_add(used))
        } else {
            None
        }
    }

    /// `0x006240a0` (`FUN_006240a0`, 207 bytes): the `seekoff` request
    /// builder.
    ///
    /// ```c
    /// if (((flags & 0x18) == 0x18) && (which == 1)) goto bad;
    /// <close_output>
    /// if (which == 0) { lo = off_lo; hi = off_hi; }
    /// else if (which == 1) {
    ///   if (flags & 8)  { d = in_end  - in_beg;  lo = d + off_lo; hi = sign(d) + off_hi + carry; }
    ///   else if (flags & 0x10) { d = out_end - out_beg; ... }
    /// } else if (which == 2) { lo = used + off_lo; hi = off_hi + carry; }
    /// else goto bad;
    /// (**(code **)(*in_ECX + 0xc))(param_1, lo, hi);
    /// ```
    ///
    /// The `which == 1` addend is sign-extended (`(int)d >> 31` fills the high
    /// word), the `which == 2` addend is zero-extended (`used` is added as an
    /// unsigned word), and `which == 0` passes the caller's pair unchanged.
    /// The `(lo, hi)` output pair is the virtual call's 64-bit result; it is
    /// returned as [`A73SeekPlan::At`].
    pub fn a73_seek_target(
        &mut self,
        off_lo: u32,
        off_hi: u32,
        which: i32,
        flags: u8,
    ) -> A73SeekResult {
        if (flags & 0x18) == 0x18 && which == 1 {
            return A73SeekResult {
                terminator: None,
                plan: A73SeekPlan::Invalid,
            };
        }
        let terminator = self.a73_close_output();
        let base = (((off_hi as u64) << 32) | off_lo as u64) as i64;
        let plan = match which {
            0 => A73SeekPlan::At(base),
            1 => {
                if flags & 8 != 0 {
                    let d = self.in_end.wrapping_sub(self.in_beg) as i32 as i64;
                    A73SeekPlan::At(base.wrapping_add(d))
                } else if flags & 0x10 != 0 {
                    let d = self.out_end.wrapping_sub(self.out_beg) as i32 as i64;
                    A73SeekPlan::At(base.wrapping_add(d))
                } else {
                    A73SeekPlan::Undefined
                }
            }
            2 => A73SeekPlan::At(base.wrapping_add(self.used as u64 as i64)),
            _ => A73SeekPlan::Invalid,
        };
        A73SeekResult { terminator, plan }
    }

    /// `0x00624190` (`FUN_00624190`, 99 bytes): move one or both cursors by a
    /// signed count and echo the four output words.
    ///
    /// ```c
    /// <close_output>
    /// if (flags & 8)  { v = in_beg  + n; if (in_end  < v) goto bad; in_cur  = v; }
    /// if (flags & 0x10) { v = out_beg + n; if (out_end < v) goto bad; out_cur = v; }
    /// param_1[0] = n; param_1[1..3] = the other three arguments;
    /// ```
    ///
    /// The bound comparisons are unsigned.  The failure path writes the
    /// `-1, -1` pair into the first two output words; here it is
    /// [`A73Fault::PastEnd`] and no cursor changes.
    pub fn a73_seek_commit(
        &mut self,
        n: i32,
        second: i32,
        third: i32,
        fourth: i32,
        flags: u8,
    ) -> Result<[u32; 4], A73Fault> {
        self.a73_close_output();
        if flags & 8 != 0 {
            let v = self.in_beg.wrapping_add(n as u32);
            if self.in_end < v {
                return Err(A73Fault::PastEnd);
            }
            self.in_cur = v;
        }
        if flags & 0x10 != 0 {
            let v = self.out_beg.wrapping_add(n as u32);
            if self.out_end < v {
                return Err(A73Fault::PastEnd);
            }
            self.out_cur = v;
        }
        Ok([n as u32, second as u32, third as u32, fourth as u32])
    }

    /// `0x00624200` (`FUN_00624200`, 24 bytes): the close-out prologue alone,
    /// returning zero.
    ///
    /// ```c
    /// uVar1 = in_ECX[5] - in_ECX[4];
    /// if (in_ECX[8] < uVar1) { *(undefined1 *)(uVar1 + in_ECX[7]) = 0; in_ECX[8] = uVar1; }
    /// return 0;
    /// ```
    pub fn a73_sync_output(&mut self) -> i32 {
        self.a73_close_output();
        0
    }
}

// ---------------------------------------------------------------------------
// 0x00624220 - single-byte read
// ---------------------------------------------------------------------------

/// `0x00624220` (`FUN_00624220`, 31 bytes): gate, then load-and-advance.
///
/// ```c
/// iVar2 = (**(code **)(*in_ECX + 0x1c))();
/// if (iVar2 == -1) return 0xffffffff;
/// bVar1 = *(byte *)in_ECX[2];
/// in_ECX[2] = (int)((byte *)in_ECX[2] + 1);
/// return (uint)bVar1;
/// ```
///
/// The gate at vtable `+0x1c` is called unconditionally, before the byte is
/// read; only its `-1` result is inspected, and the byte still comes from
/// `in_cur`.  `src` is the byte array that `in_cur` addresses and `cur` is the
/// `in_cur` offset, so the load-and-advance is `src[cur]` then `cur += 1`.
/// Returns `-1` when the gate reports end of stream (the cursor is then left
/// alone), otherwise the zero-extended byte.
pub fn a73_sbumpc(src: &[u8], cur: &mut usize, gate: i32) -> i32 {
    if gate == -1 {
        return -1;
    }
    let byte = src[*cur];
    *cur += 1;
    byte as i32
}

// ---------------------------------------------------------------------------
// 0x00624280 - sputn bookkeeping probe
// ---------------------------------------------------------------------------

/// Result of [`a73_sputn_probe`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73PutnProbe {
    /// Whether the virtual `putn` at vtable `+0x28` is called.
    pub call_virtual: bool,
    /// The state word the routine leaves behind.
    pub state: u32,
}

/// `0x00624280` (`FUN_00624280`, 96 bytes): the `sputn` fast path.
///
/// ```c
/// if ((-1 < hi) && ((0 < hi) || (lo != 0))) {
///   uVar3 = *(uint *)(*(int *)(*in_ECX + 4) + 8 + (int)in_ECX);   // state word
///   if (uVar3 == 0) {
///     lVar4 = (**(*vtable + 0x28))(buf, lo, hi);                   // virtual putn
///     if (lVar4 != CONCAT44(hi, lo)) { state |= 1; return; }       // short write
///   } else {
///     state = uVar3 | 1;                                           // already failed
///   }
/// }
/// ```
///
/// The guard is the 64-bit count `(hi:lo) > 0` tested as `hi >= 0 && (hi > 0
/// || lo != 0)`.  The state word lives at `object + vtable[1] + 8`; only its
/// zero/non-zero test and the `| 1` store matter, so it is passed in and
/// returned.  `written` is the virtual call's result; a result different from
/// the requested count sets bit 0.
pub fn a73_sputn_probe(state_word: u32, lo: u32, hi: i32, written: i64) -> A73PutnProbe {
    if hi < 0 || (hi == 0 && lo == 0) {
        return A73PutnProbe {
            call_virtual: false,
            state: state_word,
        };
    }
    if state_word != 0 {
        return A73PutnProbe {
            call_virtual: false,
            state: state_word | 1,
        };
    }
    let requested = (((hi as u32 as u64) << 32) | lo as u64) as i64;
    A73PutnProbe {
        call_virtual: true,
        state: if written == requested { 0 } else { 1 },
    }
}

// ---------------------------------------------------------------------------
// 0x006242e0 / 0x00624330 - the two bulk transfers
// ---------------------------------------------------------------------------

/// `0x006242e0` (`FUN_006242e0`, 74 bytes): read up to `out.len()` bytes,
/// calling the vtable `+0x20` refill whenever `in_cur == in_end`.
///
/// ```c
/// while (out_end > out) {
///   if (in_ECX[2] == in_ECX[3]) uVar3 = (**(*vtable + 0x20))();
///   else { uVar3 = *in_ECX[2]; in_ECX[2]++; }
///   if (uVar3 == 0xffffffff) break;
///   *out++ = (char)uVar3;
/// }
/// ```
///
/// `src`/`cur` model the readable buffer and `in_cur`; `in_end` is
/// `src.len()`.  Returns the number of bytes stored in `out`; the cursor is
/// left where the failing refill found it.
pub fn a73_sgetn(
    src: &[u8],
    cur: &mut usize,
    out: &mut [u8],
    mut refill: impl FnMut() -> i32,
) -> usize {
    let mut written = 0;
    while written < out.len() {
        let value = if *cur == src.len() {
            refill()
        } else {
            let byte = src[*cur];
            *cur += 1;
            byte as i32
        };
        if value == -1 {
            break;
        }
        out[written] = value as u8;
        written += 1;
    }
    written
}

/// Result of [`a73_xsputn`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73Putn {
    /// Number of source bytes accepted.
    pub written: usize,
    /// The `out_cur` the routine leaves behind.
    pub out_cur: u32,
}

/// `0x00624330` (`FUN_00624330`, 85 bytes): write as many bytes as the output
/// buffer takes, with one overflow call for the first byte when the buffer is
/// already full.
///
/// ```c
/// n = count; if (n == 0) return;
/// avail = out_end - out_cur;
/// if (avail == 0) {
///   if ((**(*vtable + 0x2c))(*src) == -1) return;
///   ++src; --n; avail = out_end - out_cur;
/// }
/// if (n <= avail) avail = n;
/// memcpy(out_cur, src, avail);
/// ```
///
/// The row ends in the `memcpy` tail at `FUN_00c89580`, which the decompiler
/// marks non-returning; the `out_cur += avail` store that follows it in the
/// shipped body is reconstructed from the cursor convention the neighbouring
/// rows use (load cursor, store cursor, advance by the copied length), and is
/// reported as [`A73Putn::out_cur`].
pub fn a73_xsputn(
    src: &[u8],
    out_cur: u32,
    out_end: u32,
    overflow: impl FnOnce(u8) -> i32,
) -> A73Putn {
    if src.is_empty() {
        return A73Putn {
            written: 0,
            out_cur,
        };
    }
    let mut n = src.len() as u32;
    let mut src = src;
    let mut written = 0usize;
    let mut avail = out_end.wrapping_sub(out_cur);
    if avail == 0 {
        if overflow(src[0]) == -1 {
            return A73Putn {
                written: 0,
                out_cur,
            };
        }
        src = &src[1..];
        n -= 1;
        written = 1;
        avail = out_end.wrapping_sub(out_cur);
    }
    let copy = if n <= avail { n } else { avail } as usize;
    // memcpy(out_cur, src, copy): the destination is the caller's buffer, so
    // only the copied length and the cursor advance are observable here.
    debug_assert!(copy <= src.len());
    written += copy;
    A73Putn {
        written,
        out_cur: out_cur.wrapping_add(copy as u32),
    }
}

// ---------------------------------------------------------------------------
// 0x00624430 - bucket count plan
// ---------------------------------------------------------------------------

/// Result of [`a73_capacity_plan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73CapacityPlan {
    /// The local the routine raises to at least the candidate.  The
    /// `in_ECX[0]` store happens before the raise, so the descriptor keeps the
    /// caller's value; this is the value the bucket loop then uses.
    pub first: u32,
    /// The number of buckets the doubling loop stops on.
    pub buckets: u32,
    /// `(u32)(f32)count`, the truncation at `FUN_00c86e10`.
    pub truncated_count: u32,
    /// `(u32)(f32)max_load`, the truncation stored at `in_ECX[3]`.
    pub stored_load: u32,
}

/// `0x00624430` (`FUN_00624430`, 149 bytes): initialise a hash-table
/// descriptor and choose its bucket count.
///
/// ```c
/// *in_ECX       = param_1[0];
/// in_ECX[2]     = param_1[2];
/// in_ECX[1]     = 0;
/// in_ECX[3]     = (uint)(float)param_1[3];
/// uVar4 = (u32)(f32)param_1[2];                       // FUN_00c86e10
/// buckets = 8;  candidate = uVar4 + 1;
/// if ((f32)param_1[2] <= (f32)uVar4 * (float)param_1[3]) candidate = uVar4;
/// if (*param_1 <= candidate) *param_1 = candidate;
/// if (8 < *param_1) { do { buckets *= 2; } while (buckets < *param_1); }
/// ```
///
/// The `(float)param_1[3]` conversion is numeric, not a bit reinterpretation,
/// so `max_load` is an integer-valued count.  The retail row ends in the
/// noreturn helper at `FUN_0093ec70`; the doubling loop is reproduced here and a
/// candidate above `2^31` (which makes the retail loop wrap to zero forever)
/// is reported as [`A73Fault::UnrepresentableBuckets`].
pub fn a73_capacity_plan(
    first: u32,
    element_count: u32,
    max_load: u32,
) -> Result<A73CapacityPlan, A73Fault> {
    let load = max_load as f32;
    let stored_load = a73_cvtt_f32_to_u32(load);
    let truncated = a73_cvtt_f32_to_u32(element_count as f32);
    let mut candidate = truncated.wrapping_add(1);
    if (element_count as f32) <= (truncated as f32) * load {
        candidate = truncated;
    }
    let mut first = first;
    if first <= candidate {
        first = candidate;
    }
    let mut buckets = 8u32;
    if 8 < first {
        loop {
            if buckets > 0x4000_0000 {
                return Err(A73Fault::UnrepresentableBuckets(first));
            }
            buckets = buckets.wrapping_mul(2);
            if buckets >= first {
                break;
            }
        }
    }
    Ok(A73CapacityPlan {
        first,
        buckets,
        truncated_count: truncated,
        stored_load,
    })
}

/// `cvttss2si`-style truncation to a 32-bit unsigned value: out-of-range and
/// NaN inputs produce `0x8000_0000`, the value the shipped conversion yields.
/// Both call sites convert a `u32` operand, so the input is never negative.
fn a73_cvtt_f32_to_u32(value: f32) -> u32 {
    if value.is_nan() || !(0.0..4_294_967_296.0).contains(&value) {
        return 0x8000_0000;
    }
    value.trunc() as u32
}

/// `cvttss2si`-style truncation to a 32-bit signed value.
fn a73_cvtt_f32_to_i32(value: f32) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        return i32::MIN;
    }
    value.trunc() as i32
}

/// `cvttsd2si`-style truncation to a 32-bit signed value.
fn a73_cvtt_f64_to_i32(value: f64) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        return i32::MIN;
    }
    value.trunc() as i32
}

// ---------------------------------------------------------------------------
// 0x006245f0 - descriptor version probe
// ---------------------------------------------------------------------------

/// Result of [`a73_descriptor_probe`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73DescriptorProbe {
    /// The word stored at `*in_ECX` before the probe (`1`).
    pub word: u32,
    /// Whether the tagged reference survived the low-bit clearing non-null.
    pub probed: bool,
}

/// `0x006245f0` (`FUN_006245f0`, 124 bytes): validate a tagged descriptor.
///
/// ```c
/// *in_ECX = 1;
/// if ((*param_1 & 1) != 0) param_1 = (uint *)(*param_1 & 0xfffffffe);
/// if (param_1 != 0) {
///   (**(*param_1 + 8))(&major, &minor, &flag);          // virtual probe
///   if ((flag == 0) || (0x14 < major) || (4 < minor)) goto noreturn;
///   ...
/// }
/// ```
///
/// The three recovered comparisons are the validation: the flag byte must be
/// non-zero, the major version at most `0x14` (20) and the minor at most `4`.
/// A reference word of `1` clears to null, in which case the probe is skipped
/// entirely.
pub fn a73_descriptor_probe(
    reference_word: u32,
    flag: bool,
    major: u32,
    minor: u32,
) -> Result<A73DescriptorProbe, A73Fault> {
    let pointer = if reference_word & 1 != 0 {
        reference_word & !1
    } else {
        reference_word
    };
    if pointer == 0 {
        return Ok(A73DescriptorProbe {
            word: 1,
            probed: false,
        });
    }
    if !flag {
        return Err(A73Fault::EmptyProbe);
    }
    if major > 0x14 {
        return Err(A73Fault::MajorTooNew(major));
    }
    if minor > 4 {
        return Err(A73Fault::MinorTooNew(minor));
    }
    Ok(A73DescriptorProbe {
        word: 1,
        probed: true,
    })
}

// ---------------------------------------------------------------------------
// 0x00624e90 / 0x00624f60 - the block descriptor copies
// ---------------------------------------------------------------------------

/// Result of [`a73_descriptor_take`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73DescriptorTake {
    /// The four words installed in the destination.
    pub dst: [u32; 4],
    /// The source after its `+0x04` and `+0x08` words were cleared.
    pub src_after: [u32; 4],
    /// The two-byte tag copied from source `+0x24`.
    pub tag: u16,
}

/// `0x00624e90` (`FUN_00624e90`, 130 bytes): move a block descriptor.
///
/// ```c
/// in_ECX[1] = 0;
/// in_ECX[0] = param_1[0];
/// in_ECX[3] = param_1[3];
/// in_ECX[2] = param_1[2];
/// in_ECX[1] = param_1[1];
/// param_1[2] = 0;
/// param_1[1] = 0;
/// <move the +0x10 sub-object>
/// *(undefined2 *)(in_ECX + 9) = *(undefined2 *)(param_1 + 9);   // +0x24
/// ```
///
/// The four-word copy with the source's middle words cleared and the
/// two-byte tag at `+0x24` are the whole recovered state change.  The row's
/// `+0x10` sub-object move and its `FUN_0053a5b0` tail are not part of the copy
/// arithmetic and are not reproduced.
pub fn a73_descriptor_take(src: [u32; 4], tag: u16) -> A73DescriptorTake {
    A73DescriptorTake {
        dst: src,
        src_after: [src[0], 0, 0, src[3]],
        tag,
    }
}

/// `0x00624f60` (`FUN_00624f60`, 77 bytes): copy a block descriptor with the
/// middle word zeroed.
///
/// ```c
/// uVar1 = *param_1;
/// *in_ECX   = uVar1;
/// in_ECX[2] = param_1[2];
/// in_ECX[1] = 0;
/// in_ECX[3] = param_1[3];
/// <noreturn FUN_0053e930 with uVar1>
/// ```
///
/// The row ends in the noreturn helper every time, so it is a "copy then
/// abort" path; only the copied words are returned.
pub fn a73_descriptor_borrow(src: [u32; 4]) -> [u32; 4] {
    [src[0], 0, src[2], src[3]]
}

// ---------------------------------------------------------------------------
// 0x00625300 / 0x00625343 / 0x00625399 - tagged slot release
// ---------------------------------------------------------------------------

/// The reference a tagged slot resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73SlotReset {
    /// The object reference: either the inline storage address or the
    /// out-of-line pointer cleared of its tag bit.
    pub reference: u32,
    /// True when `reference` is the inline storage address (`word & 1 == 0`).
    pub inline: bool,
    /// The word installed in the slot afterwards (`1`, the empty tag).
    pub store: u32,
}

/// `0x00625399` (`FUN_00625399`, 58 bytes): resolve the tagged slot, hand the
/// reference to the virtual call at `+0x10`, then reset the slot.
///
/// ```c
/// if ((word & 1) == 0) uVar1 = this + 0x10;
/// else uVar1 = *(uint *)(this + 0x10) & 0xfffffffe;
/// (**(code **)(*in_EAX + 0x10))(uVar1);
/// *(undefined4 *)(this + 0x10) = 1;
/// ```
pub fn a73_slot_release_ref(word: u32, inline_addr: u32) -> A73SlotReset {
    let inline = word & 1 == 0;
    A73SlotReset {
        reference: if inline { inline_addr } else { word & !1 },
        inline,
        store: 1,
    }
}

/// `0x00625343` (`FUN_00625343`, 31 bytes): release the tagged slot after
/// checking the length word at `+0x30`.
///
/// ```c
/// (**(*in_EAX + 0x10))();
/// iVar2 = *(int *)(this + 0x30);
/// if (-1 < iVar2) goto noreturn;
/// <same tagged-slot release as 0x00625399>
/// ```
///
/// The length word is the small-string flag word of the held record, and the
/// check requires its high bit set (`iVar2 < 0`).
pub fn a73_slot_release_checked(
    len_30: i32,
    word: u32,
    inline_addr: u32,
) -> Result<A73SlotReset, A73Fault> {
    if len_30 >= 0 {
        return Err(A73Fault::LengthNotSmall);
    }
    Ok(a73_slot_release_ref(word, inline_addr))
}

/// `0x00625300` (`FUN_00625300`, 67 bytes): destroy a context object, checking
/// two length words first.
///
/// ```c
/// <base destructor FUN_0053d1c0>
/// if (-1 < *(int *)(this + 0x4c)) goto noreturn;
/// if (-1 < *(int *)(this + 0x30)) goto noreturn;
/// uVar1 = (*(byte *)(this + 0x10) & 1) ? (*(uint *)(this + 0x10) & 0xfffffffe)
///                                      : this + 0x10;
/// if (uVar1 != 0) { (**(*uVar1))(0); word = 1; }
/// ```
///
/// Both length words must have their high bit set.  The released reference is
/// dereferenced through its own vtable with argument `0`; that call is not
/// reproduced, only the slot word it leaves behind.
pub fn a73_context_release(
    len_4c: i32,
    len_30: i32,
    word: u32,
    inline_addr: u32,
) -> Result<A73SlotReset, A73Fault> {
    if len_4c >= 0 || len_30 >= 0 {
        return Err(A73Fault::LengthNotSmall);
    }
    Ok(a73_slot_release_ref(word, inline_addr))
}

// ---------------------------------------------------------------------------
// 0x006253e0 / 0x00625550 / 0x00625870 - the pixel cache
// ---------------------------------------------------------------------------

/// One node of the texture-cache hash table.
///
/// The decompiled layout is `+0x00` next, `+0x04` key record, `+0x08` data
/// pointer, `+0x0c` element count, `+0x10` capacity word (high bit = tag) and
/// the payload at `+0x20`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A73HashNode {
    /// `+0x00`: next node index in the chain.
    pub next: Option<usize>,
    /// `+0x04`: key record.  `0x006253e0` compares the caller's record
    /// against it with the helper at `FUN_00539640`; the model keeps one
    /// comparable word.
    pub key: u32,
    /// `+0x20`: payload word returned by `0x006253e0`.
    pub value: u32,
}

/// Result of [`a73_hash_find`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73HashHit {
    /// Index of the matching node.
    pub node: usize,
    /// The payload word at node `+0x20`.
    pub value: u32,
}

/// `0x006253e0` (`FUN_006253e0`, 128 bytes): find the payload for a key.
///
/// ```c
/// if (in_ECX[1] == 0) FUN_0053cdd0(*in_ECX);                 // no buckets
/// uVar5 = FUN_00c62800(key_ptr, key_len, 0);                 // key hash
/// uVar5 = *in_ECX - 1U & uVar5;                              // bucket
/// piVar2 = &bucket[uVar5];
/// for (piVar1 = *piVar2; piVar1 != 0 && !equal(key, piVar1 + 1);
///      piVar1 = *piVar1) piVar2 = piVar1;
/// if (*piVar2 == 0) goto noreturn;
/// return *piVar2 + 0x20;
/// ```
///
/// The hash itself is produced by the helper at `FUN_00c62800`, so it is an
/// input here; the row's own arithmetic starts at the mask `bucket_count - 1`,
/// and the chain walk keeps the *link* that points at the match.  A null
/// bucket array is [`A73Fault::NoBuckets`] and an absent key
/// [`A73Fault::NotFound`].
pub fn a73_hash_find(
    buckets: &[Option<usize>],
    nodes: &[A73HashNode],
    hash: u32,
    key: u32,
) -> Result<A73HashHit, A73Fault> {
    if buckets.is_empty() {
        return Err(A73Fault::NoBuckets);
    }
    let index = ((buckets.len() as u32 - 1) & hash) as usize;
    let mut link = buckets[index];
    while let Some(node) = link {
        let entry = nodes.get(node).ok_or(A73Fault::NotFound)?;
        if entry.key == key {
            return Ok(A73HashHit {
                node,
                value: entry.value,
            });
        }
        link = entry.next;
    }
    Err(A73Fault::NotFound)
}

/// Result of [`a73_thread_context_ctor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73ThreadContextCtor {
    /// The vtable symbol the constructor installs.
    pub vftable: &'static str,
    /// Whether the `deleting` argument asks for the operator-delete tail.
    pub deleting: bool,
}

/// `0x00625550` (`FUN_00625550`, 44 bytes): derived-constructor tail.
///
/// ```c
/// FUN_00625300();                                  // base constructor
/// *in_ECX = blz::thread::context::vftable;
/// if ((param_1 & 1) != 0) FUN_00923e90();          // operator delete
/// ```
///
/// The only string in the row is the vtable symbol, reproduced literally.
pub fn a73_thread_context_ctor(deleting: bool) -> A73ThreadContextCtor {
    A73ThreadContextCtor {
        vftable: "blz::thread::context::vftable",
        deleting,
    }
}

/// `0x00625870` (`FUN_00625870`, 6 bytes): `return *(undefined4 *)(in_ECX + 0x1c);`
///
/// The cache id lives at object offset `0x1c`, i.e. dword index 7.
pub fn a73_cache_id(frame: &[u32]) -> Option<u32> {
    frame.get(A73_CACHE_ID_OFFSET / 4).copied()
}

/// Byte offset of the cache id inside the pixel-cache object.
pub const A73_CACHE_ID_OFFSET: usize = 0x1c;

/// Where `0x00625c70` reads a pixel from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A73PixelSlot {
    /// The hash bucket for a non-negative texture id.
    Bucket(u32),
    /// The shared "last cache" object the row falls back to when the id's
    /// high bit is set.
    FallbackLastCache,
}

/// Result of [`a73_pixel_fetch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A73PixelFetch {
    /// Bucket (or fallback) selection.
    pub slot: A73PixelSlot,
    /// `(x + y * 4) * 4`, the byte offset inside the cache's pixel array.
    pub byte_offset: i32,
}

/// `0x00625c70` (`FUN_00625c70`, 132 bytes): locate the cached pixel word.
///
/// ```c
/// if (-1 < (int)param_3) {
///   uVar3 = (param_3 >> 0x10 ^ param_3) * -0x7a143595;
///   uVar3 = (uVar3 >> 0xd ^ uVar3) * -0x3d4d51cb;
///   for (puVar1 = table[(*in_ECX + 0xc) - 1U & (uVar3 ^ uVar3 >> 0x10)];
///        param_3 != puVar1[1]; puVar1 = *puVar1) ;
///   return *(undefined4 *)(puVar1[2] + (param_1 + param_2 * 4) * 4);
/// }
/// return *(undefined4 *)(*(int *)(*fallback + 8) + (param_1 + param_2 * 4) * 4);
/// ```
///
/// The bucket word is the MurmurHash3 finaliser applied to the texture id
/// (`0x85ebca6b` then `0xc2b2ae35`, shifts 16 and 13) masked by the bucket
/// count minus one; the element offset is `x + 4 * y` words.  The walk itself
/// is [`a73_hash_find`]'s chain.
pub fn a73_pixel_fetch(bucket_count: u32, x: i32, y: i32, id: u32) -> A73PixelFetch {
    let byte_offset = x.wrapping_add(y.wrapping_mul(4)).wrapping_mul(4);
    let slot = if (id as i32) < 0 {
        A73PixelSlot::FallbackLastCache
    } else {
        let mut hash = (id >> 0x10) ^ id;
        hash = hash.wrapping_mul(0x85eb_ca6b);
        hash = (hash >> 0xd) ^ hash;
        hash = hash.wrapping_mul(0xc2b2_ae35);
        A73PixelSlot::Bucket((bucket_count.wrapping_sub(1)) & (hash ^ (hash >> 0x10)))
    };
    A73PixelFetch { slot, byte_offset }
}

// ---------------------------------------------------------------------------
// 0x00625d00 / 0x00625ee0 - the two 8x8 blurs
// ---------------------------------------------------------------------------

/// Side of the blur kernel: both routines sample an 8x8 block.
pub const A73_BLUR_SIDE: i32 = 8;

/// The 8x8 weight kernel written into the stack array of `0x00625ee0`.
///
/// The literal is a diamond-shaped bilinear kernel; the row also accumulates
/// the weights as it goes, and their sum is 128 (the divisor the routine
/// uses):
///
/// ```text
/// 1 1 1 1 1 1 1 1
/// 1 1 1 2 2 1 1 1
/// 1 1 2 4 4 2 1 1
/// 1 2 4 8 8 4 2 1
/// 1 2 4 8 8 4 2 1
/// 1 1 2 4 4 2 1 1
/// 1 1 1 2 2 1 1 1
/// 1 1 1 1 1 1 1 1
/// ```
pub const A73_KERNEL_8X8: [u16; 64] = [
    1, 1, 1, 1, 1, 1, 1, 1, //
    1, 1, 1, 2, 2, 1, 1, 1, //
    1, 1, 2, 4, 4, 2, 1, 1, //
    1, 2, 4, 8, 8, 4, 2, 1, //
    1, 2, 4, 8, 8, 4, 2, 1, //
    1, 1, 2, 4, 4, 2, 1, 1, //
    1, 1, 1, 2, 2, 1, 1, 1, //
    1, 1, 1, 1, 1, 1, 1, 1,
];

/// Result of both blurs: the four arguments of the pack call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A73Blur {
    /// The three clamped colour channels, in the order the pack call takes
    /// them (red, green, blue).
    pub rgb: [f32; 3],
    /// `alpha / 255.0`, the pack call's fourth colour argument.
    pub alpha: f32,
}

/// A sampled pixel: the three channel values the extractors at `FUN_00a8fd20`,
/// `FUN_00a8fcf0` and `FUN_00a8fcc0` return, in that order.
///
/// Both blurs divide their accumulator by the number of samples (64) or by the
/// weight total (128) and then saturate into `[0, 1]`, so the extractors
/// already hand back normalised components rather than `0..=255` bytes.
pub type A73Sample = [f32; 3];

/// The shipped saturation: `0.0` for negative values, `1.0` above one, the
/// value itself in between, and `0.0` for NaN (`0.0 <= NaN` is false).
fn a73_saturate(value: f32) -> f32 {
    if !(0.0 <= value) {
        0.0
    } else if value <= 1.0 {
        value
    } else {
        1.0
    }
}

/// `0x00625d00` (`FUN_00625d00`, 439 bytes): unweighted 8x8 box average of a
/// cached texture block.
///
/// ```c
/// fVar6 = (float)(int)param_2;  fVar7 = (float)(int)param_3;
/// local_8 = (int)((float)param_1[4] * fVar6 * 0.25);   // x offset
/// local_c = (int)((float)param_1[5] * fVar7 * 0.25);   // y offset
/// for (j = 0; j < 8; ++j)
///   for (i = 0; i < 8; ++i) {
///     (**(*param_1 + 8))(px, a + local_8, c + local_c); // sample
///     r += channel0(px); g += channel1(px); b += channel2(px);
///   }
/// fVar7 = (float)alpha / 255.0;
/// r = (float)r * 0.015625 + fVar7;                     // / 64, then tint
/// ... saturate each channel into [0, 1] ...
/// (**pack)(px, r, g, b, (float)param_4 / 255.0);
/// ```
///
/// `field4`/`field5` are the block descriptor's dword words at `+0x10` and
/// `+0x14` (the sample stride), `scale_x`/`scale_y` are `param_2`/`param_3`
/// (truncated to integers), `tint` is the low byte of `param_5` and `alpha`
/// the low byte of `param_4`.  The 64 samples come from the virtual sampler at
/// vtable `+8`, modelled as a closure returning the three normalised channels
/// ([`A73Sample`]).
pub fn a73_box_blur_8x8(
    field4: i32,
    field5: i32,
    scale_x: f32,
    scale_y: f32,
    alpha: u8,
    tint: u8,
    mut sample: impl FnMut(i32, i32) -> A73Sample,
) -> A73Blur {
    let sx = a73_cvtt_f32_to_i32(scale_x);
    let sy = a73_cvtt_f32_to_i32(scale_y);
    // `(float)field4 * fVar6` rounds to f32 first; the `* 0.25` then widens to
    // double, matching the shipped single-precision product.
    let ox = a73_cvtt_f64_to_i32((field4 as f32 * sx as f32) as f64 * 0.25);
    let oy = a73_cvtt_f64_to_i32((field5 as f32 * sy as f32) as f64 * 0.25);
    let mut accumulator = [0.0f32; 3];
    for row in 0..A73_BLUR_SIDE {
        for column in 0..A73_BLUR_SIDE {
            let channels = sample(ox + column, oy + row);
            for (total, channel) in accumulator.iter_mut().zip(channels) {
                *total += channel;
            }
        }
    }
    let tint_scaled = (tint as f32 as f64 / 255.0) as f32;
    let mut rgb = [0.0f32; 3];
    for (out, total) in rgb.iter_mut().zip(accumulator) {
        *out = a73_saturate(((total as f64) * 0.015625 + tint_scaled as f64) as f32);
    }
    A73Blur {
        rgb,
        alpha: (alpha as f32 as f64 / 255.0) as f32,
    }
}

/// `0x00625ee0` (`FUN_00625ee0`, 439 bytes): weighted 8x8 average with the
/// literal kernel in [`A73_KERNEL_8X8`].
///
/// ```c
/// weights[64] = { 1, 1, ... };                        // A73_KERNEL_8X8
/// x0 = 6 - (int)((float)param_1[4] * (float)param_2 * -0.25);
/// y0 = 5 - (int)((float)param_1[5] * (float)param_3 * -0.25);
/// for (j = 0; j < 8; ++j)
///   for (i = 0; i < 8; ++i) {
///     w = weights[8 * j + i]; total += w;
///     (**(*param_1 + 8))(px, x0 + i, y0 + j);
///     r += channel0(px) * w; g += channel1(px) * w; b += channel2(px) * w;
///   }
/// tint = (float)param_5 / 255.0;
/// r = (float)r / (float)total + tint;                 // total == 128
/// ... saturate ...
/// (**pack)(px, r, g, b, (float)param_4 / 255.0);
/// ```
///
/// The two offsets use different constants (`6` for x, `5` for y) and the
/// multiplier is `-0.25`, so `(int)` truncates a negative product toward
/// zero.  The weight is accumulated as the integer it is and the divisor is
/// the running total, never a constant.
pub fn a73_kernel_blur_8x8(
    field4: i32,
    field5: i32,
    scale_x: i32,
    scale_y: i32,
    alpha: u8,
    tint: u8,
    mut sample: impl FnMut(i32, i32) -> A73Sample,
) -> A73Blur {
    let ox = 6 - a73_cvtt_f64_to_i32((field4 as f32 * scale_x as f32) as f64 * -0.25);
    let oy = 5 - a73_cvtt_f64_to_i32((field5 as f32 * scale_y as f32) as f64 * -0.25);
    let mut accumulator = [0.0f32; 3];
    let mut total = 0i32;
    for row in 0..A73_BLUR_SIDE {
        for column in 0..A73_BLUR_SIDE {
            let weight = A73_KERNEL_8X8[(row * A73_BLUR_SIDE + column) as usize];
            total += weight as i32;
            let channels = sample(ox + column, oy + row);
            let weight = weight as f32;
            for (sum, channel) in accumulator.iter_mut().zip(channels) {
                *sum += channel * weight;
            }
        }
    }
    let divisor = total as f32;
    let tint_scaled = (tint as f32 as f64 / 255.0) as f32;
    let mut rgb = [0.0f32; 3];
    for (out, sum) in rgb.iter_mut().zip(accumulator) {
        *out = a73_saturate(sum / divisor + tint_scaled);
    }
    A73Blur {
        rgb,
        alpha: (alpha as f32 as f64 / 255.0) as f32,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent73 {
    use super::*;

    fn node(next: Option<usize>, key: u32, value: u32) -> A73HashNode {
        A73HashNode { next, key, value }
    }

    #[test]
    fn invalid_position_is_the_minus_one_pair() {
        assert_eq!(a73_invalid_position(), (u32::MAX, u32::MAX));
        assert_eq!(a73_invalid_position_alt(), a73_invalid_position());
    }

    #[test]
    fn close_output_grows_the_used_length_once() {
        let mut buf = A73StreamBuf {
            out_beg: 0x1000,
            out_cur: 0x1006,
            storage: 0x2000,
            used: 2,
            ..A73StreamBuf::default()
        };
        assert_eq!(buf.a73_close_output(), Some(0x2006));
        assert_eq!(buf.used, 6);
        // Already synchronised: no second terminator store.
        assert_eq!(buf.a73_close_output(), None);
        assert_eq!(buf.used, 6);
        assert_eq!(A73StreamBuf::default().a73_sync_output(), 0);
    }

    #[test]
    fn seek_target_uses_the_four_origins() {
        let mut buf = A73StreamBuf {
            in_beg: 0x100,
            in_cur: 0,
            in_end: 0x180,
            out_beg: 0x200,
            out_cur: 0x200,
            out_end: 0x210,
            storage: 0x3000,
            used: 0,
        };
        // which == 0 passes the pair through untouched.
        assert_eq!(
            buf.a73_seek_target(0x1234, 0, 0, 0).plan,
            A73SeekPlan::At(0x1234)
        );
        // which == 1 with bit 8: + (in_end - in_beg) = +0x80.
        assert_eq!(
            buf.a73_seek_target(0x1234, 0, 1, 8).plan,
            A73SeekPlan::At(0x12b4)
        );
        // which == 1 with bit 0x10: + (out_end - out_beg) = +0x10.
        assert_eq!(
            buf.a73_seek_target(0x1234, 0, 1, 0x10).plan,
            A73SeekPlan::At(0x1244)
        );
        // which == 2: + used, zero-extended.
        buf.used = 7;
        assert_eq!(
            buf.a73_seek_target(0x1234, 0, 2, 0).plan,
            A73SeekPlan::At(0x123b)
        );
        // Negative delta sign-extends into the high word.
        buf.in_beg = 0x180;
        buf.in_end = 0x100;
        assert_eq!(buf.a73_seek_target(0, 0, 1, 8).plan, A73SeekPlan::At(-0x80));
    }

    #[test]
    fn seek_target_rejects_the_conflicting_flag_pair() {
        let mut buf = A73StreamBuf::default();
        let result = buf.a73_seek_target(1, 0, 1, 0x18);
        assert_eq!(result.plan, A73SeekPlan::Invalid);
        assert_eq!(result.terminator, None);
        // which outside 0..=2 is invalid, but only after the prologue ran.
        buf.out_cur = 4;
        buf.out_beg = 0;
        buf.used = 0;
        buf.storage = 0x9000;
        let result = buf.a73_seek_target(1, 0, 3, 0);
        assert_eq!(result.plan, A73SeekPlan::Invalid);
        assert_eq!(result.terminator, Some(0x9004));
        // which == 1 with neither direction bit has no defined target.
        assert_eq!(
            A73StreamBuf::default().a73_seek_target(0, 0, 1, 0).plan,
            A73SeekPlan::Undefined
        );
    }

    #[test]
    fn seek_commit_moves_the_selected_cursors() {
        let mut buf = A73StreamBuf {
            in_beg: 0x100,
            in_cur: 0,
            in_end: 0x110,
            out_beg: 0x200,
            out_cur: 0x200,
            out_end: 0x208,
            storage: 0x3000,
            used: 0,
        };
        assert_eq!(buf.a73_seek_commit(4, 1, 2, 3, 0x18), Ok([4, 1, 2, 3]));
        assert_eq!(buf.in_cur, 0x104);
        assert_eq!(buf.out_cur, 0x204);
        // The bound is computed from in_beg, so +0x10 lands on in_end and is
        // still accepted; +0x11 is past it.
        assert_eq!(buf.a73_seek_commit(0x10, 0, 0, 0, 8), Ok([0x10, 0, 0, 0]));
        assert_eq!(buf.in_cur, 0x110);
        assert_eq!(
            buf.a73_seek_commit(0x11, 0, 0, 0, 8),
            Err(A73Fault::PastEnd)
        );
        assert_eq!(buf.in_cur, 0x110);
        // No flags: nothing moves, four words echoed.
        assert_eq!(buf.a73_seek_commit(-1, 5, 6, 7, 0), Ok([u32::MAX, 5, 6, 7]));
        assert_eq!(buf.in_cur, 0x110);
    }

    #[test]
    fn sbumpc_gates_before_reading() {
        let src = [0x41, 0x42];
        let mut cur = 0;
        assert_eq!(a73_sbumpc(&src, &mut cur, 0), 0x41);
        assert_eq!(cur, 1);
        assert_eq!(a73_sbumpc(&src, &mut cur, 1), 0x42);
        assert_eq!(cur, 2);
        // A -1 gate leaves the cursor where it was.
        assert_eq!(a73_sbumpc(&src, &mut cur, -1), -1);
        assert_eq!(cur, 2);
    }

    #[test]
    fn sputn_probe_matches_the_guard_and_the_short_write() {
        // Non-positive count: no call, state untouched.
        assert_eq!(
            a73_sputn_probe(0, 0, 0, 5),
            A73PutnProbe {
                call_virtual: false,
                state: 0
            }
        );
        assert_eq!(
            a73_sputn_probe(0, 1, -1, 5),
            A73PutnProbe {
                call_virtual: false,
                state: 0
            }
        );
        // Already-failed state: bit 0 is set and no call is made.
        assert_eq!(
            a73_sputn_probe(0x20, 4, 0, 4),
            A73PutnProbe {
                call_virtual: false,
                state: 0x21
            }
        );
        // Clean state, exact write: no bit is set.
        assert_eq!(
            a73_sputn_probe(0, 4, 0, 4),
            A73PutnProbe {
                call_virtual: true,
                state: 0
            }
        );
        // Clean state, short write: bit 0.
        assert_eq!(
            a73_sputn_probe(0, 4, 0, 3),
            A73PutnProbe {
                call_virtual: true,
                state: 1
            }
        );
        // The 64-bit count is (hi:lo), so hi == 1 requests 0x1_0000_0000.
        assert_eq!(
            a73_sputn_probe(0, 0, 1, 0x1_0000_0000),
            A73PutnProbe {
                call_virtual: true,
                state: 0
            }
        );
    }

    #[test]
    fn sgetn_stops_at_the_refill_eof() {
        let src = [1u8, 2, 3];
        let mut cur = 0;
        let mut out = [0u8; 5];
        let mut calls = 0;
        let written = a73_sgetn(&src, &mut cur, &mut out, || {
            calls += 1;
            -1
        });
        assert_eq!(written, 3);
        assert_eq!(&out[..3], &[1, 2, 3]);
        assert_eq!(cur, 3);
        assert_eq!(calls, 1);
    }

    #[test]
    fn sgetn_uses_the_refill_byte() {
        let src = [7u8];
        let mut cur = 1;
        let mut out = [0u8; 4];
        let mut calls = 0;
        let written = a73_sgetn(&src, &mut cur, &mut out, || {
            calls += 1;
            if calls == 1 { 0x55 } else { -1 }
        });
        // One refill supplies one byte, then a second refill reports EOF.
        assert_eq!(written, 1);
        assert_eq!(out[0], 0x55);
    }

    #[test]
    fn xsputn_copies_what_fits_after_one_overflow() {
        let src = [b'a', b'b', b'c', b'd'];
        // Room for two bytes, no overflow needed.
        let put = a73_xsputn(&src, 0x100, 0x102, |_| panic!("no overflow"));
        assert_eq!(
            put,
            A73Putn {
                written: 2,
                out_cur: 0x102
            }
        );
        // Full buffer: the first byte goes through overflow, and the buffer is
        // still full afterwards, so nothing more is copied.
        let put = a73_xsputn(&src, 0x100, 0x100, |byte| {
            assert_eq!(byte, b'a');
            0
        });
        assert_eq!(
            put,
            A73Putn {
                written: 1,
                out_cur: 0x100
            }
        );
        // Overflow failure: nothing is accepted.
        let put = a73_xsputn(&src, 0x100, 0x100, |_| -1);
        assert_eq!(
            put,
            A73Putn {
                written: 0,
                out_cur: 0x100
            }
        );
        // Empty source returns immediately.
        let put = a73_xsputn(&[], 0x100, 0x100, |_| panic!("no overflow"));
        assert_eq!(put.written, 0);
    }

    #[test]
    fn capacity_plan_doubles_from_eight() {
        // 100 elements, load 2.0 -> candidate 100 (100 <= 100 * 2), buckets 128.
        let plan = a73_capacity_plan(0, 100, 2).unwrap();
        assert_eq!(plan.truncated_count, 100);
        assert_eq!(plan.stored_load, 2);
        assert_eq!(plan.first, 100);
        assert_eq!(plan.buckets, 128);
        // The candidate wins over a smaller floor.
        assert_eq!(a73_capacity_plan(0, 30, 2).unwrap().first, 30);
        assert_eq!(a73_capacity_plan(0, 30, 2).unwrap().buckets, 32);
        // When the truncated count exceeds truncated * load the candidate is
        // one higher: 100 > 100 * 0.
        assert_eq!(a73_capacity_plan(0, 100, 0).unwrap().first, 101);
        // A bigger floor wins over the candidate.
        assert_eq!(a73_capacity_plan(1000, 0, 1).unwrap().buckets, 1024);
        // Eight or fewer stays at the minimum.
        assert_eq!(a73_capacity_plan(0, 0, 1).unwrap().buckets, 8);
        assert_eq!(a73_capacity_plan(0, 0, 1).unwrap().truncated_count, 0);
    }

    #[test]
    fn capacity_plan_rejects_an_unreachable_bucket_count() {
        assert_eq!(
            a73_capacity_plan(0x8000_0001, 0, 1),
            Err(A73Fault::UnrepresentableBuckets(0x8000_0001))
        );
    }

    #[test]
    fn descriptor_probe_checks_flag_and_versions() {
        assert_eq!(
            a73_descriptor_probe(0x5555, true, 0x14, 4),
            Ok(A73DescriptorProbe {
                word: 1,
                probed: true
            })
        );
        // word 1 clears to a null pointer: the probe is skipped.
        assert_eq!(
            a73_descriptor_probe(1, false, 0x99, 9),
            Ok(A73DescriptorProbe {
                word: 1,
                probed: false
            })
        );
        assert_eq!(
            a73_descriptor_probe(0x5555, false, 0, 0),
            Err(A73Fault::EmptyProbe)
        );
        assert_eq!(
            a73_descriptor_probe(0x5555, true, 0x15, 0),
            Err(A73Fault::MajorTooNew(0x15))
        );
        assert_eq!(
            a73_descriptor_probe(0x5555, true, 0x14, 5),
            Err(A73Fault::MinorTooNew(5))
        );
        // A tagged word resolves through the cleared pointer.
        assert_eq!(
            a73_descriptor_probe(0x5555, true, 0, 0).unwrap().probed,
            true
        );
        assert_eq!(
            a73_descriptor_probe(0x5554, true, 0, 0).unwrap().probed,
            true
        );
        assert_eq!(a73_descriptor_probe(0, true, 0, 0).unwrap().probed, false);
    }

    #[test]
    fn descriptor_copies_clear_the_middle_words() {
        let src = [0x11, 0x22, 0x33, 0x44];
        let take = a73_descriptor_take(src, 0xbeef);
        assert_eq!(take.dst, src);
        assert_eq!(take.src_after, [0x11, 0, 0, 0x44]);
        assert_eq!(take.tag, 0xbeef);
        assert_eq!(a73_descriptor_borrow(src), [0x11, 0, 0x33, 0x44]);
    }

    #[test]
    fn slot_release_resolves_the_tagged_reference() {
        // Out-of-line: low bit set, pointer cleared.
        let reset = a73_slot_release_ref(0x4001, 0x9000);
        assert_eq!(reset.reference, 0x4000);
        assert!(!reset.inline);
        assert_eq!(reset.store, 1);
        // Inline: low bit clear, the inline address itself.
        let reset = a73_slot_release_ref(0x4000, 0x9000);
        assert_eq!(reset.reference, 0x9000);
        assert!(reset.inline);
        assert_eq!(reset.store, 1);
        // The checked form requires a negative length word.
        assert_eq!(
            a73_slot_release_checked(-1, 0x4001, 0x9000)
                .unwrap()
                .reference,
            0x4000
        );
        assert_eq!(
            a73_slot_release_checked(0x10, 0x4001, 0x9000),
            Err(A73Fault::LengthNotSmall)
        );
        // The context form checks both words.
        assert!(a73_context_release(-1, -2, 0x4001, 0x9000).is_ok());
        assert_eq!(
            a73_context_release(-1, 0, 0x4001, 0x9000),
            Err(A73Fault::LengthNotSmall)
        );
        assert_eq!(
            a73_context_release(0, -2, 0x4001, 0x9000),
            Err(A73Fault::LengthNotSmall)
        );
    }

    #[test]
    fn hash_find_walks_the_chain_from_the_link() {
        let nodes = [
            node(Some(1), 0xaaa, 10),
            node(Some(2), 0xbbb, 20),
            node(None, 0xccc, 30),
        ];
        // Four buckets, mask 3: hash 5 -> bucket 1.
        let buckets = [None, Some(0), None, None];
        assert_eq!(
            a73_hash_find(&buckets, &nodes, 5, 0xbbb),
            Ok(A73HashHit { node: 1, value: 20 })
        );
        assert_eq!(
            a73_hash_find(&buckets, &nodes, 5, 0xaaa),
            Ok(A73HashHit { node: 0, value: 10 })
        );
        assert_eq!(
            a73_hash_find(&buckets, &nodes, 5, 0xccc),
            Ok(A73HashHit { node: 2, value: 30 })
        );
        assert_eq!(
            a73_hash_find(&buckets, &nodes, 5, 0xddd),
            Err(A73Fault::NotFound)
        );
        assert_eq!(
            a73_hash_find(&[], &nodes, 5, 0xaaa),
            Err(A73Fault::NoBuckets)
        );
    }

    #[test]
    fn thread_context_ctor_keeps_the_vtable_symbol() {
        let ctor = a73_thread_context_ctor(false);
        assert_eq!(ctor.vftable, "blz::thread::context::vftable");
        assert!(!ctor.deleting);
        assert!(a73_thread_context_ctor(true).deleting);
    }

    #[test]
    fn cache_id_reads_offset_1c() {
        let mut frame = [0u32; 8];
        frame[7] = 0xabc;
        assert_eq!(a73_cache_id(&frame), Some(0xabc));
        assert_eq!(a73_cache_id(&[1, 2]), None);
    }

    #[test]
    fn pixel_fetch_hashes_the_id_and_scales_the_offset() {
        // The MurmurHash3 finaliser on 0x1234, hand-computed below.
        let id = 0x1234u32;
        let mut hash = (id >> 0x10) ^ id;
        hash = hash.wrapping_mul(0x85eb_ca6b);
        hash = (hash >> 0xd) ^ hash;
        hash = hash.wrapping_mul(0xc2b2_ae35);
        let expected = (15u32) & (hash ^ (hash >> 0x10));
        let fetch = a73_pixel_fetch(16, 1, 2, id);
        assert_eq!(fetch.slot, A73PixelSlot::Bucket(expected));
        // (1 + 2 * 4) * 4 bytes.
        assert_eq!(fetch.byte_offset, 36);
        // Negative ids take the shared last-cache object.
        let fetch = a73_pixel_fetch(16, 0, 0, 0xffff_ffff);
        assert_eq!(fetch.slot, A73PixelSlot::FallbackLastCache);
        assert_eq!(fetch.byte_offset, 0);
    }

    #[test]
    fn kernel_table_is_the_shipped_diamond() {
        assert_eq!(A73_KERNEL_8X8.iter().map(|w| *w as u32).sum::<u32>(), 128);
        assert_eq!(A73_KERNEL_8X8[0], 1);
        assert_eq!(A73_KERNEL_8X8[0x1b], 8);
        assert_eq!(A73_KERNEL_8X8[0x24], 8);
        assert_eq!(A73_KERNEL_8X8[0x3f], 1);
    }

    #[test]
    fn box_blur_averages_the_eight_by_eight_block() {
        // Every sample is (0.1, 0.2, 0.3): the mean is the sample itself.
        let blur = a73_box_blur_8x8(4, 4, 2.0, 2.0, 255, 0, |_, _| [0.1, 0.2, 0.3]);
        assert!((blur.rgb[0] - 0.1).abs() < 1e-6);
        assert!((blur.rgb[1] - 0.2).abs() < 1e-6);
        assert!((blur.rgb[2] - 0.3).abs() < 1e-6);
        assert!((blur.alpha - 1.0).abs() < 1e-6);
    }

    #[test]
    fn box_blur_samples_the_offset_block() {
        // field4 * (int)scale_x * 0.25 = 4 * 2 * 0.25 = 2; same for y.
        let mut seen = Vec::new();
        let _ = a73_box_blur_8x8(4, 4, 2.0, 2.0, 0, 0, |x, y| {
            seen.push((x, y));
            [0.0, 0.0, 0.0]
        });
        assert_eq!(seen.len(), 64);
        assert_eq!(seen[0], (2, 2));
        assert_eq!(seen[63], (2 + 7, 2 + 7));
    }

    #[test]
    fn box_blur_saturates_and_adds_the_tint() {
        // Full white plus a 255 tint saturates at 1.0 on every channel.
        let blur = a73_box_blur_8x8(0, 0, 1.0, 1.0, 0, 255, |_, _| [1.0, 1.0, 1.0]);
        assert_eq!(blur.rgb, [1.0, 1.0, 1.0]);
        // A zero block with a 51 tint is 0.2 on every channel.
        let blur = a73_box_blur_8x8(0, 0, 1.0, 1.0, 0, 51, |_, _| [0.0, 0.0, 0.0]);
        for channel in blur.rgb {
            assert!((channel - 0.2).abs() < 1e-6);
        }
        assert_eq!(blur.alpha, 0.0);
    }

    #[test]
    fn kernel_blur_weights_the_diamond() {
        // A constant block: the weighted mean equals the constant, since the
        // weights sum to 128 and each contributes weight * channel.
        let blur = a73_kernel_blur_8x8(4, 4, 2, 2, 255, 0, |_, _| [1.0, 0.5, 0.25]);
        assert!((blur.rgb[0] - 1.0).abs() < 1e-6);
        assert!((blur.rgb[1] - 0.5).abs() < 1e-6);
        assert!((blur.rgb[2] - 0.25).abs() < 1e-6);
        assert!((blur.alpha - 1.0).abs() < 1e-6);
    }

    #[test]
    fn kernel_blur_offsets_use_six_and_five() {
        // x0 = 6 - (int)(4 * 2 * -0.25) = 6 - (-2) = 8;
        // y0 = 5 - (int)(4 * 2 * -0.25) = 5 - (-2) = 7.
        let mut seen = Vec::new();
        let _ = a73_kernel_blur_8x8(4, 4, 2, 2, 0, 0, |x, y| {
            seen.push((x, y));
            [0.0, 0.0, 0.0]
        });
        assert_eq!(seen.len(), 64);
        assert_eq!(seen[0], (8, 7));
        assert_eq!(seen[63], (15, 14));
    }

    #[test]
    fn kernel_blur_ignores_negative_products() {
        // A negative stride truncates toward zero: (int)(-4 * 2 * -0.25) = 2.
        let mut seen = Vec::new();
        let _ = a73_kernel_blur_8x8(-4, -4, 2, 2, 0, 0, |x, y| {
            seen.push((x, y));
            [0.0, 0.0, 0.0]
        });
        assert_eq!(seen[0], (4, 3));
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 46 `decompiled_uncited` rows plus the one thunk that are not translated
// above, grouped by why:
//
// * `FUN_00624376`: three-byte tail jump into the `memcpy` at `FUN_00c89580`;
//   there is no local work in the row.
// * `FUN_006244c6`, `FUN_00624fbe`, `FUN_0062509f`, `FUN_006259ed`, `FUN_00625acf`,
//   `FUN_00625ba9`, `FUN_00625edc`: bodies that begin mid-instruction
//   (overlapping-instruction warnings, `swi(3)`/`swi(4)`, `halt_baddata`, byte
//   arithmetic on a string literal and a store through `unaff_EBX * 4`).
//   The decompiler lost the instruction boundary, so no behaviour is
//   recoverable.
// * `FUN_0062466c`, `FUN_00624815`, `FUN_0062486f`, `FUN_00624895`, `FUN_006248ec`,
//   `FUN_00624906`, `FUN_00624937`, `FUN_0062495c`, `FUN_006249b5`, `FUN_00624a35`,
//   `FUN_00624a75`, `FUN_00624a8e`, `FUN_00624ae9`, `FUN_00624b3d`, `FUN_00624b83`,
//   `FUN_00624c78`, `FUN_00624cbe`, `FUN_00624d5d`, `FUN_00624e19`, `FUN_00624e46`,
//   `FUN_00624e6d`: unwind fragments of one `0x48`-byte-element block copy loop.
//   They address every value through `unaff_EBP`/`unaff_EBX`/`unaff_EDI`
//   stack slots belonging to a caller outside the batch, and their visible
//   work is the `FUN_0093ec70` unwind calls, the `FUN_00597e60` vector grow and
//   the same four-by-four pixel fill.  The repeated pure pieces they do show
//   fully - the Murmur finaliser bucket mask and the `cap + cap / 2` growth
//   with the `0x10` floor - are already reproduced by `0x00625c70` and
//   `0x00624430` respectively.
// * `FUN_006243b0`: destructor loop over `0x0c`-byte elements whose body is
//   `while (iVar1 != 0) iVar1 = *(int *)(param_1 + 4);` - the decompiler lost
//   the loop's exit, so the walk cannot be reproduced.
// * `0x006245f0`'s neighbours `FUN_006246c0` and `FUN_00625edc` overlap the blur
//   bodies; `FUN_006246c0` (421 bytes) is the `PixelCache` constructor whose
//   recovered text is `POPCOUNT`/`in_AF`/`extraout_CX` fragments around
//   `func_0x0063802d` and the global hash table at `0x6a08454d`.  Its only
//   complete stores are the seven-word header reproduced by the `A73StreamBuf`
//   comment above; the rest is not recoverable.
// * `FUN_006250b1`, `FUN_006250c2`, `0x00624e90`-adjacent copies: `FUN_006250b1`
//   and `FUN_006250c2` copy two fields (`+0x60`, `+0x64`) and call
//   `0x00624430`, which is translated, but their own bodies address
//   `unaff_EBP`/`unaff_ESI`/`unaff_EDI` slots and their argument lists are
//   incomplete.
// * `FUN_00625110`: eight-byte tail jump into `0x00625300`.
// * `FUN_00625120`, `FUN_00625240`, `FUN_00625250`, `FUN_006252c0`, `0x00625300`'s
//   sibling `FUN_00625200`: container release paths whose every completed
//   branch enters the noreturn helper at `FUN_0093ec70`; the only arithmetic is
//   the element stride (`0x0c` bytes at `FUN_00625250`) and the `+0x0c` length
//   word check, which is already covered by `a73_slot_release_checked`.
// * `FUN_00625362`: the same tagged-slot release as `0x00625399` without the
//   reference computation happening before the virtual call; the two rows
//   collapse to one translation.
// * `FUN_006254a9`: continues `0x006253e0`'s node construction with
//   `unaff_EBX`/`unaff_ESI`/`unaff_EDI` slots; the recovered lines are the
//   `PixelCache::vftable` word, the `+0x20`/`+0x24` growth ratio and a call to
//   `FUN_0053cdd0`.
// * `FUN_0062557f` (`__break`): a single `int3`.
// * `FUN_00625580`, `FUN_00625880`: build the `FUN_00538810` / `FUN_0053a5b0`
//   noreturn argument and enter it; the rows carry no other state change.
// * `FUN_00625aea` (`thunk_FUN_00625aff`) and `FUN_00625aff`: unwind-cleanup
//   fragment that stores `0x8000000f` into a stack slot and enters
//   `FUN_00538810`, again through `unaff_EBP` slots.
// * `FUN_00625eb7`: the last two calls of the blur bodies (`FUN_00a8fbe0`,
//   `FUN_00a902b0`) with no argument list in the row.
