#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent49.rs
// ===========================================================================
// Translation batch 49.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005f4000..=0x005f5fff` are the
// launcher/runtime's TCLAP argument parser and its MSVC C++ library
// support, not simulation logic.  The region contains the destructor
// chains of the `args::` argument objects, the single-character stream
// sentry, the string construction helper, the locale facet teardown and a
// family of 12-byte vector-triple initialisers.  None of that touches
// `World` or `SimUnit`, so this batch is written as pure free functions and
// adds no fields to the simulation records.
//
// Two retail layouts recur:
//
// * An MSVC `std::string` is 24 bytes.  The union at `+0x00` is either the
//   16-byte inline buffer or a heap pointer, `size` is at `+0x10` and
//   `capacity` at `+0x14`.  A capacity below `0x10` selects the inline
//   buffer, which is what [`A49String`] records with its `heap` flag.
// * A `std::vector` is the usual three dwords (`begin`, `end`,
//   `capacity`), modelled with [`Vec`] where the element type matters and
//   with `[u32; 3]` where only the triple initialisation is in range.
//
// The decompiler split several destructors at their deallocation calls and
// several larger routines at exception landing pads, so the doc comment on
// each function names the exact ledger address and the retail string-member
// offsets it releases.  Deallocation is reported in the return value
// instead of being modelled as an allocator, and vtable writes (which carry
// no data in this translation) are documented but not represented.

/// `std::string` capacity written by the inline clear (`+0x14 = 0xf`).
const A49_SSO_CAPACITY: u32 = 0xf;

/// Capacities below this select the 16-byte inline buffer (`+0x14 < 0x10`).
const A49_SSO_LIMIT: u32 = 0x10;

/// Observable header of one 24-byte MSVC `std::string` member.
///
/// The retail object also holds the 16-byte inline buffer or heap pointer in
/// its `+0x00` union.  The translation only needs to know which storage the
/// header selects, so that union is represented by `heap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A49String {
    /// Retail `+0x10`: number of stored bytes.
    pub len: u32,
    /// Retail `+0x14`: allocation size; below `0x10` means inline storage.
    pub cap: u32,
    /// Retail `+0x00`: `true` when the union points at a heap block.
    pub heap: bool,
}

/// A fresh inline string: `size = 0`, `capacity = 0xf`, `buf[0] = 0`.
pub const fn a49_empty_string() -> A49String {
    A49String {
        len: 0,
        cap: A49_SSO_CAPACITY,
        heap: false,
    }
}

/// The three stores of the inline branch: `+0x10 = 0`, `+0x14 = 0xf`,
/// `+0x00 = 0`.
fn a49_clear_inline(slot: &mut A49String) {
    slot.len = 0;
    slot.cap = A49_SSO_CAPACITY;
    slot.heap = false;
}

/// One `std::string` destructor step.
///
/// A capacity below `0x10` takes the inline branch and performs the three
/// stores above.  A larger capacity releases the heap block and leaves the
/// `size`/`capacity` header alone, exactly as the retail destructor does;
/// the enclosing object is dead after this call.  The boolean reports which
/// branch ran (and therefore whether a block was released).
fn a49_release_string(slot: &mut A49String) -> bool {
    if slot.cap < A49_SSO_LIMIT {
        a49_clear_inline(slot);
        false
    } else {
        slot.heap = false;
        true
    }
}

/// Release a sequence of string members in the order given.  The retail
/// destructor chains walk from the highest member offset down to the lowest,
/// which is the order the callers below pass their arguments in.
fn a49_release_chain(slots: &mut [&mut A49String]) -> usize {
    let mut freed = 0;
    for slot in slots.iter_mut() {
        if a49_release_string(slot) {
            freed += 1;
        }
    }
    freed
}

/// Destroy a `std::vector<std::string>` member.
///
/// Retail destroys every element and then releases the three-dword buffer
/// when `begin` is non-null.  A non-zero [`Vec::capacity`] is the model of a
/// non-null `begin`.  Returns the number of released blocks (element string
/// blocks plus the vector buffer).
fn a49_release_string_vector(vector: &mut Vec<A49String>) -> usize {
    let mut freed = 0;
    for slot in vector.iter_mut() {
        if a49_release_string(slot) {
            freed += 1;
        }
    }
    let buffer = vector.capacity() > 0;
    *vector = Vec::new();
    freed + usize::from(buffer)
}

