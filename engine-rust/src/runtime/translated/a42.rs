#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent42.rs
// ===========================================================================
// Translation batch 42.  Only this file is edited by the batch.
//
// `0x005e6000..=0x005e7fff` is the retail client's *gateway / general
// settings* plumbing.  The routines here do not touch the simulation: they
// read and write the client's configuration store (the same store the
// "General settings" section of the retail options dialog writes) and copy
// text through the client's small-string container.  `World` keeps no such
// store, so the rows are translated as pure free functions whose inputs are
// the already-resolved lookups (the client's own variant/settings lookup
// lives at `0x005e3f00` and is outside this batch).  The arithmetic, the
// fixed strings and the small-string layout rules are reproduced exactly.
//
// The genuine function entries in the range are the nineteen addresses
// below.  Twenty-seven ledger rows in the range are not function entries at
// all: they are MSVC exception-cleanup funclets or basic blocks in the
// middle of a larger routine (for example `FUN_005e61f9`, `FUN_005e6414` or
// `FUN_005e71c0` start in the middle of an instruction).  The two genuine
// entries whose bodies are unrecoverable are listed at the end of this file.

/// `0x005e65a0`: fill a sixteen-byte small-string container from a byte run.
///
/// The routine is the client's `basic_string::assign(pointer, length)` core.
/// It is thiscall: `ecx` is the destination container and three stack
/// arguments carry the source pointer, the byte count and the allocator.
///
/// * `len <= 13`: the destination stores its characters inline at offset 0,
///   writes `0x0d - len` into the size byte at offset `0x0d` and writes the
///   capacity word `0x1c05` at offset `0x0e`.
/// * `len >= 14`: it writes `len` at offset 0, stores the freshly allocated
///   block (exactly `len + 1` bytes) at offset 8, and writes the capacity
///   word `0x0c05` at offset `0x0e`.
///
/// Both paths then copy `len` bytes and store a terminating zero.  The
/// return value is `(capacity_word, size_field, heap)`; for the heap case
/// the requested allocation is `size_field + 1` and `size_field` is the
/// count at offset 0, while for the inline case `size_field` is the byte
/// value stored at offset `0x0d`.
pub fn a42_string_store(len: usize) -> (u16, u32, bool) {
    if len <= 0x0d {
        (0x1c05, (0x0d - len) as u32, false)
    } else {
        (0x0c05, len as u32, true)
    }
}

/// `0x005e6060`: store a looked-up text setting into the destination object.
///
/// The caller hands in the destination (`ebp+8`), a context handle and the
/// key.  The lookup produces a variant; when it is absent the routine only
/// writes the "present" byte at `destination+0x1c` as zero.  When it is
/// present it converts the variant to text, writes `destination+0x1c = 1`
/// and assigns the text into the destination.  The routine returns the
/// destination pointer.
///
/// The translated form returns `(present, text)`; the absent case leaves the
/// destination text untouched, which [`None`] records.
pub fn a42_apply_text_setting(found: Option<&str>) -> (bool, Option<String>) {
    match found {
        None => (false, None),
        Some(text) => (true, Some(text.to_owned())),
    }
}

/// `0x005e6120`: construct a text setting, falling back to a supplied pair.
///
/// The routine first asks the lookup (through the same text reader used by
/// `0x005e6060`) for the key.  When the lookup hits, the destination is
/// built from the looked-up text; when it misses, the destination is built
/// from the `{pointer, length}` pair the caller supplied in `ebp+0x10`.
///
/// Both paths initialise the sixteen-byte container to empty before copying,
/// so this is exactly "found text, else fallback text".
pub fn a42_text_or_fallback(found: Option<&str>, fallback: &str) -> String {
    found.unwrap_or(fallback).to_owned()
}

/// `0x005e6260`: construct a text setting with a locally built lookup
/// context.
///
/// This is the two-argument wrapper around `0x005e6060`: it creates the
/// lookup context with `FUN_00754c10`, calls the text-setting reader and
/// returns the destination object.  The context has no simulation
/// counterpart, so the translated row exposes the stored text only; an
/// absent key produces the object whose "present" byte stayed zero, i.e.
/// [`None`].
pub fn a42_load_text_setting(found: Option<&str>) -> Option<String> {
    a42_apply_text_setting(found).1
}

