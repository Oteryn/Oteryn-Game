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

const ANCHORS: [(&str, &str, u16, u16, i8); 6] = [
    (
        "oteryn:npc.a_restless_soul",
        "A Restless Soul",
        32255,
        31101,
        -10,
    ),
    ("oteryn:npc.chemar", "Chemar", 33269, 32438, -6),
    ("oteryn:npc.eclesius", "Eclesius", 32293, 32292, -7),
    ("oteryn:npc.eshaya", "Eshaya", 33913, 31497, -6),
    ("oteryn:npc.hagor", "Hagor", 32652, 32150, -10),
    ("oteryn:npc.malor", "Malor", 33043, 32624, -1),
];

#[test]
fn npcfix_anchor_postgres_real_map_admission_and_named_runtime_actors() -> TestResult {
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
            eprintln!("POSTGRES_STARTED NPCFIX real source anchors six NPCs");
            let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
            let regions = ANCHORS
                .iter()
                .map(|(_, _, x, y, _)| (x / 256, y / 256))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let project = project_regions(root, &regions).map_err(debug)?;
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
            for (key, _, x, y, floor) in ANCHORS {
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
                    1,
                    "one spawn anchor, not a second roaming actor"
                );
                let point = row.placements[0];
                assert_eq!(
                    (point.x, point.y, point.floor),
                    (x, y, floor),
                    "reviewed admitted roaming cell"
                );
                assert_eq!(
                    point.direction,
                    oteryn_world_bundle_compiler::spawn::Direction::North
                );
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
                7,
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
            assert_eq!(visible.npcs.len(), ANCHORS.len());
            for actor in visible.npcs {
                let key = runtime.npc_key(actor.actor).ok_or("native NPC binding")?;
                let (_, name, x, y, floor) = ANCHORS
                    .iter()
                    .find(|(wanted, _, _, _, _)| *wanted == key)
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