/// `0x005f4022`: tail of the `args::` destructor chain.
///
/// The recovered block clears the four `std::string` members at `+0x6c`,
/// `+0x54`, `+0x3c` and `+0x24`, highest offset first.  Every member takes
/// the usual inline-or-heap destructor branch: `capacity < 0x10` writes
/// `size = 0`, `capacity = 0xf`, `buf[0] = 0`; a larger capacity only
/// releases the heap block.  Returns the number of heap blocks released.
pub fn a49_args_destroy_four(
    at_6c: &mut A49String,
    at_54: &mut A49String,
    at_3c: &mut A49String,
    at_24: &mut A49String,
) -> usize {
    a49_release_chain(&mut [at_6c, at_54, at_3c, at_24])
}

/// `0x005f406b`: tail of the same destructor chain after the `+0x6c` member
/// has already been released.
///
/// Clears the members at `+0x54`, `+0x3c` and `+0x24` in that order, using
/// the same inline-clear / heap-release branch as `0x005f4022`.
pub fn a49_args_destroy_three(
    at_54: &mut A49String,
    at_3c: &mut A49String,
    at_24: &mut A49String,
) -> usize {
    a49_release_chain(&mut [at_54, at_3c, at_24])
}

/// `0x005f40ad`: tail of the destructor chain after the `+0x6c` and `+0x54`
/// members have been released.
///
/// Clears the members at `+0x3c` and `+0x24`, in that order.
pub fn a49_args_destroy_two(at_3c: &mut A49String, at_24: &mut A49String) -> usize {
    a49_release_chain(&mut [at_3c, at_24])
}

/// `0x005f40ef`: tail of the destructor chain after the `+0x6c`, `+0x54`
/// and `+0x3c` members have been released.
///
/// Clears the single `std::string` member at `+0x24`: `size = 0`,
/// `capacity = 0xf`, `buf[0] = 0` for inline storage, otherwise release the
/// heap block.  Returns `true` when a heap block was released.
pub fn a49_args_destroy_one(at_24: &mut A49String) -> bool {
    a49_release_string(at_24)
}

/// `0x005f4130`: `args::NamedBase` destructor.
///
/// The body stores the `args::NamedBase` vtable, destroys the
/// `std::vector<std::string>` at `+0x5c`, then clears the string members at
/// `+0x40`, `+0x24` and `+0x0c` in that order.  The vtable is rewritten to
/// `args::Base` before the `+0x0c` member; vtable stores are not part of
/// this model.  Returns the number of released blocks (the vector buffer,
/// its element strings and the three members).
pub fn a49_named_base_destroy(
    at_5c: &mut Vec<A49String>,
    at_40: &mut A49String,
    at_24: &mut A49String,
    at_0c: &mut A49String,
) -> usize {
    let mut freed = a49_release_string_vector(at_5c);
    freed += usize::from(a49_release_string(at_40));
    freed += usize::from(a49_release_string(at_24));
    freed += usize::from(a49_release_string(at_0c));
    freed
}

/// `0x005f4172`: destructor tail after the `+0x5c` vector and the `+0x40`
/// heap block have been released.
///
/// Clears the members at `+0x40`, `+0x24` and `+0x0c`, in that order.  The
/// `args::Base` vtable is written between the second and third members,
/// which has no data effect here.
pub fn a49_args_destroy_40_24_0c(
    at_40: &mut A49String,
    at_24: &mut A49String,
    at_0c: &mut A49String,
) -> usize {
    a49_release_chain(&mut [at_40, at_24, at_0c])
}

/// `0x005f41b4`: destructor tail after the `+0x40` member has been released.
///
/// Clears the members at `+0x24` and `+0x0c`, in that order, with the
/// `args::Base` vtable store between them.
pub fn a49_args_destroy_24_0c(at_24: &mut A49String, at_0c: &mut A49String) -> usize {
    a49_release_chain(&mut [at_24, at_0c])
}

/// `0x005f41fc`: destructor tail after the `+0x24` member has been released.
///
/// Clears the single `std::string` member at `+0x0c`.  Returns `true` when
/// that member owned a heap block.
pub fn a49_args_destroy_0c(at_0c: &mut A49String) -> bool {
    a49_release_string(at_0c)
}

/// `0x005f4480`: move-assign a three-dword vector.
///
/// Retail compares the two object addresses first and does nothing when they
/// are equal.  Otherwise it destroys the destination vector (releasing its
/// buffer and zeroing its triple), copies the three dwords from the source
/// and zeroes the source triple.  The `Vec` model performs the same
/// transition: the destination's old buffer is dropped, the source is moved
/// in and the source is left empty.  A slice of `Vec`s stands in for a set
/// of distinct vector objects so the self-address test is just `dest == src`.
/// Returns the destination index.
///
/// # Panics
///
/// Panics when an index is out of range; retail would treat that as an
/// invalid object rather than a defined branch.
pub fn a49_vector_move_assign<T>(vectors: &mut [Vec<T>], dest: usize, src: usize) -> usize {
    if dest != src {
        let moved = std::mem::take(&mut vectors[src]);
        vectors[dest] = moved;
    }
    dest
}

