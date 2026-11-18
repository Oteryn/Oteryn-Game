//! First Dragon knowledge trial through the production NPC command dispatcher.
use super::fixture::{Harness, debug, fence, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    CharacterId, CharacterLease, MovementFacing, MovementLocalPosition, NpcPlacement,
    NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.gelidrazah_s_thirst";
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const BASE_DEFINITION: &str = "a4ef0e5a69d1e2b40ae78dc42bd1c93f4de8e8c3b72961cc590558ee47c7b202";
const ACCESS: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/gelidrazah_access";

#[test]
fn gelidrazah_quiz_requires_all_three_answers_in_order() -> TestResult {
    run_case("qcx48_quiz", None)
}

#[test]
fn gelidrazahs_thirst_is_available_at_global_okolnir_cell() -> TestResult {
    // Resolve the placement inside the PG case so route-less runs do no setup.
    run_case("qcx48_placement", Some(global_placement))
}

fn run_case(case: &'static str, placement: Option<fn() -> TestResult<NpcPlacement>>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(case, placement).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Gelidrazah scenario thread panicked");
    eprintln!(
        "POSTGRES_CASE_COMPLETED {case} duration={:?}",
        started.elapsed()
    );
    result??;
    Ok(())
}

fn global_placement() -> TestResult<NpcPlacement> {
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/npc-placements/placements-00000-00055.json"
    )))?;
    let records = document["records"].as_array().ok_or("NPC placements")?;
    let mut matches = records
        .iter()
        .map(|record| &record["declaration"])
        .filter(|entry| entry["npc"] == NPC);
    let entry = matches.next().ok_or("missing global Okolnir placement")?;
    assert!(matches.next().is_none());
    let cell = &entry["cell"];
    assert_eq!(
        (
            cell["x"].as_u64(),
            cell["y"].as_u64(),
            cell["floor"].as_u64()
        ),
        (Some(32275), Some(31366), Some(4))
    );
    assert_eq!(entry["direction"], "north");
    Ok(NpcPlacement {
        x: u16::try_from(cell["x"].as_u64().ok_or("placement x")?)?,
        y: u16::try_from(cell["y"].as_u64().ok_or("placement y")?)?,
        floor: -i8::try_from(cell["floor"].as_u64().ok_or("placement floor")?)?,
        direction: MovementFacing::North,
    })
}

fn scenario(case: &str, placement: Option<fn() -> TestResult<NpcPlacement>>) -> TestResult {
    run(async |admin| {
        super::reset();
        let (controller, _) = activated_with_loot_tables(fence(1)?.runtime_scope.world_id(), None);
        let h = Harness::create_with_admission_seed(admin, case, true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active content")?);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let session = deps.admitted.game_session_id;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
                &model,
                &[NPC],
            )));
        owner
            .states
            .chat
            .admit(session, "Quest witness", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let gameplay = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "quest fence read")?
            .ok_or("quest fence")?;
        let anchor = match placement {
            Some(load) => load()?,
            None => NpcPlacement {
                x: 32275,
                y: 31366,
                floor: -4,
                direction: MovementFacing::North,
            },
        };
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 32274,
                        y: 31366,
                        floor: 4,
                    },
                )
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    CharacterLease::new(
                        CharacterId::decode(gameplay.character_id.as_bytes()).map_err(debug)?,
                        gameplay.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![anchor],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .next()
                .ok_or("quiz NPC")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        deps.initialize(&owner).await;
        if placement.is_none() {
            // A player who accepted through Tamoril on the base still holds
            // that catalogue's exact definition hash.
            let mut fixture = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *fixture)
                .await?;
            sqlx::query(
                "INSERT INTO game_character_quest_states(
                     character_id, quest_key, pinned_content_revision, definition_hash,
                     started_character_revision, committed_character_revision)
                 SELECT character_id, $1, content_revision, decode($2, 'hex'),
                        character_revision, character_revision FROM game_character_roots",
            )
            .bind(QUEST)
            .bind(BASE_DEFINITION)
            .execute(&mut *fixture)
            .await?;
            fixture.commit().await?;
        }
        owner.refresh_quest_session(session, true).await;
        let read_access = || {
            sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT max(value) FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
                )
                .bind(QUEST)
                .bind(ACCESS)
                .fetch_one(&h.pool)
        };

        // TibiaWiki spoiler oldid 1054086, Gelidrazah's Thirst transcript.
        for (index, (message, expected)) in [
            ("hi", "Have you come to answer Gelidrazah's questions?"),
            ("yes", "There are three questions."),
            ("yes", "So I ask you:"),
            ("Tahmehe", "That's right. Listen to the second question:"),
            ("Svir", "I don't know what you are talking about."),
            ("Ishara", "That's right. Listen to the third question:"),
            ("Svir", "You may pass and enter Gelidrazah's lair."),
        ]
        .into_iter()
        .enumerate()
        {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("dialogue clock")?;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "quest fence read")?
                .ok_or("quest fence")?;
            let result = owner
                .npc_talk(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 4800 + u64::try_from(index)?,
                        item_fence: Some(fence.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: message.into(),
                    },
                )
                .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok, "{message}");
            let reply = result
                .conversation
                .map(|line| line.lines)
                .unwrap_or_default();
            assert!(reply.join(" ").contains(expected), "{message}: {reply:?}");
            assert_eq!(read_access().await?, (index == 6).then_some(1));
            if placement.is_some() {
                // This case proves that the generated placement resolves to the live NPC.
                break;
            }
        }
        Ok(())
    })
}
