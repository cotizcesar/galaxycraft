#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent44.rs
// ===========================================================================
// Translation batch 44 - `decompiled_uncited` rows in `0x005ea000..=0x005ebfff`.
//
// The window is one corner of the retail client's command-line layer.  The
// splitter produced 32 rows there, but only a handful of distinct routines
// stand behind them:
//
// * `0x005ea040`..`0x005ea290` - the five `std::string` expression builders
//   that compose every diagnostic message in the window (each one builds a
//   temporary and moves it into the caller's 24-byte `std::string`: a
//   16-byte SSO buffer at `+0`, size at `+0x10`, capacity at `+0x14`).
// * `0x005ea340` - the single-argument results / mismatch path.
// * `0x005ea500` - positional dispatch, including the two injection modes.
// * `0x005eab80`, `0x005eabeb` - the option sweep and the default-injection
//   half of one loop (28-byte, `0x1c`-stride records).
// * `0x005eaf00`, `0x005eaf1d` - the two `join` helpers.
// * `0x005eafb0`, `0x005eb780` - the argument dispatch loop and its tail.
// * `0x005ebf50` - flag result validation and its message table.
// * `0x005eb5be` - the block-release thunk plus the surrounding size
//   arithmetic.
//
// Rows that are re-entry slices of one of those routines - the splitter
// linearised the same loop once per jump-table target - are listed in
// [`A44_SLICES`] and are deliberately not duplicated as separate functions.
//
// Everything below is a free function in the retail build, so the
// translations are free functions as well.  Sizes and strides are kept as
// named constants because several rows only contribute arithmetic.

/// `std::string` as laid out by the retail compiler: 16-byte small-string
/// buffer, size at `+0x10`, capacity at `+0x14`; hence `0x18` per element in
/// a `vector<std::string>`.
pub const A44_STRING_STRIDE: usize = 0x18;

/// Small-string capacity: `capacity <= 0xf` means the text lives inline.
pub const A44_SSO_CAPACITY: usize = 0xf;

/// The positional injection records are 28 bytes (`0x1c`): one flag byte, one
/// character byte, two bytes of padding and an inlined 24-byte string.
pub const A44_POSITIONAL_RECORD_STRIDE: usize = 0x1c;

/// A block at least this large is released with the `size + 0x24` cookie.
pub const A44_LARGE_BLOCK: usize = 0x1000;

/// Minimum slack between a released data pointer and its stored size.
pub const A44_BLOCK_SLACK: usize = 0x20;

/// Rows in `0x005ea000..=0x005ebfff` that are interior slices of a routine
/// translated elsewhere in this file rather than separate routines.
pub const A44_SLICES: &[(u32, u32)] = &[
    (0x005e_a79d, 0x005e_a500),
    (0x005e_a7e7, 0x005e_a500),
    (0x005e_a945, 0x005e_a500),
    (0x005e_a9ab, 0x005e_a500),
    (0x005e_a9ff, 0x005e_a500),
    (0x005e_ab46, 0x005e_a500),
    (0x005eb_af1, 0x005e_bf50),
    (0x005eb_bd7, 0x005e_bf50),
];

// ---------------------------------------------------------------------------
// The string expression builders (0x005ea040 .. 0x005ea290).
// ---------------------------------------------------------------------------

/// `0x005ea040`: move a string operand out of the expression and reset the
/// source.
///
/// The body copies the source's text into the temporary (`FUN_005fd9c0` with
/// the source's size read from `+0x10` and its heap pointer taken when the
/// capacity at `+0x14` exceeds `0xf`), moves the temporary into the
/// destination, and then rewrites the source as empty with capacity `0xf`.
/// The second printed argument is unused: the destination is returned through
/// the caller's slot.
///
/// Translation: the text is appended as-is and the source is left empty,
/// which is the only observable effect of the reset.
pub fn a44_take_string(src: &mut String) -> String {
    std::mem::take(src)
}

/// `0x005ea0a0`: build a string from a NUL-terminated literal.
///
/// The length is measured with the `do { c = *p++; } while (c != '\0')` loop
/// and the text is then assigned with the measured length (the trailing NUL
/// is not part of the string).  The result is moved into the destination and
/// the temporary is reset to empty with capacity `0xf`.
pub fn a44_string_literal(text: &str) -> String {
    let mut out = String::new();
    out.push_str(text);
    out
}

