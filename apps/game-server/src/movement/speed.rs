//! SPEED-1 effective speed and step duration (CONDITIONS-0 §4.1, §4.2).
//!
//! The step speed of an effective speed comes from the checked-in table
//! `content/movement/step_speed_v1.json` (generated offline by
//! `tools/content-schema/step-speed`), so nothing evaluates `ln` here. The ground speed of a tile
//! comes from a [`GroundSpeedSource`]: on `main` the production map is the engineering static
//! cell index, which carries no ground item, so production uses [`EngineeringGroundSpeed`] (150
//! for every tile) until MAP-CLIENT-1 switches it to the map source (ARCH-BATCH-ITEM-EQUIP §1.11).
//! [`MapGroundSpeed`] is that map source (MAP-LOAD-1): built and tested, not on the live path.

use crate::content::LogicalCell;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::world_runtime::WorldBaseHandle;
use oteryn_simulation_determinism::SemanticTimeMicros;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use std::time::Duration;

/// Player base speed at level 1 (vocation data, CONDITIONS-0 §4.1).
pub(crate) const PLAYER_BASE_SPEED: i64 = 110;
/// The effective speed clamp (§4.1); the table covers exactly this range.
pub(crate) const SPEED_MIN: u16 = 10;
pub(crate) const SPEED_MAX: u16 = u16::MAX;
/// The ground speed of a tile whose ground item names none (§4.2).
pub(crate) const DEFAULT_GROUND_SPEED: u16 = 150;
/// `SERVER_BEAT`: a step duration is rounded up to a multiple of it (§4.2).
pub(crate) const SERVER_BEAT_MS: u64 = 50;

const TABLE_JSON: &str = include_str!("../../../../content/movement/step_speed_v1.json");
const TABLE_SCHEMA: &str = "OTERYN_STEP_SPEED_TABLE/v1";
/// The digest SPEED-1 checked in; a table with another digest is never used.
pub(crate) const TABLE_SHA256_U16LE: &str =
    "323b70ceb76edc53ed341a31d67fea5543149689ead9ad51856286e792bfe436";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TableFile {
    schema: String,
    #[allow(dead_code, reason = "provenance text, checked by the generator test")]
    source: String,
    #[allow(dead_code, reason = "provenance text, checked by the generator test")]
    formula: String,
    speed_min: u16,
    speed_max: u16,
    sha256_u16le: String,
    step_speed: Vec<u16>,
}

/// The step speed of every effective speed from [`SPEED_MIN`] to [`SPEED_MAX`].
pub(crate) struct StepSpeedTable {
    step_speed: Vec<u16>,
}

impl StepSpeedTable {
    /// Parse and verify a table file: schema, range, length, digest and no zero entry.
    fn parse(text: &str) -> Option<Self> {
        let file: TableFile = serde_json::from_str(text).ok()?;
        let length = usize::from(SPEED_MAX - SPEED_MIN) + 1;
        if file.schema != TABLE_SCHEMA
            || (file.speed_min, file.speed_max) != (SPEED_MIN, SPEED_MAX)
            || file.step_speed.len() != length
            || file.step_speed.contains(&0)
            || file.sha256_u16le != TABLE_SHA256_U16LE
        {
            return None;
        }
        let mut hasher = Sha256::new();
        for value in &file.step_speed {
            hasher.update(value.to_le_bytes());
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        (digest == file.sha256_u16le).then_some(Self {
            step_speed: file.step_speed,
        })
    }

    /// The checked-in table, parsed and verified once. `None` (every paced step refused) if the
    /// embedded file does not verify, which the tests rule out.
    pub(crate) fn embedded() -> Option<&'static Self> {
        static TABLE: OnceLock<Option<StepSpeedTable>> = OnceLock::new();
        TABLE.get_or_init(|| Self::parse(TABLE_JSON)).as_ref()
    }

    /// The step speed of an effective speed within the clamp.
    pub(crate) fn step_speed(&self, speed: u16) -> Option<u16> {
        let index = usize::from(speed.checked_sub(SPEED_MIN)?);
        self.step_speed.get(index).copied()
    }