/// One streambuf get area: the bytes between `gptr` and `egptr` plus the
/// current `gptr` offset.
///
/// Retail's two empty cases - a null `gptr` and a non-positive
/// `egptr - gptr` count - both collapse to an empty remaining slice here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A49GetArea<'a> {
    /// Bytes available from the start of the get area.
    pub bytes: &'a [u8],
    /// How many bytes have already been consumed from `bytes`.
    pub pos: usize,
}

impl A49GetArea<'_> {
    /// The bytes from `gptr` to `egptr`.
    fn a49_remaining(&self) -> &[u8] {
        &self.bytes[self.pos.min(self.bytes.len())..]
    }
}

/// The one-byte stream sentry used by the two extraction helpers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A49Sentry<'a> {
    /// Retail `+0x00`: the get area, cleared when the sentry fails.
    pub area: Option<A49GetArea<'a>>,
    /// Retail `+0x04`: the cached-character flag.
    pub cached: bool,
    /// Retail `+0x05`: the cached character.
    pub value: u8,
}

/// `0x005f4750`: peek one character through a stream sentry.
///
/// The retail body:
///
/// * when the byte at `+0x04` is set, returns the zero-extended cached byte
///   at `+0x05`;
/// * otherwise, when the area pointer at `+0x00` is non-null and the get
///   count is positive, reads one byte through `gptr`;
/// * when the area exists but is empty, calls the virtual `underflow` slot
///   (modelled by the caller-supplied closure);
/// * when the result is not `0xffffffff`, stores its low byte at `+0x05`,
///   sets the flag and returns the full 32-bit result;
/// * otherwise clears the area pointer, sets the flag and returns the stale
///   cached byte.
///
/// The closure is only called when the area exists but has no bytes left;
/// a null area takes the failure path without calling `underflow`.
pub fn a49_sentry_sgetc(sentry: &mut A49Sentry<'_>, underflow: impl FnOnce() -> i32) -> i32 {
    if !sentry.cached {
        if let Some(area) = sentry.area.as_ref() {
            let value = if area.a49_remaining().is_empty() {
                underflow()
            } else {
                i32::from(area.a49_remaining()[0])
            };
            if value != -1 {
                sentry.value = value as u8;
                sentry.cached = true;
                return value;
            }
        }
        sentry.area = None;
        sentry.cached = true;
    }
    i32::from(sentry.value)
}

/// `0x005f47a0`: consume one character through a stream sentry.
///
/// The retail body:
///
/// * when the area pointer at `+0x00` is null, clears the pointer (already
///   null), sets the flag at `+0x04` and returns;
/// * when `gptr` is null or the count is not positive, calls the virtual
///   `underflow` slot at vtable `+0x1c` (modelled by the closure);
/// * otherwise decrements the count, advances `gptr`, and reads the byte;
/// * on any result other than `0xffffffff`, clears the flag and returns;
/// * on `0xffffffff`, clears the area pointer, sets the flag and returns.
///
/// Retail returns the sentry object itself and discards the character; this
/// translation returns `Some(character)` so the caller can observe which
/// branch ran, and `None` for both failure paths.
pub fn a49_sentry_sbumpc(
    sentry: &mut A49Sentry<'_>,
    underflow: impl FnOnce() -> i32,
) -> Option<i32> {
    if let Some(area) = sentry.area.as_mut() {
        if !area.a49_remaining().is_empty() {
            let value = i32::from(area.a49_remaining()[0]);
            area.pos += 1;
            sentry.cached = false;
            return Some(value);
        }
        let value = underflow();
        if value != -1 {
            sentry.cached = false;
            return Some(value);
        }
    } else {
        sentry.cached = true;
        return None;
    }
    sentry.area = None;
    sentry.cached = true;
    None
}

/// `0x005f47f0`: construct `count` empty inline strings.
///
/// The retail loop writes `+0x10 = 0`, `+0x14 = 0xf` and `+0x00 = 0`, then
/// advances the destination by `0x18` for every iteration and returns the
/// one-past-the-end pointer.  The trailing call to the container finaliser
/// is outside this batch and is not modelled.  The returned [`Vec`] holds
/// one [`A49String`] per iteration in the same order.
pub fn a49_fill_empty_strings(count: u32) -> Vec<A49String> {
    vec![a49_empty_string(); count as usize]
}

/// Storage selected by a `std::string` header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A49StringStorage {
    /// `capacity < 0x10`: the 16-byte inline buffer at `+0x00`.
    Inline,
    /// `capacity >= 0x10`: the heap pointer at `+0x00`.
    Heap,
}

