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
use std::time::Duration;

pub use oteryn_gameplay_characters::speed::*;

/// The effective speed of the player `actor` of `session` at the owner time `now`: the level
/// term and the actor's active `SPEED` condition delta from the Channel runtime's condition
/// owner (§4.1). `None` when that delta cannot be read; the step is then refused.
pub fn runtime_player_speed(
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

/// The map source (MAP-LOAD-PACKET-1 §1.3): the ground speed of the tile's ground item in the
/// World's base map, and 0 for a tile without a ground item, with a non-walkable one, or outside
/// the map, so `player_step_duration` refuses the step. `cell.z` is legacy `z` (native floor
/// `-z`).
#[derive(Debug, Clone)]
pub struct MapGroundSpeed {
    pub base: WorldBaseHandle,
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
pub struct QualifiedCellGroundSpeed<'a> {
    pub cells: &'a crate::content::NativeEntryMovementCells,
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
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    /// A one-sector base map at native floor -7 (legacy `z` 7), written by the compiler's
    /// writer: grass (300) at x 1, a non-walkable ground storing 120 at x 2, a plain item without
    /// a ground at x 3, and a non-walkable ground storing 0 at x 4.
    fn map_source() -> MapGroundSpeed {
        use oteryn_world_bundle::sector::{Attrs, Item, Tile};
        use oteryn_world_bundle::writer::{
            self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
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
            npcs: Default::default(),
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
}

/// Read-only source facts supplied by the current native definition owner. Missing source
/// membership/base speed never becomes a player/default speed or a caller-minted grant.
pub trait CurrentCreatureSpeedReader {
    fn current_native_base_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<u16>;
}
/// Source-defined base plus the ONE actual occupied actor-slot condition owner.
/// The exact physical generation and condition clock are checked by Foundation.
pub fn runtime_creature_speed(
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
    Some(effective_speed(i64::from(base), delta, 0))
}
/// Pure cadence calculation: exactly the same table, ground provider and50ms beat as players.
/// This issues neither a Movement decision nor scope/session authority. The caller still
/// uses the existing Movement owner and records its pacer only after a real moved step.
pub fn creature_step_duration(
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

/// Immutable descriptor from the native definition owner's selected Creature closure.
/// Caller supplies real loader records/profiles; digest equality is ONLY an expected pin
/// check, not permission to invent source membership, a movement actor or a scope grant.
#[derive(Debug, Clone)]
pub struct NativeCreatureSpeed {
    creature: crate::content::ProjectV2DefinitionRef,
    base: u16,
    server: [u8; 32],
}
impl NativeCreatureSpeed {
    pub fn from_native(
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
pub struct CreatureMovementCadence {
    actor: ExactActorRef,
    content: [u8; 32],
    pacer: crate::movement::pacing::CreatureStepPacer,
}
#[derive(Debug)]
pub enum CreatureCadenceError {
    StaleOwner,
    StaleActor,
    MissingSpeed,
    MissingMap,
    ClockOverflow,
    Carrier(oteryn_gameplay_domain::domain::carrier_values::CarrierError),
    Movement(crate::movement::MovementError),
}
impl CreatureMovementCadence {
    pub fn bind(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<Self> {
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
    pub fn step(
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
