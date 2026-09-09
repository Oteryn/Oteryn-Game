use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

pub const LOGICAL_TILE_PX: f32 = 32.0;
pub const MAX_LIGHTS: usize = 16;
pub const PAGE_CELLS: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Basic,
    Normal,
    Stress,
}

impl Scenario {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Normal => "normal",
            Self::Stress => "stress",
        }
    }

    pub const fn viewport_radius(self) -> (i32, i32) {
        match self {
            Self::Basic => (8, 6),
            Self::Normal => (12, 9),
            Self::Stress => (16, 12),
        }
    }

    pub const fn creature_count(self) -> usize {
        match self {
            Self::Basic => 8,
            Self::Normal => 32,
            Self::Stress => 96,
        }
    }

    pub const fn projectile_count(self) -> usize {
        match self {
            Self::Basic => 2,
            Self::Normal => 12,
            Self::Stress => 48,
        }
    }

    pub const fn area_spell_count(self) -> usize {
        match self {
            Self::Basic => 1,
            Self::Normal => 6,
            Self::Stress => 24,
        }
    }

    pub const fn weather_particles(self) -> usize {
        match self {
            Self::Basic => 32,
            Self::Normal => 160,
            Self::Stress => 640,
        }
    }

    pub const fn visible_page_count(self) -> u32 {
        match self {
            Self::Basic => 4,
            Self::Normal => 8,
            Self::Stress => 12,
        }
    }

    pub const fn total_page_count(self) -> u32 {
        match self {
            Self::Basic => 12,
            Self::Normal => 24,
            Self::Stress => 48,
        }
    }

    pub const fn decorative_budget(self) -> usize {
        match self {
            Self::Basic => 96,
            Self::Normal => 320,
            Self::Stress => 720,
        }
    }
}

impl Display for Scenario {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLayout {
    Atlas,
    Array,
}

impl ResourceLayout {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Atlas => "atlas",
            Self::Array => "array",
        }
    }
}

impl Display for ResourceLayout {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationFamily {
    Classic,
    Enhanced,
    Hd,
}

impl PresentationFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Enhanced => "enhanced",
            Self::Hd => "hd",
        }
    }

    pub const fn tint(self) -> [f32; 4] {
        match self {
            Self::Classic => [0.94, 0.94, 0.94, 1.0],
            Self::Enhanced => [1.0, 1.0, 1.0, 1.0],
            Self::Hd => [1.04, 1.02, 1.0, 1.0],
        }
    }
}

impl Display for PresentationFamily {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrewarmMode {
    None,
    Critical,
}

impl PrewarmMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub scenario: Scenario,
    pub density: u32,
    pub layout: ResourceLayout,
    pub family: PresentationFamily,
    pub warmup_frames: u64,
    pub sample_frames: u64,
    pub width: u32,
    pub height: u32,
    pub cache_pages: u32,
    pub census_path: Option<String>,
    pub prewarm: PrewarmMode,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            scenario: Scenario::Normal,
            density: 64,
            layout: ResourceLayout::Array,
            family: PresentationFamily::Enhanced,
            warmup_frames: 120,
            sample_frames: 600,
            width: 1600,
            height: 900,
            cache_pages: Scenario::Normal.visible_page_count(),
            census_path: None,
            prewarm: PrewarmMode::None,
        }
    }
}

impl BenchConfig {
    pub fn from_args() -> Result<Self, String> {
        let mut config = Self::default();
        let mut explicit_cache = false;
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--scenario" => {
                    config.scenario = match value.as_str() {
                        "basic" => Scenario::Basic,
                        "normal" => Scenario::Normal,
                        "stress" => Scenario::Stress,
                        _ => return Err(format!("unsupported scenario: {value}")),
                    };
                }
                "--density" => {
                    config.density = parse_u32(&value, "density")?;
                    if !matches!(config.density, 32 | 64 | 128) {
                        return Err("density must be 32, 64 or 128".to_owned());
                    }
                }
                "--layout" => {
                    config.layout = match value.as_str() {
                        "atlas" => ResourceLayout::Atlas,
                        "array" => ResourceLayout::Array,
                        _ => return Err(format!("unsupported layout: {value}")),
                    };
                }
                "--family" => {
                    config.family = match value.as_str() {
                        "classic" => PresentationFamily::Classic,
                        "enhanced" => PresentationFamily::Enhanced,
                        "hd" => PresentationFamily::Hd,
                        _ => return Err(format!("unsupported family: {value}")),
                    };
                }
                "--warmup" => config.warmup_frames = parse_u64(&value, "warmup")?,
                "--frames" => config.sample_frames = parse_u64(&value, "frames")?,
                "--width" => config.width = parse_u32(&value, "width")?,
                "--height" => config.height = parse_u32(&value, "height")?,
                "--cache-pages" => {
                    config.cache_pages = parse_u32(&value, "cache-pages")?;
                    explicit_cache = true;
                }
                "--census" => config.census_path = Some(value),
                "--prewarm" => {
                    config.prewarm = match value.as_str() {
                        "none" => PrewarmMode::None,
                        "critical" => PrewarmMode::Critical,
                        _ => return Err(format!("unsupported prewarm mode: {value}")),
                    };
                }
                _ => return Err(format!("unsupported argument: {flag}")),
            }
        }
        if !explicit_cache {
            config.cache_pages = config.scenario.visible_page_count();
        }
        if config.sample_frames == 0 || config.width == 0 || config.height == 0 {
            return Err("frames, width and height must be non-zero".to_owned());
        }
        if config.cache_pages < config.scenario.visible_page_count() {
            return Err("cache-pages must fit the per-frame visible page set".to_owned());
        }
        Ok(config)
    }

    pub const fn total_frames(&self) -> u64 {
        self.warmup_frames + self.sample_frames
    }
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {label} value {value}: {error}"))
}