/// `0x005f4830`: select a string's data pointer and size.
///
/// The routine loads `size` from `+0x10`, compares `capacity` at `+0x14`
/// against `0x10` and, when the capacity is at least that large, reloads the
/// data pointer from the heap pointer at `+0x00`; otherwise the pointer is
/// the string object itself (the inline buffer).  It then forwards
/// `(pointer, size)` to its consumer.  The translation returns the selected
/// storage and the size it forwards.
pub fn a49_string_storage(slot: &A49String) -> (A49StringStorage, u32) {
    let storage = if slot.cap < A49_SSO_LIMIT {
        A49StringStorage::Inline
    } else {
        A49StringStorage::Heap
    };
    (storage, slot.len)
}

/// The three pointer fields released by the `std::numpunct<char>`
/// destructor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A49Numpunct {
    /// Retail `+0x08`: the decimal-point string block.
    pub decimal_point: Option<Vec<u8>>,
    /// Retail `+0x10`: the thousands-separator string block.
    pub thousands_sep: Option<Vec<u8>>,
    /// Retail `+0x14`: the grouping string block.
    pub grouping: Option<Vec<u8>>,
}

/// `0x005f4d20`: `std::numpunct<char>` deleting destructor.
///
/// Stores the numpunct vtable, releases the blocks at `+0x08`, `+0x10` and
/// `+0x14` in that order, then rewrites the `_Facet_base` vtable and, when
/// the low bit of the argument is set, releases the 0x18-byte object itself.
/// The three field releases are unconditional in retail (a null pointer is a
/// harmless deallocation); the returned count reports how many of the model
/// slots actually held a block.
pub fn a49_numpunct_destroy(numpunct: &mut A49Numpunct) -> usize {
    let mut freed = 0;
    for slot in [
        &mut numpunct.decimal_point,
        &mut numpunct.thousands_sep,
        &mut numpunct.grouping,
    ] {
        if slot.take().is_some() {
            freed += 1;
        }
    }
    freed
}

/// Observable result of the `std::ctype<char>` destructor at `0x005f4c90`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A49CtypeRelease {
    /// Whether the table pointer at `+0x0c` was released.
    pub table: bool,
    /// Whether the second pointer at `+0x14` was released.
    pub aux: bool,
    /// Whether the deleting-destructor flag requested `this` itself.
    pub self_deleted: bool,
}

/// `0x005f4c90`: `std::ctype<char>` deleting destructor.
///
/// Stores the ctype vtable and inspects the signed selector at `+0x10`:
/// a positive value releases the table pointer at `+0x0c` through the CRT
/// free path, a negative value releases it through the operator-delete path,
/// and zero skips the table.  The pointer at `+0x14` is released
/// unconditionally, then the `_Facet_base` vtable is stored and the low bit
/// of the argument controls deletion of the 0x18-byte object.
pub fn a49_ctype_destroy(selector: i32, delete_this: bool) -> A49CtypeRelease {
    A49CtypeRelease {
        table: selector != 0,
        aux: true,
        self_deleted: delete_this,
    }
}

/// `0x005f4cf0`: `std::_Facet_base` deleting destructor.
///
/// The body only stores the base vtable and, when the low bit of the
/// argument is set, releases the 8-byte facet object.  The translation
/// reports that deletion decision.  The identical row at `FUN_005f4f00` is the
/// same base deleting destructor and is not duplicated here.
pub fn a49_facet_destroy(delete_this: bool) -> bool {
    delete_this
}

/// `0x005f4da0`: `args::Base` deleting destructor.
///
/// Stores the `args::Base` vtable, releases the `std::string` member at
/// `+0x0c` (inline clear or heap release) and, when the low bit of the
/// argument is set, releases the 0x24-byte object.  Returns
/// `(member_block_released, self_deleted)`.
pub fn a49_args_base_destroy(at_0c: &mut A49String, delete_this: bool) -> (bool, bool) {
    (a49_release_string(at_0c), delete_this)
}

/// The two dwords of the exception message payload (`std::exception`'s
/// `+0x04`/`+0x08` pair).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A49ExceptionMessage {
    /// Retail `+0x04`: the message pointer, null when the slot is empty.
    pub text: Option<Vec<u8>>,
    /// Retail `+0x08`: the `_DoFree` byte; only a set byte frees `text`.
    pub owned: bool,
}

/// `0x005f4f30`: `std::exception` deleting destructor.
///
/// Stores the exception vtable and invokes the message deallocator on the
/// `+0x04` pair: when the `_DoFree` byte at `+0x08` is set the pointer at
/// `+0x04` is released, and in both cases the pointer is nulled and the flag
/// cleared.  When the low bit of the argument is set the 0x14-byte object is
/// then released.  Returns `(message_block_released, self_deleted)`.
pub fn a49_exception_destroy(message: &mut A49ExceptionMessage, delete_this: bool) -> (bool, bool) {
    let text = message.text.take();
    let freed = message.owned && text.is_some();
    message.owned = false;
    (freed, delete_this)
}

