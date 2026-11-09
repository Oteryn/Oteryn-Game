//! NPC-LOAD-1: the World Bundle's NPC placements become the Channel's NPC actors at boot
//! (NPC-PLACE-1 §7, NPC-BEHAVIOUR-0 §3.1).
//!
//! The format v4 NPC frame is read from the same verified bundle bytes the boot checks, bound
//! to the NPC catalogue the node loaded (`--npc-data-project`), and placed once, at Channel
//! start, through [`ChannelRuntimeV1::place_npcs`], the same carrier seam the NPC actors have
//! always used: one actor per placement in frame order. The world-clock owner reconciles
//! admitted day/night and weekday NPCs through the same fenced native placement APIs.
//!
//! §7 decides what an unloadable NPC does. In a production World the bundle is refused when no
//! catalogue is loaded, when its `catalogue_sha256` is not the loaded catalogue's, or when an
//! NPC is held by the catalogue or not in it. Outside production the same cases are logged and
//! get no actor: a catalogue the bundle was not compiled against places no NPC at all, and an
//! NPC the catalogue does not admit is skipped by key.

use super::{BootError, event};
use crate::foundation::{
    ChannelRuntimeV1, MovementFacing, NpcPlacement, NpcPlacementError, NpcPlacementTable,
    NpcPlacements,
};
use crate::gameplay_transport::NpcTalkBook;
use oteryn_world_bundle::{bundle, npc};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

mod seasonal;
#[path = "bundle_npcs/weekday.rs"]
mod weekday;

/// W05c: NPCs that stand only by day or only by night. Donor
/// `data-global/scripts/world_changes/spawns_npc_by_time.lua:3-130` (zimbadev/crystalserver
/// @201189f8): a day NPC spawns at `LIGHT_STATE_SUNRISE` and leaves at `LIGHT_STATE_SUNSET`, a
/// night NPC the reverse (`onPeriodChange`, :134-174). Crystal's periods start at its world
/// minutes 360 and 1050 (`game.hpp:967-968`, `game.cpp:9061-9073`), the same instants as this
/// world clock's `SUNRISE_START` and `SUNSET_START`. Positions are the donor's `(x, y, z)`.
const TIMED_NPCS: [(&str, (u16, u16, i8), TimedPeriod); 21] = [
    (
        "oteryn:npc.ghostly_wolf",
        (33_332, 32_052, 7),
        TimedPeriod::Night,
    ),
    ("oteryn:npc.talila", (33_504, 32_222, 7), TimedPeriod::Night),
    (
        "oteryn:npc.valindara",
        (33_504, 32_222, 7),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.evrard_the_miller",
        (32_422, 32_464, 7),
        TimedPeriod::Day,
    ),
    ("oteryn:npc.evrard", (32_404, 32_493, 6), TimedPeriod::Night),
    (
        "oteryn:npc.innkeeper_alphonse",
        (32_417, 32_501, 7),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.wes_the_blacksmith_day",
        (32_423, 32_481, 5),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.wes_the_blacksmith_night",
        (32_374, 32_467, 6),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.onfroi_day",
        (32_428, 32_489, 5),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.onfroi_night",
        (32_429, 32_487, 2),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.dal_the_huntress_day",
        (32_436, 32_492, 5),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.dal_the_huntress_night",
        (32_436, 32_494, 4),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.jehan_the_baker_day",
        (32_421, 32_481, 4),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.jehan_the_baker_night",
        (32_391, 32_458, 6),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.fral_the_butcher_day",
        (32_445, 32_480, 4),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.fral_the_butcher_night",
        (32_399, 32_496, 6),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.fitzmaurice",
        (32_411, 32_486, 7),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.kesar_the_younger_day",
        (32_365, 32_478, 2),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.kesar_the_younger_night",
        (32_368, 32_473, 3),
        TimedPeriod::Night,
    ),
    (
        "oteryn:npc.kesar_s_valet_day",
        (32_367, 32_476, 2),
        TimedPeriod::Day,
    ),
    (
        "oteryn:npc.kesar_s_valet_night",
        (32_372, 32_473, 3),
        TimedPeriod::Night,
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimedPeriod {
    Day,
    Night,
}

/// Admitted day/night and source-qualified weekday NPCs kept on the map by the world clock.
///
/// Unlike the donor, which picks the startup set from the light level (`functions.lua:33-40`, so
/// a start during dusk spawns the day set) and spawns 5 s after the period change
/// (`spawns_npc_by_time.lua:159-170`), the set follows the world minute at boot and every tick.
#[derive(Debug, Clone, Copy)]
pub(super) struct TimedNpcSpawns {
    admitted: [bool; TIMED_NPCS.len()],
    weekday: Option<weekday::WeekdayNpcSpawns>,
    seasonal: seasonal::SeasonalNpcSpawns,
}

impl TimedNpcSpawns {
    pub(super) fn from_boot(boot: Option<&NpcBoot>) -> Self {
        let mut admitted = [false; TIMED_NPCS.len()];
        if let Some(NpcBoot::Place { admitted: keys, .. }) = boot {
            for (index, (key, _, _)) in TIMED_NPCS.iter().enumerate() {
                admitted[index] = keys.contains(*key);
            }
        }
        Self {
            admitted,
            weekday: weekday::WeekdayNpcSpawns::from_boot(boot),
            seasonal: seasonal::SeasonalNpcSpawns::default(),
        }
    }

    pub(super) fn with_seasonal(
        mut self,
        boot: Option<&NpcBoot>,
        events: &[String],
        enterable: impl Fn(crate::map::overlay::TilePos) -> bool,
    ) -> Self {
        self.seasonal = seasonal::SeasonalNpcSpawns::admit(boot, events, enterable);
        self
    }

    pub(super) fn set_brutus_present(&mut self, present: bool) {
        self.seasonal.set_brutus_present(present);
    }

    pub(super) fn is_empty(self) -> bool {
        !self.admitted.iter().any(|admitted| *admitted)
            && self.weekday.is_none()
            && self.seasonal.is_empty()
    }

    /// One world-clock step at `now_utc_ms` (boot and every world tick): the clock the
    /// transport's light uses (`snapshot.rs`), not a second one. `None` when the clock is
    /// unreadable; otherwise whether every admitted NPC is where its period puts it.
    pub(super) fn tick(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_utc_ms: u64,
    ) -> Option<Result<(), NpcPlacementError>> {
        let minute = crate::world_clock::WorldClockSource::new(
            crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS,
        )
        .tibian_minute(now_utc_ms)?;
        let unix_ms = i64::try_from(now_utc_ms).ok()?;
        let daily = self.reconcile(runtime, minute);
        let weekly = self
            .weekday
            .map_or(Ok(()), |weekday| weekday.tick(runtime, unix_ms));
        Some(daily.and(weekly).and(self.seasonal.tick(runtime)))
    }

    /// Removes the NPCs out of period, then places those in period that are missing. Each NPC
    /// is reconciled on its own: one that cannot move now (its cell occupied) keeps the rest
    /// going and is retried on the next tick; the first such error is returned.
    fn reconcile(
        self,
        runtime: &mut ChannelRuntimeV1,
        tibian_minute: u32,
    ) -> Result<(), NpcPlacementError> {
        let day = oteryn_gameplay_world::world_clock::in_world_time(
            tibian_minute,
            oteryn_gameplay_world::world_clock::SUNRISE_START,
            oteryn_gameplay_world::world_clock::SUNSET_START,
        );
        let wanted = |period: TimedPeriod| (period == TimedPeriod::Day) == day;
        let mut outcome = Ok(());
        for (index, (key, _, period)) in TIMED_NPCS.iter().enumerate() {
            if self.admitted[index] && !wanted(*period) {
                let removed = runtime.remove_dynamic_npc(key);
                outcome = outcome.and(removed);
            }
        }
        let present: BTreeSet<String> = runtime
            .visible_entities()
            .npcs
            .iter()
            .filter_map(|npc| runtime.npc_key(npc.actor).map(str::to_owned))
            .collect();
        for (index, (key, (x, y, z), period)) in TIMED_NPCS.iter().enumerate() {
            if self.admitted[index] && wanted(*period) && !present.contains(*key) {
                let placed = runtime.place_dynamic_npc(
                    key,
                    NpcPlacement {
                        floor: -*z,
                        y: *y,
                        x: *x,
                        direction: MovementFacing::South,
                    },
                );
                outcome = outcome.and(placed.map(|_| ()));
            }
        }
        outcome
    }
}

/// The NPC frame of a verified bundle and the catalogue digest its compiler classified with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BundleNpcs {
    table: NpcPlacementTable,
    catalogue_sha256: String,
}

/// The NPCs the loaded catalogue admits (in it and not held) and the catalogue's digest over
/// the same files the compiler hashed ([`npc::catalogue_sha256`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoadedNpcCatalogue {
    admitted: BTreeSet<String>,
    catalogue_sha256: String,
    /// What the admitted NPCs say (NPC-CMD-1a).
    talk: Arc<NpcTalkBook>,
}