fn parse_u64(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label} value {value}: {error}"))
}

#[derive(Debug, Clone, Copy)]
pub struct CorpusShape {
    pub objects: u32,
    pub outfits: u32,
    pub effects: u32,
    pub missiles: u32,
}

impl Default for CorpusShape {
    fn default() -> Self {
        Self {
            objects: 43_514,
            outfits: 1_480,
            effects: 243,
            missiles: 76,
        }
    }
}

impl CorpusShape {
    pub fn load(path: Option<&str>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(Path::new(path))
            .map_err(|error| format!("read census {path}: {error}"))?;
        let value: Value =
            serde_json::from_str(&text).map_err(|error| format!("parse census {path}: {error}"))?;
        let census = value
            .get("census")
            .ok_or_else(|| "census JSON is missing census object".to_owned())?;
        Ok(Self {
            objects: census_count(census, "object")?,
            outfits: census_count(census, "outfit")?,
            effects: census_count(census, "effect")?,
            missiles: census_count(census, "missile")?,
        })
    }
}

fn census_count(census: &Value, family: &str) -> Result<u32, String> {
    let value = census
        .get(family)
        .and_then(|entry| entry.get("appearances"))
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("census is missing {family}.appearances"))?;
    u32::try_from(value).map_err(|_| format!("{family}.appearances exceeds u32"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub x: f32,
    pub y: f32,
    pub floor: i32,
    pub zoom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceFamily {
    Object,
    Outfit,
    Effect,
    Missile,
    Generated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceRef {
    pub family: AppearanceFamily,
    pub semantic_id: u32,
    pub frame: u32,
    pub direction: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PresentationClass {
    Ground,
    Border,
    Bottom,
    Common,
    Creature,
    Effect,
    Projectile,
    Attached,
    Top,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Sprite,
    Particle,
    Trail,
    Light,
    Decal,
    Telegraph,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialClass {
    Alpha,
    Additive,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderInstance {
    pub appearance: AppearanceRef,
    pub class: PresentationClass,
    pub kind: PrimitiveKind,
    pub material: MaterialClass,
    pub world: WorldPosition,
    pub screen_x: f32,
    pub screen_y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub emissive: f32,
    pub page_id: u32,
    pub cell: u32,
    pub critical: bool,
    pub screen_space: bool,
    pub order_floor: i32,
    pub order_y: i32,
    pub order_x: i32,
    pub sequence: u32,
}

impl RenderInstance {
    fn order_key(self) -> (i32, i32, i32, PresentationClass, u32) {
        (
            self.order_floor,
            self.order_y,
            self.order_x,
            self.class,
            self.sequence,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalLight {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Snow,
    Fog,
}

impl Weather {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Rain => "rain",
            Self::Snow => "snow",
            Self::Fog => "fog",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Normal,
    Winter,
}

#[derive(Debug, Clone, Copy)]
pub struct EnvironmentState {
    pub ambient: f32,
    pub weather: Weather,
    pub season: Season,
    pub wind: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FrameSemanticStats {
    pub static_world: u32,
    pub creatures: u32,
    pub effects: u32,
    pub projectiles: u32,
    pub particles: u32,
    pub lights: u32,
    pub overlays: u32,
    pub decorative_dropped: u32,
    pub variant_fallbacks: u32,
    pub critical_visible: u32,
}

#[derive(Debug, Clone)]
pub struct FramePlan {
    pub instances: Vec<RenderInstance>,
    pub needed_pages: Vec<u32>,
    pub lights: Vec<LocalLight>,
    pub environment: EnvironmentState,
    pub camera: CameraState,
    pub camera_moved: bool,
    pub zoom_changed: bool,
    pub floor_changed: bool,
    pub gameplay_signature: u64,
    pub stats: FrameSemanticStats,
}

#[derive(Debug, Clone)]
pub struct SceneModel {
    corpus: CorpusShape,
}

impl SceneModel {
    pub const fn new(corpus: CorpusShape) -> Self {
        Self { corpus }
    }

    pub fn plan(&self, config: &BenchConfig, frame: u64) -> FramePlan {
        let camera = camera_for(frame);
        let previous = camera_for(frame.saturating_sub(1));
        let environment = environment_for(frame);
        let visible_floors = visible_floors(camera);
        let region_page_base = ((camera.x.floor() as i32).div_euclid(6)
            + (camera.y.floor() as i32).div_euclid(6) * 3)
            .unsigned_abs()
            % config.scenario.total_page_count();
        let mut builder = PlanBuilder::new(config, self.corpus, camera, region_page_base);
        for floor in visible_floors {
            builder.emit_world_floor(floor, environment);
        }
        builder.emit_creatures(frame, environment);
        builder.emit_combat(frame);
        builder.emit_weather(frame, environment);
        builder.emit_overlays();
        builder.finish(
            environment,
            camera,
            camera.x != previous.x || camera.y != previous.y,
            (camera.zoom - previous.zoom).abs() > f32::EPSILON,
            camera.floor != previous.floor,
        )
    }
}

struct PlanBuilder<'a> {
    config: &'a BenchConfig,
    corpus: CorpusShape,
    camera: CameraState,
    page_base: u32,
    sequence: u32,
    instances: Vec<RenderInstance>,
    pages: BTreeSet<u32>,
    lights: Vec<LocalLight>,
    stats: FrameSemanticStats,
    gameplay_signature: u64,
    decorative_emitted: usize,
    creature_screens: Vec<(f32, f32, u32)>,
}

impl<'a> PlanBuilder<'a> {
    fn new(
        config: &'a BenchConfig,
        corpus: CorpusShape,
        camera: CameraState,
        page_base: u32,
    ) -> Self {
        Self {
            config,
            corpus,
            camera,
            page_base,
            sequence: 0,
            instances: Vec::new(),
            pages: BTreeSet::new(),
            lights: Vec::with_capacity(MAX_LIGHTS),
            stats: FrameSemanticStats::default(),
            gameplay_signature: 0xcbf2_9ce4_8422_2325,
            decorative_emitted: 0,
            creature_screens: Vec::new(),
        }
    }

    fn emit_world_floor(&mut self, floor: i32, environment: EnvironmentState) {
        let (radius_x, radius_y) = self.config.scenario.viewport_radius();
        let center_x = self.camera.x.floor() as i32;
        let center_y = self.camera.y.floor() as i32;
        for dy in -radius_y..=radius_y {
            for dx in -radius_x..=radius_x {
                let position = WorldPosition {
                    x: center_x + dx,
                    y: center_y + dy,
                    floor,
                };
                if floor == 1 && !roof_tile(position.x, position.y) {
                    continue;
                }
                let hash = spatial_hash(position.x, position.y, floor);
                let ground_id = 1 + hash % self.corpus.objects.max(2);
                self.push_world_sprite(
                    position,
                    PresentationClass::Ground,
                    ground_id,
                    1.0,
                    1.0,
                    0.0,
                    [0.72, 0.78, 0.64, 1.0],
                );
                if hash % 11 == 0 {
                    self.push_world_sprite(
                        position,
                        PresentationClass::Border,
                        500 + hash % 300,
                        1.0,
                        1.0,
                        0.0,
                        [0.88, 0.82, 0.68, 1.0],
                    );
                }
                if hash % 17 == 0 {
                    self.push_world_sprite(
                        position,
                        PresentationClass::Bottom,
                        1_200 + hash % 700,
                        1.0,
                        1.0,
                        2.0,
                        [0.80, 0.70, 0.54, 1.0],
                    );
                }
                if hash % 7 == 0 {
                    let oversized = hash % 37 == 0;
                    let size = if oversized { 2.0 } else { 1.0 };
                    let elevation = (hash % 4) as f32 * 3.0;
                    self.push_world_sprite(
                        position,
                        PresentationClass::Common,
                        2_000 + hash % 2_500,
                        size,
                        size,
                        elevation,
                        [0.86, 0.86, 0.88, 1.0],
                    );
                }
                if wall_tile(position.x, position.y) {
                    self.push_world_sprite(
                        position,
                        PresentationClass::Top,
                        6_000 + hash % 400,
                        1.0,
                        1.5,
                        0.0,
                        [0.70, 0.72, 0.76, 1.0],
                    );
                }
                if floor == 1 && roof_tile(position.x, position.y) {
                    let mut tint = [0.62, 0.58, 0.56, 1.0];
                    if environment.season == Season::Winter {
                        tint = [0.86, 0.91, 0.96, 1.0];
                    }
                    self.push_world_sprite(
                        position,
                        PresentationClass::Top,
                        7_000 + hash % 300,
                        1.0,
                        1.0,
                        0.0,
                        tint,
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_world_sprite(
        &mut self,
        position: WorldPosition,
        class: PresentationClass,
        semantic_id: u32,
        width_tiles: f32,
        height_tiles: f32,
        elevation_px: f32,
        color: [f32; 4],
    ) {
        let (mut screen_x, mut screen_y) = project(self.camera, position);
        let tile = LOGICAL_TILE_PX * self.camera.zoom;
        screen_x -= (width_tiles - 1.0) * tile * 0.5;
        screen_y -= (height_tiles - 1.0) * tile * 0.5 + elevation_px;
        let appearance = AppearanceRef {
            family: AppearanceFamily::Object,
            semantic_id,
            frame: semantic_id % 8,
            direction: 0,
        };
        self.push_instance(InstanceSpec {
            appearance,
            class,
            kind: PrimitiveKind::Sprite,
            material: MaterialClass::Alpha,
            world: position,
            screen_x,
            screen_y,
            width: tile * width_tiles,
            height: tile * height_tiles,
            color,
            emissive: 0.0,
            critical: false,
            screen_space: false,
        });
        self.stats.static_world = self.stats.static_world.saturating_add(1);
    }

    fn emit_creatures(&mut self, frame: u64, environment: EnvironmentState) {
        let count = self.config.scenario.creature_count();
        for index in 0..count {
            let base_x = self.camera.x.floor() as i32 - 7 + (index as i32 % 15);
            let base_y = self.camera.y.floor() as i32 - 5 + ((index as i32 * 7) % 11);
            let progress = (frame % 60) as f32 / 60.0;
            let diagonal = index % 3 == 0;
            let dx = if index % 2 == 0 { progress } else { -progress };
            let dy = if diagonal { progress * 0.7 } else { 0.0 };
            let position = WorldPosition {
                x: base_x,
                y: base_y,
                floor: self.camera.floor,
            };
            let (mut screen_x, mut screen_y) = project(self.camera, position);
            let tile = LOGICAL_TILE_PX * self.camera.zoom;
            screen_x += dx * tile;
            screen_y += dy * tile;
            let outfit_id = 1 + (index as u32 * 13) % self.corpus.outfits.max(2);
            let direction = (index as u32 + (frame / 45) as u32) % 4;
            let frame_id = ((frame / 8) as u32 + index as u32) % 8;
            if index % 5 == 0 {
                self.push_creature_component(
                    position,
                    screen_x,
                    screen_y + tile * 0.18,
                    outfit_id + 400,
                    frame_id,
                    direction,
                    [0.72, 0.58, 0.42, 1.0],
                    0,
                );
            }
            self.push_creature_component(
                position,
                screen_x,
                screen_y,
                outfit_id,
                frame_id,
                direction,
                [0.92, 0.84, 0.74, 1.0],
                1,
            );
            self.push_creature_component(
                position,
                screen_x,
                screen_y - tile * 0.08,
                outfit_id + 700,
                frame_id,
                direction,
                [0.74, 0.84, 0.96, 0.78],
                2,
            );
            if self.lights.len() < MAX_LIGHTS && index % 11 == 0 {
                self.lights.push(LocalLight {
                    x: screen_x,
                    y: screen_y,
                    radius: 110.0,
                    intensity: if environment.ambient < 0.6 { 0.9 } else { 0.45 },
                });
                self.stats.lights = self.stats.lights.saturating_add(1);
            }
            self.creature_screens
                .push((screen_x, screen_y, 35 + (index as u32 * 17) % 65));
            self.stats.creatures = self.stats.creatures.saturating_add(1);
            self.mix_gameplay((position.x as u64) ^ ((position.y as u64) << 16) ^ outfit_id as u64);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_creature_component(
        &mut self,
        position: WorldPosition,
        screen_x: f32,
        screen_y: f32,
        semantic_id: u32,
        frame: u32,
        direction: u32,
        color: [f32; 4],
        component: u32,
    ) {
        let tile = LOGICAL_TILE_PX * self.camera.zoom;
        self.push_instance(InstanceSpec {
            appearance: AppearanceRef {
                family: AppearanceFamily::Outfit,
                semantic_id,
                frame,
                direction,
            },
            class: PresentationClass::Creature,
            kind: PrimitiveKind::Sprite,
            material: MaterialClass::Alpha,
            world: position,
            screen_x,
            screen_y,
            width: tile,
            height: tile * if component == 0 { 0.8 } else { 1.1 },
            color,
            emissive: 0.0,
            critical: false,
            screen_space: false,
        });
    }

    fn emit_combat(&mut self, frame: u64) {
        self.emit_projectiles(frame);
        self.emit_area_spells(frame);
        self.emit_hit_vfx(frame);
        self.emit_boss_telegraph(frame);
        self.emit_persistent_effect(frame);
    }

    fn emit_projectiles(&mut self, frame: u64) {
        let count = self.config.scenario.projectile_count();
        let progress = (frame % 90) as f32 / 90.0;
        for index in 0..count {
            let start = WorldPosition {
                x: self.camera.x.floor() as i32 - 8 + index as i32 % 6,
                y: self.camera.y.floor() as i32 - 4 + index as i32 % 9,
                floor: self.camera.floor,
            };
            let end = WorldPosition {
                x: start.x + 8,
                y: start.y + if index % 2 == 0 { 3 } else { -2 },
                floor: start.floor,
            };
            let (sx, sy) = project(self.camera, start);
            let (ex, ey) = project(self.camera, end);
            let x = sx + (ex - sx) * progress;
            let y = sy + (ey - sy) * progress;
            let missile_id = 1 + index as u32 % self.corpus.missiles.max(2);
            let direction = projectile_pattern(ex - sx, ey - sy);
            let position = WorldPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            };
            self.push_vfx(
                position,
                x,
                y,
                AppearanceFamily::Missile,
                missile_id,
                direction,
                PrimitiveKind::Sprite,
                MaterialClass::Alpha,
                [1.0, 0.82, 0.38, 1.0],
                0.4,
                false,
            );
            self.push_vfx(
                position,
                x - (ex - sx).signum() * 12.0,
                y - (ey - sy).signum() * 12.0,
                AppearanceFamily::Generated,
                90_001,
                0,
                PrimitiveKind::Trail,
                MaterialClass::Additive,
                [1.0, 0.50, 0.12, 0.55],
                0.8,
                false,
            );
            if self.lights.len() < MAX_LIGHTS && index % 6 == 0 {
                self.lights.push(LocalLight {
                    x,
                    y,
                    radius: 90.0,
                    intensity: 0.75,
                });
                self.stats.lights = self.stats.lights.saturating_add(1);
            }
            self.stats.projectiles = self.stats.projectiles.saturating_add(1);
            self.mix_gameplay((start.x as u64) ^ ((end.x as u64) << 20) ^ missile_id as u64);
        }
    }

    fn emit_area_spells(&mut self, frame: u64) {
        for index in 0..self.config.scenario.area_spell_count() {
            let position = WorldPosition {
                x: self.camera.x.floor() as i32 - 4 + (index as i32 * 3) % 9,
                y: self.camera.y.floor() as i32 - 3 + (index as i32 * 5) % 7,
                floor: self.camera.floor,
            };
            let (x, y) = project(self.camera, position);
            let effect_id = 1 + index as u32 % self.corpus.effects.max(2);
            let phase = ((frame / 5) as u32 + index as u32) % 12;
            self.push_vfx(
                position,
                x,
                y,
                AppearanceFamily::Effect,
                effect_id,
                phase,
                PrimitiveKind::Sprite,
                MaterialClass::Alpha,
                [0.44, 0.72, 1.0, 0.92],
                0.65,
                false,
            );
            self.push_vfx(
                position,
                x,
                y + 10.0,
                AppearanceFamily::Generated,
                91_000 + index as u32,
                0,
                PrimitiveKind::Decal,
                MaterialClass::Alpha,
                [0.20, 0.44, 0.72, 0.38],
                0.0,
                false,
            );
            for particle in 0..6 {
                if !self.try_decorative() {
                    continue;
                }
                let angle_seed = index as f32 * 1.7 + particle as f32 * 0.9 + frame as f32 * 0.03;
                self.push_vfx(
                    position,
                    x + angle_seed.sin() * 22.0,
                    y + angle_seed.cos() * 18.0,
                    AppearanceFamily::Generated,
                    92_000 + particle as u32,
                    0,
                    PrimitiveKind::Particle,
                    MaterialClass::Additive,
                    [0.30, 0.76, 1.0, 0.68],
                    0.7,
                    false,
                );
                self.stats.particles = self.stats.particles.saturating_add(1);
            }
            if self.lights.len() < MAX_LIGHTS {
                self.lights.push(LocalLight {
                    x,
                    y,
                    radius: 125.0,
                    intensity: 0.8,
                });
                self.stats.lights = self.stats.lights.saturating_add(1);
            }
            self.stats.effects = self.stats.effects.saturating_add(1);
            self.mix_gameplay((position.x as u64) ^ ((position.y as u64) << 13) ^ effect_id as u64);
        }
    }

    fn emit_hit_vfx(&mut self, frame: u64) {
        let Some(&(x, y, _)) = self.creature_screens.first() else {
            return;
        };
        let position = WorldPosition {
            x: self.camera.x.floor() as i32,
            y: self.camera.y.floor() as i32,
            floor: self.camera.floor,
        };
        let physical = frame % 120 < 60;
        let color = if physical {
            [1.0, 0.33, 0.25, 0.95]
        } else {
            [0.24, 0.70, 1.0, 0.95]
        };
        self.push_vfx(
            position,
            x,
            y - 18.0,
            AppearanceFamily::Effect,
            if physical { 37 } else { 91 },
            (frame / 4) as u32 % 8,
            PrimitiveKind::Sprite,
            MaterialClass::Additive,
            color,
            0.9,
            true,
        );
        self.stats.critical_visible = self.stats.critical_visible.saturating_add(1);
    }

    fn emit_boss_telegraph(&mut self, frame: u64) {
        let center = WorldPosition {
            x: self.camera.x.floor() as i32 + 2,
            y: self.camera.y.floor() as i32 + 1,
            floor: self.camera.floor,
        };
        let (cx, cy) = project(self.camera, center);
        let pulse = 0.45 + ((frame % 60) as f32 / 60.0) * 0.35;
        for y in -1..=1 {
            for x in -1..=1 {
                let position = WorldPosition {
                    x: center.x + x,
                    y: center.y + y,
                    floor: center.floor,
                };
                self.push_vfx(
                    position,
                    cx + x as f32 * LOGICAL_TILE_PX * self.camera.zoom,
                    cy + y as f32 * LOGICAL_TILE_PX * self.camera.zoom,
                    AppearanceFamily::Generated,
                    95_000 + ((x + 1) + (y + 1) * 3) as u32,
                    0,
                    PrimitiveKind::Telegraph,
                    MaterialClass::Alpha,
                    [1.0, 0.12, 0.08, pulse],
                    0.25,
                    true,
                );
                self.stats.critical_visible = self.stats.critical_visible.saturating_add(1);
            }
        }
        if self.lights.len() < MAX_LIGHTS {
            self.lights.push(LocalLight {
                x: cx,
                y: cy,
                radius: 180.0,
                intensity: 1.0,
            });
            self.stats.lights = self.stats.lights.saturating_add(1);
        }
        self.mix_gameplay(0xB055_7E1E ^ center.x as u64 ^ ((center.y as u64) << 8));
    }

    fn emit_persistent_effect(&mut self, frame: u64) {
        if frame % 240 >= 180 {
            return;
        }
        let position = WorldPosition {
            x: self.camera.x.floor() as i32 - 2,
            y: self.camera.y.floor() as i32 + 2,
            floor: self.camera.floor,
        };
        let (x, y) = project(self.camera, position);
        self.push_vfx(
            position,
            x,
            y,
            AppearanceFamily::Effect,
            117 % self.corpus.effects.max(2),
            (frame / 7) as u32 % 8,
            PrimitiveKind::Sprite,
            MaterialClass::Alpha,
            [0.62, 0.34, 0.92, 0.8],
            0.55,
            false,
        );
        self.stats.effects = self.stats.effects.saturating_add(1);
    }

    fn emit_weather(&mut self, frame: u64, environment: EnvironmentState) {
        if environment.weather == Weather::Clear {
            return;
        }
        let count = self.config.scenario.weather_particles();
        for index in 0..count {
            if !self.try_decorative() {
                continue;
            }
            let seed = xorshift32(index as u32 ^ (frame as u32).wrapping_mul(0x9e37_79b9));
            let x = (seed % self.config.width) as f32 - self.config.width as f32 * 0.5;
            let y_seed = xorshift32(seed ^ 0xa511_e9b3);
            let y = (y_seed % self.config.height) as f32 - self.config.height as f32 * 0.5;
            let (semantic_id, color, kind) = match environment.weather {
                Weather::Rain => (97_001, [0.40, 0.68, 1.0, 0.38], PrimitiveKind::Trail),
                Weather::Snow => (97_002, [0.94, 0.98, 1.0, 0.74], PrimitiveKind::Particle),
                Weather::Fog => (97_003, [0.76, 0.80, 0.84, 0.12], PrimitiveKind::Particle),
                Weather::Clear => continue,
            };
            self.push_vfx(
                WorldPosition {
                    x: self.camera.x.floor() as i32,
                    y: self.camera.y.floor() as i32,
                    floor: self.camera.floor,
                },
                x,
                y,
                AppearanceFamily::Generated,
                semantic_id,
                index as u32 % PAGE_CELLS,
                kind,
                MaterialClass::Alpha,
                color,
                0.0,
                false,
            );
            self.stats.particles = self.stats.particles.saturating_add(1);
        }
    }

    fn emit_overlays(&mut self) {
        let overlays: Vec<(f32, f32, u32)> =
            self.creature_screens.iter().copied().take(12).collect();
        for (index, (x, y, hp)) in overlays.into_iter().enumerate() {
            let width = 34.0;
            self.push_overlay_rect(x, y - 28.0, width, 4.0, [0.10, 0.10, 0.10, 0.88], 98_100);
            self.push_overlay_rect(
                x - (width - width * hp as f32 / 100.0) * 0.5,
                y - 28.0,
                width * hp as f32 / 100.0,
                3.0,
                [0.18, 0.92, 0.25, 0.95],
                98_101,
            );
            let label = if index == 0 { "BOSS" } else { "DEMON" };
            self.emit_label(label, x, y - 38.0);
        }
    }

    fn emit_label(&mut self, label: &str, center_x: f32, baseline_y: f32) {
        let glyph_width = 4.0;
        let total_width = label.chars().count() as f32 * 6.0 * glyph_width;
        let mut cursor_x = center_x - total_width * 0.5;
        for character in label.chars() {
            let mask = glyph_mask(character);
            for row in 0..7 {
                for column in 0..5 {
                    let bit = row * 5 + column;
                    if mask & (1_u64 << bit) == 0 {
                        continue;
                    }
                    self.push_overlay_rect(
                        cursor_x + column as f32 * glyph_width,
                        baseline_y + row as f32 * glyph_width,
                        glyph_width,
                        glyph_width,
                        [0.96, 0.96, 0.96, 0.95],
                        98_200 + character as u32,
                    );
                }
            }
            cursor_x += glyph_width * 6.0;
        }
    }

    fn push_overlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        semantic_id: u32,
    ) {
        self.push_instance(InstanceSpec {
            appearance: AppearanceRef {
                family: AppearanceFamily::Generated,
                semantic_id,
                frame: 0,
                direction: 0,
            },
            class: PresentationClass::Overlay,
            kind: PrimitiveKind::Overlay,
            material: MaterialClass::Alpha,
            world: WorldPosition {
                x: 0,
                y: 0,
                floor: self.camera.floor,
            },
            screen_x: x,
            screen_y: y,
            width,
            height,
            color,
            emissive: 0.0,
            critical: true,
            screen_space: true,
        });
        self.stats.overlays = self.stats.overlays.saturating_add(1);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_vfx(
        &mut self,
        position: WorldPosition,
        x: f32,
        y: f32,
        family: AppearanceFamily,
        semantic_id: u32,
        frame: u32,
        kind: PrimitiveKind,
        material: MaterialClass,
        color: [f32; 4],
        emissive: f32,
        critical: bool,
    ) {
        let size = match kind {
            PrimitiveKind::Particle => 8.0,
            PrimitiveKind::Trail => 16.0,
            PrimitiveKind::Light => 48.0,
            PrimitiveKind::Decal | PrimitiveKind::Telegraph => LOGICAL_TILE_PX * self.camera.zoom,
            PrimitiveKind::Overlay => 8.0,
            PrimitiveKind::Sprite => LOGICAL_TILE_PX * self.camera.zoom,
        };
        self.push_instance(InstanceSpec {
            appearance: AppearanceRef {
                family,
                semantic_id,
                frame,
                direction: frame % 4,
            },
            class: match kind {
                PrimitiveKind::Telegraph | PrimitiveKind::Decal => PresentationClass::Effect,
                PrimitiveKind::Trail => PresentationClass::Attached,
                _ if family == AppearanceFamily::Missile => PresentationClass::Projectile,
                _ => PresentationClass::Effect,
            },
            kind,
            material,
            world: position,
            screen_x: x,
            screen_y: y,
            width: size,
            height: size,
            color,
            emissive,
            critical,
            screen_space: matches!(kind, PrimitiveKind::Overlay),
        });
    }

    fn push_instance(&mut self, spec: InstanceSpec) {
        let family_tint = self.config.family.tint();
        let mut color = spec.color;
        for channel in 0..3 {
            color[channel] *= family_tint[channel];
        }
        let fallback =
            self.config.family == PresentationFamily::Hd && spec.appearance.semantic_id % 17 == 0;
        if fallback {
            self.stats.variant_fallbacks = self.stats.variant_fallbacks.saturating_add(1);
            color[0] *= 0.98;
            color[1] *= 0.98;
        }
        let page_id = self.resource_page(spec.appearance.semantic_id);
        self.pages.insert(page_id);
        let order_floor = if spec.screen_space {
            i32::MAX
        } else {
            spec.world.floor
        };
        let order_y = if spec.screen_space {
            i32::MAX - 1
        } else {
            (spec.screen_y * 8.0) as i32
        };
        let order_x = if spec.screen_space {
            i32::MAX - 1
        } else {
            (spec.screen_x * 8.0) as i32
        };
        self.instances.push(RenderInstance {
            appearance: spec.appearance,
            class: spec.class,
            kind: spec.kind,
            material: spec.material,
            world: spec.world,
            screen_x: spec.screen_x,
            screen_y: spec.screen_y,
            width: spec.width,
            height: spec.height,
            color,
            emissive: spec.emissive,
            page_id,
            cell: (spec.appearance.semantic_id + spec.appearance.frame) % PAGE_CELLS,
            critical: spec.critical,
            screen_space: spec.screen_space,
            order_floor,
            order_y,
            order_x,
            sequence: self.sequence,
        });
        self.sequence = self.sequence.wrapping_add(1);
    }

    fn resource_page(&self, semantic_id: u32) -> u32 {
        let visible = self.config.scenario.visible_page_count();
        (self.page_base + semantic_id % visible) % self.config.scenario.total_page_count()
    }

    fn try_decorative(&mut self) -> bool {
        if self.decorative_emitted < self.config.scenario.decorative_budget() {
            self.decorative_emitted += 1;
            true
        } else {
            self.stats.decorative_dropped = self.stats.decorative_dropped.saturating_add(1);
            false
        }
    }

    fn mix_gameplay(&mut self, value: u64) {
        self.gameplay_signature ^= value;
        self.gameplay_signature = self.gameplay_signature.wrapping_mul(0x100_0000_01b3);
    }

    fn finish(
        mut self,
        environment: EnvironmentState,
        camera: CameraState,
        camera_moved: bool,
        zoom_changed: bool,
        floor_changed: bool,
    ) -> FramePlan {
        self.instances.sort_by_key(|instance| instance.order_key());
        FramePlan {
            instances: self.instances,
            needed_pages: self.pages.into_iter().collect(),
            lights: self.lights,
            environment,
            camera,
            camera_moved,
            zoom_changed,
            floor_changed,
            gameplay_signature: self.gameplay_signature,
            stats: self.stats,
        }
    }
}

struct InstanceSpec {
    appearance: AppearanceRef,
    class: PresentationClass,
    kind: PrimitiveKind,
    material: MaterialClass,
    world: WorldPosition,
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    emissive: f32,
    critical: bool,
    screen_space: bool,
}

pub fn project(camera: CameraState, position: WorldPosition) -> (f32, f32) {
    let floor_delta = position.floor - camera.floor;
    let tile = LOGICAL_TILE_PX * camera.zoom;
    let x = (position.x as f32 - camera.x - floor_delta as f32) * tile;
    let y = (position.y as f32 - camera.y - floor_delta as f32) * tile;
    (x, y)
}

pub fn visible_floors(camera: CameraState) -> Vec<i32> {
    if camera.floor == 0 {
        if inside_roof(camera.x, camera.y) {
            vec![0]
        } else {
            vec![0, 1]
        }
    } else if camera.floor < 0 {
        vec![camera.floor, camera.floor + 1]
    } else {
        vec![camera.floor]
    }
}

fn camera_for(frame: u64) -> CameraState {
    let phase = (frame % 720) as f32;
    let x = -10.0 + phase / 24.0;
    let triangle = if phase < 360.0 {
        phase / 45.0
    } else {
        (720.0 - phase) / 45.0
    };
    let zoom = match (frame / 90) % 4 {
        0 => 1.0,
        1 => 1.15,
        2 => 0.85,
        _ => 1.35,
    };
    let floor = if (frame / 240) % 2 == 0 { 0 } else { -1 };
    CameraState {
        x,
        y: -4.0 + triangle,
        floor,
        zoom,
    }
}

fn environment_for(frame: u64) -> EnvironmentState {
    let day_phase = (frame % 600) as f32 / 600.0;
    let ambient = 0.24 + 0.72 * (0.5 + 0.5 * (day_phase * std::f32::consts::TAU).sin());
    let weather = match (frame / 120) % 4 {
        0 => Weather::Clear,
        1 => Weather::Rain,
        2 => Weather::Snow,
        _ => Weather::Fog,
    };
    EnvironmentState {
        ambient,
        weather,
        season: if (frame / 300) % 2 == 0 {
            Season::Normal
        } else {
            Season::Winter
        },
        wind: ((frame as f32) * 0.021).sin(),
    }
}

fn inside_roof(x: f32, y: f32) -> bool {
    (8.0..=14.0).contains(&x) && (8.0..=14.0).contains(&y)
}

fn roof_tile(x: i32, y: i32) -> bool {
    (8..=14).contains(&x) && (8..=14).contains(&y)
}

fn wall_tile(x: i32, y: i32) -> bool {
    ((x == 8 || x == 14) && (8..=14).contains(&y))
        || ((y == 8 || y == 14) && (8..=14).contains(&x) && x != 11)
}

fn spatial_hash(x: i32, y: i32, floor: i32) -> u32 {
    let mut value = (x as u32).wrapping_mul(0x9e37_79b9)
        ^ (y as u32).wrapping_mul(0x85eb_ca6b)
        ^ (floor as u32).wrapping_mul(0xc2b2_ae35);
    value = xorshift32(value ^ 0xa511_e9b3);
    value
}

const fn xorshift32(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

fn projectile_pattern(dx: f32, dy: f32) -> u32 {
    let horizontal = if dx < -0.1 {
        0
    } else if dx > 0.1 {
        2
    } else {
        1
    };
    let vertical = if dy < -0.1 {
        0
    } else if dy > 0.1 {
        2
    } else {
        1
    };
    vertical * 3 + horizontal
}

fn glyph_mask(character: char) -> u64 {
    match character {
        'B' => rows([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ]),
        'D' => rows([
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ]),
        'E' => rows([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
        'M' => rows([
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ]),
        'N' => rows([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
        'O' => rows([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        'S' => rows([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        _ => rows([
            0b11111, 0b10001, 0b00110, 0b00100, 0b00100, 0b00000, 0b00100,
        ]),
    }
}

fn rows(rows: [u64; 7]) -> u64 {
    rows.into_iter()
        .enumerate()
        .fold(0_u64, |mask, (row, bits)| mask | (bits << (row * 5)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> BenchConfig {
        BenchConfig::default()
    }

    #[test]
    fn native_higher_floor_projects_north_west() {
        let camera = CameraState {
            x: 10.0,
            y: 10.0,
            floor: 0,
            zoom: 1.0,
        };
        let (x, y) = project(
            camera,
            WorldPosition {
                x: 10,
                y: 10,
                floor: 1,
            },
        );
        assert_eq!((x, y), (-32.0, -32.0));
    }

    #[test]
    fn roof_hides_upper_floor_when_camera_is_inside() {
        let inside = CameraState {
            x: 10.0,
            y: 10.0,
            floor: 0,
            zoom: 1.0,
        };
        let outside = CameraState {
            x: 2.0,
            y: 2.0,
            floor: 0,
            zoom: 1.0,
        };
        assert_eq!(visible_floors(inside), vec![0]);
        assert_eq!(visible_floors(outside), vec![0, 1]);
    }

    #[test]
    fn same_tile_semantic_order_is_deterministic() {
        let model = SceneModel::new(CorpusShape::default());
        let plan = model.plan(&config(), 0);
        for pair in plan.instances.windows(2) {
            assert!(pair[0].order_key() <= pair[1].order_key());
        }
    }

    #[test]
    fn presentation_family_does_not_change_gameplay_signature() {
        let model = SceneModel::new(CorpusShape::default());
        let mut classic = config();
        classic.family = PresentationFamily::Classic;
        let mut hd = classic.clone();
        hd.family = PresentationFamily::Hd;
        assert_eq!(
            model.plan(&classic, 321).gameplay_signature,
            model.plan(&hd, 321).gameplay_signature
        );
    }

    #[test]
    fn stress_degrades_decorative_before_critical() {
        let model = SceneModel::new(CorpusShape::default());
        let mut stress = config();
        stress.scenario = Scenario::Stress;
        stress.cache_pages = stress.scenario.visible_page_count();
        let plan = model.plan(&stress, 300);
        assert!(plan.stats.decorative_dropped > 0);
        assert!(plan.stats.critical_visible >= 10);
        assert!(plan.instances.iter().any(|instance| instance.critical));
    }

    #[test]
    fn hd_missing_variants_use_deterministic_fallback() {
        let model = SceneModel::new(CorpusShape::default());
        let mut hd = config();
        hd.family = PresentationFamily::Hd;
        let left = model.plan(&hd, 200);
        let right = model.plan(&hd, 200);
        assert!(left.stats.variant_fallbacks > 0);
        assert_eq!(left.stats.variant_fallbacks, right.stats.variant_fallbacks);
    }

    #[test]
    fn projectile_pattern_is_presentation_only_and_stable() {
        assert_eq!(projectile_pattern(-1.0, -1.0), 0);
        assert_eq!(projectile_pattern(0.0, 0.0), 4);
        assert_eq!(projectile_pattern(1.0, 1.0), 8);
    }
}