    /// Step duration (§4.2): `floor(1000 × ground speed / step speed)` ms, rounded up to a multiple
    /// of `SERVER_BEAT`. `None` for ground speed 0: nothing paces on it, so the step is refused.
    /// Only cardinal steps exist (`CardinalStep`), so the diagonal × 3 never applies; a monster's
    /// near-target × 2 belongs to CREATURE-MOVE-1.
    pub(crate) fn step_duration(&self, speed: u16, ground_speed: u16) -> Option<Duration> {
        if ground_speed == 0 {
            return None;
        }
        let step_speed = u64::from(self.step_speed(speed)?);
        let raw = 1000 * u64::from(ground_speed) / step_speed;
        let beats = raw.div_ceil(SERVER_BEAT_MS);
        Some(Duration::from_millis(beats * SERVER_BEAT_MS))
    }
}

/// A player's effective speed (§4.1): base 110 + level − 1, plus the `SPEED` condition delta,
/// plus the worn-equipment speed (0 until EQUIP-RT-1 supplies it), clamped to `[10, 65,535]`.
pub(crate) fn player_effective_speed(level: u32, speed_delta: i64, equipment_speed: i64) -> u16 {
    effective_speed(
        PLAYER_BASE_SPEED + i64::from(level) - 1,
        speed_delta,
        equipment_speed,
    )
}

/// The effective speed of a `base` speed with its condition delta and equipment speed (§4.1).
pub(crate) fn effective_speed(base: i64, speed_delta: i64, equipment_speed: i64) -> u16 {
    let speed = base
        .saturating_add(speed_delta)
        .saturating_add(equipment_speed)
        .clamp(i64::from(SPEED_MIN), i64::from(SPEED_MAX));
    u16::try_from(speed).unwrap_or(SPEED_MIN)
}

/// A player's base speed at `level` (§4.1), capped at [`SPEED_MAX`]. `None` for level 0.
pub(crate) fn player_base_speed(level: u32) -> Option<u32> {
    let base = PLAYER_BASE_SPEED.checked_add(i64::from(level.checked_sub(1)?))?;
    u32::try_from(base.min(i64::from(SPEED_MAX))).ok()
}

/// The effective speed of the player `actor` of `session` at the owner time `now`: the level
/// term and the actor's active `SPEED` condition delta from the Channel runtime's condition
/// owner (§4.1). `None` when that delta cannot be read; the step is then refused.
pub(crate) fn runtime_player_speed(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    level: u32,
    now: SemanticTimeMicros,
) -> Option<u16> {
    let speed_delta = runtime
        .actor_active_speed_delta(actor, Some(session), now)
        .ok()?;
    Some(player_effective_speed(level, speed_delta, 0))
}

/// The duration of a player step at `speed` onto `onto`, with the ground speed `source` gives that
/// tile. `None` refuses the step: the table did not verify, or the ground speed is 0.
pub(crate) fn player_step_duration(
    source: &impl GroundSpeedSource,
    onto: LogicalCell,
    speed: u16,
) -> Option<Duration> {
    StepSpeedTable::embedded()?.step_duration(speed, source.ground_speed(onto))
}

/// The ground speed of the ground item on a tile (§1.11 seam).
pub(crate) trait GroundSpeedSource {
    fn ground_speed(&self, cell: LogicalCell) -> u16;
}

/// The engineering map's source: it has no ground item with another speed, so every tile is 150.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EngineeringGroundSpeed;

impl GroundSpeedSource for EngineeringGroundSpeed {
    fn ground_speed(&self, _cell: LogicalCell) -> u16 {
        DEFAULT_GROUND_SPEED
    }
}

/// The map source (MAP-LOAD-PACKET-1 §1.3): the ground speed of the tile's ground item in the
/// World's base map, and 0 for a tile without a ground item, with a non-walkable one, or outside
/// the map, so `player_step_duration` refuses the step. `cell.z` is legacy `z` (native floor
/// `-z`).
#[derive(Debug, Clone)]
pub(crate) struct MapGroundSpeed {
    pub(crate) base: WorldBaseHandle,
}

impl GroundSpeedSource for MapGroundSpeed {
    fn ground_speed(&self, cell: LogicalCell) -> u16 {
        let (Ok(x), Ok(y), Ok(z)) = (
            u16::try_from(cell.x),
            u16::try_from(cell.y),
            i8::try_from(cell.z),
        ) else {
            return 0;
        };
        if !(0..=15).contains(&z) {
            return 0;
        }
        self.base.ground_speed(x, y, -z)
    }
}

