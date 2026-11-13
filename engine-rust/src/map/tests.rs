use super::*;

fn start(owner: u8, x: i32, y: i32) -> MapUnit {
    MapUnit {
        serial: 0,
        unit_id: 214,
        x,
        y,
        owner,
        states_set: 0,
        fields_set: 0,
        hit_point_percent: 100,
        shield_percent: 100,
        energy_percent: 100,
        resource_amount: 0,
        hangar: 0,
        states: 0,
        link_serial: 0,
    }
}

#[test]
fn start_locations_are_looked_up_by_owner_not_file_order() {
    // (8)The Hunters stores owner 7 before owners 3..6.
    let sc = Scenario {
        units: vec![
            start(0, 384, 240),
            start(7, 320, 1552),
            start(3, 3712, 2608),
        ],
        ..Scenario::default()
    };
    assert_eq!(sc.start_location_of(7), Some((320, 1552)));
    assert_eq!(sc.start_location_of(3), Some((3712, 2608)));
    assert_eq!(sc.start_location_of(5), None);
    assert_eq!(sc.start_locations()[1], (320, 1552), "file order");
}

/// Build a minimal 2x2 CHK with one MTXM tile.
fn tiny_chk() -> Vec<u8> {
    let mut out = Vec::new();
    let mut push = |name: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(name);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
    };
    push(
        b"DIM ",
        &2u16
            .to_le_bytes()
            .into_iter()
            .chain(2u16.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    push(b"ERA ", &1u16.to_le_bytes());
    let mut mtxm = Vec::new();
    for _ in 0..4 {
        mtxm.extend_from_slice(&32u16.to_le_bytes());
    }
    push(b"MTXM", &mtxm);
    push(b"MASK", &[1, 1, 1, 1]);
    out
}

#[test]
fn str_strings_end_at_their_nul_even_when_offsets_are_out_of_order() {
    // Three strings; the table lists them out of file order (string 2
    // is stored after string 3), as StarEdit writes them.  Ending a
    // string at the next entry's offset made string 2 empty.
    let mut str_ = Vec::new();
    str_.extend_from_slice(&3u16.to_le_bytes());
    let base = 2 + 3 * 2;
    let one = b"Name\0";
    let three = b"staredit\\wav\\combeep0.wav\0";
    let two = b"Jim Raynor: Move out!\0";
    let o1 = base;
    let o3 = o1 + one.len();
    let o2 = o3 + three.len();
    for o in [o1, o2, o3] {
        str_.extend_from_slice(&(o as u16).to_le_bytes());
    }
    str_.extend_from_slice(one);
    str_.extend_from_slice(three);
    str_.extend_from_slice(two);
    let mut chk = tiny_chk();
    chk.extend_from_slice(b"STR ");
    chk.extend_from_slice(&(str_.len() as u32).to_le_bytes());
    chk.extend_from_slice(&str_);
    let s = Scenario::parse(&chk).unwrap();
    assert_eq!(s.string(1), Some("Name"));
    assert_eq!(s.string(2), Some("Jim Raynor: Move out!"));
    assert_eq!(s.string(3), Some("staredit\\wav\\combeep0.wav"));
    assert_eq!(s.string(0), None);
    assert_eq!(s.string(4), None);
    // SPRP names the title/description strings (here 2 and 1).
    assert_eq!(s.title(), None, "no SPRP yet");
    chk.extend_from_slice(b"SPRP");
    chk.extend_from_slice(&4u32.to_le_bytes());
    chk.extend_from_slice(&2u16.to_le_bytes());
    chk.extend_from_slice(&1u16.to_le_bytes());
    let s = Scenario::parse(&chk).unwrap();
    assert_eq!(s.title(), Some("Jim Raynor: Move out!"));
    assert_eq!(s.description(), Some("Name"));
}

#[test]
fn strx_strings_use_32_bit_offsets() {
    let mut strx = Vec::new();
    let one = b"Name\0";
    let two = b"Briefing\0";
    strx.extend_from_slice(&2u32.to_le_bytes());
    let base = 4 + 2 * 4;
    strx.extend_from_slice(&(base as u32).to_le_bytes());
    strx.extend_from_slice(&((base + one.len()) as u32).to_le_bytes());
    strx.extend_from_slice(one);
    strx.extend_from_slice(two);
    let mut chk = tiny_chk();
    chk.extend_from_slice(b"STRx");
    chk.extend_from_slice(&(strx.len() as u32).to_le_bytes());
    chk.extend_from_slice(&strx);
    chk.extend_from_slice(b"SPRP");
    chk.extend_from_slice(&4u32.to_le_bytes());
    chk.extend_from_slice(&2u16.to_le_bytes());
    chk.extend_from_slice(&1u16.to_le_bytes());
    let s = Scenario::parse(&chk).unwrap();
    assert_eq!(s.string(1), Some("Name"));
    assert_eq!(s.string(2), Some("Briefing"));
    assert_eq!(s.title(), Some("Briefing"));
    assert_eq!(s.description(), Some("Name"));
}

#[test]
fn parses_dimensions_and_tiles() {
    let s = Scenario::parse(&tiny_chk()).unwrap();
    assert_eq!((s.width, s.height), (2, 2));
    assert_eq!(s.era, 1);
    assert_eq!(s.tiles, vec![32; 4]);
}

#[test]
fn protected_map_sections_are_recovered() {
    // Distributed ladder maps are wrapped by tools that scramble section
    // order, rename optional sections and replace the leading header with
    // junk.  The sections StarCraft needs are intact; the parser must
    // resynchronise instead of failing on the first damaged header.
    let mut out = Vec::new();
    out.extend_from_slice(b"JUNK");
    out.extend_from_slice(&u32::MAX.to_le_bytes()); // impossible size
    let push = |out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(name);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
    };
    let mut dim = Vec::new();
    dim.extend_from_slice(&2u16.to_le_bytes());
    dim.extend_from_slice(&2u16.to_le_bytes());
    push(&mut out, b"UPGx", &[0u8; 198]); // scrambled optional section
    let mut unit = vec![0u8; 36];
    unit[8..10].copy_from_slice(&37u16.to_le_bytes()); // SCV
    unit[18] = 0;
    push(&mut out, b"UNIT", &unit);
    // Remastered stores flags in the high bits of ERA; the low three bits
    // select the tileset.
    push(&mut out, b"ERA ", &0xF1u16.to_le_bytes());
    push(&mut out, b"DIM ", &dim);
    let mut mtxm = Vec::new();
    for _ in 0..4 {
        mtxm.extend_from_slice(&32u16.to_le_bytes());
    }
    push(&mut out, b"MTXM", &mtxm);
    push(&mut out, b"MASK", &[1, 1, 0, 1]);
    let mut trig = vec![0u8; 2400];
    trig[15] = 22; // always
    trig[320 + 26] = 1; // victory
    trig[2372] = 1;
    push(&mut out, b"TRIG", &trig);

    let s = Scenario::parse(&out).unwrap();
    assert_eq!((s.width, s.height), (2, 2));
    assert_eq!(s.era, 1, "ERA high bits are flags, low bits the tileset");
    assert_eq!(s.tiles, vec![32; 4]);
    assert_eq!(s.units.len(), 1);
    assert_eq!(s.units[0].unit_id, 37);
    assert_eq!(s.triggers.len(), 2400);
}

