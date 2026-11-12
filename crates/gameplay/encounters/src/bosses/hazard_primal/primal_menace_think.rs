//! Crystal the_primal_menace.lua:156-347 (onThink): beast and pod waves and beast decay.
use oteryn_gameplay_entities::MovementLocalPosition;

pub use oteryn_gameplay_content::content::bosses::primal_closure::{
    FUNGOSAURUS_KEY, PRIMAL_BEAST_POOL, PRIMAL_MENACE_KEY,
};
pub const SPAWN_RADIUS_TILES: u32 = 5;
pub const SPAWN_ATTEMPTS: u8 = 6;
pub const SPAWN_WARMUP_MS: u64 = 3_000;
pub const BEAST_LIFETIME_MS: u64 = 20_000;
pub const FIRST_BEAST_WAVE_MS: u64 = 10_000;
pub const FIRST_POD_WAVE_MS: u64 = 20_000;
pub const DRAW_PPM: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveConfig {
    pub interval_base_ms: f64,
    pub per_tenth_missing: f64,
    pub per_hazard: f64,
    pub count_base: f64,
    pub variance_rate: f64,
    pub growth_per_hazard: f64,
    pub count_max: u32,
}

pub const BEAST_WAVE: WaveConfig = WaveConfig {
    interval_base_ms: 30_000.0,
    per_tenth_missing: 0.98,
    per_hazard: 0.985,
    count_base: 4.0,
    variance_rate: 0.5,
    growth_per_hazard: 1.05,
    count_max: 6,
};
pub const POD_WAVE: WaveConfig = WaveConfig {
    interval_base_ms: 30_000.0,
    per_tenth_missing: 0.98,
    per_hazard: 0.985,
    count_base: 2.0,
    variance_rate: 0.5,
    growth_per_hazard: 1.1,
    count_max: 4,
};

/// hazard.lua:30-50: minimum level among damage-map players, else 1.
#[must_use]
pub fn hazard_points(damage_map_levels: &[u16]) -> u16 {
    damage_map_levels.iter().copied().min().unwrap_or(1)
}

/// :180-193, floored to whole ms. `max_health == 0` counts as no missing health.
#[must_use]
pub fn wave_interval_ms(config: &WaveConfig, health: u64, max_health: u64, hazard: u16) -> u64 {
    let missing_tenths = if max_health == 0 {
        0
    } else {
        (((max_health.saturating_sub(health)) as f64 / max_health as f64) * 10.0).floor() as i32
    };
    let interval = config.interval_base_ms
        * config.per_tenth_missing.powi(missing_tenths)
        * config.per_hazard.powi(i32::from(hazard));
    interval.floor().clamp(0.0, u64::MAX as f64) as u64
}

/// :195-206. Both draws are in `0..DRAW_PPM`; sign is negative below half.
#[must_use]
pub fn wave_count(config: &WaveConfig, hazard: u16, variance_ppm: u32, sign_ppm: u32) -> u32 {
    let variance = f64::from(variance_ppm) / f64::from(DRAW_PPM) * config.variance_rate;
    let signed_variance = if sign_ppm < DRAW_PPM / 2 {
        -variance
    } else {
        variance
    };
    let count = (config.count_base * config.growth_per_hazard.powi(i32::from(hazard))
        + signed_variance)
        .floor()
        .clamp(0.0, f64::from(u32::MAX)) as u32;
    count.min(config.count_max)
}