/// `0x005ea230`: same constructor as [`a44_string_literal`] but through the
/// forced-allocation copy at `FUN_006000c0(0, text, len)`.
///
/// The retail pair exists because one call site wants the inline buffer and
/// the other wants a heap block; the resulting text is identical.  The
/// translation reserves exactly `len` bytes so the allocation stays out of
/// the inline buffer for non-empty text, matching the second constructor.
pub fn a44_string_literal_heap(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    out.push_str(text);
    out
}

/// `0x005ea1a0`: copy `text` and append one character byte.
///
/// The body reserves `size + 1` (`FUN_00602790(param_2[4] + 1)`), assigns the
/// text, then calls `push_back` with the byte taken from the third argument.
/// The retail byte is always an option character, so the translation appends
/// it as a `char`; the capacity request is mirrored because the reserve is
/// observable through the 24-byte layout.
pub fn a44_string_with_byte(text: &str, byte: u8) -> String {
    let mut out = String::with_capacity(text.len() + 1);
    out.push_str(text);
    out.push(byte as char);
    out
}

/// `0x005ea290`: concatenate a literal prefix and a string operand.
///
/// The body reserves `strlen(prefix) + value.size` (`FUN_00602790`) and then
/// appends the prefix followed by the value - the shape used for every
/// `"Flag '" + name` and `"Unknown command: " + arg` message in the window.
pub fn a44_string_pair(prefix: &str, value: &str) -> String {
    let mut out = String::with_capacity(prefix.len() + value.len());
    out.push_str(prefix);
    out.push_str(value);
    out
}

// ---------------------------------------------------------------------------
// Single-argument results (0x005ea340).
// ---------------------------------------------------------------------------

