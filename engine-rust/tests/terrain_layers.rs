//! Terrain must be opaque: the CV5 group background is drawn under each
//! megatile, so no map tile may leave holes in the framebuffer.
//!
//! Regression for the "black rectangles in the terrain" bug: before the
//! two-layer fix, 2 of 698 distinct tiles of Boxer.chk drew nothing and
//! dozens were only 30-90% covered.

use galaxycraft::map::Scenario;
use galaxycraft::render::{Framebuffer, Rgb, Tileset};
use std::collections::BTreeSet;

#[test]
fn every_shipped_map_tile_is_fully_opaque() {
    let raw = std::fs::read("assets/maps/(2)Challenger.chk").expect("bundled badlands map");
    let sc = Scenario::parse(&raw).expect("parse");
    let ts = Tileset::load(std::path::Path::new("assets/TileSet"), sc.era, &sc)
        .expect("bundled tileset");

    let distinct: BTreeSet<u16> = sc.tiles.iter().copied().collect();
    assert!(distinct.len() > 100, "expected a varied map");

    let mut holes = Vec::new();
    for &tile in &distinct {
        let mut fb = Framebuffer::new(32, 32);
        fb.clear(Rgb::new(0, 0, 0));
        ts.draw_tile(&mut fb, tile, 0, 0);
        let mut painted = 0;
        for y in 0..32 {
            for x in 0..32 {
                if fb.get(x, y).0 != 0 {
                    painted += 1;
                }
            }
        }
        if painted < 1024 {
            holes.push((tile, painted));
        }
    }
    assert!(
        holes.is_empty(),
        "tiles with transparent holes (tile, painted): {holes:?}"
    );
}

#[test]
fn the_retail_colour_cycles_rotate_the_terrain_palette() {
    let raw = std::fs::read("assets/maps/(2)Challenger.chk").expect("bundled badlands map");
    let sc = Scenario::parse(&raw).expect("parse");
    let mut ts = Tileset::load(std::path::Path::new("assets/TileSet"), sc.era, &sc)
        .expect("bundled tileset");
    let before: Vec<Rgb> = ts.palette.colors[1..=13].to_vec();
    // Badlands rotates 1..6, 7..13 and 248..254 every three game ticks.
    ts.update_color_cycle(3, true);
    assert_eq!(ts.palette.colors[2], before[0]);
    assert_eq!(ts.palette.colors[1], before[5], "6 wraps to 1");
    assert_eq!(ts.palette.colors[8], before[6], "second range rotates");
    // Turning the option off restores the base palette exactly.
    ts.update_color_cycle(6, false);
    assert_eq!(ts.palette.colors[1..=13], before[..]);
}

#[test]
fn platforms_and_installations_do_not_cycle_colours() {
    let raw = std::fs::read("assets/maps/(2)Space Madness.chk").expect("bundled map");
    let sc = Scenario::parse(&raw).expect("parse");
    assert_eq!(sc.era, 1, "Space Madness is a platform map");
    let mut ts = Tileset::load(std::path::Path::new("assets/TileSet"), sc.era, &sc)
        .expect("bundled tileset");
    let before = ts.palette.colors;
    ts.update_color_cycle(300, true);
    assert_eq!(ts.palette.colors, before);
}