/// The qualified spell tiles of the active generation's cells: a tile whose ground is present
/// and qualified gives its ground item's speed (0 meaning none, so 150); an unknown tile, or one
/// without qualified ground, gives 0, so a step onto it is refused rather than guessed.
#[derive(Clone, Copy)]
pub(crate) struct QualifiedCellGroundSpeed<'a> {
    pub(crate) cells: &'a crate::content::NativeEntryMovementCells,
}

/// The ground speed of a qualified tile's ground item: 0 names none, so 150 (§4.2).
pub(crate) fn qualified_ground_speed(present: bool, speed: Option<u16>) -> u16 {
    match speed {
        Some(0) if present => DEFAULT_GROUND_SPEED,
        Some(speed) if present => speed,
        _ => 0,
    }
}

impl GroundSpeedSource for QualifiedCellGroundSpeed<'_> {
    fn ground_speed(&self, cell: LogicalCell) -> u16 {
        self.cells
            .spell_tiles()
            .lookup(self.cells.scope(), cell)
            .map_or(0, |tile| {
                qualified_ground_speed(tile.ground_present(), tile.ground_speed())
            })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn table() -> &'static StepSpeedTable {
        StepSpeedTable::embedded().expect("the checked-in table verifies")
    }

    #[test]
    fn the_embedded_table_verifies_and_matches_canary_samples() {
        // Canary `04b83b51` step speeds (`creature.hpp:76-78, 1061-1067`), as the generator test.
        for (speed, step_speed) in [
            (10, 9),
            (40, 99),
            (110, 278),
            (111, 280),
            (150, 366),
            (220, 500),
            (1000, 1326),
            (65_535, 4717),
        ] {
            assert_eq!(table().step_speed(speed), Some(step_speed), "{speed}");
        }
        assert_eq!(table().step_speed(SPEED_MIN - 1), None);
    }

    #[test]
    fn a_table_with_another_digest_or_shape_is_refused() {
        assert!(StepSpeedTable::parse(TABLE_JSON).is_some());
        let tampered = TABLE_JSON.replacen("    9,", "    8,", 1);
        assert_ne!(tampered, TABLE_JSON);
        assert!(StepSpeedTable::parse(&tampered).is_none());
        let renamed = TABLE_JSON.replacen(TABLE_SCHEMA, "OTERYN_STEP_SPEED_TABLE/v2", 1);
        assert!(StepSpeedTable::parse(&renamed).is_none());
        assert!(StepSpeedTable::parse("{}").is_none());
    }

    #[test]
    fn effective_speed_adds_level_condition_and_equipment_and_clamps() {
        assert_eq!(player_effective_speed(1, 0, 0), 110);
        assert_eq!(player_effective_speed(8, 0, 0), 117);
        assert_eq!(player_effective_speed(111, 0, 0), 220);
        assert_eq!(player_effective_speed(8, 40, 0), 157);
        assert_eq!(player_effective_speed(8, -77, 0), 40);
        assert_eq!(player_effective_speed(8, 0, 20), 137);
        assert_eq!(player_effective_speed(1, -1_000, 0), SPEED_MIN);
        assert_eq!(
            player_effective_speed(u32::MAX, i64::MAX, i64::MAX),
            SPEED_MAX
        );
        assert_eq!(player_effective_speed(1, i64::MIN, i64::MIN), SPEED_MIN);
    }

    #[test]
    fn step_duration_rounds_up_to_the_beat_and_follows_ground_speed() {
        // 1000 × 150 / 278 = 539 → 550 ms: a level 1 player on default ground.
        assert_eq!(
            table().step_duration(110, DEFAULT_GROUND_SPEED),
            Some(Duration::from_millis(550))
        );
        // 1000 × 150 / 500 = 300, already on the beat.
        assert_eq!(
            table().step_duration(220, DEFAULT_GROUND_SPEED),
            Some(Duration::from_millis(300))
        );
        // A slower ground: 1000 × 200 / 278 = 719 → 750 ms.
        assert_eq!(
            table().step_duration(110, 200),
            Some(Duration::from_millis(750))
        );
        // The clamp floor and the bounded ground ceiling stay finite.
        assert_eq!(
            table().step_duration(SPEED_MIN, 1_000),
            Some(Duration::from_millis(111_150))
        );
        assert_eq!(table().step_duration(110, 0), None);
        assert_eq!(table().step_duration(SPEED_MIN - 1, 150), None);
    }

    /// An injected source: one tile with another ground speed, one with 0, 150 elsewhere.
    struct Slow;

    impl GroundSpeedSource for Slow {
        fn ground_speed(&self, cell: LogicalCell) -> u16 {
            match (cell.x, cell.y) {
                (1, 0) => 200,
                (2, 0) => 0,
                _ => DEFAULT_GROUND_SPEED,
            }
        }
    }

    #[test]
    fn a_step_onto_a_tile_with_another_ground_speed_takes_its_duration() {
        let speed = player_effective_speed(1, 0, 0);
        let at = |x| LogicalCell { x, y: 0, z: 7 };
        assert_eq!(
            player_step_duration(&Slow, at(0), speed),
            Some(Duration::from_millis(550))
        );
        assert_eq!(
            player_step_duration(&Slow, at(1), speed),
            Some(Duration::from_millis(750))
        );
        assert_eq!(player_step_duration(&Slow, at(2), speed), None);
        assert_eq!(
            player_step_duration(&EngineeringGroundSpeed, at(1), speed),
            Some(Duration::from_millis(550))
        );
    }

    #[test]
    fn base_speed_and_qualified_ground_are_never_guessed() {
        assert_eq!(player_base_speed(1), Some(110));
        assert_eq!(player_base_speed(291), Some(400));
        assert_eq!(player_base_speed(100_000), Some(u32::from(SPEED_MAX)));
        assert_eq!(player_base_speed(0), None);
        assert_eq!(effective_speed(400, 107, 0), 507);
        assert_eq!(effective_speed(i64::MAX, i64::MAX, 0), SPEED_MAX);
        assert_eq!(qualified_ground_speed(true, Some(400)), 400);
        assert_eq!(qualified_ground_speed(true, Some(0)), DEFAULT_GROUND_SPEED);
        assert_eq!(qualified_ground_speed(true, None), 0);
        assert_eq!(qualified_ground_speed(false, Some(400)), 0);
        // A haste delta shortens the step on slow qualified ground.
        assert_eq!(
            table().step_duration(effective_speed(400, 0, 0), 400),
            Some(Duration::from_millis(550))
        );
        assert_eq!(
            table().step_duration(effective_speed(400, 107, 0), 400),
            Some(Duration::from_millis(450))
        );
    }

    #[test]
    fn the_engineering_source_is_150_everywhere() {
        for cell in [
            LogicalCell { x: 0, y: 0, z: 7 },
            LogicalCell {
                x: i32::MIN,
                y: i32::MAX,
                z: 0,
            },
        ] {
            assert_eq!(EngineeringGroundSpeed.ground_speed(cell), 150);
        }
    }

    /// A one-sector base map at native floor -7 (legacy `z` 7), written by the compiler's
    /// writer: grass (300) at x 1, a non-walkable ground storing 120 at x 2, a plain item without
    /// a ground at x 3, and a non-walkable ground storing 0 at x 4.
    fn map_source() -> MapGroundSpeed {
        use oteryn_world_bundle_compiler::bundle::{
            self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
            TerrainKind,
        };
        use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
        let ground = |key: &str, id, walkable, speed| PaletteEntry {
            key: key.into(),
            family: Family::Terrain,
            id,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(walkable),
                ground_speed: Some(speed),
            }),
        };
        let manifest = Manifest {
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
            projection_class: "server".into(),
            compiler_version: "test".into(),
            build_class: BuildClass::Production,
            identity: Identity {
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "rev-1".into(),
                ..Identity::default()
            },
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 32,
                max_y: 32,
                floors: vec![-7],
            },
            palette: vec![
                ground("terrain:mud", 1, true, 300),
                ground("terrain:lava", 2, false, 120),
                PaletteEntry {
                    key: "item:chest".into(),
                    family: Family::Item,
                    id: 3,
                    terrain: None,
                },
                ground("terrain:void", 4, false, 0),
            ],
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
        };
        let tile = |x, palette| Tile {
            x,
            y: 0,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: vec![Item {
                palette,
                depth: 0,
                attrs: Attrs::default(),
            }],
        };
        let sector = Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: (1..=4).map(|x| tile(x, u32::from(x) - 1)).collect(),
        };
        let bytes =
            bundle::write(&manifest, &[sector], &Default::default()).expect("written bundle");
        let digest = bundle::read(&bytes).expect("read bundle").digest;
        let pins = crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: true,
        };
        let base = crate::map::load(&bytes, &pins).expect("loaded base");
        MapGroundSpeed {
            base: std::sync::Arc::new(base),
        }
    }

    #[test]
    fn map_load_the_map_source_gives_the_ground_item_speed() {
        let source = map_source();
        let at = |x, z| LogicalCell { x, y: 0, z };
        assert_eq!(source.ground_speed(at(1, 7)), 300);
        // Non-walkable grounds, a tile without a ground item, and no tile at all: 0.
        assert_eq!(source.ground_speed(at(2, 7)), 0);
        assert_eq!(
            source
                .base
                .tile(2, 0, -7)
                .expect("lava")
                .stored_ground_speed(),
            120
        );
        assert_eq!(source.ground_speed(at(3, 7)), 0);
        assert_eq!(source.ground_speed(at(4, 7)), 0);
        assert_eq!(source.ground_speed(at(5, 7)), 0);
        // Another floor, and coordinates outside every floor or the u16 grid.
        assert_eq!(source.ground_speed(at(1, 6)), 0);
        for cell in [at(1, -1), at(1, 16), at(-1, 7), at(70_000, 7)] {
            assert_eq!(source.ground_speed(cell), 0, "{cell:?}");
        }
    }

    #[test]
    fn map_load_a_step_paces_on_the_map_ground_speed_and_refuses_0() {
        let source = map_source();
        let speed = player_effective_speed(1, 0, 0);
        let at = |x| LogicalCell { x, y: 0, z: 7 };
        // 1000 × 300 / 278 = 1079 → 1100 ms.
        assert_eq!(
            player_step_duration(&source, at(1), speed),
            Some(Duration::from_millis(1100))
        );
        for x in [2, 3, 4, 5] {
            assert_eq!(player_step_duration(&source, at(x), speed), None, "{x}");
        }
    }

    /// A charm's speed condition from the authored rows, as the condition owner tests load it.
    fn charm(name: &str) -> crate::foundation::ConditionDefinition {
        use crate::foundation::{ConditionDefinition, ConditionValues, SpeedRange};
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/condition-authoring/authored-conditions.json"
        ))
        .expect("authored conditions");
        let row = rows["rows"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["key"] == format!("charm.{name}"))
            })
            .expect("charm row");
        let values = &row["speed"];
        let n = |key: &str| i32::try_from(values[key].as_i64().expect("i64")).expect("i32");
        ConditionDefinition::new(
            &format!("oteryn:condition.charm.{name}"),
            1,
            ConditionValues::Speed {
                paralysis: values["kind"] == "paralysis",
                range: SpeedRange {
                    a_min: n("a_min"),
                    b_min: n("b_min"),
                    a_max: n("a_max"),
                    b_max: n("b_max"),
                },
                duration_ms: u32::try_from(values["duration_ms"].as_u64().expect("u64"))
                    .expect("u32"),
            },
        )
        .expect("definition")
    }

    /// The step duration of the player after `name` (if any) was applied to them at time 0.
    fn duration_with(name: Option<&str>, tag: u8) -> Option<Duration> {
        use crate::foundation::{
            ActorConditionTransition, ApplicationFacts, ConditionSource, ConditionSourceKind,
        };
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(tag);
        let at = SemanticTimeMicros::from_micros;
        if let Some(name) = name {
            let definition = charm(name);
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            let plan = runtime
                .prepare_actor_condition(
                    actor,
                    Some(session),
                    ActorConditionTransition::Apply {
                        definition: &definition,
                        source: ConditionSource {
                            actor,
                            session: Some(session),
                            kind: ConditionSourceKind::SelfUse,
                        },
                        immunities: &[],
                        facts: ApplicationFacts {
                            now: 0,
                            base_speed: player_effective_speed(1, 0, 0),
                            mana_shield_capacity: 0,
                            target_reentry_protected: false,
                            source_reentry_protected: false,
                            target_is_player: true,
                            decision_root: &root,
                            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
                        },
                    },
                    at(0),
                )
                .expect("prepared");
            assert_eq!(runtime.commit_actor_condition(&plan, at(0)), Ok(true));
        }
        let speed = runtime_player_speed(&runtime, actor, session, 1, at(1))?;
        player_step_duration(
            &EngineeringGroundSpeed,
            LogicalCell { x: 0, y: 0, z: 7 },
            speed,
        )
    }

    #[test]
    fn haste_and_paralysis_on_the_runtime_actor_change_the_step_duration() {
        let plain = duration_with(None, 0x81).expect("plain");
        assert_eq!(plain, Duration::from_millis(550));
        let haste = duration_with(Some("adrenaline_burst"), 0x82).expect("haste");
        let paralysis = duration_with(Some("cripple"), 0x83).expect("paralysis");
        assert!(haste < plain, "{haste:?} < {plain:?}");
        assert!(paralysis > plain, "{paralysis:?} > {plain:?}");
    }
}