/// `0x005ea340`: collect the results for a one-element argument range.
///
/// Recovered guard table:
///
/// * the routine does nothing unless the app flag at `+0x188` is set and the
///   incoming range holds exactly one 24-byte string (`param_2 + 0x18 ==
///   param_3`);
/// * the candidate names are copied from the app vector at `+0x18c`;
/// * a candidate is kept when it is empty **or** when its own `find` of the
///   incoming text returns zero (the second operand of that `find` is lost
///   with the accumulator register, so the translation spells it
///   `candidate.starts_with(incoming)`);
/// * when the 4-byte config tag at argument `+0xbc` compares equal, the
///   app's text at `+0x120` is empty, the candidate is longer than the app
///   limit at `+0x130` and the app's suffix field at `+0x150` is present,
///   the candidate is stored with its 1-based slot number appended
///   (`FUN_00603150` builds the number, `FUN_00602730` appends it);
/// * otherwise the candidate is stored unchanged.
///
/// The retail routine then raises the argument-mismatch exception through
/// `FUN_005f5100`/`FUN_005f1cc0` after appending to the argument's result
/// vector at `+0xb0`; the translation returns the strings that would have
/// been appended so the guard table can be tested.
pub fn a44_single_argument_results(
    enabled: bool,
    pending: &[&str],
    names: &[&str],
    config_tag_ok: bool,
    argument_text_empty: bool,
    min_len: usize,
    suffix_present: bool,
) -> Vec<String> {
    if !enabled || pending.len() != 1 {
        return Vec::new();
    }
    let incoming = pending[0];
    let mut out = Vec::new();
    for (slot, name) in names.iter().enumerate() {
        if !name.is_empty() && !name.starts_with(incoming) {
            continue;
        }
        if config_tag_ok && argument_text_empty && min_len < name.len() && suffix_present {
            out.push(format!("{}{}", name, slot + 1));
        } else {
            out.push((*name).to_string());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Positional dispatch (0x005ea500).
// ---------------------------------------------------------------------------

/// `0x005ea500`: the two injection modes of the positional step.
///
/// The recovered tail of the routine has two branches, selected by the count
/// `iStack_a8` built from two app probes (the values are `0`, `1` or `2`):
///
/// * mode `0`: when the flag at `+0x181` is set, the pointer at `+0x160` is
///   non-null and the text at `+0x150` is found, the consumed prefix of
///   `args` is erased (`FUN_005f1370(args, 0, index, ...)`) and every entry
///   of the app vector at `+0x18c` is appended to `args` behind the app name
///   at `+0x138` (the conversion at `FUN_005ea100` reads the same name text
///   and the branch reuses the `+0x138` prefix, so the translation takes one
///   `name` for both modes);
/// * mode `1`: when the flag at `+0x180` is set, the word at `+0x148` is a
///   valid index and the one-byte probe accepts the character, each entry is
///   appended as `name + character + entry`;
/// * any other mode leaves `args` untouched.
///
/// Returns whether the mode fired (the retail flag at `-0x79`).
pub fn a44_positional_injection(
    mode: u8,
    args: &mut Vec<String>,
    index: usize,
    name: &str,
    expansions: &[String],
    allow_expand: bool,
    allow_char: bool,
    accepted: bool,
) -> bool {
    match mode {
        0 => {
            if !(allow_expand && accepted) {
                return false;
            }
            let consumed = index.min(args.len());
            args.drain(..consumed);
            for entry in expansions {
                let mut out = String::from(name);
                out.push_str(entry);
                args.push(out);
            }
            true
        }
        1 => {
            if !allow_char || index + 1 >= args.len() {
                return false;
            }
            let ch = args[index].as_bytes().first().copied().unwrap_or(0) as char;
            if !accepted {
                return false;
            }
            for entry in expansions {
                let mut out = String::from(name);
                out.push(ch);
                out.push_str(entry);
                args.push(out);
            }
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Option sweep and default injection (0x005eab80, 0x005eabeb).
// ---------------------------------------------------------------------------

/// `0x005eab80`: append the names of the still-unused options to `args`.
///
/// The loop walks an array of option pointers with a 4-byte stride, skips an
/// option whose flag byte at `+4` carries `0x40`, and appends
/// `text(+0x138) + option name` with the push helper at `FUN_005f4fc0`.  The
/// push result is inverted into the loop flag at `-0x79`: a failed push ends
/// the sweep (that is how the retail container reports "already present").
/// The translation models the bounded push with `capacity`.
///
/// Returns `true` when the sweep ran to the end without a failed push.
pub fn a44_option_sweep(
    args: &mut Vec<String>,
    options: &[(&str, u8)],
    prefix: &str,
    capacity: usize,
) -> bool {
    for (name, flags) in options {
        if flags & 0x40 != 0 {
            continue;
        }
        if args.len() >= capacity {
            return false;
        }
        args.push(format!("{prefix}{name}"));
    }
    true
}

/// `0x005eabeb`: append the pending positional defaults to `args`.
///
/// The app vector at `+0x18c` holds 28-byte records: a flag byte at `+0`, a
/// character at `+1` and a 24-byte string at `+4`.  A zero flag appends the
/// record text (through the conversion at `FUN_005ea100`), a non-zero flag
/// appends `text(+0x138) + character`.  The teardown then releases
/// `count * 0x1c` bytes with the `+0x23` cookie when the block is at least
/// `0x1000` bytes (see [`a44_default_record_count`] and
/// [`a44_block_release_size`]).
///
/// Returns the number of entries appended.
pub fn a44_inject_defaults(
    args: &mut Vec<String>,
    defaults: &[(bool, u8, &str)],
    name: &str,
    capacity: usize,
) -> usize {
    let mut pushed = 0;
    for (is_char, ch, text) in defaults {
        if args.len() >= capacity {
            break;
        }
        if *is_char {
            let mut out = String::from(name);
            out.push(*ch as char);
            args.push(out);
        } else {
            args.push((*text).to_string());
        }
        pushed += 1;
    }
    pushed
}

/// Element count of the `0x1c`-stride record vector: `(end - begin) / 0x1c`,
/// the expression used by `0x005eabeb` (and by the teardown of
/// `0x005eab80`) before the block is released.
pub fn a44_default_record_count(bytes: usize) -> usize {
    bytes / A44_POSITIONAL_RECORD_STRIDE
}

// ---------------------------------------------------------------------------
// The two join helpers (0x005eaf00, 0x005eaf1d).
// ---------------------------------------------------------------------------

/// `0x005eaf00`: join strings with a separator.
///
/// The separator is appended only when the accumulator is already non-empty,
/// so an empty first element suppresses the separator before the second one -
/// that is the retail check (`*(int *)(dest + 0x10) != 0`), not a "first
/// element" flag.  The body is:
///
/// ```text
/// for each element:
///     if (dest.size != 0) append(separator)
///     append(element)
/// ```
pub fn a44_join(parts: &[String], separator: &str) -> String {
    let mut out = String::new();
    for part in parts {
        if !out.is_empty() {
            out.push_str(separator);
        }
        out.push_str(part);
    }
    out
}

/// `0x005eaf1d`: the same join with the `__thiscall` hidden return slot.
///
/// The body reads the destination from `[ebp+8]`, the vector from `[ebp+0xc]`
/// and the separator from `[ebp+0x10]`, walks the vector with a 24-byte
/// stride and applies the identical "append the separator only when the
/// accumulator is non-empty" rule.  It is a distinct entry from
/// [`a44_join`], so it is kept as its own translation.
pub fn a44_join_range(parts: &[String], separator: &str) -> String {
    let mut out = String::new();
    let mut index = 0;
    while index < parts.len() {
        if !out.is_empty() {
            out.push_str(separator);
        }
        out.push_str(&parts[index]);
        index += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Argument dispatch (0x005eafb0, 0x005eb780).
// ---------------------------------------------------------------------------

/// `0x005eafb0`: one pass of the argument dispatch loop.
///
/// The recovered order of operations per token is:
///
/// 1. offer the token to the positional step at `0x005ea500`; a consumed
///    token restarts the loop;
/// 2. compare the token against the app's terminator text (a two-byte field
///    at `+0x168`, size `+0x178`); a match sets the terminator flag at
///    `-0xc1` and everything after it is positional;
/// 3. otherwise walk the option table with the byte compare at
///    `FUN_005f02b0`, set `app[0x3f]` to the match and mark the option used
///    (byte at `+8`);
/// 4. an unmatched token falls through to the two result-splitting helpers
///    (`FUN_005ec590` / `FUN_005ecab0`) and finally becomes an extra.
///
/// The translation keeps steps 2 and 3 - the decidable part of the table -
/// and returns the matched options together with their attached value, plus
/// the tokens the loop did not consume.  `results_full` mirrors the app
/// counter at `+0x130`: once enough results have been collected the rest of
/// the line is handed on untouched.
pub fn a44_parse_arg_loop(
    args: &mut Vec<String>,
    options: &[String],
    terminator: &str,
    results_full: usize,
) -> (Vec<(usize, Option<String>)>, Vec<String>) {
    let mut matched = Vec::new();
    loop {
        let Some(head) = args.first().cloned() else {
            break;
        };
        if head == terminator {
            args.remove(0);
            break;
        }
        let Some(hit) = a44_match_option_token(&head, options) else {
            break;
        };
        if matched.len() >= results_full {
            break;
        }
        matched.push(hit);
        args.remove(0);
    }
    let rest = std::mem::take(args);
    (matched, rest)
}

/// Matching helper for [`a44_parse_arg_loop`]'s step 3.
///
/// The retail loop compares the whole token first; when that fails it retries
/// on the text in front of an attached value.  Both spellings occur in the
/// window: `--name=value` (long form) and `-xVALUE` (short form with the
/// value glued to the switch).  The returned pair is the option index and the
/// attached value, if any.
pub fn a44_match_option_token(token: &str, options: &[String]) -> Option<(usize, Option<String>)> {
    if token.is_empty() {
        return None;
    }
    if let Some(index) = options.iter().position(|option| option == token) {
        return Some((index, None));
    }
    if let Some(rest) = token.strip_prefix("--") {
        if let Some((name, value)) = rest.split_once('=') {
            let full = format!("--{name}");
            if let Some(index) = options.iter().position(|option| *option == full) {
                return Some((index, Some(value.to_string())));
            }
        }
        return None;
    }
    if let Some(rest) = token.strip_prefix('-') {
        if rest.len() > 1 {
            let (name, value) = rest.split_at(1);
            let full = format!("-{name}");
            if let Some(index) = options.iter().position(|option| *option == full) {
                return Some((index, Some(value.to_string())));
            }
        }
    }
    None
}

/// `0x005eb780`: flush the tokens the dispatch loop did not consume.
///
/// After the loop the routine calls the virtual at `(*app + 8)` with the two
/// app fields at `+0x138`/`+0x120` and writes the end cursor back through the
/// caller's pointer (`*param_1 = param_3`).  The translation performs the
/// observable part: it hands back the first `cursor` tokens as extras and
/// leaves the rest in place.
pub fn a44_parse_arg_tail(args: &mut Vec<String>, cursor: usize) -> Vec<String> {
    let taken = cursor.min(args.len());
    args.drain(..taken).collect()
}

/// Number of 24-byte argument slots in `bytes`, the stride arithmetic shared
/// by the dispatch loop and its tail (`(end - begin) / 0x18`).
pub fn a44_arg_span_count(bytes: usize) -> usize {
    bytes / A44_STRING_STRIDE
}

// ---------------------------------------------------------------------------
// Flag result validation (0x005ebf50).
// ---------------------------------------------------------------------------

/// `0x005ebf50`: validate a flag's collected results and build its message.
///
/// The routine clears the result vector, reads the expected count from the
/// option (`min`, `local_14c`), and then applies four rules:
///
/// * a flag that does not expect a value (`takes_value == false`) either
///   rejects a separate argument outright when `allow_separate` is clear, or
///   consumes following tokens while the results are below `max` and the
///   caller's acceptance test (`accept_following`) allows it;
/// * a flag that does expect a value rejects a joined argument when
///   `allow_joined` is clear and there is an expected count, and otherwise
///   stores the single extracted `value` unless the caller asked for no
///   value at all (`single_value` with `max != 0`);
/// * more results than `max` is "passed an argument into a non-argument
///   flag";
/// * fewer results than `min` picks one of the four "requires ..." texts,
///   depending on whether `min` is one and whether `min` equals `max`.
///
/// The message table is byte-for-byte the retail one; every branch ends in
/// the exception raised at `LAB_005ec562` (the translation returns the text
/// instead).
pub fn a44_flag_result_validation(
    name: &str,
    results: &mut Vec<String>,
    min: usize,
    max: usize,
    allow_separate: bool,
    allow_joined: bool,
    takes_value: bool,
    single_value: bool,
    value: Option<&str>,
    following: &[String],
    accept_following: bool,
) -> Result<(), String> {
    if !takes_value {
        if !allow_separate {
            if min != 0 {
                return Err(format!(
                    "Flag '{}' was passed a separate argument, but these are disallowed",
                    name
                ));
            }
        } else {
            let mut taken = 0;
            while taken < following.len() && results.len() < max && (min == max || accept_following)
            {
                results.push(following[taken].clone());
                taken += 1;
            }
        }
    } else {
        if !allow_joined && min != 0 {
            return Err(format!(
                "Flag '{}' was passed a joined argument, but these are disallowed",
                name
            ));
        }
        if (!single_value || max != 0) && value.is_some() {
            results.push(value.unwrap_or_default().to_string());
        }
    }

    let count = results.len();
    if max < count {
        return Err(format!(
            "Passed an argument into a non-argument flag: {}",
            name
        ));
    }
    if count < min {
        if min == 1 {
            if max == 1 {
                return Err(format!(
                    "Flag '{}' requires an argument but received none",
                    name
                ));
            }
            return Err(format!(
                "Flag '{}' requires at least one argument but received none",
                name
            ));
        }
        if min == max {
            return Err(format!(
                "Flag '{}' requires {} arguments but received {}",
                name, min, count
            ));
        }
        return Err(format!(
            "Flag '{}' requires at least {} arguments but received {}",
            name, min, count
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Block release (0x005eb5be).
// ---------------------------------------------------------------------------

/// `0x005eb5be`: the block release thunk and the size arithmetic around it.
///
/// The seven-byte row is a tail call to the allocator release
/// (`FUN_00923e90`).  The teardowns in this window compute the release size
/// the same way up to the cookie: a candidate of `size + 1` below `0x1000`
/// releases the block as `size + 1`, and a larger block uses the debug cookie
/// (`size + 0x24` in the `FUN_005ea79d` slice, `size + 0x23` in the
/// `0x005eabeb` tail - the translation keeps the `+0x24` form).  In both
/// cases the data pointer must sit at least `0x20` bytes past the stored size
/// (`(data - *(data - 4)) - 4 >= 0x20`), otherwise the invalid-parameter path
/// is taken.
///
/// Returns the release size, or `Err(())` when the slack check fails.
pub fn a44_block_release_size(data: usize, stored: usize, size: usize) -> Result<usize, ()> {
    if data.wrapping_sub(stored).wrapping_sub(4) < A44_BLOCK_SLACK {
        return Err(());
    }
    if size + 1 < A44_LARGE_BLOCK {
        Ok(size + 1)
    } else {
        Ok(size + 0x24)
    }
}

#[cfg(test)]
mod tests_agent44 {
    use super::*;

    #[test]
    fn a44_take_string_moves_and_clears_the_source() {
        let mut source = String::from("verbose");
        let taken = a44_take_string(&mut source);
        assert_eq!(taken, "verbose");
        assert!(source.is_empty());
    }

    #[test]
    fn a44_take_string_of_empty_source() {
        let mut source = String::new();
        assert_eq!(a44_take_string(&mut source), "");
        assert!(source.is_empty());
    }

    #[test]
    fn a44_string_literal_copies_without_the_nul() {
        assert_eq!(a44_string_literal("Flag '"), "Flag '");
        assert_eq!(a44_string_literal(""), "");
    }

    #[test]
    fn a44_string_literal_heap_matches_the_inline_builder() {
        let text = "Unknown command: ";
        assert_eq!(a44_string_literal_heap(text), a44_string_literal(text));
        assert_eq!(a44_string_literal_heap(text).len(), 17);
    }

    #[test]
    fn a44_string_with_byte_appends_one_character() {
        assert_eq!(a44_string_with_byte("Flag '", b'x'), "Flag 'x");
        assert_eq!(a44_string_with_byte("", b'Z'), "Z");
    }

    #[test]
    fn a44_string_pair_concatenates_prefix_and_value() {
        assert_eq!(a44_string_pair("Flag '", "verbose"), "Flag 'verbose");
        assert_eq!(
            a44_string_pair("Unknown command: ", "go"),
            "Unknown command: go"
        );
        assert_eq!(a44_string_pair("", "x"), "x");
    }

    #[test]
    fn a44_single_argument_results_filters_and_keeps_order() {
        let names = ["alpha", "beta", ""];
        let out = a44_single_argument_results(true, &["al"], &names, false, false, 4, false);
        assert_eq!(out, vec!["alpha".to_string(), "".to_string()]);
    }

    #[test]
    fn a44_single_argument_results_index_suffix_path() {
        let names = ["alpha", "beta"];
        let out = a44_single_argument_results(true, &["al"], &names, true, true, 4, true);
        // Slot 1 ("alpha") is longer than the limit and gets its 1-based
        // number appended; "beta" does not match the incoming text.
        assert_eq!(out, vec!["alpha1".to_string()]);
    }

    #[test]
    fn a44_single_argument_results_needs_the_flag_and_one_argument() {
        let names = ["alpha"];
        assert!(
            a44_single_argument_results(false, &["al"], &names, false, false, 0, false).is_empty()
        );
        assert!(a44_single_argument_results(true, &[], &names, false, false, 0, false).is_empty());
        assert!(
            a44_single_argument_results(true, &["a", "b"], &names, false, false, 0, false)
                .is_empty()
        );
    }

    #[test]
    fn a44_positional_injection_mode_zero_erases_then_expands() {
        let mut args = vec!["prog".to_string(), "-q".to_string(), "3".to_string()];
        let expansions = ["x".to_string(), "y".to_string()];
        // index is the consumed prefix length, so the first two tokens go.
        let fired = a44_positional_injection(0, &mut args, 2, "--", &expansions, true, false, true);
        assert!(fired);
        assert_eq!(
            args,
            vec!["3".to_string(), "--x".to_string(), "--y".to_string()]
        );
    }

    #[test]
    fn a44_positional_injection_mode_zero_can_erase_everything() {
        let mut args = vec!["prog".to_string(), "-q".to_string()];
        let expansions = ["x".to_string()];
        let fired = a44_positional_injection(0, &mut args, 2, "-", &expansions, true, false, true);
        assert!(fired);
        assert_eq!(args, vec!["-x".to_string()]);
    }

    #[test]
    fn a44_positional_injection_mode_one_glues_the_character() {
        let mut args = vec!["prog".to_string(), "Z".to_string(), String::new()];
        let expansions = ["o1".to_string()];
        let fired = a44_positional_injection(1, &mut args, 1, "--", &expansions, false, true, true);
        assert!(fired);
        assert_eq!(
            args,
            vec![
                "prog".to_string(),
                "Z".to_string(),
                String::new(),
                "--Zo1".to_string()
            ]
        );
    }

    #[test]
    fn a44_positional_injection_requires_both_the_flag_and_acceptance() {
        let mut args = vec!["a".to_string()];
        let expansions = ["x".to_string()];
        assert!(!a44_positional_injection(
            0,
            &mut args,
            0,
            "-",
            &expansions,
            false,
            true,
            true
        ));
        assert!(!a44_positional_injection(
            0,
            &mut args,
            0,
            "-",
            &expansions,
            true,
            false,
            false
        ));
        assert!(!a44_positional_injection(
            1,
            &mut args,
            0,
            "-",
            &expansions,
            true,
            true,
            false
        ));
        assert_eq!(args, vec!["a".to_string()]);
    }

    #[test]
    fn a44_option_sweep_skips_used_options_and_keeps_order() {
        let mut args = Vec::new();
        let options = [("--alpha", 0u8), ("--used", 0x40u8), ("--beta", 0x01u8)];
        assert!(a44_option_sweep(&mut args, &options, "-x", 8));
        assert_eq!(args, vec!["-x--alpha".to_string(), "-x--beta".to_string()]);
    }

    #[test]
    fn a44_option_sweep_stops_on_a_full_container() {
        let mut args = Vec::new();
        let options = [("a", 0u8), ("b", 0u8), ("c", 0u8)];
        assert!(!a44_option_sweep(&mut args, &options, "", 2));
        assert_eq!(args, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn a44_inject_defaults_text_and_character_records() {
        let mut args = Vec::new();
        let defaults = [
            (false, 0u8, "one"),
            (true, b'k', "ignored"),
            (false, 0u8, "two"),
        ];
        let pushed = a44_inject_defaults(&mut args, &defaults, "--", 8);
        assert_eq!(pushed, 3);
        assert_eq!(
            args,
            vec!["one".to_string(), "--k".to_string(), "two".to_string()]
        );
    }

    #[test]
    fn a44_inject_defaults_honours_the_capacity() {
        let mut args = vec!["first".to_string()];
        let defaults = [(false, 0u8, "one"), (true, b'k', "x")];
        assert_eq!(a44_inject_defaults(&mut args, &defaults, "-", 2), 1);
        assert_eq!(args, vec!["first".to_string(), "one".to_string()]);
    }

    #[test]
    fn a44_default_record_count_uses_the_1c_stride() {
        assert_eq!(a44_default_record_count(0), 0);
        assert_eq!(
            a44_default_record_count(A44_POSITIONAL_RECORD_STRIDE * 4),
            4
        );
        assert_eq!(a44_default_record_count(0x1c * 3 + 7), 3);
    }

    #[test]
    fn a44_join_inserts_the_separator_between_non_empty_results() {
        let parts = ["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(a44_join(&parts, ","), "a,b,c");
        assert_eq!(a44_join(&parts, ""), "abc");
        assert_eq!(a44_join(&[], ","), "");
    }

    #[test]
    fn a44_join_suppresses_the_separator_after_an_empty_first_part() {
        // The retail check is "accumulator non-empty", not "not the first
        // element", so an empty first part also loses the separator.
        let parts = [String::new(), "b".to_string()];
        assert_eq!(a44_join(&parts, ","), "b");
        let parts = ["a".to_string(), String::new(), "c".to_string()];
        assert_eq!(a44_join(&parts, ","), "a,,c");
    }

    #[test]
    fn a44_join_range_matches_the_join_helper() {
        let parts = ["one".to_string(), "two".to_string(), "three".to_string()];
        assert_eq!(a44_join_range(&parts, " "), a44_join(&parts, " "));
        assert_eq!(a44_join_range(&parts, " "), "one two three");
    }

    #[test]
    fn a44_match_option_token_covers_all_three_spellings() {
        let options = [
            "--verbose".to_string(),
            "--count".to_string(),
            "-n".to_string(),
        ];
        assert_eq!(
            a44_match_option_token("--verbose", &options),
            Some((0, None))
        );
        assert_eq!(
            a44_match_option_token("--count=3", &options),
            Some((1, Some("3".to_string())))
        );
        assert_eq!(
            a44_match_option_token("-n5", &options),
            Some((2, Some("5".to_string())))
        );
        assert_eq!(a44_match_option_token("--missing", &options), None);
        assert_eq!(a44_match_option_token("plain", &options), None);
    }

    #[test]
    fn a44_parse_arg_loop_consumes_until_terminator_or_unknown() {
        let mut args = vec![
            "--verbose".to_string(),
            "--count=3".to_string(),
            "file".to_string(),
        ];
        let options = ["--verbose".to_string(), "--count".to_string()];
        let (matched, rest) = a44_parse_arg_loop(&mut args, &options, "--", 8);
        assert_eq!(matched, vec![(0, None), (1, Some("3".to_string()))]);
        assert_eq!(rest, vec!["file".to_string()]);
        assert!(args.is_empty());
    }

    #[test]
    fn a44_parse_arg_loop_stops_at_the_terminator() {
        let mut args = vec!["--".to_string(), "--verbose".to_string()];
        let options = ["--verbose".to_string()];
        let (matched, rest) = a44_parse_arg_loop(&mut args, &options, "--", 8);
        assert!(matched.is_empty());
        assert_eq!(rest, vec!["--verbose".to_string()]);
    }

    #[test]
    fn a44_parse_arg_loop_stops_when_results_are_full() {
        let mut args = vec!["--verbose".to_string(), "--verbose".to_string()];
        let options = ["--verbose".to_string()];
        let (matched, rest) = a44_parse_arg_loop(&mut args, &options, "--", 1);
        assert_eq!(matched, vec![(0, None)]);
        assert_eq!(rest, vec!["--verbose".to_string()]);
    }

    #[test]
    fn a44_parse_arg_tail_flushes_the_cursor() {
        let mut args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(
            a44_parse_arg_tail(&mut args, 2),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(args, vec!["c".to_string()]);
        // A cursor past the end is clamped, like the retail end pointer.
        assert_eq!(a44_parse_arg_tail(&mut args, 9), vec!["c".to_string()]);
        assert!(args.is_empty());
    }

    #[test]
    fn a44_arg_span_count_uses_the_18_byte_stride() {
        assert_eq!(a44_arg_span_count(0), 0);
        assert_eq!(a44_arg_span_count(A44_STRING_STRIDE * 3), 3);
        assert_eq!(a44_arg_span_count(0x18 * 2 + 0x10), 2);
    }

    fn ok_flag(
        results: &mut Vec<String>,
        min: usize,
        max: usize,
        value: &str,
    ) -> Result<(), String> {
        a44_flag_result_validation(
            "--count",
            results,
            min,
            max,
            true,
            true,
            true,
            false,
            Some(value),
            &[],
            true,
        )
    }

    #[test]
    fn a44_flag_result_validation_accepts_a_value_in_range() {
        let mut results = Vec::new();
        assert_eq!(ok_flag(&mut results, 1, 1, "3"), Ok(()));
        assert_eq!(results, vec!["3".to_string()]);
    }

    #[test]
    fn a44_flag_result_validation_separate_argument_disallowed() {
        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            1,
            1,
            false,
            true,
            false,
            false,
            None,
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Flag '--flag' was passed a separate argument, but these are disallowed"
        );
    }

    #[test]
    fn a44_flag_result_validation_joined_argument_disallowed() {
        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            1,
            1,
            true,
            false,
            true,
            false,
            Some("v"),
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Flag '--flag' was passed a joined argument, but these are disallowed"
        );
    }

    #[test]
    fn a44_flag_result_validation_consumes_separate_arguments() {
        let mut results = Vec::new();
        let following = ["one".to_string(), "two".to_string(), "three".to_string()];
        let out = a44_flag_result_validation(
            "--flag",
            &mut results,
            2,
            2,
            true,
            true,
            false,
            false,
            None,
            &following,
            true,
        );
        assert_eq!(out, Ok(()));
        assert_eq!(results, vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn a44_flag_result_validation_rejects_extra_results() {
        let mut results = vec!["already".to_string()];
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            0,
            0,
            true,
            true,
            true,
            false,
            Some("v"),
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(err, "Passed an argument into a non-argument flag: --flag");
    }

    #[test]
    fn a44_flag_result_validation_requires_an_argument() {
        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            1,
            1,
            true,
            true,
            true,
            true,
            None,
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(err, "Flag '--flag' requires an argument but received none");

        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            1,
            3,
            true,
            true,
            true,
            true,
            None,
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Flag '--flag' requires at least one argument but received none"
        );
    }

    #[test]
    fn a44_flag_result_validation_requires_exact_and_at_least_counts() {
        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            2,
            2,
            true,
            true,
            true,
            false,
            Some("v"),
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(err, "Flag '--flag' requires 2 arguments but received 1");

        let mut results = Vec::new();
        let err = a44_flag_result_validation(
            "--flag",
            &mut results,
            2,
            4,
            true,
            true,
            true,
            false,
            Some("v"),
            &[],
            true,
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Flag '--flag' requires at least 2 arguments but received 1"
        );
    }

    #[test]
    fn a44_block_release_size_keeps_the_two_cookies() {
        assert_eq!(a44_block_release_size(0x1000, 0x1000, 0x10), Ok(0x11));
        // size + 1 == 0x1000 flips to the debug cookie.
        assert_eq!(
            a44_block_release_size(0x2000, 0x1000, 0xfff),
            Ok(0xfff + 0x24)
        );
        assert_eq!(a44_block_release_size(0x2000, 0x1000, 0x1000), Ok(0x1024));
    }

    #[test]
    fn a44_block_release_size_rejects_a_short_slack() {
        // data - stored - 4 == 0x1f, one byte below the required 0x20.
        assert_eq!(a44_block_release_size(0x1023, 0x1000, 0x10), Err(()));
        assert_eq!(a44_block_release_size(0x1024, 0x1000, 0x10), Ok(0x11));
    }

    #[test]
    fn a44_slice_table_names_the_parent_routine() {
        assert_eq!(A44_SLICES[0], (0x005e_a79d, 0x005e_a500));
        assert!(A44_SLICES.iter().all(|(_, parent)| *parent >= 0x005e_a340));
        assert_eq!(A44_SSO_CAPACITY, 0xf);
    }
}