/// `0x005e6300`: build a string list from a looked-up vector of strings.
///
/// The routine first initialises an intrusive list head as
/// `{next = self, prev = self, count = 0}`.  It then reads the setting; the
/// value must be a vector (its type word at offset `0x0e` is 4) and each
/// element is a sixteen-byte string container.  For every element it takes
/// the inline characters when bit `0x1000` of the element's flag word is
/// set, otherwise the pointer at element offset 8, computes the length with
/// a C-string scan, allocates a `0x24`-byte node and **inserts the node
/// directly after the head**.  The count at list offset 8 is incremented.
///
/// Head insertion means the resulting list is the reverse of the vector
/// order; that is the behaviour reproduced here.  An absent setting, a
/// non-vector value or an empty vector all leave the list empty.
pub fn a42_collect_setting_strings(values: &[&str]) -> Vec<String> {
    let mut list = Vec::with_capacity(values.len());
    for value in values {
        list.insert(0, (*value).to_owned());
    }
    list
}

/// `0x005e64e0`: store an integer setting only when it is present.
///
/// The routine builds the lookup context, queries the key and, on a miss,
/// writes only the presence byte at `output+4 = 0`.  On a hit it converts
/// the variant to an integer and writes `output[0] = value` together with
/// `output+4 = 1`.  The conversion itself is the client helper `FUN_00705bf0`;
/// this row begins at the lookup result, so the translated form takes the
/// already-converted integer.
pub fn a42_optional_int_setting(found: Option<i32>) -> Option<i32> {
    found
}

/// `0x005e6b50`: build the sixteen-byte record used for a numeric setting.
///
/// Byte argument `slot` is widened to sixteen bits, `9` is added *as a
/// sixteen-bit addition* (`add cx, 9`, so `0xffff` wraps to 8) and the
/// result is placed in the high half of record dword 3.  The routine only
/// reads the low byte of the caller's argument.  All other bytes of the
/// record are zero.  The routine then forwards the caller's key string
/// plus this record to the settings writer at `0x005e6bf0`; the returned
/// tuple keeps both pieces of that call.
///
/// For example `slot = 0` produces `0x0009_0000` in dword 3 and
/// `slot = 0xff` produces `0x0108_0000`.
pub fn a42_setting_record(key: &str, slot: u8) -> (&str, [u32; 4]) {
    let numeric = (slot as u16).wrapping_add(9) as u32;
    (key, [0, 0, 0, numeric << 16])
}

/// `0x005e6bf0`: compose the key of a general setting.
///
/// The routine reads the seven-byte format literal `"{0}-{1}"` at
/// `0x00d37fd4`, formats its two text operands into it, looks up the
/// `"General settings"` section (`0x00d37ec4`, nineteen bytes including the
/// terminator) and writes the payload into that section.  Only the key
/// composition is reproducible without the client's configuration store, so
/// the translated row returns the formatted `"{first}-{second}"` string.
pub fn a42_general_settings_key(first: &str, second: &str) -> String {
    format!("{first}-{second}")
}

/// `0x005e7680`: write the "Game Custom Hotkeys" numeric setting.
///
/// The fixed key is `"Game Custom Hotkeys"` at `0x00d37f80`; the routine
/// copies nineteen bytes, builds the record from its byte argument through
/// `0x005e6b50` and forwards both to `0x005e6bf0`.  The returned tuple is the
/// key and the record handed to that writer.
pub fn a42_game_custom_hotkeys_setting(slot: u8) -> (&'static str, [u32; 4]) {
    a42_setting_record("Game Custom Hotkeys", slot)
}

/// `0x005e7750`: write the "Game Extended Unit Limits" numeric setting.
///
/// The fixed key is `"Game Extended Unit Limits"` at `0x00d37f94`; the
/// routine copies twenty-five bytes and otherwise mirrors `0x005e7680`.
pub fn a42_game_extended_unit_limits_setting(slot: u8) -> (&'static str, [u32; 4]) {
    a42_setting_record("Game Extended Unit Limits", slot)
}