/// `random(#pool)` as `ppm × 11 / 1_000_000`.
#[must_use]
pub fn pick_beast(pool_ppm: u32) -> Option<&'static str> {
    if pool_ppm >= DRAW_PPM {
        return None;
    }
    let index = (pool_ppm as usize * PRIMAL_BEAST_POOL.len()) / DRAW_PPM as usize;
    PRIMAL_BEAST_POOL.get(index).copied()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaveKind {
    Beast,
    Pod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimalMenaceEffect {
    SpawnSparkle {
        position: MovementLocalPosition,
        at_ms: u64,
    },
    ScheduledSpawn {
        kind: WaveKind,
        position: MovementLocalPosition,
        at_ms: u64,
    },
    ReplaceWithFungosaurus {
        beast: u64,
        position: MovementLocalPosition,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrimalMenaceRefusal {
    InvalidDraw,
}

#[derive(Clone, Copy, Debug)]
pub struct PrimalMenaceThinkFacts<'a> {
    pub position: MovementLocalPosition,
    pub now_ms: u64,
    pub health: u64,
    pub max_health: u64,
    pub damage_map_levels: &'a [u16],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimalMenaceThinkOwner {
    centre: Option<MovementLocalPosition>,
    next_beast_ms: u64,
    next_pod_ms: u64,
    beasts: Vec<(u64, u64)>,
}

impl PrimalMenaceThinkOwner {
    pub fn think(
        &mut self,
        facts: &PrimalMenaceThinkFacts<'_>,
        mut draw_ppm: impl FnMut() -> u32,
        tile_free: impl Fn(MovementLocalPosition) -> bool,
        beast_position: impl Fn(u64) -> Option<MovementLocalPosition>,
    ) -> Result<Vec<PrimalMenaceEffect>, PrimalMenaceRefusal> {
        let mut next = self.clone();
        if next.centre.is_none() {
            next.centre = Some(facts.position);
            next.next_beast_ms = facts.now_ms.saturating_add(FIRST_BEAST_WAVE_MS);
            next.next_pod_ms = facts.now_ms.saturating_add(FIRST_POD_WAVE_MS);
        }
        let centre = next.centre.unwrap_or(facts.position);
        let hazard = hazard_points(facts.damage_map_levels);
        let mut effects = Vec::new();

        if facts.now_ms > next.next_beast_ms {
            schedule_wave(
                &mut effects,
                facts.now_ms,
                centre,
                WaveKind::Beast,
                &BEAST_WAVE,
                hazard,
                &mut draw_ppm,
                &tile_free,
            )?;
            next.next_beast_ms = facts.now_ms.saturating_add(wave_interval_ms(
                &BEAST_WAVE,
                facts.health,
                facts.max_health,
                hazard,
            ));
        }
        if facts.now_ms > next.next_pod_ms {
            schedule_wave(
                &mut effects,
                facts.now_ms,
                centre,
                WaveKind::Pod,
                &POD_WAVE,
                hazard,
                &mut draw_ppm,
                &tile_free,
            )?;
            next.next_pod_ms = facts.now_ms.saturating_add(wave_interval_ms(
                &POD_WAVE,
                facts.health,
                facts.max_health,
                hazard,
            ));
        }

        next.beasts.retain(|&(actor, created_ms)| {
            let Some(position) = beast_position(actor) else {
                return false;
            };
            if facts.now_ms.saturating_sub(created_ms) > BEAST_LIFETIME_MS {
                effects.push(PrimalMenaceEffect::ReplaceWithFungosaurus {
                    beast: actor,
                    position,
                });
                false
            } else {
                true
            }
        });
        *self = next;
        Ok(effects)
    }

    pub fn record_beast(&mut self, actor: u64, created_ms: u64) {
        self.beasts.push((actor, created_ms));
    }

    #[must_use]
    pub fn centre(&self) -> Option<MovementLocalPosition> {
        self.centre
    }

    #[must_use]
    pub fn next_waves_ms(&self) -> (u64, u64) {
        (self.next_beast_ms, self.next_pod_ms)
    }

    #[must_use]
    pub fn beasts(&self) -> &[(u64, u64)] {
        &self.beasts
    }
}

// One call site; the wave inputs are distinct borrows and callbacks, not a reusable bundle.
#[allow(clippy::too_many_arguments)]
fn schedule_wave(
    effects: &mut Vec<PrimalMenaceEffect>,
    now_ms: u64,
    centre: MovementLocalPosition,
    kind: WaveKind,
    config: &WaveConfig,
    hazard: u16,
    draw_ppm: &mut impl FnMut() -> u32,
    tile_free: &impl Fn(MovementLocalPosition) -> bool,
) -> Result<(), PrimalMenaceRefusal> {
    let variance = valid_draw(draw_ppm)?;
    let sign = valid_draw(draw_ppm)?;
    let count = wave_count(config, hazard, variance, sign);
    for _ in 0..count {
        let position = spawn_position(centre, draw_ppm, tile_free)?;
        for delay_ms in [1_000, 2_000, SPAWN_WARMUP_MS] {
            effects.push(PrimalMenaceEffect::SpawnSparkle {
                position,
                at_ms: now_ms.saturating_add(delay_ms),
            });
        }
        effects.push(PrimalMenaceEffect::ScheduledSpawn {
            kind,
            position,
            at_ms: now_ms.saturating_add(SPAWN_WARMUP_MS),
        });
    }
    Ok(())
}

fn spawn_position(
    centre: MovementLocalPosition,
    draw_ppm: &mut impl FnMut() -> u32,
    tile_free: &impl Fn(MovementLocalPosition) -> bool,
) -> Result<MovementLocalPosition, PrimalMenaceRefusal> {
    for _ in 0..SPAWN_ATTEMPTS {
        let x_magnitude = magnitude(valid_draw(draw_ppm)?);
        let x_sign = sign(valid_draw(draw_ppm)?);
        let y_magnitude = magnitude(valid_draw(draw_ppm)?);
        let y_sign = sign(valid_draw(draw_ppm)?);
        let candidate = MovementLocalPosition {
            x: centre.x + x_sign * x_magnitude,
            y: centre.y + y_sign * y_magnitude,
            floor: centre.floor,
        };
        if tile_free(candidate) {
            return Ok(candidate);
        }
    }
    Ok(centre)
}

fn valid_draw(draw_ppm: &mut impl FnMut() -> u32) -> Result<u32, PrimalMenaceRefusal> {
    let draw = draw_ppm();
    if draw >= DRAW_PPM {
        Err(PrimalMenaceRefusal::InvalidDraw)
    } else {
        Ok(draw)
    }
}

fn magnitude(draw_ppm: u32) -> i32 {
    (draw_ppm * SPAWN_RADIUS_TILES).div_ceil(DRAW_PPM) as i32
}

fn sign(draw_ppm: u32) -> i32 {
    if draw_ppm < DRAW_PPM / 2 { -1 } else { 1 }
}

#[cfg(test)]
#[path = "primal_menace_think_tests.rs"]
mod tests;