/// Read-only source facts supplied by the current native definition owner. Missing source
/// membership/base speed never becomes a player/default speed or a caller-minted grant.
pub(crate) trait CurrentCreatureSpeedReader {
    fn current_native_base_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<u16>;
}
/// Source-defined base plus the ONE actual occupied actor-slot condition owner.
/// The exact physical generation and condition clock are checked by Foundation.
pub(crate) fn runtime_creature_speed(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    source: &dyn CurrentCreatureSpeedReader,
    now: SemanticTimeMicros,
) -> Option<u16> {
    if !runtime.contains_live_creature(actor) {
        return None;
    }
    let base = source.current_native_base_speed(runtime, actor)?;
    if base == 0 {
        return None;
    } // source immobility is not a manufactured speed floor
    let delta = runtime.actor_active_speed_delta(actor, None, now).ok()?;
    u16::try_from(
        i64::from(base)
            .saturating_add(delta)
            .clamp(i64::from(SPEED_MIN), i64::from(SPEED_MAX)),
    )
    .ok()
}
/// Pure cadence calculation: exactly the same table, ground provider and50ms beat as players.
/// This issues neither a Movement decision nor scope/session authority. The caller still
/// uses the existing Movement owner and records its pacer only after a real moved step.
pub(crate) fn creature_step_duration(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    source: &dyn CurrentCreatureSpeedReader,
    ground: &dyn GroundSpeedSource,
    onto: LogicalCell,
    now: SemanticTimeMicros,
) -> Option<Duration> {
    StepSpeedTable::embedded()?.step_duration(
        runtime_creature_speed(runtime, actor, source, now)?,
        ground.ground_speed(onto),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod creature_native_speed_tests {
    use super::*;
    use crate::foundation::{
        ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
        ConditionSourceKind, ConditionValues, MovementLocalPosition, SpeedRange,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    struct Native {
        actor: ExactActorRef,
        base: u16,
        pin: [u8; 32],
    }
    impl CurrentCreatureSpeedReader for Native {
        fn current_native_base_speed(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> Option<u16> {
            (a == self.actor
                && r.contains_live_creature(a)
                && r.content_pin().server_artifact_digest() == self.pin)
                .then_some(self.base)
        }
    }
    #[test]
    fn creature_cadence_reads_real_slot_haste_expiry_ground_and_stale_generation() {
        let (mut r, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x71);
        let position = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let actor = r.admit_test_creature(position).unwrap();
        let src = Native {
            actor,
            base: 220,
            pin: r.content_pin().server_artifact_digest(),
        };
        let onto = LogicalCell {
            x: 101,
            y: 100,
            z: 7,
        };
        let at = SemanticTimeMicros::from_micros;
        assert_eq!(
            creature_step_duration(&r, actor, &src, &EngineeringGroundSpeed, onto, at(0)),
            Some(Duration::from_millis(300))
        );
        let definition = ConditionDefinition::new(
            "speed.creature.fixture",
            1,
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 0,
                    b_min: 330,
                    a_max: 0,
                    b_max: 330,
                },
                duration_ms: 1000,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let plan = r
            .prepare_actor_condition(
                actor,
                None,
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor,
                        session: None,
                        kind: ConditionSourceKind::Creature,
                    },
                    immunities: &[],
                    facts: ApplicationFacts {
                        now: 100,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
                    },
                },
                at(100),
            )
            .unwrap();
        assert!(r.commit_actor_condition(&plan, at(100)).unwrap());
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(99)), None);
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(101)), Some(330));
        assert_eq!(
            creature_step_duration(&r, actor, &src, &EngineeringGroundSpeed, onto, at(101)),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            creature_step_duration(
                &r,
                actor,
                &src,
                &EngineeringGroundSpeed,
                onto,
                at(1_000_100)
            ),
            Some(Duration::from_millis(300))
        );
        struct Unknown;
        impl GroundSpeedSource for Unknown {
            fn ground_speed(&self, _: LogicalCell) -> u16 {
                0
            }
        }
        assert_eq!(
            creature_step_duration(&r, actor, &src, &Unknown, onto, at(1_000_100)),
            None
        );
        let wrong = Native {
            pin: [0; 32],
            ..src
        };
        assert_eq!(
            runtime_creature_speed(&r, actor, &wrong, at(1_000_100)),
            None
        );
        r.remove_test_actor(actor).unwrap();
        let replacement = r.admit_test_creature(position).unwrap();
        assert_ne!(replacement, actor);
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(1_000_100)), None);
    }
    #[test]
    fn native_player110_and_creature220_share_golden_beats_but_not_default_base() {
        let table = StepSpeedTable::embedded().unwrap();
        for (base, ground, expected) in [
            (110, 150, 550),
            (110, 200, 750),
            (128, 150, 500),
            (220, 150, 300),
            (220, 200, 400),
            (330, 150, 250),
            (440, 150, 200),
        ] {
            assert_eq!(
                table.step_duration(base, ground),
                Some(Duration::from_millis(expected))
            );
        }
        assert_eq!(PLAYER_BASE_SPEED, 110);
    }
    #[test]
    #[ignore = "explicit local bounded kernel benchmark; no owner/world mutations"]
    fn benchmark_canonical_creature_cadence_lookup() {
        let table = StepSpeedTable::embedded().unwrap();
        let started = std::time::Instant::now();
        let mut sum = 0u128;
        for i in 0..1_000_000u32 {
            let speed = 10 + (i % 65_526) as u16;
            let value = table
                .step_duration(std::hint::black_box(speed), std::hint::black_box(150))
                .unwrap();
            sum += std::hint::black_box(value.as_micros());
        }
        assert!(sum > 0);
        eprintln!(
            "canonical_step_duration_1m_elapsed_ns={}",
            started.elapsed().as_nanos()
        );
    }
}

