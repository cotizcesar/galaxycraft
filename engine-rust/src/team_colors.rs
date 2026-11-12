//! Retail team colours: the eight shades of each slot colour, read from
//! `game\tunit.pcx`, for the SD unit drawing (palette entries 8..=15).

use crate::render::{PLAYER_COLORS, Rgb};

/// Each slot colour's eight retail shades from `game\tunit.pcx` (one
/// 128 x 1 row: eight pixels per player, coloured by the PCX's own
/// palette), once the client has read it.
static TEAM_RAMPS: std::sync::OnceLock<Vec<[Rgb; 8]>> = std::sync::OnceLock::new();

/// Decode `tunit.pcx` into per-player shade ramps.
pub fn tunit_ramps(pcx: &[u8]) -> Option<Vec<[Rgb; 8]>> {
    if pcx.len() < 128 + 769 || pcx[pcx.len() - 769] != 0x0c {
        return None;
    }
    let bpl = u16::from_le_bytes([pcx[66], pcx[67]]) as usize;
    let mut row = Vec::with_capacity(bpl);
    let mut i = 128;
    while row.len() < bpl && i < pcx.len() - 769 {
        let v = pcx[i];
        i += 1;
        if v >= 0xc0 {
            let n = (v & 0x3f) as usize;
            let c = *pcx.get(i)?;
            i += 1;
            row.extend(std::iter::repeat_n(c, n));
        } else {
            row.push(v);
        }
    }
    let pal = &pcx[pcx.len() - 768..];
    let ramps = row
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| {
            c.map(|ix| {
                let o = ix as usize * 3;
                Rgb::new(pal[o], pal[o + 1], pal[o + 2])
            })
        })
        .collect::<Vec<_>>();
    (!ramps.is_empty()).then_some(ramps)
}

/// Use the retail shade ramps for the slot colours (SD unit drawing).
pub fn set_team_ramps_from_tunit(pcx: &[u8]) -> bool {
    tunit_ramps(pcx).is_some_and(|r| TEAM_RAMPS.set(r).is_ok())
}

/// Read the retail shades from the install, if there is one.
pub fn load(install: Option<&crate::retail::RetailInstall>) {
    if let Some(pcx) = install.and_then(|i| i.read_by_path("game/tunit.pcx").ok()) {
        set_team_ramps_from_tunit(&pcx);
    }
}

/// Shade `k` (0 = brightest) of a slot colour, once the ramps are loaded.
pub fn shade(team: Rgb, k: u8) -> Option<Rgb> {
    let p = PLAYER_COLORS.iter().position(|c| *c == team)?;
    TEAM_RAMPS.get()?.get(p).map(|r| r[k as usize & 7])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunit_rows_decode_into_eight_shades_per_player() {
        // A 16 x 1 PCX: raw bytes for player 0, an RLE run for player 1.
        let mut pcx = vec![0u8; 128];
        pcx[66] = 16;
        pcx.extend([1, 2, 3, 4, 5, 6, 7, 8]);
        pcx.extend([0xc8, 9]);
        pcx.push(0x0c);
        let mut pal = vec![0u8; 768];
        for i in 0..256 {
            pal[i * 3] = i as u8;
        }
        pcx.extend(pal);
        let r = tunit_ramps(&pcx).expect("ramps");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0][0], Rgb::new(1, 0, 0));
        assert_eq!(r[0][7], Rgb::new(8, 0, 0));
        assert_eq!(r[1], [Rgb::new(9, 0, 0); 8]);
    }
}