#[test]
fn cv5_resolves_the_megatile_whose_vf4_flags_apply() {
    // tiny_chk's tile is 32 = group 2, index 0.  The cv5 maps group 2
    // index 0 to megatile 1; only megatile 1's record is walkable.
    let s = Scenario::parse(&tiny_chk()).unwrap();
    let mut cv5 = vec![0u8; 52 * 3];
    cv5[2 * 52 + 20] = 1;
    let mut vf4 = vec![0u8; 96];
    for i in 0..16 {
        vf4[32 + i * 2] = 1;
    }
    let tiles = TileData::new(&cv5, &vf4);
    assert_eq!(tiles.megatile(32), 1);
    assert!(Terrain::from_scenario(&s, &tiles).walkable_px(0, 0));
    // Indexing by the group (record 2) would read the unwalkable record.
    assert!(!Terrain::from_scenario(&s, &TileData::synthetic(&vf4)).walkable_px(0, 0));
}

#[test]
fn vf4_flags_define_walkability() {
    let s = Scenario::parse(&tiny_chk()).unwrap();
    // Megatile 2 (tile 32 >> 4) is at record offset 64.  Make every
    // minitile walkable in the first entry and unbuildable otherwise.
    let mut vf4 = vec![0u8; 96];
    for i in 0..16 {
        vf4[64 + i * 2] = 1;
    }
    let t = Terrain::from_scenario(&s, &TileData::synthetic(&vf4));
    assert!(t.walkable_px(0, 0));
    assert!(t.walkable_px(63, 63));
    assert!(t.buildable_px(0, 0));
}