/// `0x005e7820`: write the "Game Name" text setting.
///
/// The fixed key is `"Game Name"` at `0x00d37f64`; nine bytes are copied.
/// The value arrives as a sixteen-byte `{pointer, length}` text object, is
/// re-copied into a fresh container and is then forwarded to the settings
/// writer with the key.  The translated form returns the key paired with
/// the supplied value.
pub fn a42_game_name_setting(value: &str) -> (&'static str, String) {
    ("Game Name", value.to_owned())
}

/// `0x005e78f0`: write the "Game Password" text setting.
///
/// The fixed key is `"Game Password"` at `0x00d37f70`; thirteen bytes are
/// copied.  The value handling is the same re-copy as `0x005e7820`.
pub fn a42_game_password_setting(value: &str) -> (&'static str, String) {
    ("Game Password", value.to_owned())
}

/// `0x005e79c0`: resolve a game-speed name and store its parsed form.
///
/// The routine indexes the fixed speed-name table at `0x00f04668` with an
/// unchecked index (`[0x00f04668 + index*4]`), scans the C string for its
/// length, copies it into a container and hands it to the client parser
/// `FUN_006fa550`.  The parsed result is then stored under `"Game Speed"`
/// (`0x00d37f30`, ten bytes).
///
/// The table has exactly seven entries, in this order, and the translated
/// row returns the table element; indices past the seventh have no defined
/// retail value and return [`None`].
pub fn a42_game_speed_name(index: usize) -> Option<&'static str> {
    const NAMES: [&str; 7] = [
        "gameSLOWEST",
        "gameSLOWER",
        "gameSLOW",
        "gameNORMAL",
        "gameFAST",
        "gameFASTER",
        "gameFASTEST",
    ];
    NAMES.get(index).copied()
}

/// `0x005e7b50`: write the "Game Type" text setting.
///
/// The fixed key is `"Game Type"` at `0x00d37f4c`; nine bytes are copied and
/// the supplied text value is re-copied and forwarded, exactly as in
/// `0x005e7820`.
pub fn a42_game_type_setting(value: &str) -> (&'static str, String) {
    ("Game Type", value.to_owned())
}

/// `0x005e7c20`: write the "Custom Type" text setting.
///
/// The fixed key is `"Custom Type"` at `0x00d37f58`; eleven bytes are
/// copied.  The value handling matches `0x005e7b50`.
pub fn a42_custom_type_setting(value: &str) -> (&'static str, String) {
    ("Custom Type", value.to_owned())
}

/// `0x005e7cf0`: the two fixed keys used by the recent-gateway lookup.
///
/// The routine materialises `"Recent gateway id"` from `0x00d37ff0`
/// (seventeen bytes) and `"Configuration"` from `0x00d38004` (thirteen
/// bytes), then calls the gateway-history reader `FUN_005e85b0` with both keys
/// and its two arguments.  The reader has no simulation counterpart, so the
/// translated row returns the key pair in call order.
pub fn a42_recent_gateway_keys() -> (&'static str, &'static str) {
    ("Recent gateway id", "Configuration")
}

/// A value stored in the "Gateway History" list by `0x005e7e40`.
///
/// The retail routine writes sixteen-byte containers.  The name and account
/// entries are text containers; the timestamp entry is a 64-bit integer
/// container whose flag word at offset `0x0e` carries the sign/zero
/// classification described on [`a42_gateway_history_entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A42HistoryValue {
    Text(String),
    Timestamp { value: i64, tag: u16 },
}