/// The owned auxiliary block of a `std::basic_streambuf` (`+0x34`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A49OwnedFacet {
    /// `+0x04` of the owned block: whether a tracked sub-object exists.
    pub subobject: bool,
    /// Whether the virtual `+0x08` accessor returned a non-null object.
    pub accessor_hit: bool,
}

/// Observable result of `0x005f4bd0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A49StreambufRelease {
    /// Whether the sub-object's deleting virtual call ran.
    pub subobject_deleted: bool,
    /// Whether the 8-byte owned block was released.
    pub owned_block: bool,
    /// Whether the deleting-destructor flag requested `this` itself.
    pub self_deleted: bool,
}

/// `0x005f4bd0`: `std::basic_streambuf` deleting destructor.
///
/// Stores the streambuf vtable and, when the pointer at `+0x34` is non-null,
/// inspects its `+0x04` member.  If that member is non-null the virtual
/// accessor at `+0x08` is called and, when it returns non-null, the returned
/// object's deleting virtual slot is invoked with argument 1.  The 8-byte
/// owned block is released afterwards.  The low bit of the argument then
/// controls deletion of the 0x38-byte streambuf.
pub fn a49_streambuf_destroy(
    owned: Option<A49OwnedFacet>,
    delete_this: bool,
) -> A49StreambufRelease {
    match owned {
        Some(facet) => A49StreambufRelease {
            subobject_deleted: facet.subobject && facet.accessor_hit,
            owned_block: true,
            self_deleted: delete_this,
        },
        None => A49StreambufRelease {
            subobject_deleted: false,
            owned_block: false,
            self_deleted: delete_this,
        },
    }
}

/// `0x005f51c0`: zero a three-dword vector header.
///
/// The body writes `0` to `+0x00`, `+0x04` and `+0x08` and returns.  The
/// caller passes the destination pointer as the single stack argument
/// (`ret 4`).
pub fn a49_vector_triple_init(triple: &mut [u32; 3]) {
    *triple = [0; 3];
}

/// `0x005f5570`: zero a three-dword vector header, second site.
///
/// The body is identical to `0x005f51c0` but the frame returns with `ret 8`,
/// so the retail caller also popped an additional unused argument.  Only the
/// three zero stores are observable.
pub fn a49_vector_triple_init_5570(triple: &mut [u32; 3]) {
    *triple = [0; 3];
}

#[cfg(test)]
mod tests_agent49 {
    use super::*;

    /// `0x005f4022`: the four-member chain releases the heap members at
    /// `+0x6c` and `+0x3c`, clears the inline members at `+0x54` and
    /// `+0x24`, and leaves a released heap header untouched apart from the
    /// storage flag.
    #[test]
    fn four_string_destructor_releases_from_the_top() {
        let mut at_6c = A49String {
            len: 3,
            cap: 0x20,
            heap: true,
        };
        let mut at_54 = A49String {
            len: 5,
            cap: 0x0f,
            heap: false,
        };
        let mut at_3c = A49String {
            len: 2,
            cap: 0x10,
            heap: true,
        };
        let mut at_24 = A49String {
            len: 9,
            cap: 0x0f,
            heap: false,
        };

        assert_eq!(
            a49_args_destroy_four(&mut at_6c, &mut at_54, &mut at_3c, &mut at_24),
            2
        );
        assert_eq!(
            at_6c,
            A49String {
                len: 3,
                cap: 0x20,
                heap: false
            }
        );
        assert_eq!(at_54, a49_empty_string());
        assert_eq!(
            at_3c,
            A49String {
                len: 2,
                cap: 0x10,
                heap: false
            }
        );
        assert_eq!(at_24, a49_empty_string());
    }

    /// `0x005f406b`: the three-member tail starts at `+0x54` and ends at
    /// `+0x24`; only the inline members are rewritten.
    #[test]
    fn three_string_destructor_clears_inline_members() {
        let mut at_54 = A49String {
            len: 8,
            cap: 0x20,
            heap: true,
        };
        let mut at_3c = A49String {
            len: 7,
            cap: 0x0f,
            heap: false,
        };
        let mut at_24 = A49String {
            len: 1,
            cap: 0x0f,
            heap: false,
        };

        assert_eq!(
            a49_args_destroy_three(&mut at_54, &mut at_3c, &mut at_24),
            1
        );
        assert_eq!(
            at_54,
            A49String {
                len: 8,
                cap: 0x20,
                heap: false
            }
        );
        assert_eq!(at_3c, a49_empty_string());
        assert_eq!(at_24, a49_empty_string());
    }

