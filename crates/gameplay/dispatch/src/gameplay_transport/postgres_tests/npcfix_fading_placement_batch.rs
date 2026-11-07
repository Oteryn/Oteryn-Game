//! Captured spawn anchors must be admitted on real cells before native actors exist.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs, realize_spawns};
use oteryn_world_bundle_compiler::source::{project_regions, registry, world_extent};
use std::path::Path;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    x: u16,
    y: u16,
    floor: i8,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    npc: String,
    name: String,
    expected: Point,
    before: Option<Point>,
    #[serde(default)]
    additional: Vec<Point>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    id: String,
    before_commit: String,
    source: String,
    cases: Vec<Case>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    schema: u32,
    batches: Vec<Batch>,
}

#[test]
fn npcfix_fading_and_retained_placements_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC anchor thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "npcfix_anchor", true, false).await?;
        let result: TestResult = async {
            eprintln!("POSTGRES_STARTED NPCFIX retained source placement batches");
            let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
            let data: Cases = serde_json::from_slice(&std::fs::read(root.join(
                "tools/content-schema/npc-authoring/npcfix-qualified-placement-cases.json",
            ))?)?;
            assert_eq!(data.schema, 1);
            assert!(!data.batches.is_empty());
            assert!(
                data.batches
                    .iter()
                    .all(|batch| (3..=6).contains(&batch.cases.len())
                        && !batch.id.is_empty()
                        && !batch.source.is_empty()
                        && batch.before_commit.len() == 40)
            );
            let all = data
                .batches
                .iter()
                .flat_map(|batch| &batch.cases)
                .collect::<Vec<_>>();
            let unique = all
                .iter()
                .map(|case| &case.npc)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                unique.len(),
                all.len(),
                "retain every qualified case without duplicates"
            );
            assert!(
                all.len() <= 512
                    && all
                        .iter()
                        .all(|case| (-15..=0).contains(&case.expected.floor))
            );
            let anchors = all
                .iter()
                .flat_map(|case| {
                    std::iter::once(&case.expected)
                        .chain(&case.additional)
                        .map(move |point| {
                            (
                                case.npc.as_str(),
                                case.name.as_str(),
                                point.x,
                                point.y,
                                point.floor,
                            )
                        })
                })
                .collect::<Vec<_>>();
            let baseline = match std::env::var("NPCFIX_PG_BASELINE") {
                Ok(value) if value == "1" => true,
                Err(std::env::VarError::NotPresent) => false,
                _ => return Err("invalid baseline mode".into()),
            };
            let mut region_set = std::collections::BTreeSet::new();
            if baseline {
                for case in &data.batches.last().ok_or("batch")?.cases {
                    if let Some(before) = &case.before {
                        region_set.insert((before.x / 256, before.y / 256));
                    }
                }
            }
            region_set.extend(anchors.iter().map(|(_, _, x, y, _)| (x / 256, y / 256)));
            let regions = region_set.into_iter().collect::<Vec<_>>();
            let mut project = project_regions(root, &regions).map_err(debug)?;
            if baseline {
                let last = data.batches.last().ok_or("batch")?;
                for case in &last.cases {
                    project
                        .families
                        .npc_placements
                        .retain(|point| point.npc != case.npc);
                    if let Some(before) = &case.before {
                        assert!((-15..=0).contains(&before.floor));
                        project.families.npc_placements.push(
                            oteryn_world_bundle_compiler::project::NpcPlacement {
                                key: format!(
                                    "oteryn:npc_placement.{}.x{}_y{}_z{}",
                                    case.npc.trim_start_matches("oteryn:npc."),
                                    before.x,
                                    before.y,
                                    -before.floor
                                ),
                                npc: case.npc.clone(),
                                cell: (before.x, before.y, u8::try_from(-before.floor)?),
                                direction: oteryn_world_bundle_compiler::spawn::Direction::North,
                            },
                        );
                    }
                }
            }
            let (resolver, palette) = registry(root, &project).map_err(debug)?;
            let input = Input {
                regions: &project.regions,
                palette: &palette,
                identity: Identity::default(),
                world: world_extent(root).map_err(debug)?,
                build_class: BuildClass::NonProduction,
                draft_areas: Vec::new(),
                families: &project.families,
            };
            let (spawns, _) = realize_spawns(&input, &resolver).map_err(debug)?;
            let (compiled, report) = realize_npcs(&input, &resolver, &spawns).map_err(debug)?;
            let mut placements = Vec::new();
            for &(key, _, x, y, floor) in &anchors {
                let row = compiled.npcs.iter().find(|row| row.key == key);
                assert!(
                    row.is_some(),
                    "{key} not admitted: {:?}",
                    report
                        .held
                        .iter()
                        .filter(|held| held.npc == key)
                        .collect::<Vec<_>>()
                );
                let row = row.ok_or("admitted NPC")?;
                assert_eq!(
                    row.placements.len(),
                    anchors
                        .iter()
                        .filter(|(wanted, _, _, _, _)| *wanted == key)
                        .count(),
                    "exact explicitly reviewed source anchors"
                );
                let point = row
                    .placements
                    .iter()
                    .find(|point| (point.x, point.y, point.floor) == (x, y, floor))
                    .ok_or("reviewed source anchor absent")?;
                assert_eq!(
                    (point.x, point.y, point.floor),
                    (x, y, floor),
                    "reviewed admitted roaming cell"
                );
                assert_eq!(
                    point.direction,
                    oteryn_world_bundle_compiler::spawn::Direction::North
                );
                if let Some(existing) = placements
                    .iter_mut()
                    .find(|entry: &&mut NpcPlacements| entry.key == key)
                {
                    existing.placements.push(NpcPlacement {
                        x,
                        y,
                        floor,
                        direction: MovementFacing::North,
                    });
                    continue;
                }
                placements.push(NpcPlacements {
                    key: key.into(),
                    placements: vec![NpcPlacement {
                        x,
                        y,
                        floor,
                        direction: MovementFacing::North,
                    }],
                });
            }
            // Batch history is append-only; the runtime table requires canonical key order.
            placements.sort_by(|left, right| left.key.cmp(&right.key));
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let deps = Dependencies::with_content_and_capacity(
                ChannelContentPin::test(world),
                crate::content::qualify_native_entry_room(world).map_err(debug)?,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                anchors.len() + 1,
            )?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let catalogue = NpcDataCatalogue::load(
                &root.join("content"),
                std::ffi::OsStr::new("world"),
                npc_catalogue_preproduction_limits(),
            )
            .map_err(debug)?;
            let model = catalogue.service_model_for_project().map_err(debug)?;
            let book = NpcTalkBook::from_model(&model);
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable { npcs: placements },
                    |key| book.display_name(key).is_some(),
                    true,
                )
                .map_err(debug)?;
            let visible = runtime.visible_entities();
            assert_eq!(visible.npcs.len(), anchors.len());
            for actor in visible.npcs {
                let key = runtime.npc_key(actor.actor).ok_or("native NPC binding")?;
                let (_, name, x, y, floor) = anchors
                    .iter()
                    .find(|(wanted, _, x, y, floor)| {
                        *wanted == key
                            && (actor.position.x, actor.position.y, actor.position.floor)
                                == (i32::from(*x), i32::from(*y), -i16::from(*floor))
                    })
                    .ok_or("owned NPC")?;
                assert_eq!(book.display_name(key), Some(*name));
                assert_eq!(
                    (actor.position.x, actor.position.y, actor.position.floor),
                    (i32::from(*x), i32::from(*y), -i16::from(*floor))
                );
            }
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