/// Immutable descriptor from the native definition owner's selected Creature closure.
/// Caller supplies real loader records/profiles; digest equality is ONLY an expected pin
/// check, not permission to invent source membership, a movement actor or a scope grant.
#[derive(Debug, Clone)]
pub(crate) struct NativeCreatureSpeed {
    creature: crate::content::ProjectV2DefinitionRef,
    base: u16,
    server: [u8; 32],
}
impl NativeCreatureSpeed {
    pub(crate) fn from_native(
        runtime: &ChannelRuntimeV1,
        creature: &crate::content::ProjectV2DefinitionRef,
        records: &[crate::content::ProjectReferenceRecord],
        profiles: &[crate::content::ProjectV2AuthoringProfile],
        loader_digest: [u8; 32],
    ) -> Option<Self> {
        use crate::content::{
            ProjectReferenceRecord, ProjectV2AuthoringProfileData, ProjectV2Family,
        };
        if creature.family != ProjectV2Family::Creature
            || loader_digest != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let mut core=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..}if identity.family=="Creature"&&identity.key==creature.key&&identity.revision==creature.revision));
        core.next()?;
        if core.next().is_some() {
            return None;
        }
        let mut values = profiles.iter().filter(|p| p.target == *creature);
        let value = values.next()?;
        if values.next().is_some() {
            return None;
        }
        let ProjectV2AuthoringProfileData::Creature(profile) = &value.data else {
            return None;
        };
        let base = u16::try_from(profile.speed?).ok()?;
        Some(Self {
            creature: creature.clone(),
            base,
            server: loader_digest,
        })
    }
}
impl CurrentCreatureSpeedReader for NativeCreatureSpeed {
    fn current_native_base_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<u16> {
        if self.server != runtime.content_pin().server_artifact_digest()
            || !runtime.matches_live_creature_identity(actor, self.creature.key.as_bytes())
        {
            return None;
        }
        runtime.read_actor_position(actor).ok()?;
        Some(self.base)
    }
}