/// What the Channel does with the bundle's NPCs at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NpcBoot {
    /// Place the table; an NPC outside `admitted` refuses (production) or is skipped.
    Place {
        table: NpcPlacementTable,
        /// The bundle catalogue digest already matched by `bind`, retained for native boot.
        catalogue_sha256: String,
        admitted: BTreeSet<String>,
        production: bool,
        talk: Arc<NpcTalkBook>,
    },
    /// Outside production only: no NPC actor, for the logged reason.
    Skip {
        reason: &'static str,
        npcs: usize,
        placements: usize,
    },
}

impl NpcBoot {
    /// Whether the loaded catalogue admits `key` for a placement at Channel start.
    pub(super) fn admits(&self, key: &str) -> bool {
        matches!(self, Self::Place { admitted, .. } if admitted.contains(key))
    }
}

fn facing(direction: npc::Direction) -> MovementFacing {
    match direction {
        npc::Direction::North => MovementFacing::North,
        npc::Direction::East => MovementFacing::East,
        npc::Direction::South => MovementFacing::South,
        npc::Direction::West => MovementFacing::West,
    }
}

fn refused(error: oteryn_world_bundle::Error) -> BootError {
    BootError::WorldBundle(crate::map::boot::BootRefusal::Load(
        crate::map::LoadError::from(error),
    ))
}

/// The NPC frame of the bundle bytes, verified like every other part of the bundle (a reader
/// refusal refuses the World). The sectors are verified and dropped one at a time.
pub(super) fn read(data: &[u8]) -> Result<BundleNpcs, BootError> {
    let visited = bundle::visit(data, |_, _| Ok(())).map_err(refused)?;
    let table = NpcPlacementTable {
        npcs: visited
            .npcs
            .npcs
            .into_iter()
            .map(|entry| NpcPlacements {
                key: entry.key,
                placements: entry
                    .placements
                    .into_iter()
                    .map(|placement| NpcPlacement {
                        floor: placement.floor,
                        y: placement.y,
                        x: placement.x,
                        direction: facing(placement.direction),
                    })
                    .collect(),
            })
            .collect(),
    };
    Ok(BundleNpcs {
        table,
        catalogue_sha256: visited.manifest.npcs.catalogue_sha256,
    })
}