#[test]
fn open_terrain_is_fully_open() {
    let t = Terrain::open(4, 4);
    assert!(t.walkable_px(0, 0));
    assert!(t.walkable_px(127, 127));
    assert!(!t.walkable_px(128, 0));
}

#[test]
fn fog_mask_does_not_veto_terrain_buildability() {
    // tiny_chk's tile is megatile group 2, record 64.  Make all 16
    // minitiles walkable; a clear MASK cell must not disable them.
    let mut s = Scenario::parse(&tiny_chk()).unwrap();
    s.mask = vec![0; 4];
    let mut vf4 = vec![0u8; 96];
    for i in 0..16 {
        vf4[64 + i * 2] = 1;
    }
    let t = Terrain::from_scenario(&s, &TileData::synthetic(&vf4));
    assert!(t.buildable_px(0, 0));
    assert!(t.buildable_px(31, 31));
    assert!(t.buildable_px(63, 63));
}

#[test]
fn unis_unix_upgs_tecs_and_puni_parse_at_their_retail_offsets() {
    let mut chk = tiny_chk();
    let mut unis = vec![1u8; 0xFD0];
    unis[0] = 0;
    unis[0x0E4..0x0E8].copy_from_slice(&5120u32.to_le_bytes());
    unis[0x474..0x476].copy_from_slice(&50u16.to_le_bytes());
    unis[0x63C] = 2;
    unis[0x720..0x722].copy_from_slice(&300u16.to_le_bytes());
    unis[0x8E8..0x8EA].copy_from_slice(&25u16.to_le_bytes());
    unis[0xC78..0xC7A].copy_from_slice(&5u16.to_le_bytes());
    unis[0xE40..0xE42].copy_from_slice(&8u16.to_le_bytes());
    unis[0xE40 + 100 * 2..0xE40 + 100 * 2 + 2].copy_from_slice(&2u16.to_le_bytes());
    chk.extend_from_slice(b"UNIS");
    chk.extend_from_slice(&(unis.len() as u32).to_le_bytes());
    chk.extend_from_slice(&unis);

    let mut unix = vec![1u8; 0x1048];
    unix[0] = 0;
    unix[0xE40 + 129 * 2..0xE40 + 129 * 2 + 2].copy_from_slice(&77u16.to_le_bytes());
    unix[0xE40 + 130 * 2 + 129 * 2..0xE40 + 130 * 2 + 129 * 2 + 2]
        .copy_from_slice(&9u16.to_le_bytes());
    chk.extend_from_slice(b"UNIx");
    chk.extend_from_slice(&(unix.len() as u32).to_le_bytes());
    chk.extend_from_slice(&unix);

    let mut upgs = vec![1u8; 0x256];
    upgs[7] = 0;
    upgs[0x02E + 2 * 7..0x02E + 2 * 7 + 2].copy_from_slice(&100u16.to_le_bytes());
    upgs[0x08A + 2 * 7..0x08A + 2 * 7 + 2].copy_from_slice(&10u16.to_le_bytes());
    chk.extend_from_slice(b"UPGS");
    chk.extend_from_slice(&(upgs.len() as u32).to_le_bytes());
    chk.extend_from_slice(&upgs);

    let mut tecs = vec![1u8; 0xD8];
    tecs[0] = 0;
    tecs[0x18..0x1A].copy_from_slice(&150u16.to_le_bytes());
    tecs[0xA8..0xAA].copy_from_slice(&50u16.to_le_bytes());
    chk.extend_from_slice(b"TECS");
    chk.extend_from_slice(&(tecs.len() as u32).to_le_bytes());
    chk.extend_from_slice(&tecs);

    let mut puni = vec![1u8; 0x1644];
    puni[37] = 0; // player 0, unit 37 denied
    puni[0xB94 + 37] = 0; // but this player uses the per-player byte
    chk.extend_from_slice(b"PUNI");
    chk.extend_from_slice(&(puni.len() as u32).to_le_bytes());
    chk.extend_from_slice(&puni);

    let s = Scenario::parse(&chk).unwrap();
    let u = s.unis.expect("UNIS");
    assert_eq!(u.use_default[0], 0);
    assert_eq!(u.hit_points[0], 5120);
    assert_eq!(u.shields[0], 50);
    assert_eq!(u.armor[0], 2);
    assert_eq!(u.build_time[0], 300);
    assert_eq!(u.mineral_cost[0], 25);
    assert_eq!(u.string_id[0], 5);
    assert_eq!(u.weapon_damage[0], 8);
    assert_eq!(u.weapon_bonus[0], 2);
    let x = s.unix.expect("UNIx");
    assert_eq!(x.weapon_damage.len(), 130);
    assert_eq!(x.weapon_damage[129], 77);
    assert_eq!(x.weapon_bonus[129], 9);
    assert_eq!(s.upgs.as_ref().unwrap().mineral_base[7], 100);
    assert_eq!(s.tecs.as_ref().unwrap().mineral_cost[0], 150);
    assert_eq!(s.tecs.as_ref().unwrap().energy_cost[0], 50);
    assert!(!s.puni.as_ref().unwrap().available(0, 37));
    assert!(s.puni.as_ref().unwrap().available(1, 37));
}

