use crate::presentation::{NormalizedProgram, ProgramCursor};
use crate::scene::{QualificationBundle, SceneDrawPrimitive};
use crate::visible_set::{VisibleSpriteEntry, VisibleSpriteSet};

pub const TILE_UNITS: f32 = 32.0;
pub const CARRIER_UNITS: f32 = 64.0;
const CARRIER_OVERHANG_UNITS: f32 = CARRIER_UNITS - TILE_UNITS;
const DEMO_PATH_MS: u64 = 2_400;
const DEMO_LEG_MS: u64 = DEMO_PATH_MS / 2;
const DEMO_MISSILE_MS: u64 = 800;
const DEMO_CAMERA_MS: u64 = 4_000;
const STATIC_ROLE_RANK: u8 = 0;
const OUTFIT_ROLE_RANK: u8 = 1;
const MISSILE_ROLE_RANK: u8 = 2;
const EFFECT_ROLE_RANK: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoDirection {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteRole {
    StaticWorld,
    OutfitBase,
    OutfitMask,
    Effect,
    Missile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Alpha,
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DrawOrderKey {
    pub scene_y: i32,
    pub scene_x: i32,
    pub plane: i64,
    pub order: i64,
    pub layer: usize,
    pub primitive: usize,
    pub role_rank: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldRect {
    pub x_units: f32,
    pub y_units: f32,
    pub width_units: f32,
    pub height_units: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteDraw {
    pub order_key: DrawOrderKey,
    pub role: SpriteRole,
    pub blend: BlendMode,
    pub sprite_source_id: u32,
    pub dense_index: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub rect: WorldRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemoCamera {
    pub center_x_units: f32,
    pub center_y_units: f32,
    pub zoom: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActorOverlay {
    pub name: String,
    pub anchor_x_units: f32,
    pub anchor_y_units: f32,
    pub hp_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelegraphOverlay {
    pub center_x_units: f32,
    pub center_y_units: f32,
    pub radius_units: f32,
    pub pulse: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub center_x_units: f32,
    pub center_y_units: f32,
    pub radius_units: f32,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightingState {
    pub ambient: f32,
    pub point: PointLight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemoFrame {
    pub elapsed_ms: u64,
    pub camera: DemoCamera,
    pub sprites: Vec<SpriteDraw>,
    pub actor_overlay: ActorOverlay,
    pub telegraph: TelegraphOverlay,
    pub lighting: LightingState,
    pub outfit_phase: usize,
    pub effect_phase: usize,
    pub missile_phase: usize,
    pub actor_direction: DemoDirection,
}

pub fn build_demo_frame(
    bundle: &QualificationBundle,
    visible: &VisibleSpriteSet,
    elapsed_ms: u64,
) -> Result<DemoFrame, String> {
    let actor = actor_state(elapsed_ms);
    let effect_anchor = (21.0 * TILE_UNITS, 15.0 * TILE_UNITS);
    let missile = missile_position(elapsed_ms, actor.position_units, effect_anchor);

    let outfit = bundle.programs().outfit();
    if !(1..=2).contains(&outfit.layers) {
        return Err(format!(
            "demo outfit requires 1 or 2 normalized layers, got {}",
            outfit.layers
        ));
    }
    if outfit.patterns.height == 0 || outfit.patterns.depth == 0 {
        return Err("demo outfit normalized pattern geometry is empty".to_owned());
    }
    let outfit_phase = presentation_phase(outfit, elapsed_ms)?;
    let outfit_pattern_x = direction_pattern_x(outfit, actor.direction)?;

    let effect = bundle.programs().effect();
    let missile_program = bundle.programs().missile();
    let effect_phase = presentation_phase(effect, elapsed_ms)?;
    let missile_phase = presentation_phase(missile_program, elapsed_ms)?;

    let mut sprites = Vec::with_capacity(
        bundle
            .scene()
            .primitives()
            .len()
            .saturating_add(outfit.layers)
            .saturating_add(effect.layers)
            .saturating_add(missile_program.layers),
    );

    for primitive in bundle.scene().primitives() {
        sprites.push(static_draw(primitive, visible)?);
    }

    append_program_draws(
        &mut sprites,
        outfit,
        visible,
        DynamicProgramDraw {
            phase: outfit_phase,
            pattern_z: 0,
            pattern_y: 0,
            pattern_x: outfit_pattern_x,
            position_units: actor.position_units,
            scene_tile: actor.scene_tile,
            role: SpriteRole::OutfitBase,
            role_rank: OUTFIT_ROLE_RANK,
            blend: BlendMode::Alpha,
            order: 10_000,
        },
    )?;
    if outfit.layers == 2
        && let Some(last) = sprites.last_mut()
    {
        last.role = SpriteRole::OutfitMask;
    }

    append_program_draws(
        &mut sprites,
        missile_program,
        visible,
        DynamicProgramDraw {
            phase: missile_phase,
            pattern_z: 0,
            pattern_y: 0,
            pattern_x: 0,
            position_units: missile,
            scene_tile: world_to_scene_tile(missile),
            role: SpriteRole::Missile,
            role_rank: MISSILE_ROLE_RANK,
            blend: BlendMode::Alpha,
            order: 10_001,
        },
    )?;

    append_program_draws(
        &mut sprites,
        effect,
        visible,
        DynamicProgramDraw {
            phase: effect_phase,
            pattern_z: 0,
            pattern_y: 0,
            pattern_x: 0,
            position_units: effect_anchor,
            scene_tile: world_to_scene_tile(effect_anchor),
            role: SpriteRole::Effect,
            role_rank: EFFECT_ROLE_RANK,
            blend: BlendMode::Alpha,
            order: 10_002,
        },
    )?;

    sprites.sort_by_key(|draw| draw.order_key);

    let camera = camera_state(elapsed_ms);
    let telegraph_pulse = triangle01(elapsed_ms % 1_000, 1_000);
    let ambient_cycle = triangle01(elapsed_ms % 8_000, 8_000);
    let ambient = 0.55 + 0.45 * ambient_cycle;

    Ok(DemoFrame {
        elapsed_ms,
        camera,
        sprites,
        actor_overlay: ActorOverlay {
            name: "15.32 REAL CONTENT".to_owned(),
            anchor_x_units: actor.position_units.0 + TILE_UNITS * 0.5,
            anchor_y_units: actor.position_units.1 - 4.0,
            hp_ratio: 0.73,
        },
        telegraph: TelegraphOverlay {
            center_x_units: effect_anchor.0 + TILE_UNITS * 0.5,
            center_y_units: effect_anchor.1 + TILE_UNITS * 0.5,
            radius_units: 28.0 + telegraph_pulse * 10.0,
            pulse: telegraph_pulse,
        },
        lighting: LightingState {
            ambient,
            point: PointLight {
                center_x_units: actor.position_units.0 + TILE_UNITS * 0.5,
                center_y_units: actor.position_units.1 + TILE_UNITS * 0.5,
                radius_units: 144.0,
                intensity: 0.85,
            },
        },
        outfit_phase,
        effect_phase,
        missile_phase,
        actor_direction: actor.direction,
    })
}

fn static_draw(
    primitive: &SceneDrawPrimitive,
    visible: &VisibleSpriteSet,
) -> Result<SpriteDraw, String> {
    let entry = visible_entry(visible, primitive.sprite_source_id)?;
    let tile_x = primitive.tile.scene_x as f32 * TILE_UNITS;
    let tile_y = primitive.tile.scene_y as f32 * TILE_UNITS;
    Ok(SpriteDraw {
        order_key: DrawOrderKey {
            scene_y: primitive.tile.scene_y,
            scene_x: primitive.tile.scene_x,
            plane: primitive.presentation_order.plane,
            order: primitive.presentation_order.order,
            layer: primitive.layer_index,
            primitive: primitive.primitive_index,
            role_rank: STATIC_ROLE_RANK,
        },
        role: SpriteRole::StaticWorld,
        blend: BlendMode::Alpha,
        sprite_source_id: primitive.sprite_source_id,
        dense_index: entry.dense_index,
        source_width: entry.source_width,
        source_height: entry.source_height,
        rect: carrier_rect(
            tile_x + primitive.displacement.dx_units as f32,
            tile_y + primitive.displacement.dy_units as f32,
        ),
    })
}

#[derive(Debug, Clone, Copy)]
struct DynamicProgramDraw {
    phase: usize,
    pattern_z: usize,
    pattern_y: usize,
    pattern_x: usize,
    position_units: (f32, f32),
    scene_tile: (i32, i32),
    role: SpriteRole,
    role_rank: u8,
    blend: BlendMode,
    order: i64,
}

fn append_program_draws(
    target: &mut Vec<SpriteDraw>,
    program: &NormalizedProgram,
    visible: &VisibleSpriteSet,
    request: DynamicProgramDraw,
) -> Result<(), String> {
    if request.pattern_z >= program.patterns.depth
        || request.pattern_y >= program.patterns.height
        || request.pattern_x >= program.patterns.width
    {
        return Err(format!(
            "{} demo pattern ({},{},{}) outside normalized dimensions {}x{}x{}",
            program.category,
            request.pattern_x,
            request.pattern_y,
            request.pattern_z,
            program.patterns.width,
            program.patterns.height,
            program.patterns.depth
        ));
    }
    for layer in 0..program.layers {
        let sprite_source_id = program.sprite_source_id(ProgramCursor {
            phase: request.phase,
            pattern_z: request.pattern_z,
            pattern_y: request.pattern_y,
            pattern_x: request.pattern_x,
            layer,
        })?;
        let entry = visible_entry(visible, sprite_source_id)?;
        target.push(SpriteDraw {
            order_key: DrawOrderKey {
                scene_y: request.scene_tile.1,
                scene_x: request.scene_tile.0,
                plane: i64::MAX / 4,
                order: request.order,
                layer,
                primitive: layer,
                role_rank: request.role_rank,
            },
            role: request.role,
            blend: request.blend,
            sprite_source_id,
            dense_index: entry.dense_index,
            source_width: entry.source_width,
            source_height: entry.source_height,
            rect: carrier_rect(request.position_units.0, request.position_units.1),
        });
    }
    Ok(())
}

fn visible_entry(
    visible: &VisibleSpriteSet,
    sprite_source_id: u32,
) -> Result<VisibleSpriteEntry, String> {
    visible.entry(sprite_source_id).ok_or_else(|| {
        format!(
            "demo sprite_source_id {sprite_source_id} is absent from bounded visible working set"
        )
    })
}

fn presentation_phase(program: &NormalizedProgram, elapsed_ms: u64) -> Result<usize, String> {
    if program.phase_count == 1 {
        return Ok(0);
    }
    let animation = program.animation.as_ref().ok_or_else(|| {
        format!(
            "{} animated demo program lacks normalized animation descriptor",
            program.category
        )
    })?;
    if animation.loop_type != "infinite" {
        return Err(format!(
            "{} demo program uses unsupported loop_type {}; qualification fixture only advances normalized infinite loops",
            program.category, animation.loop_type
        ));
    }
    if animation.random_start_phase {
        return Err(format!(
            "{} demo program requests random_start_phase; qualification fixture requires an explicit deterministic seed policy",
            program.category
        ));
    }
    program.phase_in_forward_cycle(elapsed_ms, animation.default_start_phase)
}

fn direction_pattern_x(
    program: &NormalizedProgram,
    direction: DemoDirection,
) -> Result<usize, String> {
    match program.patterns.width {
        1 => {
            if direction == DemoDirection::South {
                Ok(0)
            } else {
                Err(format!(
                    "{} pattern_width=1 only exposes south in Game-owned outfit semantics",
                    program.category
                ))
            }
        }
        width if width >= 4 => Ok(match direction {
            DemoDirection::North => 0,
            DemoDirection::East => 1,
            DemoDirection::South => 2,
            DemoDirection::West => 3,
        }),
        width => Err(format!(
            "{} pattern_width={width} has unsupported direction semantics",
            program.category
        )),
    }
}

#[derive(Debug, Clone, Copy)]
struct ActorState {
    position_units: (f32, f32),
    scene_tile: (i32, i32),
    direction: DemoDirection,
}

fn actor_state(elapsed_ms: u64) -> ActorState {
    let local = elapsed_ms % DEMO_PATH_MS;
    let start = (18.0 * TILE_UNITS, 15.0 * TILE_UNITS);
    let end = (21.0 * TILE_UNITS, 15.0 * TILE_UNITS);
    if local < DEMO_LEG_MS {
        let progress = ratio(local, DEMO_LEG_MS);
        let position = lerp2(start, end, progress);
        ActorState {
            position_units: position,
            scene_tile: world_to_scene_tile(position),
            direction: DemoDirection::East,
        }
    } else {
        let progress = ratio(local - DEMO_LEG_MS, DEMO_LEG_MS);
        let position = lerp2(end, start, progress);
        ActorState {
            position_units: position,
            scene_tile: world_to_scene_tile(position),
            direction: DemoDirection::West,
        }
    }
}

fn missile_position(elapsed_ms: u64, actor_position: (f32, f32), target: (f32, f32)) -> (f32, f32) {
    let progress = ratio(elapsed_ms % DEMO_MISSILE_MS, DEMO_MISSILE_MS);
    let start = (actor_position.0 - 6.0 * TILE_UNITS, actor_position.1);
    lerp2(start, target, progress)
}

fn camera_state(elapsed_ms: u64) -> DemoCamera {
    let local = elapsed_ms % DEMO_CAMERA_MS;
    let progress = triangle01(local, DEMO_CAMERA_MS);
    DemoCamera {
        center_x_units: 20.0 * TILE_UNITS + (progress - 0.5) * 96.0,
        center_y_units: 15.0 * TILE_UNITS,
        zoom: 1.0 + progress * 0.375,
    }
}

fn carrier_rect(anchor_x_units: f32, anchor_y_units: f32) -> WorldRect {
    WorldRect {
        x_units: anchor_x_units - CARRIER_OVERHANG_UNITS,
        y_units: anchor_y_units - CARRIER_OVERHANG_UNITS,
        width_units: CARRIER_UNITS,
        height_units: CARRIER_UNITS,
    }
}

fn world_to_scene_tile(position: (f32, f32)) -> (i32, i32) {
    (
        (position.0 / TILE_UNITS).floor() as i32,
        (position.1 / TILE_UNITS).floor() as i32,
    )
}

fn triangle01(local_ms: u64, period_ms: u64) -> f32 {
    if period_ms == 0 {
        return 0.0;
    }
    let half = period_ms / 2;
    if half == 0 {
        return 0.0;
    }
    if local_ms < half {
        ratio(local_ms, half)
    } else {
        1.0 - ratio(local_ms - half, period_ms - half)
    }
}

fn ratio(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn lerp2(start: (f32, f32), end: (f32, f32), progress: f32) -> (f32, f32) {
    (
        start.0 + (end.0 - start.0) * progress,
        start.1 + (end.1 - start.1) * progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::{FrameGroupIdentity, NormalizedAnimation, PatternShape};

    fn direction_program(width: usize) -> NormalizedProgram {
        NormalizedProgram {
            appearance_source_id: 1,
            category: "outfit".to_owned(),
            frame_group: FrameGroupIdentity {
                id: 0,
                semantic: "outfit-moving".to_owned(),
                kind: 1,
            },
            index_order: vec![
                "phase".to_owned(),
                "pattern_z".to_owned(),
                "pattern_y".to_owned(),
                "pattern_x".to_owned(),
                "layer".to_owned(),
            ],
            layers: 1,
            patterns: PatternShape {
                depth: 1,
                height: 1,
                width,
            },
            phase_count: 1,
            source_profile_id: "test".to_owned(),
            sprite_source_ids: vec![1; width],
            animation: None,
            program_id: "test".to_owned(),
        }
    }

    #[test]
    fn rejects_counted_animation_in_demo_projection() {
        let mut program = direction_program(4);
        program.phase_count = 2;
        program.sprite_source_ids = vec![1; 8];
        program.animation = Some(NormalizedAnimation {
            default_start_phase: 0,
            duration_ranges_ms: vec![[100, 100], [100, 100]],
            effective_duration_ranges_ms: vec![[100, 100], [100, 100]],
            loop_count: 1,
            loop_type: "counted".to_owned(),
            presentation_durations_ms: vec![100, 100],
            random_start_phase: false,
            synchronized: true,
            timing_policy: "source-range-first-nonzero-fallback+deterministic-midpoint-v1"
                .to_owned(),
        });
        assert!(presentation_phase(&program, 0).is_err());
    }

    #[test]
    fn maps_game_owned_cardinal_direction_patterns() -> Result<(), String> {
        let program = direction_program(4);
        assert_eq!(direction_pattern_x(&program, DemoDirection::North)?, 0);
        assert_eq!(direction_pattern_x(&program, DemoDirection::East)?, 1);
        assert_eq!(direction_pattern_x(&program, DemoDirection::South)?, 2);
        assert_eq!(direction_pattern_x(&program, DemoDirection::West)?, 3);
        Ok(())
    }

    #[test]
    fn rejects_non_south_direction_for_single_pattern_outfit() {
        let program = direction_program(1);
        assert!(direction_pattern_x(&program, DemoDirection::North).is_err());
        assert_eq!(direction_pattern_x(&program, DemoDirection::South), Ok(0));
    }

    #[test]
    fn carrier_bottom_right_anchor_preserves_tile_footprint() {
        let rect = carrier_rect(64.0, 96.0);
        assert!((rect.x_units - 32.0).abs() < f32::EPSILON);
        assert!((rect.y_units - 64.0).abs() < f32::EPSILON);
        assert!((rect.x_units + rect.width_units - 96.0).abs() < f32::EPSILON);
        assert!((rect.y_units + rect.height_units - 128.0).abs() < f32::EPSILON);
    }

    #[test]
    fn actor_motion_is_bounded_and_directional() {
        let start = actor_state(0);
        let east_mid = actor_state(DEMO_LEG_MS / 2);
        let turn = actor_state(DEMO_LEG_MS);
        let west_mid = actor_state(DEMO_LEG_MS + DEMO_LEG_MS / 2);
        assert_eq!(start.direction, DemoDirection::East);
        assert_eq!(east_mid.direction, DemoDirection::East);
        assert_eq!(turn.direction, DemoDirection::West);
        assert_eq!(west_mid.direction, DemoDirection::West);
        assert!(east_mid.position_units.0 > start.position_units.0);
        assert!(west_mid.position_units.0 < turn.position_units.0);
    }

    #[test]
    fn fractional_zoom_and_scroll_stay_within_demo_envelope() {
        for elapsed in [0, 500, 1_000, 2_000, 3_000, 3_999] {
            let camera = camera_state(elapsed);
            assert!((1.0..=1.375).contains(&camera.zoom));
            assert!((592.0..=688.0).contains(&camera.center_x_units));
            assert!((camera.center_y_units - 480.0).abs() < f32::EPSILON);
        }
    }
}