    /// `0x005f40ad`, `0x005f40ef`: the two- and one-member tails use the
    /// same branch, and the returned flag is exactly the capacity test
    /// `capacity >= 0x10`.
    #[test]
    fn short_destructor_tails_report_the_heap_branch() {
        let mut at_3c = A49String {
            len: 4,
            cap: 0x0f,
            heap: false,
        };
        let mut at_24 = A49String {
            len: 6,
            cap: 0x18,
            heap: true,
        };
        assert_eq!(a49_args_destroy_two(&mut at_3c, &mut at_24), 1);
        assert_eq!(at_3c, a49_empty_string());
        assert!(!at_24.heap);
        assert_eq!(at_24.len, 6);
        assert_eq!(at_24.cap, 0x18);

        let mut single = A49String {
            len: 2,
            cap: 0x10,
            heap: true,
        };
        assert!(a49_args_destroy_one(&mut single));
        assert!(!single.heap);
        // The heap branch leaves the header untouched, so the model still
        // shows the old size and capacity on the dead object.
        assert_eq!(single.len, 2);
        assert_eq!(single.cap, 0x10);

        let mut inline = A49String {
            len: 2,
            cap: 0x0f,
            heap: false,
        };
        assert!(!a49_args_destroy_one(&mut inline));
        assert_eq!(inline, a49_empty_string());
    }

    /// `0x005f4130`: the named-base destructor releases the vector at
    /// `+0x5c` first, then the three string members at `+0x40`, `+0x24`
    /// and `+0x0c`.
    #[test]
    fn named_base_destructor_releases_vector_then_strings() {
        let mut vector = vec![
            A49String {
                len: 1,
                cap: 0x0f,
                heap: false,
            },
            A49String {
                len: 2,
                cap: 0x20,
                heap: true,
            },
        ];
        let mut at_40 = A49String {
            len: 1,
            cap: 0x20,
            heap: true,
        };
        let mut at_24 = a49_empty_string();
        let mut at_0c = A49String {
            len: 4,
            cap: 0x30,
            heap: true,
        };

        // Vector buffer + one heap element + the +0x40 and +0x0c members.
        assert_eq!(
            a49_named_base_destroy(&mut vector, &mut at_40, &mut at_24, &mut at_0c),
            4
        );
        assert!(vector.is_empty());
        assert!(!at_40.heap);
        assert_eq!(at_24, a49_empty_string());
        assert!(!at_0c.heap);
    }

    /// `0x005f4172`, `0x005f41b4` and `0x005f41fc`: the remaining tails
    /// cover the `+0x40`/`+0x24`/`+0x0c` group down to the final `+0x0c`
    /// member.
    #[test]
    fn remaining_destructor_tails_cover_the_low_members() {
        let mut a = A49String {
            len: 1,
            cap: 0x20,
            heap: true,
        };
        let mut b = A49String {
            len: 2,
            cap: 0x0f,
            heap: false,
        };
        let mut c = A49String {
            len: 3,
            cap: 0x0f,
            heap: false,
        };
        assert_eq!(a49_args_destroy_40_24_0c(&mut a, &mut b, &mut c), 1);
        assert_eq!(b, a49_empty_string());
        assert_eq!(c, a49_empty_string());

        let mut d = A49String {
            len: 4,
            cap: 0x18,
            heap: true,
        };
        let mut e = A49String {
            len: 5,
            cap: 0x0f,
            heap: false,
        };
        assert_eq!(a49_args_destroy_24_0c(&mut d, &mut e), 1);
        assert!(!d.heap);
        assert_eq!(e, a49_empty_string());

        let mut f = A49String {
            len: 6,
            cap: 0x24,
            heap: true,
        };
        assert!(a49_args_destroy_0c(&mut f));
        assert!(!f.heap);
    }

    /// `0x005f4480`: moving a vector steals the three pointers, zeroes the
    /// source, and the self-address test makes `dest == src` a no-op.
    #[test]
    fn vector_move_assign_steals_and_zeroes_the_source() {
        let mut vectors = vec![vec![1_u32, 2, 3], vec![4, 5]];
        assert_eq!(a49_vector_move_assign(&mut vectors, 0, 1), 0);
        assert_eq!(vectors[0], vec![4, 5]);
        assert!(vectors[1].is_empty());

        assert_eq!(a49_vector_move_assign(&mut vectors, 0, 0), 0);
        assert_eq!(vectors[0], vec![4, 5]);
        assert!(vectors[1].is_empty());
    }