impl BundleNpcs {
    fn placements(&self) -> usize {
        self.table
            .npcs
            .iter()
            .map(|entry| entry.placements.len())
            .sum()
    }
}

impl LoadedNpcCatalogue {
    /// The admitted keys of the catalogue loaded from `project_root` and the digest of the
    /// content tree that holds it (the project root's parent, as the compiler hashes it).
    pub(super) fn from_catalogue(
        catalogue: &crate::content::NpcDataCatalogue,
        project_root: &Path,
    ) -> Result<Self, BootError> {
        let model = catalogue
            .service_model_for_project()
            .map_err(|_| BootError::ContentActivation("NPC catalogue service model"))?;
        let content = project_root
            .parent()
            .ok_or(BootError::ContentActivation("NPC catalogue content root"))?;
        let catalogue_sha256 = npc::catalogue_sha256(content)
            .map_err(|_| BootError::ContentActivation("NPC catalogue digest"))?;
        let source = oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime()
            .map_err(|_| BootError::ContentActivation("native NPC quest catalogue"))?;
        let rewards = oteryn_gameplay_quests::quest::reward_callbacks::load()
            .map_err(|_| BootError::ContentActivation("native NPC reward catalogue"))?;
        let native_npcs = source
            .programs()
            .iter()
            .flat_map(|program| program.document.rules.iter().map(|rule| rule.npc.as_str()))
            .chain(
                rewards
                    .iter()
                    .filter(|reward| reward.carried_item.is_none())
                    .map(|reward| reward.npc.as_str()),
            )
            .chain(std::iter::once("oteryn:npc.viola"))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut talk = NpcTalkBook::from_model_with_native_quests(&model, &native_npcs);
        talk.bind_source_voices(catalogue, &model, |text| {
            crate::chat::ChatText::parse(text)
                .ok()
                .map(|text| text.as_str().to_owned())
        })
        .map_err(|_| BootError::ContentActivation("NPC source voices"))?;
        Ok(Self {
            admitted: model
                .npcs()
                .keys()
                .map(|reference| reference.key.clone())
                .collect(),
            catalogue_sha256,
            talk: Arc::new(talk),
        })
    }
}

/// §7: binds the bundle's NPC frame to the loaded catalogue. A production World refuses a
/// non-empty frame without a catalogue and any catalogue whose digest is not the bundle's.
pub(super) fn bind(
    frame: BundleNpcs,
    catalogue: Option<&LoadedNpcCatalogue>,
    production: bool,
) -> Result<NpcBoot, BootError> {
    let skip = |reason| {
        if production {
            return Err(BootError::ContentActivation("NPC catalogue"));
        }
        Ok(NpcBoot::Skip {
            reason,
            npcs: frame.table.npcs.len(),
            placements: frame.placements(),
        })
    };
    match catalogue {
        None if frame.table.npcs.is_empty() => Ok(NpcBoot::Skip {
            reason: "no_npcs",
            npcs: 0,
            placements: 0,
        }),
        None => skip("no_catalogue"),
        Some(loaded) if loaded.catalogue_sha256 != frame.catalogue_sha256 => {
            skip("catalogue_digest_mismatch")
        }
        Some(loaded) => Ok(NpcBoot::Place {
            table: frame.table,
            catalogue_sha256: frame.catalogue_sha256,
            admitted: loaded.admitted.clone(),
            production,
            talk: Arc::clone(&loaded.talk),
        }),
    }
}

/// The bundle's NPC boot for the node: `None` without a bundle World.
pub(super) fn prepare(
    frame: Option<BundleNpcs>,
    catalogue: Option<(&crate::content::NpcDataCatalogue, &Path)>,
    production: bool,
) -> Result<Option<NpcBoot>, BootError> {
    let Some(frame) = frame else {
        return Ok(None);
    };
    let loaded = catalogue
        .map(|(catalogue, root)| LoadedNpcCatalogue::from_catalogue(catalogue, root))
        .transpose()?;
    bind(frame, loaded.as_ref(), production).map(Some)
}

/// The verified bundle catalogue digest retained by a successful NPC boot binding.
///
/// This is the NPC-LOAD-1 digest over the compiler's catalogue files. It does not establish
/// membership in an active Content generation or substitute for the World Project tree digest.
/// Skipped or absent NPC boot state carries no verified catalogue binding.
pub(super) fn catalogue_sha256(boot: Option<&NpcBoot>) -> Option<&str> {
    match boot {
        Some(NpcBoot::Place {
            catalogue_sha256, ..
        }) => Some(catalogue_sha256.as_str()),
        _ => None,
    }
}

/// The conversation book of the placed NPCs, for the gameplay listener.
pub(super) fn talk_book(boot: Option<&NpcBoot>) -> Option<Arc<NpcTalkBook>> {
    match boot {
        Some(NpcBoot::Place { talk, .. }) => Some(Arc::clone(talk)),
        _ => None,
    }
}

