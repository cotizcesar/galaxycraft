//! Building effects support: lift-off / landing dust LO offsets
//! (`images.dat` landing-dust and lift-off overlays) read from the install
//! for the animator, and the `site` script step.

use super::*;

impl App {
    /// Give the animator the dust offsets of buildings that are lifting,
    /// flying or landing (once per image).
    pub(super) fn feed_dust_offsets(&mut self) {
        let _ = self.image_table();
        let Some(install) = self.install.as_ref() else {
            return;
        };
        let names = self.image_names.as_deref().unwrap_or(&[]);
        let read = |index: u32| -> Vec<(u8, i8, i8)> {
            if index == 0 {
                return Vec::new();
            }
            names
                .get(index as usize - 1)
                .and_then(|n| install.read(&format!("unit/{}", n.replace('\\', "/"))))
                .and_then(|b| galaxycraft::unit_anim::parse_lo_slots(&b))
                .unwrap_or_default()
        };
        for u in self
            .world
            .units
            .iter()
            .filter(|u| u.alive && (u.lifted || u.lift_phase.is_some()))
        {
            let Some(image) = galaxycraft::unit_anim::unit_image(&self.data, u.type_id) else {
                continue;
            };
            if !self.animator.needs_dust_offsets(image) {
                continue;
            }
            let Some(im) = self.data.image(image) else {
                continue;
            };
            let (landing, lift) = (
                read(im.landing_dust_filename_index),
                read(im.lift_off_filename_index),
            );
            self.animator.set_dust_offsets(image, landing, lift);
        }
    }

    /// A building of `ty` under construction at `pct` % with no builder,
    /// so it holds that stage (`--game-script` `site:`).
    pub(super) fn spawn_site(&mut self, owner: i32, ty: i32, at: (i32, i32), pct: i32) {
        let Ok(idx) = self
            .world
            .spawn_px(owner.max(0) as usize, ty.max(0) as u16, at.0, at.1)
        else {
            return;
        };
        let total = galaxycraft::sim::build_ticks(self.data.unit(ty.max(0) as u16).build_time);
        let u = &mut self.world.units[idx];
        u.a13_clear_completed_flag();
        u.build_total = total;
        u.build_remaining = total * (100 - pct.clamp(0, 99)) / 100;
    }

    /// Effects drawn mirrored (the second half of the building dust).
    pub(super) fn mirrored_effects(&self) -> Vec<(u16, i32, i32)> {
        self.animator
            .effects()
            .filter(|e| e.flip)
            .map(|e| (e.image, e.x, e.y))
            .collect()
    }

    /// First `playfram` of an image's animation (the AlmostBuilt frame).
    pub(super) fn first_frame_of(&self, image: u16, anim: usize) -> Option<usize> {
        let script = self.data.image(image)?.iscript_id as u16;
        let mut st = self.data.iscript.start_animation(script, anim, 0)?;
        let mut ctx = galaxycraft::iscript::AnimContext::default();
        for _ in 0..4 {
            for e in self.data.iscript.tick_anim(&mut st, &mut ctx) {
                if let galaxycraft::iscript::AnimEvent::Frame { raw, .. } = e {
                    return Some(raw as usize);
                }
            }
        }
        None
    }
}