#[test]
fn ptec_ptex_upgr_and_pupx_parse_per_player_blocks() {
    let mut chk = tiny_chk();
    let mut ptec = vec![1u8; 0x390];
    ptec[0] = 0; // player 0 tech 0 unavailable
    ptec[24] = 1; // player 0 tech 0 researched
    ptec[0x270] = 0; // player 0 tech 0 uses its own byte
    chk.extend_from_slice(b"PTEC");
    chk.extend_from_slice(&(ptec.len() as u32).to_le_bytes());
    chk.extend_from_slice(&ptec);

    let mut ptex = vec![1u8; 0x688];
    ptex[43] = 0; // player 0 tech 43 unavailable
    ptex[0x478 + 43] = 0; // use the per-player byte
    chk.extend_from_slice(b"PTEx");
    chk.extend_from_slice(&(ptex.len() as u32).to_le_bytes());
    chk.extend_from_slice(&ptex);

    let mut upgr = vec![1u8; 0x6D4];
    upgr[7] = 2; // player 0 upgrade 7 max
    upgr[0x228 + 7] = 1; // player 0 upgrade 7 start
    upgr[0x4AC + 7] = 0; // use per-player
    chk.extend_from_slice(b"UPGR");
    chk.extend_from_slice(&(upgr.len() as u32).to_le_bytes());
    chk.extend_from_slice(&upgr);

    let mut pupx = vec![1u8; 0x90E];
    pupx[60] = 4;
    pupx[0x2DC + 60] = 3;
    pupx[0x632 + 60] = 0;
    chk.extend_from_slice(b"PUPx");
    chk.extend_from_slice(&(pupx.len() as u32).to_le_bytes());
    chk.extend_from_slice(&pupx);

    let s = Scenario::parse(&chk).unwrap();
    let t = s.ptec.as_ref().unwrap();
    assert!(!t.available(0, 0));
    assert!(t.researched(0, 0));
    assert!(t.available(1, 0));
    let tx = s.ptex.as_ref().unwrap();
    assert!(!tx.available(0, 43));
    assert!(tx.available(0, 42));
    let u = s.upgr.as_ref().unwrap();
    assert_eq!(u.max_level(0, 7), 2);
    assert_eq!(u.start_level(0, 7), 1);
    let ux = s.pupx.as_ref().unwrap();
    assert_eq!(ux.max_level(0, 60), 4);
    assert_eq!(ux.start_level(0, 60), 3);
}