/// Places the bound NPCs on the Channel once, before its creatures, and logs the outcome.
/// Returns the number of NPC actors.
pub(super) fn place(
    runtime: &mut ChannelRuntimeV1,
    boot: Option<&NpcBoot>,
) -> Result<usize, BootError> {
    if let Some(catalogue_sha256) = catalogue_sha256(boot) {
        event(&format!(
            "event=npc_catalogue state=bound catalogue_sha256={catalogue_sha256}"
        ));
    }
    match boot {
        None => Ok(0),
        Some(NpcBoot::Skip {
            reason,
            npcs,
            placements,
        }) => {
            event(&format!(
                "event=npc_actors state=skipped reason={reason} npcs={npcs} placements={placements} actors=0"
            ));
            Ok(0)
        }
        Some(NpcBoot::Place {
            table,
            admitted,
            production,
            ..
        }) => {
            let mut table = table.clone();
            let unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
                .unwrap_or(0);
            oteryn_gameplay_entities::rashid_schedule::rotate_placements(&mut table, unix_ms);
            let outcome = runtime
                .place_npcs(&table, |key| admitted.contains(key), *production)
                .map_err(|error| match error {
                    NpcPlacementError::NpcNotAdmitted(_) => {
                        BootError::ContentActivation("NPC not admitted by the NPC catalogue")
                    }
                    _ => BootError::Readiness("NPC actor placement"),
                })?;
            for key in &outcome.not_admitted {
                event(&format!(
                    "event=npc_actor_skipped reason=not_admitted npc_key={key}"
                ));
            }
            let placements: usize = table.npcs.iter().map(|entry| entry.placements.len()).sum();
            event(&format!(
                "event=npc_actors state=placed npcs={} placements={placements} actors={} not_admitted={}",
                table.npcs.len(),
                outcome.actors.len(),
                outcome.not_admitted.len(),
            ));
            Ok(outcome.actors.len())
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, NodeId, WorldId};

    const SMITH: &str = "oteryn:npc.smith";
    const TRADER: &str = "oteryn:npc.trader";

    fn digest(seed: &str) -> String {
        npc::catalogue_digest(&[("npcs/definitions/seed.json", seed.as_bytes())])
    }

    fn timed_keys(runtime: &ChannelRuntimeV1) -> Vec<&str> {
        let mut keys: Vec<_> = runtime
            .visible_entities()
            .npcs
            .iter()
            .filter_map(|npc| runtime.npc_key(npc.actor))
            .collect();
        keys.sort_unstable();
        keys
    }

    /// A one-floor synthetic bundle (native floor -7, legacy `z` 7) of walkable grass on row
    /// 0, x 1..=7, carrying `npcs` and compiled against the catalogue digest `catalogue`.
    fn bundle_with_npcs(npcs: &npc::Table, catalogue: &str) -> Vec<u8> {
        use oteryn_world_bundle_compiler::bundle::{
            self, BuildClass, Extent, Family, Identity, Manifest, NpcCounts, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
        use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
        let manifest = Manifest {
            subareas: Default::default(),
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
            projection_class: "server".into(),
            compiler_version: "engineering".into(),
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
            palette: vec![PaletteEntry {
                key: "terrain:grass".into(),
                family: Family::Terrain,
                id: 0,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                    blocks_projectile: None,
                }),
            }],
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
            npcs: NpcCounts {
                npcs: u32::try_from(npcs.npcs.len()).expect("npcs"),
                placements: u32::try_from(npcs.placement_count()).expect("placements"),
                catalogue_sha256: catalogue.to_owned(),
                ..NpcCounts::default()
            },
        };
        let sector = Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: (1..=7)
                .map(|x| Tile {
                    x,
                    y: 0,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                })
                .collect(),
        };
        bundle::write_with_npcs(&manifest, &[sector], &Default::default(), npcs)
            .expect("written bundle")
    }

    fn two_npcs() -> npc::Table {
        let at = |x, direction| npc::Placement {
            floor: -7,
            x,
            y: 0,
            direction,
        };
        npc::Table {
            npcs: vec![
                npc::Npc {
                    key: SMITH.into(),
                    placements: vec![at(2, npc::Direction::South)],
                },
                npc::Npc {
                    key: TRADER.into(),
                    placements: vec![at(5, npc::Direction::West)],
                },
            ],
        }
    }

    fn loaded(admitted: &[&str], catalogue: &str) -> LoadedNpcCatalogue {
        LoadedNpcCatalogue {
            admitted: admitted.iter().map(|key| (*key).to_owned()).collect(),
            catalogue_sha256: catalogue.to_owned(),
            talk: Arc::default(),
        }
    }

    fn channel_runtime() -> ChannelRuntimeV1 {
        channel_runtime_with(8)
    }

    fn channel_runtime_with(capacity: usize) -> ChannelRuntimeV1 {
        let world = WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world");
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .expect("runtime")
    }

    /// NPC-LOAD-1: a synthetic bundle with two NPCs boots two NPC actors bound to the Channel's
    /// own scope, in frame order, on the bundle's cells, and the join snapshot sees both.
    #[test]
    fn a_bundle_with_two_npcs_boots_two_npc_actors_bound_to_the_channel_scope() -> Result<(), String>
    {
        let catalogue = digest("v1");
        let frame = read(&bundle_with_npcs(&two_npcs(), &catalogue)).expect("frame");
        let boot = bind(frame, Some(&loaded(&[SMITH, TRADER], &catalogue)), true).expect("bind");
        assert_eq!(catalogue_sha256(Some(&boot)), Some(catalogue.as_str()));
        let mut runtime = channel_runtime();
        assert_eq!(place(&mut runtime, Some(&boot)).expect("placed"), 2);
        assert_eq!(catalogue_sha256(Some(&boot)), Some(catalogue.as_str()));
        let binding = runtime.binding();
        // W02: the scoped binding now returns Option; this fixture still requires Channel.
        let channel = binding.channel_id().ok_or("pinned Channel NPC fixture")?;
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), 2);
        let keys: Vec<_> = visible
            .npcs
            .iter()
            .map(|entry| {
                assert_eq!(entry.actor.world_id(), binding.world_id());
                assert_eq!(entry.actor.channel_id(), Some(channel));
                assert_eq!(entry.actor.scope_generation(), binding.scope_generation());
                (
                    runtime.npc_key(entry.actor).expect("npc key").to_owned(),
                    (entry.position.x, entry.position.y, entry.position.floor),
                )
            })
            .collect();
        assert_eq!(
            keys,
            [
                (SMITH.to_owned(), (2, 0, 7)),
                (TRADER.to_owned(), (5, 0, 7))
            ]
        );
        assert!(visible.creatures.is_empty());
        // Placement runs once, at Channel start.
        assert!(matches!(
            place(&mut runtime, Some(&boot)),
            Err(BootError::Readiness("NPC actor placement"))
        ));
        Ok(())
    }

    fn timed_boot(keys: &[&str]) -> NpcBoot {
        NpcBoot::Place {
            table: NpcPlacementTable::default(),
            catalogue_sha256: digest("v1"),
            admitted: keys.iter().map(|key| (*key).to_owned()).collect(),
            production: true,
            talk: Arc::new(NpcTalkBook::default()),
        }
    }

    #[test]
    fn w05d_santa_placements_require_active_calendar_catalogue_and_real_tiles() {
        let santa = "oteryn:npc.santa_claus";
        let boot = timed_boot(&[santa]);
        let mut runtime = channel_runtime_with(32);
        assert!(seasonal::SeasonalNpcSpawns::admit(Some(&boot), &[], |_| true).is_empty());
        assert!(
            seasonal::SeasonalNpcSpawns::admit(None, &["christmas".into()], |_| true).is_empty()
        );
        assert!(
            seasonal::SeasonalNpcSpawns::admit(Some(&boot), &["christmas".into()], |_| false)
                .is_empty()
        );
        let mut seasonal =
            seasonal::SeasonalNpcSpawns::admit(Some(&boot), &["christmas".into()], |_| true);
        seasonal.tick(&mut runtime).expect("source placements");
        seasonal.tick(&mut runtime).expect("idempotent retry");
        assert_eq!(runtime.visible_entities().npcs.len(), 15);
        let evidence: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/reference/content-prep/season-20261107/seasonal-calendar-source.json"
        )))
        .expect("retained structured placement evidence");
        let santa_source = evidence["records"]
            .as_array()
            .expect("source records")
            .iter()
            .find(|record| record["title"] == "Santa Claus")
            .expect("pinned Santa source");
        let expected: BTreeSet<_> = santa_source["placements"]
            .as_array()
            .expect("source placements")
            .iter()
            .map(|row| {
                (
                    i32::try_from(row["x"].as_i64().expect("source x")).expect("coordinate"),
                    i32::try_from(row["y"].as_i64().expect("source y")).expect("coordinate"),
                    i16::try_from(row["z"].as_i64().expect("source floor")).expect("floor"),
                )
            })
            .collect();
        let actual: BTreeSet<_> = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .map(|npc| (npc.position.x, npc.position.y, npc.position.floor))
            .collect();
        assert_eq!(
            actual, expected,
            "all native actors must use retained source coordinates"
        );

        // The normal NPC movement owner may move Santa off the source cell;
        // reconciliation must retain the exact live actor rather than clone it.
        let santa = runtime.visible_entities().npcs[0].actor;
        let before = runtime
            .borrow_movement_position()
            .read(santa)
            .expect("placed actor");
        let mut moved = before.position();
        moved.x += 1;
        runtime
            .borrow_movement_position()
            .commit_cardinal(before, moved)
            .expect("Santa wanders");
        seasonal
            .tick(&mut runtime)
            .expect("retains wandering actor");
        assert_eq!(runtime.visible_entities().npcs.len(), 15);
        assert_eq!(
            runtime
                .borrow_movement_position()
                .read(santa)
                .expect("same actor")
                .position(),
            moved
        );
    }

    #[test]
    fn w05d_cake_bakers_use_all_five_source_positions_and_retry_without_duplicates() {
        let keys = [
            "oteryn:npc.benny_the_baker",
            "oteryn:npc.biff_the_baker",
            "oteryn:npc.brian_the_baker",
            "oteryn:npc.balduin_the_baker",
            "oteryn:npc.billy_the_baker",
        ];
        let boot = timed_boot(&keys);
        let mut runtime = channel_runtime_with(16);
        let mut seasonal =
            seasonal::SeasonalNpcSpawns::admit(Some(&boot), &["piece_of_cake".into()], |_| true);
        seasonal.tick(&mut runtime).expect("five source bakers");
        seasonal
            .tick(&mut runtime)
            .expect("idempotent reconciliation");
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), 5);
        for key in keys {
            assert_eq!(
                visible
                    .npcs
                    .iter()
                    .filter(|npc| runtime.npc_key(npc.actor) == Some(key))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn w05d_brutus_follows_saved_projection_independently_of_calendar_and_actor_wandering() {
        let key = "oteryn:npc.brutus_the_baker";
        let boot = timed_boot(&[key]);
        let mut runtime = channel_runtime_with(8);
        let mut seasonal = seasonal::SeasonalNpcSpawns::admit(Some(&boot), &[], |_| true);
        assert!(
            !seasonal.is_empty(),
            "inactive eligible Brutus must keep projection refresh alive"
        );
        seasonal.tick(&mut runtime).expect("initially absent");
        assert!(runtime.visible_entities().npcs.is_empty());
        seasonal.set_brutus_present(true);
        seasonal
            .tick(&mut runtime)
            .expect("committed next-save flag");
        let visible = runtime.visible_entities();
        let npc = &visible.npcs[0];
        assert_eq!(
            (npc.position.x, npc.position.y, npc.position.floor),
            (33303, 32064, 7)
        );
        let before = runtime
            .borrow_movement_position()
            .read(npc.actor)
            .expect("current Brutus");
        let mut moved = before.position();
        moved.x += 1;
        runtime
            .borrow_movement_position()
            .commit_cardinal(before, moved)
            .expect("native wandering");
        seasonal.tick(&mut runtime).expect("retains exact actor");
        assert_eq!(runtime.visible_entities().npcs.len(), 1);
        seasonal.set_brutus_present(false);
        seasonal.tick(&mut runtime).expect("source cleanup");
        assert!(runtime.visible_entities().npcs.is_empty());
        seasonal.tick(&mut runtime).expect("idempotent cleanup");
        // Reallocation may reuse its carrier slot: a historical actor reference
        // must never remove the successor when an old cleanup retries.
        seasonal.set_brutus_present(true);
        seasonal.tick(&mut runtime).expect("new occurrence");
        let successor = runtime.visible_entities().npcs[0].actor;
        assert_ne!(npc.actor, successor);
        runtime
            .remove_dynamic_npc_actor(npc.actor)
            .expect("stale no-op");
        assert_eq!(runtime.npc_key(successor), Some(key));
    }

    #[test]
    fn w05d_brutus_force_placement_retains_existing_actor_on_exact_source_tile() {
        let boot = timed_boot(&["oteryn:npc.brutus_the_baker"]);
        let mut runtime = channel_runtime_with(8);
        let blocker = runtime
            .place_dynamic_npc(
                "oteryn:npc.obstruction",
                NpcPlacement {
                    x: 33303,
                    y: 32064,
                    floor: -7,
                    direction: MovementFacing::South,
                },
            )
            .expect("existing actor");
        let mut seasonal = seasonal::SeasonalNpcSpawns::admit(Some(&boot), &[], |_| true);
        seasonal.set_brutus_present(true);
        seasonal
            .tick(&mut runtime)
            .expect("source force flag permits overlap");
        assert_eq!(runtime.visible_entities().npcs.len(), 2);
        seasonal.set_brutus_present(false);
        seasonal
            .tick(&mut runtime)
            .expect("remove exact Brutus only");
        assert_eq!(runtime.visible_entities().npcs.len(), 1);
        assert_eq!(runtime.npc_key(blocker), Some("oteryn:npc.obstruction"));
    }

    /// The UTC instant of world minute `minute` on the production world clock.
    fn at_minute(minute: u64) -> u64 {
        let now = minute * crate::world_clock::TIBIAN_MINUTE_MS;
        let clock = crate::world_clock::WorldClockSource::new(
            crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS,
        );
        assert_eq!(clock.tibian_minute(now), u32::try_from(minute % 1_440).ok());
        now
    }

    fn period_keys(period: TimedPeriod) -> Vec<&'static str> {
        let mut keys: Vec<_> = TIMED_NPCS
            .iter()
            .filter(|(_, _, of)| *of == period)
            .map(|(key, _, _)| *key)
            .collect();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn world_time_tick_swaps_day_and_night_npcs_at_sunrise_and_sunset() {
        let keys: Vec<&str> = TIMED_NPCS.iter().map(|(key, _, _)| *key).collect();
        let mut timed = TimedNpcSpawns::from_boot(Some(&timed_boot(&keys)));
        let mut runtime = channel_runtime_with(32);
        let (day, night) = (
            period_keys(TimedPeriod::Day),
            period_keys(TimedPeriod::Night),
        );
        assert_eq!((day.len(), night.len()), (9, 12));
        for (minute, expected) in [
            (359, &night),
            (360, &day),
            (1_049, &day),
            (1_050, &night),
            (1_439, &night),
            (1_440 + 360, &day),
        ] {
            timed
                .tick(&mut runtime, at_minute(minute))
                .expect("clock")
                .expect("reconciled");
            assert_eq!(&timed_keys(&runtime), expected, "minute {minute}");
        }
        // Talila (night) and Valindara (day) share one cell: the swap frees it first.
        let valindara = runtime
            .visible_entities()
            .npcs
            .iter()
            .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.valindara"))
            .map(|npc| (npc.position.x, npc.position.y, npc.position.floor));
        assert_eq!(valindara, Some((33_504, 32_222, 7)));
        // The NPCs the catalogue does not admit are never placed.
        let mut timed = TimedNpcSpawns::from_boot(Some(&timed_boot(&["oteryn:npc.talila"])));
        let mut runtime = channel_runtime_with(32);
        timed
            .tick(&mut runtime, at_minute(359))
            .expect("clock")
            .expect("night");
        assert_eq!(timed_keys(&runtime), ["oteryn:npc.talila"]);
        assert!(TimedNpcSpawns::from_boot(None).is_empty());
    }

    #[test]
    fn a_timed_npc_that_cannot_stand_yet_does_not_hold_back_the_others() {
        let mut timed = TimedNpcSpawns::from_boot(Some(&timed_boot(&[
            "oteryn:npc.valindara",
            "oteryn:npc.evrard_the_miller",
        ])));
        let mut runtime = channel_runtime_with(32);
        // Someone stands on Valindara's cell at sunrise.
        runtime
            .place_dynamic_npc(
                "oteryn:npc.blocker",
                NpcPlacement {
                    floor: -7,
                    y: 32_222,
                    x: 33_504,
                    direction: MovementFacing::South,
                },
            )
            .expect("blocker");
        assert_eq!(
            timed.tick(&mut runtime, at_minute(360)).expect("clock"),
            Err(NpcPlacementError::CellOccupied)
        );
        assert_eq!(
            timed_keys(&runtime),
            ["oteryn:npc.blocker", "oteryn:npc.evrard_the_miller"]
        );
        // The next tick places her once the cell is free.
        runtime
            .remove_dynamic_npc("oteryn:npc.blocker")
            .expect("free");
        timed
            .tick(&mut runtime, at_minute(361))
            .expect("clock")
            .expect("placed");
        assert_eq!(
            timed_keys(&runtime),
            ["oteryn:npc.evrard_the_miller", "oteryn:npc.valindara"]
        );
    }

    /// §7: an NPC the loaded catalogue does not have refuses a production World and leaves the
    /// Channel without any NPC actor; outside production only that NPC is skipped.
    #[test]
    fn an_unknown_npc_refuses_a_production_world_and_is_skipped_outside_it() {
        let catalogue = digest("v1");
        let bytes = bundle_with_npcs(&two_npcs(), &catalogue);
        let only_smith = loaded(&[SMITH], &catalogue);

        let boot = bind(read(&bytes).expect("frame"), Some(&only_smith), true).expect("bind");
        let mut runtime = channel_runtime();
        assert!(matches!(
            place(&mut runtime, Some(&boot)),
            Err(BootError::ContentActivation(
                "NPC not admitted by the NPC catalogue"
            ))
        ));
        assert!(runtime.visible_entities().npcs.is_empty());
        // The read-only digest proves the catalogue binding, not admission or placement.
        assert_eq!(catalogue_sha256(Some(&boot)), Some(catalogue.as_str()));

        let boot = bind(read(&bytes).expect("frame"), Some(&only_smith), false).expect("bind");
        let mut runtime = channel_runtime();
        assert_eq!(place(&mut runtime, Some(&boot)).expect("placed"), 1);
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), 1);
        assert_eq!(runtime.npc_key(visible.npcs[0].actor), Some(SMITH));
    }

    /// §7: a catalogue the bundle was not compiled against, or none, refuses a production World
    /// and places no NPC outside it.
    #[test]
    fn a_missing_or_mismatched_catalogue_refuses_production_and_places_nothing_outside_it() {
        let bytes = bundle_with_npcs(&two_npcs(), &digest("v1"));
        let other = loaded(&[SMITH, TRADER], &digest("v2"));
        assert_eq!(catalogue_sha256(None), None);
        for catalogue in [None, Some(&other)] {
            assert!(matches!(
                bind(read(&bytes).expect("frame"), catalogue, true),
                Err(BootError::ContentActivation("NPC catalogue"))
            ));
            let boot = bind(read(&bytes).expect("frame"), catalogue, false).expect("bind");
            assert_eq!(catalogue_sha256(Some(&boot)), None);
            assert!(matches!(
                boot,
                NpcBoot::Skip {
                    npcs: 2,
                    placements: 2,
                    ..
                }
            ));
            let mut runtime = channel_runtime();
            assert_eq!(place(&mut runtime, Some(&boot)).expect("skipped"), 0);
            assert!(runtime.visible_entities().npcs.is_empty());
        }
        // A bundle without NPCs needs no catalogue, in production too.
        let empty = bundle_with_npcs(&npc::Table::default(), &digest("v1"));
        let boot = bind(read(&empty).expect("frame"), None, true).expect("empty boot");
        assert_eq!(catalogue_sha256(Some(&boot)), None);
        assert!(matches!(
            boot,
            NpcBoot::Skip {
                reason: "no_npcs",
                ..
            }
        ));
    }

    /// A bundle the reader refuses is refused as a World bundle, not read as having no NPCs.
    #[test]
    fn a_damaged_bundle_is_refused() {
        let mut bytes = bundle_with_npcs(&two_npcs(), &digest("v1"));
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        assert!(matches!(read(&bytes), Err(BootError::WorldBundle(_))));
    }
    #[test]
    fn questpc8_weekday_npc_ticks_preserve_actor_then_rotate_and_retry() {
        use oteryn_gameplay_entities::rashid_schedule::{RASHID_NPC_KEY, destination_at};
        const MONDAY: i64 = 4 * 86_400_000;
        let placements = (0..7)
            .map(|day| destination_at(MONDAY + day * 86_400_000).placement)
            .collect();
        let mut boot = timed_boot(&[RASHID_NPC_KEY]);
        if let NpcBoot::Place { table, .. } = &mut boot {
            table.npcs.push(NpcPlacements {
                key: RASHID_NPC_KEY.into(),
                placements,
            });
        }
        let mut scheduled = TimedNpcSpawns::from_boot(Some(&boot));
        assert!(
            !scheduled.is_empty(),
            "the compiled weekday NPC must be reconciled by the live clock"
        );
        let mut runtime = channel_runtime_with(8);
        let mut previous = None;
        for day in 0..8 {
            let now = MONDAY + day * 86_400_000;
            scheduled
                .tick(&mut runtime, u64::try_from(now).expect("timestamp"))
                .expect("clock")
                .expect("weekday NPC");
            let actors = runtime.visible_entities().npcs;
            assert_eq!(actors.len(), 1);
            let actor = actors[0].actor;
            let expected = destination_at(now).placement;
            assert_eq!(
                (
                    actors[0].position.x,
                    actors[0].position.y,
                    actors[0].position.floor
                ),
                (
                    i32::from(expected.x),
                    i32::from(expected.y),
                    -i16::from(expected.floor)
                )
            );
            if let Some(old) = previous {
                assert!(
                    runtime.borrow_movement_position().read(old).is_err(),
                    "the prior day's identity cannot survive rotation"
                );
            }
            scheduled
                .tick(&mut runtime, u64::try_from(now + 1000).expect("timestamp"))
                .expect("clock")
                .expect("same day");
            assert_eq!(
                runtime.visible_entities().npcs[0].actor,
                actor,
                "a repeated tick cannot duplicate or replace the actor"
            );
            previous = Some(actor);
        }
        let now = MONDAY + 8 * 86_400_000;
        let target = destination_at(now).placement;
        runtime
            .place_dynamic_npc("oteryn:npc.blocker", target)
            .expect("blocker");
        assert_eq!(
            scheduled
                .tick(&mut runtime, u64::try_from(now).expect("timestamp"))
                .expect("clock"),
            Err(NpcPlacementError::CellOccupied)
        );
        assert_eq!(timed_keys(&runtime), ["oteryn:npc.blocker"]);
        runtime
            .remove_dynamic_npc("oteryn:npc.blocker")
            .expect("free cell");
        scheduled
            .tick(&mut runtime, u64::try_from(now + 1000).expect("timestamp"))
            .expect("clock")
            .expect("retry");
        assert_eq!(timed_keys(&runtime), [RASHID_NPC_KEY]);
        // A catalogue key alone cannot manufacture a placement not admitted by the bundle.
        assert!(TimedNpcSpawns::from_boot(Some(&timed_boot(&[RASHID_NPC_KEY]))).is_empty());
    }
    #[test]
    fn questpc8_weekday_npc_wandering_keeps_identity_until_destination_changes() {
        use oteryn_gameplay_entities::rashid_schedule::{RASHID_NPC_KEY, destination_at};
        const MONDAY: i64 = 4 * 86_400_000;
        let mut boot = timed_boot(&[RASHID_NPC_KEY]);
        if let NpcBoot::Place { table, .. } = &mut boot {
            table.npcs.push(NpcPlacements {
                key: RASHID_NPC_KEY.into(),
                placements: (0..7)
                    .map(|day| destination_at(MONDAY + day * 86_400_000).placement)
                    .collect(),
            });
        }
        let mut scheduled = TimedNpcSpawns::from_boot(Some(&boot));
        let mut runtime = channel_runtime_with(8);
        scheduled
            .tick(&mut runtime, MONDAY as u64)
            .expect("clock")
            .expect("Monday");
        let actor = runtime.visible_entities().npcs[0].actor;
        let before = runtime
            .borrow_movement_position()
            .read(actor)
            .expect("Rashid");
        let mut wandered = before.position();
        wandered.x += 1;
        runtime
            .borrow_movement_position()
            .commit_cardinal(before, wandered)
            .expect("native wander");
        scheduled
            .tick(&mut runtime, (MONDAY + 2500) as u64)
            .expect("clock")
            .expect("same day");
        assert_eq!(runtime.visible_entities().npcs.len(), 1);
        assert_eq!(
            runtime.visible_entities().npcs[0].actor,
            actor,
            "wandering must preserve conversation and shop references"
        );
        assert_eq!(
            runtime
                .borrow_movement_position()
                .read(actor)
                .expect("same actor")
                .position(),
            wandered
        );
        scheduled
            .tick(&mut runtime, (MONDAY + 86_400_000) as u64)
            .expect("clock")
            .expect("Tuesday");
        assert_eq!(runtime.visible_entities().npcs.len(), 1);
        assert_ne!(runtime.visible_entities().npcs[0].actor, actor);
        assert!(runtime.borrow_movement_position().read(actor).is_err());
        let expected = destination_at(MONDAY + 86_400_000).placement;
        let position = runtime.visible_entities().npcs[0].position;
        assert_eq!(
            (position.x, position.y, position.floor),
            (
                i32::from(expected.x),
                i32::from(expected.y),
                -i16::from(expected.floor)
            )
        );
    }

    #[test]
    fn questpc8_weekday_npc_missing_bundle_cell_or_admission_cannot_spawn() {
        use oteryn_gameplay_entities::rashid_schedule::{RASHID_NPC_KEY, destination_at};
        const MONDAY: i64 = 4 * 86_400_000;
        let mut boot = timed_boot(&[RASHID_NPC_KEY]);
        if let NpcBoot::Place { table, .. } = &mut boot {
            table.npcs.push(NpcPlacements {
                key: RASHID_NPC_KEY.into(),
                placements: (1..7)
                    .map(|day| destination_at(MONDAY + day * 86_400_000).placement)
                    .collect(),
            });
        }
        let mut scheduled = TimedNpcSpawns::from_boot(Some(&boot));
        let mut runtime = channel_runtime_with(8);
        scheduled
            .tick(&mut runtime, u64::try_from(MONDAY).expect("timestamp"))
            .expect("clock")
            .expect("missing cell");
        assert!(runtime.visible_entities().npcs.is_empty());
        scheduled
            .tick(
                &mut runtime,
                u64::try_from(MONDAY + 86_400_000).expect("timestamp"),
            )
            .expect("clock")
            .expect("admitted cell");
        assert_eq!(timed_keys(&runtime), [RASHID_NPC_KEY]);
        scheduled
            .tick(
                &mut runtime,
                u64::try_from(MONDAY + 7 * 86_400_000).expect("timestamp"),
            )
            .expect("clock")
            .expect("missing cell next week");
        assert!(runtime.visible_entities().npcs.is_empty());
        if let NpcBoot::Place { admitted, .. } = &mut boot {
            admitted.clear();
        }
        assert!(
            TimedNpcSpawns::from_boot(Some(&boot)).is_empty(),
            "bundle coordinates cannot replace catalogue admission"
        );
    }
}