    /// `0x005f4750`: the cached flag short-circuits, a non-empty get area
    /// returns `*gptr` without advancing, and `-1` from `underflow` clears
    /// the area and returns the stale cached byte.
    #[test]
    fn sentry_peek_uses_the_cache_and_the_get_area() {
        let mut sentry = A49Sentry {
            area: Some(A49GetArea {
                bytes: b"ab",
                pos: 0,
            }),
            cached: false,
            value: 0,
        };
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            a49_sentry_sgetc(&mut sentry, || {
                calls.set(calls.get() + 1);
                -1
            }),
            i32::from(b'a')
        );
        assert_eq!(calls.get(), 0);
        assert!(sentry.cached);
        assert_eq!(sentry.value, b'a');
        // The peek does not advance the get pointer.
        assert_eq!(sentry.area.as_ref().unwrap().pos, 0);

        // Cached calls do not consult the area or the underflow slot.
        assert_eq!(
            a49_sentry_sgetc(&mut sentry, || panic!("must not run")),
            i32::from(b'a')
        );

        let mut empty = A49Sentry {
            area: Some(A49GetArea { bytes: b"", pos: 0 }),
            cached: false,
            value: 0x41,
        };
        // The low byte is cached but the full underflow result is returned.
        assert_eq!(a49_sentry_sgetc(&mut empty, || 0x1234), 0x1234);
        assert_eq!(empty.value, 0x34);
        assert!(empty.cached);