/// Per-exact-actor cadence state; no actor census, condition store or Movement authority.
/// Caller retains this alongside the existing native actor owner and supplies current work.
pub(crate) struct CreatureMovementCadence {
    actor: ExactActorRef,
    content: [u8; 32],
    pacer: crate::movement::pacing::CreatureStepPacer,
}
#[derive(Debug)]
pub(crate) enum CreatureCadenceError {
    StaleOwner,
    StaleActor,
    MissingSpeed,
    MissingMap,
    ClockOverflow,
    Carrier(crate::foundation::CarrierError),
    Movement(crate::movement::MovementError),
}
impl CreatureMovementCadence {
    pub(crate) fn bind(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<Self> {
        if !runtime.contains_live_creature(actor) {
            return None;
        }
        Some(Self {
            actor,
            content: runtime.content_pin().server_artifact_digest(),
            pacer: Default::default(),
        })
    }
    /// Native intrinsic summons use their existing typed removal-aware Movement sibling;
    /// this regular-creature path must never bypass its distance cleanup.
    // Keep step ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        source: &dyn CurrentCreatureSpeedReader,
        ground: &dyn GroundSpeedSource,
        now: SemanticTimeMicros,
        scope: &crate::content::static_cell_engine::EngineeringStaticCellScope,
        index: &crate::content::static_cell_engine::EngineeringStaticCellIndex,
        blocking: &[LogicalCell],
        direction: crate::movement::CardinalStep,
    ) -> Result<Option<crate::foundation::MovementPositionSnapshot>, CreatureCadenceError> {
        use CreatureCadenceError as E;
        let b = runtime.binding();
        if !current.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(E::StaleOwner);
        }
        if self.content != runtime.content_pin().server_artifact_digest()
            || !runtime.contains_live_creature(self.actor)
            || runtime
                .native_summon_role(self.actor)
                .map_err(E::Carrier)?
                .is_some()
        {
            return Err(E::StaleActor);
        }
        let snapshot = runtime
            .read_actor_position(self.actor)
            .map_err(E::Carrier)?;
        if scope.world_id != snapshot.world_id() || scope.generation_digest != self.content {
            return Err(E::MissingMap);
        }
        let at = snapshot.position();
        let (dx, dy) = match direction {
            crate::movement::CardinalStep::North => (0, -1),
            crate::movement::CardinalStep::East => (1, 0),
            crate::movement::CardinalStep::South => (0, 1),
            crate::movement::CardinalStep::West => (-1, 0),
        };
        let next = crate::foundation::MovementLocalPosition {
            x: at.x.checked_add(dx).ok_or(E::MissingMap)?,
            y: at.y.checked_add(dy).ok_or(E::MissingMap)?,
            floor: at.floor,
        };
        let onto = LogicalCell {
            x: next.x,
            y: next.y,
            z: i32::from(next.floor),
        };
        let duration = creature_step_duration(runtime, self.actor, source, ground, onto, now)
            .ok_or(E::MissingSpeed)?;
        if self.pacer.wait_until(now).is_some() {
            return Ok(None);
        }
        self.pacer
            .prepare_moved_deadline(now, duration)
            .ok_or(E::ClockOverflow)?;
        if blocking.contains(&onto) || !runtime.native_summon_cell_free(next) {
            return Err(E::MissingMap);
        }
        let selection = crate::movement::MovementEngineeringSelection {
            owner_context: snapshot.context(),
            content_scope: scope,
        };
        let outcome = {
            let mut turn =
                crate::movement::MovementOwnerTurn::begin(runtime, std::num::NonZeroUsize::MIN);
            turn.try_step(self.actor, snapshot, &selection, index, direction)
                .map_err(E::Movement)?
        };
        match outcome {
            crate::movement::MovementTurnOutcome::Applied(after) => {
                self.pacer.record(now, Some(duration));
                Ok(Some(after))
            }
            crate::movement::MovementTurnOutcome::Deferred => Ok(None),
        }
    }
}
