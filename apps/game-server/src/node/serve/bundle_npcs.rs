//! NPC-LOAD-1: the World Bundle's NPC placements become the Channel's NPC actors at boot
//! (NPC-PLACE-1 §7, NPC-BEHAVIOUR-0 §3.1).
//!
//! The format v4 NPC frame is read from the same verified bundle bytes the boot checks, bound
//! to the NPC catalogue the node loaded (`--npc-data-project`), and placed once, at Channel
//! start, through [`ChannelRuntimeV1::place_npcs`], the same carrier seam the NPC actors have
//! always used: one actor per placement in frame order, never removed, never respawned.
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
use oteryn_world_bundle::{bundle, npc};
use std::collections::BTreeSet;
use std::path::Path;

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
}

/// What the Channel does with the bundle's NPCs at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NpcBoot {
    /// Place the table; an NPC outside `admitted` refuses (production) or is skipped.
    Place {
        table: NpcPlacementTable,
        admitted: BTreeSet<String>,
        production: bool,
    },
    /// Outside production only: no NPC actor, for the logged reason.
    Skip {
        reason: &'static str,
        npcs: usize,
        placements: usize,
    },
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
        Ok(Self {
            admitted: model
                .npcs()
                .keys()
                .map(|reference| reference.key.clone())
                .collect(),
            catalogue_sha256,
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
            admitted: loaded.admitted.clone(),
            production,
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

/// Places the bound NPCs on the Channel once, before its creatures, and logs the outcome.
/// Returns the number of NPC actors.
pub(super) fn place(
    runtime: &mut ChannelRuntimeV1,
    boot: Option<&NpcBoot>,
) -> Result<usize, BootError> {
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
        }) => {
            let outcome = runtime
                .place_npcs(table, |key| admitted.contains(key), *production)
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

    /// A one-floor synthetic bundle (native floor -7, legacy `z` 7) of walkable grass on row
    /// 0, x 1..=7, carrying `npcs` and compiled against the catalogue digest `catalogue`.
    fn bundle_with_npcs(npcs: &npc::Table, catalogue: &str) -> Vec<u8> {
        use oteryn_world_bundle_compiler::bundle::{
            self, BuildClass, Extent, Family, Identity, Manifest, NpcCounts, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
        use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
        let manifest = Manifest {
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
        }
    }

    fn channel_runtime() -> ChannelRuntimeV1 {
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
            8,
            crate::foundation::ChannelContentPin::test(world),
        )
        .expect("runtime")
    }

    /// NPC-LOAD-1: a synthetic bundle with two NPCs boots two NPC actors bound to the Channel's
    /// own scope, in frame order, on the bundle's cells, and the join snapshot sees both.
    #[test]
    fn a_bundle_with_two_npcs_boots_two_npc_actors_bound_to_the_channel_scope() {
        let catalogue = digest("v1");
        let frame = read(&bundle_with_npcs(&two_npcs(), &catalogue)).expect("frame");
        let boot = bind(frame, Some(&loaded(&[SMITH, TRADER], &catalogue)), true).expect("bind");
        let mut runtime = channel_runtime();
        assert_eq!(place(&mut runtime, Some(&boot)).expect("placed"), 2);
        let binding = runtime.binding();
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), 2);
        let keys: Vec<_> = visible
            .npcs
            .iter()
            .map(|entry| {
                assert_eq!(entry.actor.world_id(), binding.world_id());
                assert_eq!(entry.actor.channel_id(), binding.channel_id());
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
        for catalogue in [None, Some(&other)] {
            assert!(matches!(
                bind(read(&bytes).expect("frame"), catalogue, true),
                Err(BootError::ContentActivation("NPC catalogue"))
            ));
            let boot = bind(read(&bytes).expect("frame"), catalogue, false).expect("bind");
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
        assert!(matches!(
            bind(read(&empty).expect("frame"), None, true),
            Ok(NpcBoot::Skip {
                reason: "no_npcs",
                ..
            })
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
}