#[test]
fn cv5_buildability_uses_the_byte_at_offset_two_and_ignores_creep_bit() {
    let mut cv5 = vec![0u8; 52 * 2];
    cv5[52 + 2] = 0x80;
    for slot in 0..16 {
        let at = 52 + 20 + slot * 2;
        cv5[at..at + 2].copy_from_slice(&7u16.to_le_bytes());
    }
    let tiles = TileData::new(&cv5, &[]);
    assert!(!tiles.unbuildable(0x0000));
    assert!(tiles.unbuildable(0x0010));
    assert!(tiles.unbuildable(0x8010), "bit 15 is the creep marker");
    assert_eq!(tiles.megatile(0x8010), 7);
}

#[test]
fn thg2_records_are_ten_bytes_and_keep_their_flags() {
    let mut chk = tiny_chk();
    let mut thg2 = Vec::new();
    for (id, flags) in [(0xcdu16, 0x6280u16), (0x00cbu16, 0xa280u16)] {
        thg2.extend_from_slice(&id.to_le_bytes());
        thg2.extend_from_slice(&320u16.to_le_bytes());
        thg2.extend_from_slice(&320u16.to_le_bytes());
        thg2.push(0);
        thg2.push(0);
        thg2.extend_from_slice(&flags.to_le_bytes());
    }
    chk.extend_from_slice(b"THG2");
    chk.extend_from_slice(&(thg2.len() as u32).to_le_bytes());
    chk.extend_from_slice(&thg2);
    let s = Scenario::parse(&chk).unwrap();
    assert_eq!(s.thingies.len(), 2);
    assert_eq!(s.thingies[0].unit_id, 0xcd);
    assert_eq!(s.thingies[0].flags, 0x6280);
    assert_eq!(s.thingies[1].unit_id, 0xcb);
    assert_eq!(s.thingies[1].flags, 0xa280);
}

#[test]
fn rect_in_bounds_matches_the_retail_margin() {
    let t = Terrain::open(64, 64);
    assert!(t.rect_in_bounds((0, 0, 32, 32)));
    assert!(!t.rect_in_bounds((-1, 0, 32, 32)));
    assert!(!t.rect_in_bounds((0, 0, 64 * 32, 32)));
    assert!(!t.rect_in_bounds((0, 0, 32, 64 * 32 - 1)));
}

#[test]
fn rect_upper_in_bounds_matches_the_second_retail_check() {
    let t = Terrain::open(64, 64);
    assert!(t.rect_upper_in_bounds((-100, 0, 32, 32)));
    assert!(!t.rect_upper_in_bounds((0, -1, 32, 32)));
    assert!(!t.rect_upper_in_bounds((0, 0, 64 * 32, 32)));
    assert!(!t.rect_upper_in_bounds((0, 0, 32, 64 * 32 - 1)));
}