        let mut failed = A49Sentry {
            area: Some(A49GetArea { bytes: b"", pos: 0 }),
            cached: false,
            value: 0x41,
        };
        assert_eq!(a49_sentry_sgetc(&mut failed, || -1), 0x41);
        assert!(failed.area.is_none());
        assert!(failed.cached);
    }

    /// `0x005f4750`: a null area takes the failure path without invoking the
    /// virtual underflow slot.
    #[test]
    fn sentry_peek_without_an_area_skips_underflow() {
        let mut sentry = A49Sentry {
            area: None,
            cached: false,
            value: 0x5a,
        };
        assert_eq!(
            a49_sentry_sgetc(&mut sentry, || panic!("must not run")),
            0x5a
        );
        assert!(sentry.cached);
        assert!(sentry.area.is_none());
    }

    /// `0x005f47a0`: consuming advances `pos`, decrements the remaining
    /// count, clears the cached flag, and falls through to `underflow` when
    /// the area is exhausted.
    #[test]
    fn sentry_bump_consumes_then_underflows() {
        let mut sentry = A49Sentry {
            area: Some(A49GetArea {
                bytes: b"ab",
                pos: 0,
            }),
            cached: true,
            value: 0xff,
        };
        assert_eq!(a49_sentry_sbumpc(&mut sentry, || -1), Some(i32::from(b'a')));
        assert_eq!(sentry.area.as_ref().unwrap().pos, 1);
        assert!(!sentry.cached);

        assert_eq!(a49_sentry_sbumpc(&mut sentry, || -1), Some(i32::from(b'b')));
        assert_eq!(sentry.area.as_ref().unwrap().pos, 2);

        // The area exists but is empty: underflow supplies the next byte.
        assert_eq!(a49_sentry_sbumpc(&mut sentry, || 0x2a), Some(0x2a));
        assert!(sentry.area.is_some());
        assert!(!sentry.cached);

        // An exhausted underflow clears the area and sets the flag.
        assert_eq!(a49_sentry_sbumpc(&mut sentry, || -1), None);
        assert!(sentry.area.is_none());
        assert!(sentry.cached);
    }

    /// `0x005f47a0`: a null area sets the flag and returns without calling
    /// the underflow slot.
    #[test]
    fn sentry_bump_without_an_area_is_immediate_failure() {
        let mut sentry = A49Sentry {
            area: None,
            cached: false,
            value: 0,
        };
        assert_eq!(
            a49_sentry_sbumpc(&mut sentry, || panic!("must not run")),
            None
        );
        assert!(sentry.cached);
    }

    /// `0x005f47f0`: the fill loop emits one empty inline header per
    /// iteration, with the 0x18 stride.
    #[test]
    fn fill_empty_strings_uses_the_inline_header() {
        let filled = a49_fill_empty_strings(3);
        assert_eq!(filled.len(), 3);
        assert!(filled.iter().all(|s| *s == a49_empty_string()));
        assert!(a49_fill_empty_strings(0).is_empty());
    }

    /// `0x005f4830`: the capacity comparison selects the inline object or
    /// the heap pointer, and the size at `+0x10` is forwarded unchanged.
    #[test]
    fn string_storage_selection_uses_the_capacity_header() {
        assert_eq!(
            a49_string_storage(&A49String {
                len: 7,
                cap: 0x0f,
                heap: false
            }),
            (A49StringStorage::Inline, 7)
        );
        assert_eq!(
            a49_string_storage(&A49String {
                len: 7,
                cap: 0x10,
                heap: true
            }),
            (A49StringStorage::Heap, 7)
        );
    }

    /// `0x005f4c90`: the signed selector frees the table for every non-zero
    /// value (the two deallocator paths) and only zero skips it; the second
    /// pointer is always released, and the delete flag is passed through.
    #[test]
    fn ctype_destructor_branches_on_the_signed_selector() {
        let positive = a49_ctype_destroy(1, false);
        assert!(positive.table);
        assert!(positive.aux);
        assert!(!positive.self_deleted);

        let zero = a49_ctype_destroy(0, true);
        assert!(!zero.table);
        assert!(zero.aux);
        assert!(zero.self_deleted);

        let negative = a49_ctype_destroy(-1, false);
        assert!(negative.table);
        assert!(negative.aux);
    }

    /// `0x005f4d20`: all three pointer slots are visited in order and the
    /// count follows which slots held a block.
    #[test]
    fn numpunct_destructor_clears_the_three_pointer_slots() {
        let mut numpunct = A49Numpunct {
            decimal_point: Some(vec![b'.']),
            thousands_sep: None,
            grouping: Some(vec![b'3']),
        };
        assert_eq!(a49_numpunct_destroy(&mut numpunct), 2);
        assert_eq!(numpunct, A49Numpunct::default());
        assert_eq!(a49_numpunct_destroy(&mut numpunct), 0);
    }

    /// `0x005f4cf0` and `0x005f4da0`: the facet/base destructors report the
    /// deleting flag and the `+0x0c` string release.
    #[test]
    fn facet_and_base_destructors_report_their_delete_flags() {
        assert!(!a49_facet_destroy(false));
        assert!(a49_facet_destroy(true));

        let mut inline = A49String {
            len: 3,
            cap: 0x0f,
            heap: false,
        };
        assert_eq!(a49_args_base_destroy(&mut inline, false), (false, false));
        assert_eq!(inline, a49_empty_string());

        let mut heap = A49String {
            len: 3,
            cap: 0x20,
            heap: true,
        };
        assert_eq!(a49_args_base_destroy(&mut heap, true), (true, true));
        assert!(!heap.heap);
        assert_eq!(heap.len, 3);
    }

    /// `0x005f4f30`: only an owned message block is released; the pointer
    /// and `_DoFree` byte are cleared in both cases.
    #[test]
    fn exception_destructor_frees_only_an_owned_message() {
        let mut owned = A49ExceptionMessage {
            text: Some(b"boom".to_vec()),
            owned: true,
        };
        assert_eq!(a49_exception_destroy(&mut owned, false), (true, false));
        assert_eq!(owned, A49ExceptionMessage::default());

        let mut borrowed = A49ExceptionMessage {
            text: Some(b"static".to_vec()),
            owned: false,
        };
        assert_eq!(a49_exception_destroy(&mut borrowed, true), (false, true));
        assert!(!borrowed.owned);
        assert!(borrowed.text.is_none());

        let mut empty = A49ExceptionMessage::default();
        assert_eq!(a49_exception_destroy(&mut empty, false), (false, false));
    }

    /// `0x005f4bd0`: the owned block and its optional sub-object are only
    /// released when the corresponding pointers are non-null; the `this`
    /// deletion follows the low bit of the argument.
    #[test]
    fn streambuf_destructor_releases_the_owned_block() {
        let none = a49_streambuf_destroy(None, true);
        assert_eq!(
            none,
            A49StreambufRelease {
                subobject_deleted: false,
                owned_block: false,
                self_deleted: true
            }
        );

        let hit = a49_streambuf_destroy(
            Some(A49OwnedFacet {
                subobject: true,
                accessor_hit: true,
            }),
            false,
        );
        assert!(hit.subobject_deleted);
        assert!(hit.owned_block);
        assert!(!hit.self_deleted);

        let miss = a49_streambuf_destroy(
            Some(A49OwnedFacet {
                subobject: true,
                accessor_hit: false,
            }),
            false,
        );
        assert!(!miss.subobject_deleted);
        assert!(miss.owned_block);

        let no_subobject = a49_streambuf_destroy(
            Some(A49OwnedFacet {
                subobject: false,
                accessor_hit: true,
            }),
            false,
        );
        assert!(!no_subobject.subobject_deleted);
        assert!(no_subobject.owned_block);
    }

    /// `0x005f51c0` and `0x005f5570`: both sites write three zero dwords.
    #[test]
    fn vector_triple_initialisers_zero_three_dwords() {
        let mut first = [1_u32, 2, 3];
        a49_vector_triple_init(&mut first);
        assert_eq!(first, [0, 0, 0]);

        let mut second = [4_u32, 5, 6];
        a49_vector_triple_init_5570(&mut second);
        assert_eq!(second, [0, 0, 0]);
    }
}