/// `0x005e7e40`: expand the gateway list into "Gateway History" pairs.
///
/// The routine walks the vector of gateway nodes it receives.  Null entries
/// are skipped.  For every live node it appends three key/value pairs to a
/// vector, in this order and with these fixed keys:
///
/// 1. `"gateway"` (`0x00d37ee8`, seven bytes) with the node's name at
///    node offset 8 (size at +0x0c);
/// 2. `"account"` (`0x00d37ef0`, seven bytes) with the account string at
///    node offset 0x28 (size at +0x2c);
/// 3. `"timestamp"` (`0x00d37ef8`, nine bytes) with the 64-bit integer at
///    node offset 0x48 (high dword at +0x4c).
///
/// The whole vector is then stored under the section literal
/// `"Gateway History"` at `0x00d37ed8`.  Text pairs are appended as they
/// are; the timestamp container's flag word starts at `0x116` for a negative
/// value and `0x196` for a non-negative one, gains `0x40` when the high dword
/// is zero, and gains `0x20` when the value also fits in a non-negative
/// 32-bit integer.
pub fn a42_gateway_history_entries(
    nodes: &[Option<(&str, &str, i64)>],
) -> Vec<(&'static str, A42HistoryValue)> {
    let mut entries = Vec::with_capacity(nodes.len() * 3);
    for node in nodes.iter().flatten() {
        let (name, account, timestamp) = *node;
        entries.push(("gateway", A42HistoryValue::Text(name.to_owned())));
        entries.push(("account", A42HistoryValue::Text(account.to_owned())));
        entries.push((
            "timestamp",
            A42HistoryValue::Timestamp {
                value: timestamp,
                tag: a42_timestamp_tag(timestamp),
            },
        ));
    }
    entries
}

/// The flag word built by the timestamp arm of `0x005e7e40`.
///
/// The routine starts from `0x116` when the high dword is negative and from
/// `0x196` otherwise.  It then ORs `0x40` when the high dword is zero and
/// `0x20` when the high dword is zero and the low dword is non-negative.
fn a42_timestamp_tag(value: i64) -> u16 {
    let mut tag = if value < 0 { 0x116 } else { 0x196 };
    if ((value as u64) >> 32) == 0 {
        tag |= 0x40;
    }
    if (0..=i32::MAX as i64).contains(&value) {
        tag |= 0x20;
    }
    tag
}

// Ledger rows in `0x005e6000..=0x005e7fff` that are *not* translated:
//
// * `FUN_005e6600` and `FUN_005e6d90` are genuine function entries, but both
//   bodies embed switch/jump data inside `.text`.  The recovered listings
//   carry dozens of overlapping-instruction and unreachable-block warnings
//   and mix table bytes (`rdtsc`, absolute writes) into the instruction
//   stream, so the control flow and therefore the semantics cannot be
//   recovered from the shipped artifact.  Nothing is guessed for them.
// * `FUN_005e61f9`, `FUN_005e622b`, `FUN_005e63fe`, `FUN_005e6414`, `FUN_005e6575`,
//   `FUN_005e685e`, `FUN_005e68c4`, `FUN_005e694f`, `FUN_005e6973`, `FUN_005e6a01`,
//   `FUN_005e6a74`, `FUN_005e6a90`, `FUN_005e6aa3`, `FUN_005e6afe`, `FUN_005e6bc8`,
//   `FUN_005e6e94`, `FUN_005e71c0`, `FUN_005e71fa`, `FUN_005e7235`, `FUN_005e7350`,
//   `FUN_005e7374`, `FUN_005e73ff`, `FUN_005e7511`, `FUN_005e75ad`, `FUN_005e7991`,
//   `FUN_005e7bc1`, `FUN_005e7bfa` are MSVC exception-cleanup funclets or
//   interior blocks.  They have no prologue and begin mid-instruction in the
//   retail `.text` section, so they are not independent functions to
//   translate.

#[cfg(test)]
mod tests_agent42 {
    use super::*;

    /// `0x005e6060`: a hit writes the text and the presence byte; a miss
    /// writes only a zero presence byte.
    #[test]
    fn text_setting_writes_only_when_present() {
        assert_eq!(a42_apply_text_setting(None), (false, None));
        assert_eq!(
            a42_apply_text_setting(Some("Marine")),
            (true, Some("Marine".to_owned()))
        );
        assert_eq!(
            a42_apply_text_setting(Some("")),
            (true, Some(String::new())),
            "an empty stored value is still present"
        );
    }

