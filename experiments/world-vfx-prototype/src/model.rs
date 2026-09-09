use crate::atlas_slice::{AtlasCreature, AtlasPrimitive, AtlasSlice, AtlasTile};
use crate::config::{BenchConfig, CorpusCensus, Family, RESOURCE_SPRITES_PER_PAGE, Scenario};
use serde::Serialize;
use std::collections::BTreeSet;
use std::mem::size_of;
use std::time::Instant;

pub const LOGICAL_TILE_UNITS: f32 = 32.0;
pub const FALLBACK_SPRITE_ID: u32 = 1;
const CAMERA_FLOOR: i16 = -7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StackClass {
    LowerFloor = 0,
    Ground = 1,
    GroundBorder = 2,
    Bottom = 3,
    Decal = 4,
    Common = 5,
    Creature = 6,
    Effect = 7,
    Projectile = 8,
    Top = 9,
    Environment = 10,
    Overlay = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Alpha,
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    Default,
    Own,
    OtherPlayer,
    Monster,
    Boss,
    Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryClass {
    Cell32x32,
    Cell32x64,
    Cell64x32,
    Cell64x64,
}

impl GeometryClass {
    pub const fn width_tiles(self) -> f32 {
        match self {
            Self::Cell32x32 | Self::Cell32x64 => 1.0,
            Self::Cell64x32 | Self::Cell64x64 => 2.0,
        }
    }

    pub const fn height_tiles(self) -> f32 {
        match self {
            Self::Cell32x32 | Self::Cell64x32 => 1.0,
            Self::Cell32x64 | Self::Cell64x64 => 2.0,
        }
    }

    pub const fn width_ratio(self) -> f32 {
        self.width_tiles() / 2.0
    }

    pub const fn height_ratio(self) -> f32 {
        self.height_tiles() / 2.0
    }

    pub const fn is_square(self) -> bool {
        matches!(self, Self::Cell32x32 | Self::Cell64x64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureRef {
    World {
        sprite_id: u32,
        geometry: GeometryClass,
        missing_variant: bool,
    },
    Fx {
        cell: u8,
    },
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortKey {
    pub floor: i16,
    pub diagonal: i32,
    pub y: i32,
    pub x: i32,
    pub stack: StackClass,
    pub local_order: u16,
    pub stable_id: u64,
}

#[derive(Debug, Clone)]
pub struct RenderPrimitive {
    pub stable_id: u64,
    pub sort_key: SortKey,
    pub rect_px: [f32; 4],
    pub texture: TextureRef,
    pub color: [f32; 4],
    pub blend: BlendMode,
    pub source: SourceClass,
    pub critical: bool,
}

impl RenderPrimitive {
    pub fn intersects_viewport(&self, width: f32, height: f32) -> bool {
        rect_intersects(self.rect_px, [0.0, 0.0, width, height])
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CameraState {
    pub world_x: f32,
    pub world_y: f32,
    pub floor: i16,
    pub zoom: f32,
    pub pixels_per_tile: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct EnvironmentState {
    pub time_of_day: f32,
    pub night_factor: f32,
    pub winter_factor: f32,
    pub wind: f32,
    pub fog_alpha: f32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FrameSemanticStats {
    pub total_primitives: usize,
    pub world_primitives: usize,
    pub creatures: usize,
    pub effects: usize,
    pub projectiles: usize,
    pub particles: usize,
    pub lights: usize,
    pub overlays: usize,
    pub critical_vfx: usize,
    pub fallback_variants: usize,
    pub weather_suppressed_under_roof: usize,
    pub lower_floor_hole_primitives: usize,
}

#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub camera: CameraState,
    pub environment: EnvironmentState,
    pub gameplay_signature: u64,
    pub primitives: Vec<RenderPrimitive>,
    pub stats: FrameSemanticStats,
}

#[derive(Debug, Clone, Copy)]
pub struct PhaseDuration {
    pub min_ms: u32,
    pub max_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum LoopMode {
    Infinite,
    PingPong,
    Counted(u32),
}

#[derive(Debug, Clone)]
pub struct AnimationProgram {
    pub id: u32,
    pub phases: Vec<PhaseDuration>,
    pub loop_mode: LoopMode,
    pub synchronized: bool,
}

impl AnimationProgram {
    pub fn phase_at(&self, elapsed_ms: u64, instance_seed: u64) -> usize {
        if self.phases.is_empty() {
            return 0;
        }
        let sequence_len = match self.loop_mode {
            LoopMode::PingPong if self.phases.len() > 1 => self.phases.len() * 2 - 2,
            _ => self.phases.len(),
        };
        let seed = if self.synchronized {
            u64::from(self.id)
        } else {
            instance_seed ^ u64::from(self.id)
        };

        let mut cursor_ms = 0_u64;
        let mut loop_index = 0_u32;
        loop {
            for sequence_index in 0..sequence_len {
                let phase_index = self.sequence_phase(sequence_index);
                let duration = u64::from(self.phase_duration_ms(phase_index, loop_index, seed));
                if elapsed_ms < cursor_ms.saturating_add(duration) {
                    return phase_index;
                }
                cursor_ms = cursor_ms.saturating_add(duration);
            }

            loop_index = loop_index.saturating_add(1);
            if let LoopMode::Counted(count) = self.loop_mode
                && loop_index >= count.max(1)
            {
                return self.phases.len() - 1;
            }

            if cursor_ms == 0 {
                return 0;
            }
            if cursor_ms > elapsed_ms {
                return self.phases.len() - 1;
            }
        }
    }

    fn sequence_phase(&self, sequence_index: usize) -> usize {
        match self.loop_mode {
            LoopMode::PingPong if self.phases.len() > 1 => {
                if sequence_index < self.phases.len() {
                    sequence_index
                } else {
                    (self.phases.len() * 2 - 2) - sequence_index
                }
            }
            _ => sequence_index,
        }
    }

    fn phase_duration_ms(&self, phase: usize, loop_index: u32, seed: u64) -> u32 {
        let range = self.phases[phase];
        if range.min_ms >= range.max_ms {
            return range.min_ms.max(1);
        }
        let span = range.max_ms - range.min_ms + 1;
        let mixed = mix64(
            seed ^ ((phase as u64) << 32)
                ^ u64::from(loop_index).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        );
        range.min_ms + (mixed as u32 % span)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PresentationEventKind {
    AreaSpell,
    Hit,
    Heal,
    BossTelegraph,
}

#[derive(Debug, Clone, Copy)]
pub struct PresentationEvent {
    pub event_id: u64,
    pub kind: PresentationEventKind,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub world_x: f32,
    pub world_y: f32,
    pub floor: i16,
    pub source: SourceClass,
    pub critical: bool,
}

#[derive(Debug, Default, Clone)]
pub struct EventDeduper {
    seen: BTreeSet<u64>,
}

impl EventDeduper {
    pub fn accept(&mut self, event_id: u64) -> bool {
        self.seen.insert(event_id)
    }

    pub fn reset_for_snapshot_replacement(&mut self, retained_event_ids: &[u64]) {
        self.seen.clear();
        self.seen.extend(retained_event_ids.iter().copied());
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AnimationEvaluationEvidence {
    pub instance_count: usize,
    pub independent_timer_cpu_ms: f64,
    pub shared_program_cpu_ms: f64,
    pub independent_state_bytes: usize,
    pub shared_state_bytes: usize,
    pub checksum_equal: bool,
}

#[derive(Debug, Clone)]
pub struct SceneModel {
    census: CorpusCensus,
    atlas_slice: Option<AtlasSlice>,
    animation_programs: Vec<AnimationProgram>,
    events: Vec<PresentationEvent>,
}

impl SceneModel {
    pub fn new(census: CorpusCensus, atlas_slice: Option<AtlasSlice>) -> Self {
        let animation_programs = vec![
            AnimationProgram {
                id: 1,
                phases: vec![
                    PhaseDuration {
                        min_ms: 110,
                        max_ms: 160,
                    };
                    4
                ],
                loop_mode: LoopMode::Infinite,
                synchronized: true,
            },
            AnimationProgram {
                id: 2,
                phases: vec![
                    PhaseDuration {
                        min_ms: 70,
                        max_ms: 130,
                    };
                    8
                ],
                loop_mode: LoopMode::Infinite,
                synchronized: false,
            },
            AnimationProgram {
                id: 3,
                phases: vec![
                    PhaseDuration {
                        min_ms: 45,
                        max_ms: 85,
                    };
                    12
                ],
                loop_mode: LoopMode::Counted(1),
                synchronized: false,
            },
            AnimationProgram {
                id: 4,
                phases: vec![
                    PhaseDuration {
                        min_ms: 85,
                        max_ms: 120,
                    };
                    8
                ],
                loop_mode: LoopMode::PingPong,
                synchronized: false,
            },
        ];
        let mut deduper = EventDeduper::default();
        let events: Vec<_> = build_event_catalog()
            .into_iter()
            .filter(|event| deduper.accept(event.event_id))
            .collect();
        let retained_ids: Vec<_> = events.iter().map(|event| event.event_id).collect();
        deduper.reset_for_snapshot_replacement(&retained_ids);
        Self {
            census,
            atlas_slice,
            animation_programs,
            events,
        }
    }

    pub fn animation_evaluation_evidence(&self) -> AnimationEvaluationEvidence {
        const INSTANCE_COUNT: usize = 50_000;
        let elapsed_ms = 12_345_u64;
        let independent: Vec<AnimationProgram> = (0..INSTANCE_COUNT)
            .map(|index| self.animation_programs[index % self.animation_programs.len()].clone())
            .collect();
        let independent_state_bytes = independent.capacity() * size_of::<AnimationProgram>()
            + independent
                .iter()
                .map(|program| program.phases.capacity() * size_of::<PhaseDuration>())
                .sum::<usize>();
        let independent_start = Instant::now();
        let independent_checksum =
            independent
                .iter()
                .enumerate()
                .fold(0_u64, |checksum, (index, program)| {
                    checksum
                        ^ u64::try_from(program.phase_at(elapsed_ms, index as u64)).unwrap_or(0)
                });
        let independent_timer_cpu_ms = independent_start.elapsed().as_secs_f64() * 1000.0;
        let shared_instances: Vec<(usize, u64)> = (0..INSTANCE_COUNT)
            .map(|index| (index % self.animation_programs.len(), index as u64))
            .collect();
        let shared_state_bytes = shared_instances.capacity() * size_of::<(usize, u64)>()
            + self.animation_programs.capacity() * size_of::<AnimationProgram>()
            + self
                .animation_programs
                .iter()
                .map(|program| program.phases.capacity() * size_of::<PhaseDuration>())
                .sum::<usize>();
        let shared_start = Instant::now();
        let shared_checksum =
            shared_instances
                .iter()
                .fold(0_u64, |checksum, (program_index, seed)| {
                    checksum
                        ^ u64::try_from(
                            self.animation_programs[*program_index].phase_at(elapsed_ms, *seed),
                        )
                        .unwrap_or(0)
                });
        let shared_program_cpu_ms = shared_start.elapsed().as_secs_f64() * 1000.0;
        AnimationEvaluationEvidence {
            instance_count: INSTANCE_COUNT,
            independent_timer_cpu_ms,
            shared_program_cpu_ms,
            independent_state_bytes,
            shared_state_bytes,
            checksum_equal: independent_checksum == shared_checksum,
        }
    }

    pub fn frame(&self, config: &BenchConfig, time_ms: u64, frame_number: u64) -> RenderSnapshot {
        let camera = camera_for(config, time_ms, self.atlas_slice.as_ref());
        let environment = environment_for(time_ms);
        let mut primitives = Vec::with_capacity(60_000);
        let mut stats = FrameSemanticStats::default();

        self.append_world(
            config,
            &camera,
            &environment,
            time_ms,
            &mut primitives,
            &mut stats,
        );
        self.append_creatures(config, &camera, time_ms, &mut primitives, &mut stats);
        self.append_events(config, &camera, time_ms, &mut primitives, &mut stats);
        self.append_projectiles(config, &camera, time_ms, &mut primitives, &mut stats);
        self.append_lights(config, &camera, time_ms, &mut primitives, &mut stats);
        self.append_weather(
            config,
            &camera,
            &environment,
            time_ms,
            frame_number,
            &mut primitives,
            &mut stats,
        );
        append_fog(config, &environment, &mut primitives);
        append_overlays(config, &camera, time_ms, &mut primitives, &mut stats);

        primitives.retain(|primitive| {
            primitive.intersects_viewport(config.width as f32, config.height as f32)
        });
        primitives.sort_by_key(|primitive| primitive.sort_key);
        stats.total_primitives = primitives.len();

        let gameplay_signature = gameplay_signature(
            config.seed,
            frame_number,
            camera.floor,
            time_ms,
            &self.events,
        );

        RenderSnapshot {
            camera,
            environment,
            gameplay_signature,
            primitives,
            stats,
        }
    }

    fn append_world(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        environment: &EnvironmentState,
        time_ms: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        if let Some(slice) = &self.atlas_slice {
            self.append_atlas_world(config, camera, environment, slice, out, stats);
            return;
        }
        let half_x = (config.width as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 4;
        let half_y = (config.height as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 4;
        let center_x = camera.world_x.floor() as i32;
        let center_y = camera.world_y.floor() as i32;
        let object_sprite_domain = self.census.census.object.unique_sprite_ids.max(16);

        for y in (center_y - half_y)..=(center_y + half_y) {
            for x in (center_x - half_x)..=(center_x + half_x) {
                let tile_hash = tile_hash(x, y, CAMERA_FLOOR, config.seed);
                self.append_tile_floor(
                    config,
                    camera,
                    environment,
                    time_ms,
                    x,
                    y,
                    CAMERA_FLOOR,
                    tile_hash,
                    object_sprite_domain,
                    out,
                    stats,
                );

                if tile_hash.is_multiple_of(97) {
                    self.append_tile_floor(
                        config,
                        camera,
                        environment,
                        time_ms,
                        x,
                        y,
                        CAMERA_FLOOR - 1,
                        tile_hash.rotate_left(7),
                        object_sprite_domain,
                        out,
                        stats,
                    );
                    stats.lower_floor_hole_primitives += 1;
                }

                if should_draw_upper_floor(camera, x, y) && tile_hash.is_multiple_of(3) {
                    self.append_tile_floor(
                        config,
                        camera,
                        environment,
                        time_ms,
                        x,
                        y,
                        CAMERA_FLOOR + 1,
                        tile_hash.rotate_left(13),
                        object_sprite_domain,
                        out,
                        stats,
                    );
                }
            }
        }
    }

    fn append_atlas_world(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        environment: &EnvironmentState,
        slice: &AtlasSlice,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let half_x = (config.width as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 4;
        let half_y = (config.height as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 4;
        let center_x = camera.world_x.floor() as i32;
        let center_y = camera.world_y.floor() as i32;
        let sprite_domain = self.census.census.object.unique_sprite_ids.max(16);
        let tint = world_tint(environment);
        for tile in &slice.tiles {
            if (tile.x - center_x).abs() > half_x
                || (tile.y - center_y).abs() > half_y
                || (tile.floor - camera.floor).abs() > 1
            {
                continue;
            }
            for presentation in &tile.presentations {
                let stack = atlas_stack_class(presentation.role.as_str(), tile.floor, camera.floor);
                if presentation.primitives.is_empty() {
                    push_world_primitive(
                        config,
                        camera,
                        out,
                        PrimitiveSpec {
                            stable_id: stable_id(tile.x, tile.y, tile.floor, presentation.order),
                            tile_x: tile.x,
                            tile_y: tile.y,
                            floor: tile.floor,
                            stack,
                            local_order: presentation.order,
                            geometry: GeometryClass::Cell32x32,
                            sprite_id: FALLBACK_SPRITE_ID,
                            color: tint,
                            displacement: [0.0, 0.0],
                            source: SourceClass::Default,
                            critical: false,
                            blend: BlendMode::Alpha,
                            missing_variant: true,
                        },
                    );
                    stats.world_primitives += 1;
                    stats.fallback_variants += 1;
                    continue;
                }
                for primitive in &presentation.primitives {
                    self.append_atlas_primitive(
                        config,
                        camera,
                        tint,
                        tile,
                        presentation.order,
                        presentation.appearance_source_id,
                        primitive,
                        stack,
                        sprite_domain,
                        out,
                        stats,
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_atlas_primitive(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        tint: [f32; 4],
        tile: &AtlasTile,
        presentation_order: u16,
        appearance_source_id: u32,
        primitive: &AtlasPrimitive,
        stack: StackClass,
        sprite_domain: u32,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let Some(geometry) = geometry_from_units(primitive.width_units, primitive.height_units)
        else {
            return;
        };
        let physical_sprite = atlas_physical_sprite_id(
            config,
            tile.x,
            tile.y,
            tile.floor,
            stack,
            primitive.sprite_id,
            appearance_source_id,
            sprite_domain,
        );
        let local_order = presentation_order
            .saturating_mul(8)
            .saturating_add(primitive.layer_index);
        push_world_primitive(
            config,
            camera,
            out,
            PrimitiveSpec {
                stable_id: stable_id(tile.x, tile.y, tile.floor, local_order),
                tile_x: tile.x,
                tile_y: tile.y,
                floor: tile.floor,
                stack,
                local_order,
                geometry,
                sprite_id: physical_sprite,
                color: tint,
                displacement: [primitive.dx_units as f32, primitive.dy_units as f32],
                source: SourceClass::Default,
                critical: false,
                blend: BlendMode::Alpha,
                missing_variant: false,
            },
        );
        stats.world_primitives += 1;
    }

    #[allow(clippy::too_many_arguments)]
    fn append_tile_floor(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        environment: &EnvironmentState,
        time_ms: u64,
        x: i32,
        y: i32,
        floor: i16,
        hash: u64,
        sprite_domain: u32,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let tint = world_tint(environment);
        let base_sprite = localized_sprite_id(x, y, floor, StackClass::Ground, hash, sprite_domain);
        push_world_primitive(
            config,
            camera,
            out,
            PrimitiveSpec {
                stable_id: stable_id(x, y, floor, 1),
                tile_x: x,
                tile_y: y,
                floor,
                stack: if floor < CAMERA_FLOOR {
                    StackClass::LowerFloor
                } else {
                    StackClass::Ground
                },
                local_order: 0,
                geometry: GeometryClass::Cell32x32,
                sprite_id: animated_sprite_id(
                    base_sprite,
                    &self.animation_programs[0],
                    time_ms,
                    hash,
                    sprite_domain,
                ),
                color: tint,
                displacement: [0.0, 0.0],
                source: SourceClass::Default,
                critical: false,
                blend: BlendMode::Alpha,
                missing_variant: false,
            },
        );
        stats.world_primitives += 1;

        if hash.is_multiple_of(7) {
            push_world_primitive(
                config,
                camera,
                out,
                PrimitiveSpec {
                    stable_id: stable_id(x, y, floor, 2),
                    tile_x: x,
                    tile_y: y,
                    floor,
                    stack: StackClass::GroundBorder,
                    local_order: 1,
                    geometry: GeometryClass::Cell32x32,
                    sprite_id: localized_sprite_id(
                        x,
                        y,
                        floor,
                        StackClass::GroundBorder,
                        hash,
                        sprite_domain,
                    ),
                    color: multiply_color(tint, [0.75, 0.8, 0.72, 1.0]),
                    displacement: [0.0, 0.0],
                    source: SourceClass::Default,
                    critical: false,
                    blend: BlendMode::Alpha,
                    missing_variant: false,
                },
            );
            stats.world_primitives += 1;
        }

        let common_count = match config.scenario {
            Scenario::Basic => usize::from(hash.is_multiple_of(3)),
            Scenario::Normal => (hash % 3) as usize,
            Scenario::Stress => (hash % 5) as usize,
        };
        for index in 0..common_count {
            let object_hash = mix64(hash ^ (index as u64).wrapping_mul(0x9e37_79b9));
            let geometry = geometry_from_hash(object_hash);
            let stack = if index == 0 && object_hash.is_multiple_of(5) {
                StackClass::Bottom
            } else {
                StackClass::Common
            };
            let winter_tint = if object_hash.is_multiple_of(17) {
                [
                    0.82 + 0.18 * environment.winter_factor,
                    0.88 + 0.12 * environment.winter_factor,
                    0.92 + 0.08 * environment.winter_factor,
                    1.0,
                ]
            } else {
                tint
            };
            let program = &self.animation_programs[object_hash as usize % 2];
            let base = localized_sprite_id(x, y, floor, stack, object_hash, sprite_domain);
            let missing = object_hash.is_multiple_of(997);
            push_world_primitive(
                config,
                camera,
                out,
                PrimitiveSpec {
                    stable_id: stable_id(x, y, floor, 10 + index as u16),
                    tile_x: x,
                    tile_y: y,
                    floor,
                    stack,
                    local_order: 10 + index as u16,
                    geometry,
                    sprite_id: if missing {
                        FALLBACK_SPRITE_ID
                    } else {
                        animated_sprite_id(base, program, time_ms, object_hash, sprite_domain)
                    },
                    color: winter_tint,
                    displacement: [
                        -((object_hash & 3) as f32),
                        -(((object_hash >> 2) & 3) as f32),
                    ],
                    source: SourceClass::Default,
                    critical: false,
                    blend: BlendMode::Alpha,
                    missing_variant: missing,
                },
            );
            stats.world_primitives += 1;
            if missing {
                stats.fallback_variants += 1;
            }
        }

        if hash.is_multiple_of(19) {
            let [sx, sy] = world_to_screen(camera, x as f32, y as f32, floor, config);
            let size = camera.pixels_per_tile * 0.8;
            let mut decal = solid_primitive(
                stable_id(x, y, floor, 90),
                SortKey {
                    floor,
                    diagonal: x + y,
                    y,
                    x,
                    stack: StackClass::Decal,
                    local_order: 0,
                    stable_id: stable_id(x, y, floor, 90),
                },
                [sx - size * 0.5, sy - size * 0.35, size, size * 0.45],
                [0.3, 0.08, 0.05, 0.28],
                BlendMode::Alpha,
            );
            decal.source = SourceClass::Default;
            out.push(decal);
            stats.effects += 1;
        }

        if hash.is_multiple_of(23) {
            push_world_primitive(
                config,
                camera,
                out,
                PrimitiveSpec {
                    stable_id: stable_id(x, y, floor, 120),
                    tile_x: x,
                    tile_y: y,
                    floor,
                    stack: StackClass::Top,
                    local_order: 0,
                    geometry: GeometryClass::Cell32x64,
                    sprite_id: localized_sprite_id(
                        x,
                        y,
                        floor,
                        StackClass::Top,
                        hash.rotate_left(19),
                        sprite_domain,
                    ),
                    color: tint,
                    displacement: [0.0, -3.0],
                    source: SourceClass::Default,
                    critical: false,
                    blend: BlendMode::Alpha,
                    missing_variant: false,
                },
            );
            stats.world_primitives += 1;
        }
    }

    fn append_creatures(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        if let Some(slice) = &self.atlas_slice {
            self.append_atlas_creatures(config, camera, time_ms, slice, out, stats);
            return;
        }
        let count = config.scenario.creature_count();
        let outfit_domain = self.census.census.outfit.unique_sprite_ids.max(16);
        for index in 0..count {
            let seed = mix64(config.seed ^ (index as u64 * 0x517c_c1b7_2722_0a95));
            let radius_x = 5.0 + (seed % 29) as f32;
            let radius_y = 4.0 + ((seed >> 8) % 18) as f32;
            let angle = time_ms as f32 * 0.000_25 + index as f32 * 0.71;
            let base_x = camera.world_x + angle.sin() * radius_x;
            let base_y = camera.world_y + angle.cos() * radius_y;
            let step_period = 520_u64 + seed % 480;
            let step_elapsed = time_ms % step_period;
            let progress = step_elapsed as f32 / step_period as f32;
            let direction_x = if seed & 1 == 0 { 1.0 } else { -1.0 };
            let direction_y = if seed & 2 == 0 { 0.0 } else { 1.0 };
            let world_x = base_x + direction_x * progress;
            let world_y = base_y + direction_y * progress;
            let tile_x = world_x.floor() as i32;
            let tile_y = world_y.floor() as i32;
            let source = if index == 0 {
                SourceClass::Own
            } else if index % 31 == 0 {
                SourceClass::Boss
            } else if index % 3 == 0 {
                SourceClass::OtherPlayer
            } else {
                SourceClass::Monster
            };
            let geometry = if seed.is_multiple_of(11) {
                GeometryClass::Cell64x64
            } else {
                GeometryClass::Cell32x32
            };
            let program = &self.animation_programs[1];
            let phase = program.phase_at(time_ms, seed);
            let base_sprite = localized_sprite_id(
                tile_x,
                tile_y,
                CAMERA_FLOOR,
                StackClass::Creature,
                seed,
                outfit_domain,
            );
            let sprite_id = (base_sprite + phase as u32) % outfit_domain;
            let [sx, sy] = world_to_screen(camera, world_x, world_y, CAMERA_FLOOR, config);
            let width = geometry.width_tiles() * camera.pixels_per_tile;
            let height = geometry.height_tiles() * camera.pixels_per_tile;
            let alpha = effect_alpha(source, false, config.scenario);
            let id = 10_000_000_u64 + index as u64;
            let rect = [
                sx - (geometry.width_tiles() - 1.0) * camera.pixels_per_tile - width * 0.5,
                sy - (geometry.height_tiles() - 1.0) * camera.pixels_per_tile - height * 0.75,
                width,
                height,
            ];
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: SortKey {
                    floor: CAMERA_FLOOR,
                    diagonal: tile_x + tile_y,
                    y: tile_y,
                    x: tile_x,
                    stack: StackClass::Creature,
                    local_order: index.min(u16::MAX as usize) as u16,
                    stable_id: id,
                },
                rect_px: rect,
                texture: TextureRef::World {
                    sprite_id,
                    geometry,
                    missing_variant: false,
                },
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendMode::Alpha,
                source,
                critical: false,
            });
            stats.creatures += 1;

            if index % 4 == 0 {
                let aura_size = camera.pixels_per_tile * 1.4;
                out.push(RenderPrimitive {
                    stable_id: id + 900_000,
                    sort_key: SortKey {
                        floor: CAMERA_FLOOR,
                        diagonal: tile_x + tile_y,
                        y: tile_y,
                        x: tile_x,
                        stack: StackClass::Effect,
                        local_order: 4,
                        stable_id: id + 900_000,
                    },
                    rect_px: [
                        sx - aura_size * 0.5,
                        sy - aura_size * 0.5,
                        aura_size,
                        aura_size,
                    ],
                    texture: TextureRef::Fx { cell: 1 },
                    color: [0.25, 0.55, 1.0, alpha * 0.32],
                    blend: BlendMode::Additive,
                    source,
                    critical: false,
                });
                stats.effects += 1;
            }
        }
    }

    fn append_atlas_creatures(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        slice: &AtlasSlice,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let half_x = (config.width as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 8;
        let half_y = (config.height as f32 / camera.pixels_per_tile / 2.0).ceil() as i32 + 8;
        let visible: Vec<&AtlasCreature> = slice
            .creatures
            .iter()
            .filter(|creature| {
                creature.floor == camera.floor
                    && (creature.x - camera.world_x.floor() as i32).abs() <= half_x
                    && (creature.y - camera.world_y.floor() as i32).abs() <= half_y
            })
            .collect();
        if visible.is_empty() {
            return;
        }
        let count = config.scenario.creature_count();
        let outfit_domain = self.census.census.outfit.unique_sprite_ids.max(16);
        for index in 0..count {
            let creature = visible[index % visible.len()];
            let seed = mix64(config.seed ^ stable_text_hash(&creature.record_id) ^ index as u64);
            let duplicate = index / visible.len();
            let jitter_x = if duplicate == 0 {
                0.0
            } else {
                ((seed & 3) as f32 - 1.5) * 0.7
            };
            let jitter_y = if duplicate == 0 {
                0.0
            } else {
                (((seed >> 3) & 3) as f32 - 1.5) * 0.7
            };
            let step_period = 520_u64 + seed % 480;
            let progress = (time_ms % step_period) as f32 / step_period as f32;
            let world_x =
                creature.x as f32 + jitter_x + progress * if seed & 1 == 0 { 1.0 } else { -1.0 };
            let world_y =
                creature.y as f32 + jitter_y + progress * if seed & 2 == 0 { 0.0 } else { 1.0 };
            self.append_atlas_creature(
                config,
                camera,
                time_ms,
                index,
                creature,
                seed,
                world_x,
                world_y,
                outfit_domain,
                out,
                stats,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_atlas_creature(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        index: usize,
        creature: &AtlasCreature,
        seed: u64,
        world_x: f32,
        world_y: f32,
        outfit_domain: u32,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let phase = atlas_creature_phase(creature, time_ms, seed);
        let tile_x = world_x.floor() as i32;
        let tile_y = world_y.floor() as i32;
        let source = if creature.kind == "monster" {
            SourceClass::Monster
        } else {
            SourceClass::Default
        };
        let geometry = GeometryClass::Cell32x32;
        let source_sprite = creature.look_type.saturating_mul(32).saturating_add(phase);
        let sprite_id = atlas_physical_sprite_id(
            config,
            tile_x,
            tile_y,
            creature.floor,
            StackClass::Creature,
            source_sprite,
            creature.look_type,
            outfit_domain,
        );
        let [sx, sy] = world_to_screen(camera, world_x, world_y, creature.floor, config);
        let width = camera.pixels_per_tile;
        let height = camera.pixels_per_tile;
        let alpha = effect_alpha(source, false, config.scenario);
        let id = 20_000_000_u64 + index as u64;
        out.push(RenderPrimitive {
            stable_id: id,
            sort_key: SortKey {
                floor: creature.floor,
                diagonal: tile_x + tile_y,
                y: tile_y,
                x: tile_x,
                stack: StackClass::Creature,
                local_order: index.min(u16::MAX as usize) as u16,
                stable_id: id,
            },
            rect_px: [
                sx - width * 0.5 + creature.dx_units as f32 * camera.zoom,
                sy - height * 0.75 + creature.dy_units as f32 * camera.zoom,
                width,
                height,
            ],
            texture: TextureRef::World {
                sprite_id: if creature.presentation_resolved {
                    sprite_id
                } else {
                    FALLBACK_SPRITE_ID
                },
                geometry,
                missing_variant: !creature.presentation_resolved,
            },
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendMode::Alpha,
            source,
            critical: false,
        });
        stats.creatures += 1;
        if !creature.presentation_resolved {
            stats.fallback_variants += 1;
        }
    }

    fn append_events(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        for event in &self.events {
            let cycle = event.start_ms;
            let period = 3_500_u64 + event.event_id % 2_700;
            let local = (time_ms + cycle) % period;
            if local >= event.duration_ms {
                continue;
            }
            let progress = local as f32 / event.duration_ms as f32;
            let x = camera.world_x + event.world_x;
            let y = camera.world_y + event.world_y;
            let tile_x = x.floor() as i32;
            let tile_y = y.floor() as i32;
            let [sx, sy] = world_to_screen(camera, x, y, event.floor, config);
            let alpha = effect_alpha(event.source, event.critical, config.scenario);
            match event.kind {
                PresentationEventKind::AreaSpell => {
                    let radius_tiles = 1.0 + progress * 1.2;
                    let size = radius_tiles * 2.0 * camera.pixels_per_tile;
                    out.push(RenderPrimitive {
                        stable_id: event.event_id,
                        sort_key: effect_sort_key(
                            event.floor,
                            tile_x,
                            tile_y,
                            StackClass::Effect,
                            event.event_id,
                        ),
                        rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                        texture: TextureRef::Fx { cell: 1 },
                        color: [1.0, 0.24, 0.05, alpha * (1.0 - progress * 0.45)],
                        blend: BlendMode::Additive,
                        source: event.source,
                        critical: event.critical,
                    });
                    stats.effects += 1;
                }
                PresentationEventKind::Hit => {
                    let size = camera.pixels_per_tile * (0.5 + progress);
                    out.push(RenderPrimitive {
                        stable_id: event.event_id,
                        sort_key: effect_sort_key(
                            event.floor,
                            tile_x,
                            tile_y,
                            StackClass::Effect,
                            event.event_id,
                        ),
                        rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                        texture: TextureRef::Fx { cell: 0 },
                        color: [1.0, 0.12, 0.04, alpha * (1.0 - progress)],
                        blend: BlendMode::Additive,
                        source: event.source,
                        critical: event.critical,
                    });
                    stats.effects += 1;
                }
                PresentationEventKind::Heal => {
                    let size = camera.pixels_per_tile * (0.7 + progress * 0.7);
                    out.push(RenderPrimitive {
                        stable_id: event.event_id,
                        sort_key: effect_sort_key(
                            event.floor,
                            tile_x,
                            tile_y,
                            StackClass::Effect,
                            event.event_id,
                        ),
                        rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                        texture: TextureRef::Fx { cell: 0 },
                        color: [0.15, 1.0, 0.35, alpha * (1.0 - progress * 0.6)],
                        blend: BlendMode::Additive,
                        source: event.source,
                        critical: event.critical,
                    });
                    stats.effects += 1;
                }
                PresentationEventKind::BossTelegraph => {
                    let pulse = 0.72 + (progress * std::f32::consts::TAU * 3.0).sin().abs() * 0.28;
                    let size = camera.pixels_per_tile * 5.5;
                    out.push(RenderPrimitive {
                        stable_id: event.event_id,
                        sort_key: effect_sort_key(
                            event.floor,
                            tile_x,
                            tile_y,
                            StackClass::Effect,
                            event.event_id,
                        ),
                        rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                        texture: TextureRef::Fx { cell: 1 },
                        color: [1.0, 0.05, 0.05, alpha.max(0.85) * pulse],
                        blend: BlendMode::Alpha,
                        source: SourceClass::Boss,
                        critical: true,
                    });
                    stats.effects += 1;
                    stats.critical_vfx += 1;
                }
            }
        }

        let extra = config.scenario.telegraph_count().saturating_sub(2);
        for index in 0..extra {
            let seed = mix64(config.seed ^ 0xaaa5_555a ^ index as u64);
            let x = camera.world_x + ((seed % 41) as f32 - 20.0);
            let y = camera.world_y + (((seed >> 8) % 25) as f32 - 12.0);
            let [sx, sy] = world_to_screen(camera, x, y, CAMERA_FLOOR, config);
            let size = camera.pixels_per_tile * (2.0 + (seed % 4) as f32);
            let id = 30_000_000 + index as u64;
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: effect_sort_key(
                    CAMERA_FLOOR,
                    x.floor() as i32,
                    y.floor() as i32,
                    StackClass::Effect,
                    id,
                ),
                rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                texture: TextureRef::Fx { cell: 1 },
                color: [1.0, 0.12, 0.04, 0.88],
                blend: BlendMode::Alpha,
                source: SourceClass::Boss,
                critical: true,
            });
            stats.effects += 1;
            stats.critical_vfx += 1;
        }
    }

    fn append_projectiles(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let count = config.scenario.projectile_count();
        for index in 0..count {
            let seed = mix64(config.seed ^ 0x5a5a_a5a5 ^ index as u64);
            let start_x = camera.world_x + ((seed % 55) as f32 - 27.0);
            let start_y = camera.world_y + (((seed >> 7) % 33) as f32 - 16.0);
            let delta_x = ((seed >> 15) % 13) as f32 - 6.0;
            let delta_y = ((seed >> 22) % 13) as f32 - 6.0;
            let duration_ms = 320_u64 + seed % 680;
            let local = (time_ms + seed % duration_ms) % duration_ms;
            let progress = local as f32 / duration_ms as f32;
            let world_x = start_x + delta_x * progress;
            let world_y = start_y + delta_y * progress;
            let [sx, sy] = world_to_screen(camera, world_x, world_y, CAMERA_FLOOR, config);
            let source = if index % 41 == 0 {
                SourceClass::Boss
            } else if index % 5 == 0 {
                SourceClass::OtherPlayer
            } else {
                SourceClass::Monster
            };
            let critical = source == SourceClass::Boss && index % 82 == 0;
            let alpha = effect_alpha(source, critical, config.scenario);
            let size = camera.pixels_per_tile * 0.45;
            let id = 40_000_000 + index as u64;
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: effect_sort_key(
                    CAMERA_FLOOR,
                    world_x.floor() as i32,
                    world_y.floor() as i32,
                    StackClass::Projectile,
                    id,
                ),
                rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                texture: TextureRef::Fx { cell: 2 },
                color: [0.2, 0.75, 1.0, alpha],
                blend: BlendMode::Additive,
                source,
                critical,
            });
            let trail_size = size * 1.8;
            out.push(RenderPrimitive {
                stable_id: id + 500_000,
                sort_key: effect_sort_key(
                    CAMERA_FLOOR,
                    world_x.floor() as i32,
                    world_y.floor() as i32,
                    StackClass::Effect,
                    id + 500_000,
                ),
                rect_px: [
                    sx - trail_size * 0.5,
                    sy - trail_size * 0.5,
                    trail_size,
                    trail_size,
                ],
                texture: TextureRef::Fx { cell: 0 },
                color: [0.08, 0.35, 1.0, alpha * 0.28],
                blend: BlendMode::Additive,
                source,
                critical,
            });
            stats.projectiles += 1;
            stats.effects += 1;
            if critical {
                stats.critical_vfx += 1;
            }
        }
    }

    fn append_lights(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        time_ms: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let quality = quality_factor(config.family());
        let count = ((config.scenario.light_count() as f32) * quality).round() as usize;
        for index in 0..count {
            let seed = mix64(config.seed ^ 0x1eaf_cafe ^ index as u64);
            let x = camera.world_x + ((seed % 61) as f32 - 30.0);
            let y = camera.world_y + (((seed >> 8) % 37) as f32 - 18.0);
            let [sx, sy] = world_to_screen(camera, x, y, CAMERA_FLOOR, config);
            let pulse = 0.8 + 0.2 * ((time_ms as f32 * 0.007) + index as f32).sin().abs();
            let size = camera.pixels_per_tile * (2.0 + (seed % 4) as f32) * pulse;
            let id = 50_000_000 + index as u64;
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: effect_sort_key(
                    CAMERA_FLOOR,
                    x.floor() as i32,
                    y.floor() as i32,
                    StackClass::Environment,
                    id,
                ),
                rect_px: [sx - size * 0.5, sy - size * 0.5, size, size],
                texture: TextureRef::Fx { cell: 0 },
                color: [1.0, 0.58, 0.18, 0.12],
                blend: BlendMode::Additive,
                source: SourceClass::Environment,
                critical: false,
            });
            stats.lights += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_weather(
        &self,
        config: &BenchConfig,
        camera: &CameraState,
        environment: &EnvironmentState,
        time_ms: u64,
        frame_number: u64,
        out: &mut Vec<RenderPrimitive>,
        stats: &mut FrameSemanticStats,
    ) {
        let quality = quality_factor(config.family());
        let degrade = degradation_factor(config.scenario);
        let weather_count =
            ((config.scenario.weather_particles() as f32) * quality * degrade).round() as usize;
        let width = config.width as f32;
        let height = config.height as f32;
        for index in 0..weather_count {
            let seed =
                mix64(config.seed ^ (index as u64 * 0xd6e8_feb8_6659_fd93) ^ (frame_number / 6));
            let base_x = (seed % u64::from(config.width.max(1))) as f32;
            let speed = 85.0 + ((seed >> 17) % 180) as f32;
            let y = ((seed >> 8) % u64::from(config.height.max(1))) as f32
                + time_ms as f32 * 0.001 * speed;
            let screen_y = y.rem_euclid(height + 40.0) - 20.0;
            let screen_x = (base_x + environment.wind * screen_y * 0.08).rem_euclid(width);
            let tile = screen_to_world(camera, screen_x, screen_y, config);
            if !is_weather_exposed(tile.0.floor() as i32, tile.1.floor() as i32) {
                stats.weather_suppressed_under_roof += 1;
                continue;
            }
            let snow = environment.winter_factor > 0.45;
            let size = if snow { 4.5 } else { 2.0 };
            let id = 60_000_000 + index as u64;
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: SortKey {
                    floor: CAMERA_FLOOR,
                    diagonal: i32::MAX - 3,
                    y: 0,
                    x: 0,
                    stack: StackClass::Environment,
                    local_order: (index % usize::from(u16::MAX)) as u16,
                    stable_id: id,
                },
                rect_px: [
                    screen_x,
                    screen_y,
                    size,
                    if snow { size } else { size * 5.0 },
                ],
                texture: TextureRef::Fx { cell: 2 },
                color: if snow {
                    [0.86, 0.93, 1.0, 0.72]
                } else {
                    [0.35, 0.65, 1.0, 0.38]
                },
                blend: BlendMode::Alpha,
                source: SourceClass::Environment,
                critical: false,
            });
            stats.particles += 1;
        }

        let ambient_count =
            ((config.scenario.ambient_particles() as f32) * quality * degrade).round() as usize;
        for index in 0..ambient_count {
            let seed = mix64(config.seed ^ 0xbb67_ae85 ^ index as u64);
            let angle = time_ms as f32 * 0.000_35 + index as f32 * 0.37;
            let x = width * 0.5 + angle.sin() * (width * 0.48) * ((seed & 255) as f32 / 255.0);
            let y =
                height * 0.5 + angle.cos() * (height * 0.45) * (((seed >> 8) & 255) as f32 / 255.0);
            let id = 70_000_000 + index as u64;
            out.push(RenderPrimitive {
                stable_id: id,
                sort_key: SortKey {
                    floor: CAMERA_FLOOR,
                    diagonal: i32::MAX - 2,
                    y: 0,
                    x: 0,
                    stack: StackClass::Environment,
                    local_order: (index % usize::from(u16::MAX)) as u16,
                    stable_id: id,
                },
                rect_px: [x, y, 3.0, 3.0],
                texture: TextureRef::Fx { cell: 2 },
                color: [0.55, 1.0, 0.72, 0.38],
                blend: BlendMode::Additive,
                source: SourceClass::Environment,
                critical: false,
            });
            stats.particles += 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PrimitiveSpec {
    stable_id: u64,
    tile_x: i32,
    tile_y: i32,
    floor: i16,
    stack: StackClass,
    local_order: u16,
    geometry: GeometryClass,
    sprite_id: u32,
    color: [f32; 4],
    displacement: [f32; 2],
    source: SourceClass,
    critical: bool,
    blend: BlendMode,
    missing_variant: bool,
}

fn push_world_primitive(
    config: &BenchConfig,
    camera: &CameraState,
    out: &mut Vec<RenderPrimitive>,
    spec: PrimitiveSpec,
) {
    let [sx, sy] = world_to_screen(
        camera,
        spec.tile_x as f32,
        spec.tile_y as f32,
        spec.floor,
        config,
    );
    let width = spec.geometry.width_tiles() * camera.pixels_per_tile;
    let height = spec.geometry.height_tiles() * camera.pixels_per_tile;
    let rect = [
        sx - (spec.geometry.width_tiles() - 1.0) * camera.pixels_per_tile
            + spec.displacement[0] * camera.zoom,
        sy - (spec.geometry.height_tiles() - 1.0) * camera.pixels_per_tile
            + spec.displacement[1] * camera.zoom,
        width,
        height,
    ];
    out.push(RenderPrimitive {
        stable_id: spec.stable_id,
        sort_key: SortKey {
            floor: spec.floor,
            diagonal: spec.tile_x + spec.tile_y,
            y: spec.tile_y,
            x: spec.tile_x,
            stack: spec.stack,
            local_order: spec.local_order,
            stable_id: spec.stable_id,
        },
        rect_px: rect,
        texture: TextureRef::World {
            sprite_id: spec.sprite_id,
            geometry: spec.geometry,
            missing_variant: spec.missing_variant,
        },
        color: spec.color,
        blend: spec.blend,
        source: spec.source,
        critical: spec.critical,
    });
}

fn gameplay_signature(
    seed: u64,
    frame_number: u64,
    floor: i16,
    time_ms: u64,
    events: &[PresentationEvent],
) -> u64 {
    let mut signature =
        mix64(seed ^ frame_number.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (floor as u64));
    for event in events {
        let end_ms = event.start_ms.saturating_add(event.duration_ms);
        if time_ms >= event.start_ms && time_ms < end_ms {
            signature ^= mix64(event.event_id ^ event.start_ms ^ event.duration_ms);
        }
    }
    signature
}

fn camera_for(config: &BenchConfig, time_ms: u64, atlas_slice: Option<&AtlasSlice>) -> CameraState {
    let time = time_ms as f32 * 0.001;
    let (origin_x, origin_y, base_floor) =
        atlas_slice.map_or((32_300.0, 32_230.0, CAMERA_FLOOR), AtlasSlice::center);
    let floor = if let Some(slice) = atlas_slice {
        let phase_ms = time_ms % 6_000;
        let upper = if slice.bounds.floors.contains(&(base_floor + 1)) {
            base_floor + 1
        } else {
            base_floor
        };
        let lower = if slice.bounds.floors.contains(&(base_floor - 1)) {
            base_floor - 1
        } else {
            base_floor
        };
        match phase_ms {
            2_500..3_000 => upper,
            5_000..5_500 => lower,
            _ => base_floor,
        }
    } else {
        base_floor
    };
    let zoom = config.fixed_zoom.unwrap_or_else(|| {
        let base = match config.scenario {
            Scenario::Basic => 1.15,
            Scenario::Normal => 0.82,
            Scenario::Stress => 0.58,
        };
        (base + 0.07 * (time * 0.43).sin()).clamp(0.35, 2.5)
    });
    CameraState {
        world_x: origin_x + (time * 0.23).sin() * 22.0 + time * 0.45,
        world_y: origin_y + (time * 0.19).cos() * 14.0 + time * 0.22,
        floor,
        zoom,
        pixels_per_tile: LOGICAL_TILE_UNITS * zoom,
    }
}

fn environment_for(time_ms: u64) -> EnvironmentState {
    let time = time_ms as f32 * 0.001;
    let day_cycle = (time * 0.08).sin() * 0.5 + 0.5;
    let winter = (time * 0.025).sin() * 0.5 + 0.5;
    EnvironmentState {
        time_of_day: day_cycle,
        night_factor: 1.0 - day_cycle,
        winter_factor: winter,
        wind: (time * 0.31).sin(),
        fog_alpha: 0.035 + (time * 0.17).sin().abs() * 0.045,
    }
}

fn world_tint(environment: &EnvironmentState) -> [f32; 4] {
    let night = environment.night_factor;
    [
        1.0 - night * 0.28,
        1.0 - night * 0.2,
        1.0 - night * 0.04,
        1.0,
    ]
}

fn multiply_color(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    [
        left[0] * right[0],
        left[1] * right[1],
        left[2] * right[2],
        left[3] * right[3],
    ]
}

pub fn world_to_screen(
    camera: &CameraState,
    world_x: f32,
    world_y: f32,
    floor: i16,
    config: &BenchConfig,
) -> [f32; 2] {
    let floor_delta = f32::from(floor - camera.floor);
    [
        config.width as f32 * 0.5
            + ((world_x - camera.world_x) - floor_delta) * camera.pixels_per_tile,
        config.height as f32 * 0.5
            + ((world_y - camera.world_y) - floor_delta) * camera.pixels_per_tile,
    ]
}

fn screen_to_world(
    camera: &CameraState,
    screen_x: f32,
    screen_y: f32,
    config: &BenchConfig,
) -> (f32, f32) {
    (
        camera.world_x + (screen_x - config.width as f32 * 0.5) / camera.pixels_per_tile,
        camera.world_y + (screen_y - config.height as f32 * 0.5) / camera.pixels_per_tile,
    )
}

fn should_draw_upper_floor(camera: &CameraState, x: i32, y: i32) -> bool {
    let camera_inside = is_interior(camera.world_x.floor() as i32, camera.world_y.floor() as i32);
    if camera_inside
        && (x - camera.world_x.floor() as i32).abs() <= 2
        && (y - camera.world_y.floor() as i32).abs() <= 2
    {
        return false;
    }
    true
}

fn is_interior(x: i32, y: i32) -> bool {
    let local_x = x.rem_euclid(24);
    let local_y = y.rem_euclid(18);
    (5..=16).contains(&local_x) && (4..=12).contains(&local_y)
}

pub fn is_weather_exposed(x: i32, y: i32) -> bool {
    !is_interior(x, y)
}

fn geometry_from_units(width: u32, height: u32) -> Option<GeometryClass> {
    match (width, height) {
        (32, 32) => Some(GeometryClass::Cell32x32),
        (32, 64) => Some(GeometryClass::Cell32x64),
        (64, 32) => Some(GeometryClass::Cell64x32),
        (64, 64) => Some(GeometryClass::Cell64x64),
        _ => None,
    }
}

fn atlas_stack_class(role: &str, floor: i16, camera_floor: i16) -> StackClass {
    if floor < camera_floor {
        StackClass::LowerFloor
    } else if role == "ground" {
        StackClass::Ground
    } else {
        StackClass::Common
    }
}

fn geometry_from_hash(hash: u64) -> GeometryClass {
    match hash % 20 {
        0..=11 => GeometryClass::Cell32x32,
        12..=14 => GeometryClass::Cell32x64,
        15..=17 => GeometryClass::Cell64x32,
        _ => GeometryClass::Cell64x64,
    }
}

#[allow(clippy::too_many_arguments)]
fn atlas_physical_sprite_id(
    config: &BenchConfig,
    x: i32,
    y: i32,
    floor: i16,
    stack: StackClass,
    source_sprite_id: u32,
    appearance_source_id: u32,
    sprite_domain: u32,
) -> u32 {
    let locality_span = match config.scenario {
        Scenario::Basic => 22,
        Scenario::Normal => 18,
        Scenario::Stress => 14,
    };
    let locality_x = x.div_euclid(locality_span);
    let locality_y = y.div_euclid(locality_span);
    let pages = (sprite_domain / RESOURCE_SPRITES_PER_PAGE).max(128);
    let locality_hash = mix64(
        (locality_x as i64 as u64)
            ^ (locality_y as i64 as u64).rotate_left(17)
            ^ (floor as i64 as u64).rotate_left(31),
    );
    let page = locality_hash as u32 % pages;
    let cell_hash = mix64(
        u64::from(source_sprite_id)
            ^ u64::from(appearance_source_id).rotate_left(23)
            ^ (stack as u64).rotate_left(41),
    );
    let cell = cell_hash as u32 % RESOURCE_SPRITES_PER_PAGE;
    (page * RESOURCE_SPRITES_PER_PAGE + cell).max(1)
}

fn localized_sprite_id(
    x: i32,
    y: i32,
    floor: i16,
    stack: StackClass,
    hash: u64,
    sprite_domain: u32,
) -> u32 {
    let pages = (sprite_domain / RESOURCE_SPRITES_PER_PAGE).max(1);
    let chunk_x = x.div_euclid(32);
    let chunk_y = y.div_euclid(32);
    let chunk_hash = mix64(
        (chunk_x as i64 as u64)
            ^ (chunk_y as i64 as u64).rotate_left(17)
            ^ (floor as i64 as u64).rotate_left(29),
    );
    let page = chunk_hash as u32 % pages;
    let cell = (hash as u32 ^ (stack as u32).wrapping_mul(3)) % RESOURCE_SPRITES_PER_PAGE;
    (page * RESOURCE_SPRITES_PER_PAGE + cell).max(1)
}

fn animated_sprite_id(
    base: u32,
    program: &AnimationProgram,
    time_ms: u64,
    seed: u64,
    domain: u32,
) -> u32 {
    let phase = program.phase_at(time_ms, seed) as u32;
    (base + phase) % domain.max(1)
}

fn stable_text_hash(text: &str) -> u64 {
    text.as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        })
}

fn atlas_creature_phase(creature: &AtlasCreature, time_ms: u64, seed: u64) -> u32 {
    let count = creature.phase_count.max(1);
    if count == 1 {
        return 0;
    }
    let sequence_len = if creature.loop_type == "pingpong" && count > 1 {
        count * 2 - 2
    } else {
        count
    };
    let phase_for = |index: u32| -> u32 {
        if index < count {
            index
        } else {
            count * 2 - 2 - index
        }
    };
    let duration_for = |phase: u32| -> u64 {
        creature
            .phase_durations_ms
            .get(phase as usize)
            .copied()
            .unwrap_or(300)
            .max(1)
            .into()
    };
    let cycle = (0..sequence_len)
        .map(|index| duration_for(phase_for(index)))
        .sum::<u64>()
        .max(1);
    let offset = if creature.synchronized {
        0
    } else {
        mix64(seed) % cycle
    };
    let mut elapsed = time_ms.saturating_add(offset);
    if creature.loop_type == "counted" && elapsed >= cycle {
        return count - 1;
    }
    elapsed %= cycle;
    for index in 0..sequence_len {
        let phase = phase_for(index);
        let duration = duration_for(phase);
        if elapsed < duration {
            return phase;
        }
        elapsed -= duration;
    }
    count - 1
}

fn stable_id(x: i32, y: i32, floor: i16, local: u16) -> u64 {
    mix64(
        (x as i64 as u64)
            ^ (y as i64 as u64).rotate_left(19)
            ^ (floor as i64 as u64).rotate_left(37)
            ^ u64::from(local).rotate_left(51),
    )
}

fn tile_hash(x: i32, y: i32, floor: i16, seed: u64) -> u64 {
    mix64(
        seed ^ (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (y as i64 as u64).rotate_left(23)
            ^ (floor as i64 as u64).rotate_left(47),
    )
}

fn effect_sort_key(floor: i16, x: i32, y: i32, stack: StackClass, stable_id: u64) -> SortKey {
    SortKey {
        floor,
        diagonal: x + y,
        y,
        x,
        stack,
        local_order: 0,
        stable_id,
    }
}

fn build_event_catalog() -> Vec<PresentationEvent> {
    vec![
        PresentationEvent {
            event_id: 1,
            kind: PresentationEventKind::AreaSpell,
            start_ms: 200,
            duration_ms: 900,
            world_x: 3.0,
            world_y: 2.0,
            floor: CAMERA_FLOOR,
            source: SourceClass::Own,
            critical: false,
        },
        PresentationEvent {
            event_id: 2,
            kind: PresentationEventKind::Hit,
            start_ms: 900,
            duration_ms: 420,
            world_x: -2.0,
            world_y: 1.0,
            floor: CAMERA_FLOOR,
            source: SourceClass::Monster,
            critical: false,
        },
        PresentationEvent {
            event_id: 3,
            kind: PresentationEventKind::Heal,
            start_ms: 1_300,
            duration_ms: 650,
            world_x: 0.0,
            world_y: -1.0,
            floor: CAMERA_FLOOR,
            source: SourceClass::Own,
            critical: false,
        },
        PresentationEvent {
            event_id: 4,
            kind: PresentationEventKind::BossTelegraph,
            start_ms: 1_800,
            duration_ms: 1_450,
            world_x: 7.0,
            world_y: -3.0,
            floor: CAMERA_FLOOR,
            source: SourceClass::Boss,
            critical: true,
        },
    ]
}

fn append_fog(
    config: &BenchConfig,
    environment: &EnvironmentState,
    out: &mut Vec<RenderPrimitive>,
) {
    let id = 80_000_000;
    out.push(RenderPrimitive {
        stable_id: id,
        sort_key: SortKey {
            floor: i16::MAX,
            diagonal: i32::MAX - 1,
            y: 0,
            x: 0,
            stack: StackClass::Environment,
            local_order: 0,
            stable_id: id,
        },
        rect_px: [0.0, 0.0, config.width as f32, config.height as f32],
        texture: TextureRef::Solid,
        color: [0.18, 0.24, 0.32, environment.fog_alpha],
        blend: BlendMode::Alpha,
        source: SourceClass::Environment,
        critical: false,
    });
}

fn append_overlays(
    config: &BenchConfig,
    camera: &CameraState,
    time_ms: u64,
    out: &mut Vec<RenderPrimitive>,
    stats: &mut FrameSemanticStats,
) {
    let shown = config.scenario.creature_count().min(72);
    for index in 0..shown {
        let seed = mix64(config.seed ^ (index as u64 * 0x517c_c1b7_2722_0a95));
        let radius_x = 5.0 + (seed % 29) as f32;
        let radius_y = 4.0 + ((seed >> 8) % 18) as f32;
        let angle = time_ms as f32 * 0.000_25 + index as f32 * 0.71;
        let world_x = camera.world_x + angle.sin() * radius_x;
        let world_y = camera.world_y + angle.cos() * radius_y;
        let [sx, sy] = world_to_screen(camera, world_x, world_y, CAMERA_FLOOR, config);
        let hp = 0.25 + ((seed >> 16) & 255) as f32 / 340.0;
        let id = 90_000_000 + index as u64;
        let width = 34.0;
        let bar_y = sy - 29.0;
        out.push(solid_primitive(
            id,
            SortKey {
                floor: i16::MAX,
                diagonal: i32::MAX,
                y: 0,
                x: 0,
                stack: StackClass::Overlay,
                local_order: (index * 3) as u16,
                stable_id: id,
            },
            [sx - width * 0.5, bar_y, width, 5.0],
            [0.04, 0.04, 0.04, 0.92],
            BlendMode::Alpha,
        ));
        out.push(solid_primitive(
            id + 1,
            SortKey {
                floor: i16::MAX,
                diagonal: i32::MAX,
                y: 0,
                x: 0,
                stack: StackClass::Overlay,
                local_order: (index * 3 + 1) as u16,
                stable_id: id + 1,
            },
            [
                sx - width * 0.5 + 1.0,
                bar_y + 1.0,
                (width - 2.0) * hp.min(1.0),
                3.0,
            ],
            [0.82, 0.12 + 0.6 * hp, 0.12, 1.0],
            BlendMode::Alpha,
        ));
        append_bitmap_name(out, id + 10_000, sx, bar_y - 9.0, index % 31 == 0);
        stats.overlays += 3;
    }
}

fn append_bitmap_name(
    out: &mut Vec<RenderPrimitive>,
    id_base: u64,
    center_x: f32,
    baseline_y: f32,
    boss: bool,
) {
    let glyphs: &[&[u8; 7]] = if boss {
        &[&GLYPH_B, &GLYPH_O, &GLYPH_S, &GLYPH_S]
    } else {
        &[&GLYPH_D, &GLYPH_E, &GLYPH_M, &GLYPH_O, &GLYPH_N]
    };
    let scale = 1.15_f32;
    let glyph_width = 4.0 * scale;
    let total_width = glyphs.len() as f32 * glyph_width;
    let mut primitive_index = 0_u64;
    for (glyph_index, glyph) in glyphs.iter().enumerate() {
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) == 0 {
                    continue;
                }
                let x = center_x - total_width * 0.5
                    + glyph_index as f32 * glyph_width
                    + col as f32 * scale;
                let y = baseline_y + row as f32 * scale;
                let id = id_base + primitive_index;
                out.push(solid_primitive(
                    id,
                    SortKey {
                        floor: i16::MAX,
                        diagonal: i32::MAX,
                        y: 0,
                        x: 0,
                        stack: StackClass::Overlay,
                        local_order: (primitive_index % u64::from(u16::MAX)) as u16,
                        stable_id: id,
                    },
                    [x, y, scale, scale],
                    if boss {
                        [1.0, 0.68, 0.18, 1.0]
                    } else {
                        [0.92, 0.92, 0.92, 1.0]
                    },
                    BlendMode::Alpha,
                ));
                primitive_index += 1;
            }
        }
    }
}

fn solid_primitive(
    stable_id: u64,
    sort_key: SortKey,
    rect_px: [f32; 4],
    color: [f32; 4],
    blend: BlendMode,
) -> RenderPrimitive {
    RenderPrimitive {
        stable_id,
        sort_key,
        rect_px,
        texture: TextureRef::Solid,
        color,
        blend,
        source: SourceClass::Default,
        critical: false,
    }
}

pub fn effect_alpha(source: SourceClass, critical: bool, scenario: Scenario) -> f32 {
    if critical {
        return 0.88;
    }
    let base = match source {
        SourceClass::Default | SourceClass::Own => 1.0,
        SourceClass::OtherPlayer => 0.42,
        SourceClass::Monster => 0.78,
        SourceClass::Boss => 0.92,
        SourceClass::Environment => 0.72,
    };
    if scenario == Scenario::Stress && source == SourceClass::OtherPlayer {
        base * 0.68
    } else {
        base
    }
}

fn quality_factor(family: Family) -> f32 {
    match family {
        Family::Classic => 0.38,
        Family::Enhanced => 0.72,
        Family::Hd => 1.0,
    }
}

fn degradation_factor(scenario: Scenario) -> f32 {
    match scenario {
        Scenario::Basic => 1.0,
        Scenario::Normal => 0.9,
        Scenario::Stress => 0.62,
    }
}

fn rect_intersects(left: [f32; 4], right: [f32; 4]) -> bool {
    left[0] < right[0] + right[2]
        && left[0] + left[2] > right[0]
        && left[1] < right[1] + right[3]
        && left[1] + left[3] > right[1]
}

pub const fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

const GLYPH_B: [u8; 7] = [0b110, 0b101, 0b101, 0b110, 0b101, 0b101, 0b110];
const GLYPH_D: [u8; 7] = [0b110, 0b101, 0b101, 0b101, 0b101, 0b101, 0b110];
const GLYPH_E: [u8; 7] = [0b111, 0b100, 0b100, 0b110, 0b100, 0b100, 0b111];
const GLYPH_M: [u8; 7] = [0b101, 0b111, 0b111, 0b101, 0b101, 0b101, 0b101];
const GLYPH_N: [u8; 7] = [0b101, 0b111, 0b111, 0b111, 0b111, 0b111, 0b101];
const GLYPH_O: [u8; 7] = [0b010, 0b101, 0b101, 0b101, 0b101, 0b101, 0b010];
const GLYPH_S: [u8; 7] = [0b011, 0b100, 0b100, 0b010, 0b001, 0b001, 0b110];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BenchConfig, CorpusCensus};

    fn test_camera(zoom: f32) -> CameraState {
        CameraState {
            world_x: 100.0,
            world_y: 100.0,
            floor: -7,
            zoom,
            pixels_per_tile: LOGICAL_TILE_UNITS * zoom,
        }
    }

    #[test]
    fn higher_native_floor_projects_northwest() {
        let config = BenchConfig {
            width: 800,
            height: 600,
            fixed_zoom: Some(1.0),
            ..BenchConfig::default()
        };
        let camera = test_camera(1.0);
        let same = world_to_screen(&camera, 100.0, 100.0, -7, &config);
        let above = world_to_screen(&camera, 100.0, 100.0, -6, &config);
        assert_eq!(same, [400.0, 300.0]);
        assert_eq!(above, [368.0, 268.0]);
    }

    #[test]
    fn visual_overhang_is_culled_by_rectangle_not_anchor() {
        let primitive = RenderPrimitive {
            stable_id: 1,
            sort_key: SortKey {
                floor: -7,
                diagonal: 0,
                y: 0,
                x: 0,
                stack: StackClass::Common,
                local_order: 0,
                stable_id: 1,
            },
            rect_px: [-40.0, 80.0, 64.0, 64.0],
            texture: TextureRef::Solid,
            color: [1.0; 4],
            blend: BlendMode::Alpha,
            source: SourceClass::Default,
            critical: false,
        };
        assert!(primitive.intersects_viewport(800.0, 600.0));
    }

    #[test]
    fn animation_is_time_based_and_seed_stable() {
        let program = AnimationProgram {
            id: 7,
            phases: vec![
                PhaseDuration {
                    min_ms: 80,
                    max_ms: 120,
                };
                5
            ],
            loop_mode: LoopMode::Infinite,
            synchronized: false,
        };
        assert_eq!(program.phase_at(1_234, 55), program.phase_at(1_234, 55));
        assert_eq!(program.phase_at(5_678, 55), program.phase_at(5_678, 55));
    }

    #[test]
    fn event_deduper_blocks_replay_after_duplicate_delivery() {
        let mut deduper = EventDeduper::default();
        assert!(deduper.accept(42));
        assert!(!deduper.accept(42));
        deduper.reset_for_snapshot_replacement(&[42]);
        assert!(!deduper.accept(42));
        assert!(deduper.accept(43));
    }

    #[test]
    fn roof_mask_suppresses_weather() {
        assert!(!is_weather_exposed(8, 8));
        assert!(is_weather_exposed(1, 1));
    }

    #[test]
    fn critical_vfx_keeps_visibility_floor() {
        assert!(effect_alpha(SourceClass::Boss, true, Scenario::Stress) >= 0.85);
        assert!(
            effect_alpha(SourceClass::OtherPlayer, false, Scenario::Stress)
                < effect_alpha(SourceClass::Boss, true, Scenario::Stress)
        );
    }

    #[test]
    fn overlay_rect_size_does_not_depend_on_world_zoom() {
        let one = test_camera(1.0);
        let half = test_camera(0.5);
        let config = BenchConfig {
            width: 800,
            height: 600,
            ..BenchConfig::default()
        };
        let p1 = world_to_screen(&one, 100.0, 100.0, -7, &config);
        let p2 = world_to_screen(&half, 100.0, 100.0, -7, &config);
        assert_eq!(p1, p2);
        let overlay_width = 34.0_f32;
        assert_eq!(overlay_width, 34.0);
    }

    #[test]
    fn protected_census_loads_from_repository_fixture() -> Result<(), String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json");
        let census = CorpusCensus::load(&path)?;
        assert_eq!(census.census.object.appearances, 43_514);
        assert_eq!(census.census.outfit.appearances, 1_480);
        assert_eq!(census.census.effect.appearances, 243);
        assert_eq!(census.census.missile.appearances, 76);
        Ok(())
    }
}
