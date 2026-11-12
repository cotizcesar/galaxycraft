//! `impl World`: projectiles (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// Create one travelling bullet.  The same routine handles every weapon
    /// behaviour; `Projectile::state` selects the per-tick movement rule.
    pub(super) fn spawn_projectile(
        &mut self,
        attacker: usize,
        target: usize,
        weapon_id: u8,
        weapon: &WeaponType,
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let a = &self.units[attacker];
        let (ax, ay) = (a.x, a.y);
        let facing = a.facing as i32;
        let (dx, dy) = direction_vector(facing);
        let forward = weapon.forward_offset as i32 * fp(1);
        let mut px = ax + dx * forward / 256;
        let mut py = ay + dy * forward / 256;
        let target_unit = self.units.get(target).map(|u| (u.id, u.x, u.y));
        let Some((target_id, tx, ty)) = target_unit else {
            return;
        };
        let mut state = match weapon.bullet_type {
            behavior::FLY => BulletState::Fly,
            behavior::FOLLOW_TARGET => BulletState::Follow,
            behavior::BOUNCE => BulletState::Bounce,
            behavior::ATTACK_TARGET_POS => BulletState::HitNearTarget,
            behavior::EXTEND_TO_MAX_RANGE => BulletState::Extend,
            behavior::PERSIST_AT_TARGET_POS => BulletState::Persist,
            behavior::APPEAR_AT_SOURCE_UNIT => BulletState::Fly,
            _ => return,
        };
        let mut target_pos = (tx, ty);
        let mut hit_flags = 0u8;
        if self.units[attacker].status.hallucination {
            hit_flags |= 2;
        }
        // Miss chance.  Ground units under Dark Swarm are almost impossible
        // to hit; a target on higher ground or behind cover also has a chance
        // to dodge.  The roll is deterministic (the world's LCG).
        if matches!(
            weapon.bullet_type,
            behavior::FLY | behavior::FOLLOW_TARGET | behavior::BOUNCE
        ) && self.target_dodges(attacker, target)
        {
            hit_flags |= 1;
            target_pos.0 -= dx * 30 / 256;
            target_pos.1 -= dy * 30 / 256;
        }
        match weapon.bullet_type {
            behavior::PERSIST_AT_TARGET_POS => {
                px = tx;
                py = ty;
                state = BulletState::Persist;
            }
            behavior::ATTACK_TARGET_POS => {
                let off = self.projectile_near_offset(attacker);
                px = ax;
                py = ay;
                target_pos = (tx + off.0, ty + off.1);
            }
            behavior::EXTEND_TO_MAX_RANGE => {
                let range = weapon.max_range as i32 + 20;
                target_pos = (ax + dx * range * fp(1) / 256, ay + dy * range * fp(1) / 256);
            }
            behavior::APPEAR_AT_SOURCE_UNIT => {
                px = ax;
                py = ay;
            }
            _ => {}
        }
        let bounces = if weapon.bullet_type == behavior::BOUNCE {
            3
        } else {
            0
        };
        let speed = self.data.weapon_speed(weapon).max(256);
        let _ = state;
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner: self.units[attacker].owner,
            source: self.units[attacker].id,
            weapon: weapon_id,
            behavior: weapon.bullet_type,
            hit_type: weapon.explosion,
            state: match weapon.bullet_type {
                behavior::FLY => BulletState::Fly,
                behavior::FOLLOW_TARGET => BulletState::Follow,
                behavior::BOUNCE => BulletState::Bounce,
                behavior::ATTACK_TARGET_POS => BulletState::HitNearTarget,
                behavior::EXTEND_TO_MAX_RANGE => BulletState::Extend,
                behavior::PERSIST_AT_TARGET_POS => BulletState::Persist,
                behavior::APPEAR_AT_SOURCE_UNIT => BulletState::Fly,
                _ => BulletState::Fly,
            },
            x: px,
            y: py,
            target: Some(target_id),
            target_pos,
            speed,
            remaining_time: weapon.lifetime.max(1) as i32,
            bounces_left: bounces,
            hit_flags,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    /// Create a stationary `bullet_type` 3 field at a point.  Psionic Storm
    /// uses this path (`0x568a5a`); `0x568aa5` re-applies its radial hit
    /// every seven updates for the weapon's 60-update lifetime.
    /// A bullet that flies from `source` to a point on the ground and goes
    /// off there (EMP Shockwave).
    pub(super) fn spawn_point_bullet(
        &mut self,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        target: (i32, i32),
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let (owner, source_id, x, y) = {
            let u = &self.units[source];
            (u.owner, u.id, u.x, u.y)
        };
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner,
            source: source_id,
            weapon: weapon_id,
            behavior: behavior::FLY,
            hit_type: weapon.explosion,
            state: BulletState::Fly,
            x,
            y,
            target: None,
            target_pos: target,
            speed: self.data.weapon_speed(weapon).max(256),
            remaining_time: 300,
            bounces_left: 0,
            hit_flags: 0,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    pub(super) fn spawn_persistent_field(
        &mut self,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
    ) {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return;
        }
        let (owner, source_id) = {
            let u = &self.units[source];
            (u.owner, u.id)
        };
        let bullet = Projectile::persistent(
            self.next_projectile_id,
            owner,
            source_id,
            weapon_id,
            weapon,
            pos,
            self.tick as i64,
        );
        self.projectiles.push(bullet);
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    pub(super) fn projectile_near_offset(&mut self, _attacker: usize) -> (i32, i32) {
        // OpenBW `hit_near_target_positions`: bullets of type "attack
        // target position" (Halo Rockets, Corrosive Acid) aim at the target
        // plus the next of these 14 offsets, so a volley spreads around it.
        // (Retail keeps the index per shooter; the bullet counter cycles
        // through the same table.)
        const NEAR: [(i32, i32); 14] = [
            (-1, -1),
            (-51, -31),
            (-7, -12),
            (36, 52),
            (49, -25),
            (-40, 46),
            (4, 19),
            (-28, -50),
            (-50, 15),
            (54, 13),
            (-11, 53),
            (-15, 9),
            (19, -50),
            (17, -8),
        ];
        let (x, y) = NEAR[(self.next_projectile_id % 14) as usize];
        (x * fp(1), y * fp(1))
    }

    pub(super) fn update_projectiles(&mut self) {
        if self.projectiles.is_empty() {
            return;
        }
        let bullets = std::mem::take(&mut self.projectiles);
        let mut kept = Vec::with_capacity(bullets.len());
        for mut b in bullets {
            if self.step_projectile(&mut b) {
                kept.push(b);
            }
        }
        self.projectiles = kept;
    }

    /// Advance one bullet.  Returns false when it has been consumed.
    pub(super) fn step_projectile(&mut self, b: &mut Projectile) -> bool {
        let Some(weapon) = self.data.weapon(b.weapon).cloned() else {
            return false;
        };
        if b.state == BulletState::Persist {
            b.remaining_time -= 1;
            if b.remaining_time <= 0 {
                return false;
            }
            if self.tick as i64 - b.last_dot_tick >= PERSIST_DAMAGE_INTERVAL {
                b.last_dot_tick = self.tick as i64;
                self.resolve_hit_type(
                    b.owner,
                    b.target,
                    self.index_of(b.source).unwrap_or(usize::MAX),
                    b.weapon,
                    &weapon,
                    b.target_pos,
                    1,
                    1,
                );
            }
            return true;
        }
        if b.state == BulletState::Follow
            && let Some(tid) = b.target
        {
            if let Some(t) = self.unit(tid) {
                if t.alive {
                    b.target_pos = (t.x, t.y);
                } else {
                    b.target = None;
                    b.state = BulletState::Fly;
                }
            } else {
                b.target = None;
                b.state = BulletState::Fly;
            }
        }
        if b.state == BulletState::Bounce
            && let Some(tid) = b.target
            && let Some(t) = self.unit(tid)
            && t.alive
        {
            b.target_pos = (t.x, t.y);
        }
        b.remaining_time -= 1;
        let dx = b.target_pos.0 - b.x;
        let dy = b.target_pos.1 - b.y;
        let dist = isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)) as i32;
        if dist <= b.speed || b.remaining_time <= 0 {
            b.x = b.target_pos.0;
            b.y = b.target_pos.1;
            return self.projectile_impact(b, &weapon);
        }
        b.x += (dx as i64 * b.speed as i64 / dist as i64) as i32;
        b.y += (dy as i64 * b.speed as i64 / dist as i64) as i32;
        true
    }

    pub(super) fn projectile_impact(&mut self, b: &mut Projectile, weapon: &WeaponType) -> bool {
        // A missed shot still produces its hit animation in the retail game
        // but deals no damage.  Hallucinated attackers likewise produce a fake
        // impact with no damage.
        if b.hit_flags & 3 != 0 {
            return false;
        }
        let source_idx = self.index_of(b.source).unwrap_or(usize::MAX);
        if b.state == BulletState::Bounce {
            if b.bounces_left > 0
                && b.hit_flags & 1 == 0
                && let Some(tid) = b.target
            {
                let divisor = 3u32.saturating_pow(3 - b.bounces_left as u32);
                self.resolve_hit_type(
                    b.owner,
                    Some(tid),
                    source_idx,
                    b.weapon,
                    weapon,
                    (b.x, b.y),
                    divisor,
                    1,
                );
                if b.bounces_left > 1
                    && let Some(next) = self.find_bounce_target(b, tid)
                {
                    b.previous_target = Some(tid);
                    b.target = Some(next);
                    b.bounces_left -= 1;
                    b.remaining_time = b.remaining_time.max(30);
                    b.state = BulletState::Bounce;
                    return true;
                }
            }
            return false;
        }
        let divisor = 1;
        self.resolve_hit_type(
            b.owner,
            b.target,
            source_idx,
            b.weapon,
            weapon,
            (b.x, b.y),
            divisor,
            1,
        );
        false
    }

    pub(super) fn find_bounce_target(&self, b: &Projectile, current: u32) -> Option<u32> {
        let mut best: Option<(i32, u32)> = None;
        for u in &self.units {
            if !u.alive
                || !self.is_enemy(b.owner, u.owner)
                || u.id == current
                || Some(u.id) == b.previous_target
            {
                continue;
            }
            if !self.can_see_unit(b.owner, self.index_of(u.id)?) {
                continue;
            }
            let dx = (u.x - b.x) as i64;
            let dy = (u.y - b.y) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            if d <= 96 * fp(1) && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, u.id));
            }
        }
        best.map(|(_, id)| id)
    }
}