    /// `0x005e6120`: the looked-up text wins; the fallback pair is copied
    /// only on a miss.  `0x005e6260` is the same store without the context.
    #[test]
    fn text_setting_or_fallback_picks_the_lookup() {
        assert_eq!(
            a42_text_or_fallback(Some("looked up"), "fallback"),
            "looked up"
        );
        assert_eq!(a42_text_or_fallback(None, "fallback"), "fallback");
        assert_eq!(a42_text_or_fallback(Some(""), "fallback"), "");
        assert_eq!(
            a42_load_text_setting(Some("Ghost")),
            Some("Ghost".to_owned())
        );
        assert_eq!(a42_load_text_setting(None), None);
    }

    /// `0x005e6300`: each node is inserted directly after the list head, so
    /// the resulting order is the reverse of the source vector.
    #[test]
    fn collect_setting_strings_prepends_each_node() {
        assert!(a42_collect_setting_strings(&[]).is_empty());
        assert_eq!(
            a42_collect_setting_strings(&["first", "second", "third"]),
            vec!["third".to_owned(), "second".to_owned(), "first".to_owned()]
        );
        assert_eq!(a42_collect_setting_strings(&[""]), vec![String::new()]);
    }

    /// `0x005e64e0`: the absent path clears the presence byte and leaves the
    /// value slot alone; the present path copies the converted integer.
    #[test]
    fn optional_int_tracks_presence() {
        assert_eq!(a42_optional_int_setting(None), None);
        assert_eq!(a42_optional_int_setting(Some(0)), Some(0));
        assert_eq!(a42_optional_int_setting(Some(-5)), Some(-5));
        assert_eq!(a42_optional_int_setting(Some(i32::MIN)), Some(i32::MIN));
        assert_eq!(a42_optional_int_setting(Some(i32::MAX)), Some(i32::MAX));
    }

    /// `0x005e65a0`: thirteen bytes still fit the inline buffer; fourteen
    /// switch to the heap path with a `len + 1` allocation.
    #[test]
    fn string_store_switches_at_fourteen_bytes() {
        assert_eq!(a42_string_store(0), (0x1c05, 13, false));
        assert_eq!(a42_string_store(13), (0x1c05, 0, false));
        assert_eq!(a42_string_store(14), (0x0c05, 14, true));
        assert_eq!(a42_string_store(100), (0x0c05, 100, true));
        assert_eq!(
            a42_string_store(13).0,
            a42_string_store(0).0,
            "both inline lengths share the capacity word"
        );
    }

    /// `0x005e6b50`: the numeric field is `(slot + 9) << 16`, with the
    /// addition wrapping at sixteen bits.
    #[test]
    fn setting_record_encodes_slot_plus_nine() {
        assert_eq!(
            a42_setting_record("Game Name", 0),
            ("Game Name", [0, 0, 0, 9 << 16])
        );
        assert_eq!(
            a42_setting_record("Game Name", 7),
            ("Game Name", [0, 0, 0, 16 << 16])
        );
        assert_eq!(
            a42_setting_record("Game Name", 0xff),
            ("Game Name", [0, 0, 0, 0x0108_0000]),
            "0xff + 9 wraps to 0x108 in the sixteen-bit add"
        );
    }

    /// `0x005e6bf0`: the retail format literal joins the two operands with
    /// one hyphen and does not add anything else.
    #[test]
    fn general_settings_key_joins_two_parts() {
        assert_eq!(a42_general_settings_key("gateway", "7"), "gateway-7");
        assert_eq!(a42_general_settings_key("", "Game Name"), "-Game Name");
        assert_eq!(a42_general_settings_key("name", ""), "name-");
        assert_eq!(a42_general_settings_key("", ""), "-");
    }

    /// `0x005e7680`, `0x005e7750`, `0x005e7820`, `0x005e78f0`,
    /// `0x005e7b50` and `0x005e7c20`: the six fixed keys and their exact
    /// byte counts.
    #[test]
    fn general_setting_names_are_exact() {
        let (hotkeys, record) = a42_game_custom_hotkeys_setting(2);
        assert_eq!(hotkeys, "Game Custom Hotkeys");
        assert_eq!(hotkeys.len(), 19);
        assert_eq!(record, [0, 0, 0, 11 << 16]);

        let (limits, record) = a42_game_extended_unit_limits_setting(0);
        assert_eq!(limits, "Game Extended Unit Limits");
        assert_eq!(limits.len(), 25);
        assert_eq!(record, [0, 0, 0, 9 << 16]);

        assert_eq!(
            a42_game_name_setting("Jim"),
            ("Game Name", "Jim".to_owned())
        );
        assert_eq!("Game Name".len(), 9);

        assert_eq!(
            a42_game_password_setting("hunter2"),
            ("Game Password", "hunter2".to_owned())
        );
        assert_eq!("Game Password".len(), 13);

        assert_eq!(
            a42_game_type_setting("melee"),
            ("Game Type", "melee".to_owned())
        );
        assert_eq!("Game Type".len(), 9);

        assert_eq!(a42_custom_type_setting(""), ("Custom Type", String::new()));
        assert_eq!("Custom Type".len(), 11);
    }

    /// `0x005e79c0`: the seven speed names in table order; the eighth index
    /// is outside the retail table.
    #[test]
    fn game_speed_names_follow_the_retail_table() {
        let expected = [
            "gameSLOWEST",
            "gameSLOWER",
            "gameSLOW",
            "gameNORMAL",
            "gameFAST",
            "gameFASTER",
            "gameFASTEST",
        ];
        for (index, name) in expected.iter().enumerate() {
            assert_eq!(a42_game_speed_name(index), Some(*name));
        }
        assert_eq!(a42_game_speed_name(6).unwrap().len(), 11);
        assert_eq!(a42_game_speed_name(7), None);
        assert_eq!(a42_game_speed_name(usize::MAX), None);
    }

    /// `0x005e7cf0`: the two literals passed to the gateway reader.
    #[test]
    fn recent_gateway_keys_are_exact() {
        let (id, section) = a42_recent_gateway_keys();
        assert_eq!(id, "Recent gateway id");
        assert_eq!(id.len(), 17);
        assert_eq!(section, "Configuration");
        assert_eq!(section.len(), 13);
    }

    /// `0x005e7e40`: each live node contributes gateway/account/timestamp in
    /// that order; null nodes contribute nothing.  The timestamp flag word
    /// follows the sign and width rules of the routine.
    #[test]
    fn gateway_history_entries_expand_each_node() {
        assert!(a42_gateway_history_entries(&[]).is_empty());
        assert!(a42_gateway_history_entries(&[None, None]).is_empty());

        let entries = a42_gateway_history_entries(&[
            Some(("Korhal", "acct-1", 0)),
            None,
            Some(("Char", "acct-2", -1)),
        ]);
        assert_eq!(entries.len(), 6);

        assert_eq!(entries[0].0, "gateway");
        assert_eq!(entries[0].1, A42HistoryValue::Text("Korhal".to_owned()));
        assert_eq!(entries[1].0, "account");
        assert_eq!(entries[1].1, A42HistoryValue::Text("acct-1".to_owned()));
        assert_eq!(entries[2].0, "timestamp");
        assert_eq!(
            entries[2].1,
            A42HistoryValue::Timestamp {
                value: 0,
                tag: 0x1f6
            },
            "zero is non-negative, high dword zero, and fits in 32 bits"
        );

        assert_eq!(entries[3].0, "gateway");
        assert_eq!(entries[5].0, "timestamp");
        assert_eq!(
            entries[5].1,
            A42HistoryValue::Timestamp {
                value: -1,
                tag: 0x116
            },
            "a negative timestamp keeps only the negative base flag"
        );
    }

    /// `0x005e7e40`: the flag-word boundaries for values just on each side
    /// of the 32-bit and sign thresholds.
    #[test]
    fn timestamp_flags_follow_the_thresholds() {
        let entries = a42_gateway_history_entries(&[
            Some(("a", "b", i32::MAX as i64)),
            Some(("a", "b", 0x8000_0000)),
            Some(("a", "b", 0x1_0000_0000)),
            Some(("a", "b", i64::MIN)),
        ]);
        let tags: Vec<u16> = entries
            .iter()
            .filter_map(|(key, value)| match value {
                A42HistoryValue::Timestamp { tag, .. } if *key == "timestamp" => Some(*tag),
                _ => None,
            })
            .collect();
        assert_eq!(tags, vec![0x1f6, 0x1d6, 0x196, 0x116]);
    }
}
