//! PARITY_PENDING entry boundaries for the journeys in the W16 content evidence.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const MAKE_BELIEVE: &str = "oteryn:quest.authored.make_believe_quest";
const SHARDS: &str = "oteryn:quest.authored.shards_of_a_broken_moon_quest";

#[test]
fn summer_make_believe_first_entry_waits_for_wayland() -> TestResult {
    // waits Content: oteryn:npc.wayland_smythers (Venore entry and final report)
    // waits Content: oteryn:npc.mayor_pocaro (Salgadara entry and raid report)
    // W16.full and W16.client fail at the compiled bundle census. Stop at
    // first entry without placing either NPC or attempting their dialogue.
    scenario_thread::run_async(async |admin| {
        assert_held_bindings(content_root(), MAKE_BELIEVE)?;
        let h =
            Harness::create_with_admission_seed(admin, "summer_make_believe_entry", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let session = deps.admitted.game_session_id;
        let before = quest_effects(&h, MAKE_BELIEVE).await?;
        {
            let mut runtime = deps.runtime.lock().await;
            let entry = runtime.content_pin().entry_start();
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let actual = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                (actual.x, actual.y, actual.floor),
                (entry.x, entry.y, entry.floor)
            );
        }
        let fence = super::rotten_blood_progression_fixtures::current(&owner, session).await?;
        assert_eq!(fence.item_fence().character_id.as_bytes(), &id(41));
        assert_eq!(before, (before.0, false, false, false, false));
        assert_eq!(quest_effects(&h, MAKE_BELIEVE).await?, before);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!("W16 first entry reached; waits Content: oteryn:npc.wayland_smythers");
        eprintln!("W16 downstream stage waits Content: oteryn:npc.mayor_pocaro");
        Ok(())
    })
}

#[test]
fn summer_shards_both_opening_contacts_have_no_durable_entry() -> TestResult {
    // PARITY_PENDING: neither route has a qualified native producer for the
    // opening quest state or the downstream clue, encounter and reward chain.
    scenario_thread::run_async(shards_entry)
}

fn content_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"))
}

fn placement(content: &Path, npc: &str) -> TestResult<NpcPlacement> {
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        content.join("world/npc-placements/index.json"),
    )?)?;
    let mut placements = Vec::new();
    for shard in index["shards"].as_array().ok_or("NPC placement shards")? {
        let path = shard
            .as_str()
            .and_then(|path| path.strip_prefix("content/"))
            .ok_or("NPC shard path")?;
        let rows: serde_json::Value = serde_json::from_slice(&std::fs::read(content.join(path))?)?;
        for row in rows["records"].as_array().ok_or("NPC placement records")? {
            let declaration = &row["declaration"];
            if declaration["npc"] != npc {
                continue;
            }
            let cell = &declaration["cell"];
            assert_eq!(declaration["direction"], "north", "{npc}");
            placements.push(NpcPlacement {
                x: u16::try_from(cell["x"].as_u64().ok_or("NPC x")?)?,
                y: u16::try_from(cell["y"].as_u64().ok_or("NPC y")?)?,
                floor: -i8::try_from(cell["floor"].as_u64().ok_or("NPC floor")?)?,
                direction: MovementFacing::North,
            });
        }
    }
    assert_eq!(placements.len(), 1, "pinned opening contact {npc}");
    placements
        .into_iter()
        .next()
        .ok_or_else(|| format!("No pinned placement for {npc}").into())
}

fn assert_held_bindings(content: &Path, quest: &str) -> TestResult {
    let plan: serde_json::Value = serde_json::from_slice(&std::fs::read(
        content.join("quests/missions/completion-binding-plan.json"),
    )?)?;
    let row = plan["records"]
        .as_array()
        .ok_or("binding plan records")?
        .iter()
        .find(|row| row["quest"] == quest)
        .ok_or("Summer binding plan")?;
    assert_eq!(row["runtime_enabled"], false, "PARITY_PENDING {quest}");
    assert!(row["native_reward_delivery_binding"].is_null());
    let stages = row["stages"].as_array().ok_or("quest stages")?;
    assert!(!stages.is_empty());
    for stage in stages {
        assert!(
            stage["native_dispatch_binding"].is_null(),
            "PARITY_PENDING {quest}: {stage}"
        );
    }
    Ok(())
}

async fn shards_entry(admin: String) -> TestResult {
    let content = content_root();
    assert_held_bindings(content, SHARDS)?;
    let quest_catalogue =
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
    let quest_catalogue = Arc::new(quest_catalogue.catalogue().clone());
    let catalogue = NpcDataCatalogue::load(
        content,
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = Arc::new(NpcTalkBook::from_model(
        &catalogue.service_model_for_project().map_err(debug)?,
    ));
    for npc in ["oteryn:npc.saraki", "oteryn:npc.nilavarna"] {
        let placement = placement(content, npc)?;
        let h =
            Harness::create_with_admission_seed(admin.clone(), "summer_quest_entry", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut owner = deps.owner(&h, &authority);
        owner.quest_catalogue = Some(Arc::clone(&quest_catalogue));
        let session = deps.admitted.game_session_id;
        let actor = deps.actor;
        let before = quest_effects(&h, SHARDS).await?;
        assert_eq!(before, (before.0, false, false, false, false));
        owner.states.chat.install_npc_talk(Arc::clone(&book));
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc_actor = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: i32::from(placement.x) - 1,
                        y: i32::from(placement.y),
                        floor: -i16::from(placement.floor),
                    },
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: npc.into(),
                            placements: vec![placement],
                        }],
                    },
                    |key| key == npc,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        for (command_id, text) in [(1, "hi"), (2, "moon")] {
            owner.clock_origin -= Duration::from_secs(2);
            let fence = super::rotten_blood_progression_fixtures::current(&owner, session).await?;
            let reply = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id,
                        item_fence: Some(fence.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "{npc}: {text}");
            assert!(!reply.unknown, "{npc}: {text}");
            if text == "hi" {
                assert!(reply.conversation.is_some(), "{npc} greeting");
            }
        }
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?,
            position,
            "{npc}"
        );
        assert_eq!(quest_effects(&h, SHARDS).await?, before, "{npc}");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!("W16 PARITY_PENDING NPC entry boundary reached: {npc}");
    }
    Ok(())
}

async fn quest_effects(h: &Harness, quest: &str) -> TestResult<(i64, bool, bool, bool, bool)> {
    Ok(sqlx::query_as(
            "SELECT character_revision::bigint,
                    EXISTS (SELECT 1 FROM game_character_quest_states WHERE character_id=r.character_id AND quest_key=$2),
                    EXISTS (SELECT 1 FROM game_character_quest_tracks WHERE character_id=r.character_id AND quest_key=$2),
                    EXISTS (SELECT 1 FROM game_character_quest_receipts WHERE character_id=r.character_id AND quest_key=$2),
                    EXISTS (SELECT 1 FROM game_reward_claim_mint_receipts WHERE character_id=r.character_id)
             FROM game_character_roots r WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .bind(quest)
        .fetch_one(&h.pool)
        .await?)
}

#[test]
fn summer_make_believe_boss_windows_refuse_held_rooms() -> TestResult {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use crate::gameplay_transport::item_fence_of;
    use oteryn_protocol_oteryn::{CommandStatus, boss_difficulty as wire};

    // PARITY_PENDING: pinned creature profiles supply no admitted room, no
    // phase controller, and no victory outcome for these difficulty bosses.
    scenario_thread::run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "summer_boss_windows", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut session = deps.admitted;
        session.first_entry = FirstEntryOutcome::Positioned;
        session.item_fence = item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session.game_session_id)
                .await?
                .0,
        );
        session.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                wire::CAPABILITY_BOSS_DIFFICULTY_V1,
            ],
        )
        .ok_or("difficulty bundle capability")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let expected_reply = wire::encode_reply(&wire::Reply::Refused {
            context: None,
            reason: wire::Refusal::BossUnavailable,
        })
        .map_err(debug)?;
        for boss_key in [
            "oteryn:boss.mimar_haffar",
            "oteryn:boss.maior_domus",
            "oteryn:boss.phosphorus",
        ] {
            let command_id = session.continuity.next_command_id;
            let request = wire::encode_request(&wire::Request::Open {
                boss_key: boss_key.into(),
            })
            .map_err(debug)?;
            let (status, reply) = super::boss_difficulty_fixtures::exchange(
                &owner,
                &mut session,
                command_id,
                &request,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted, "{boss_key}");
            assert_eq!(reply, expected_reply, "{boss_key}");
            let receipt: (Vec<u8>, Vec<u8>) = sqlx::query_as(
                "SELECT request, reply FROM game_boss_difficulty_receipts
                 WHERE character_id=encode($1,'hex')::uuid
                   AND game_session_id=encode($2,'hex')::uuid
                   AND connection_generation=$3::text::numeric
                   AND command_id=$4::text::numeric",
            )
            .bind(id(41).as_slice())
            .bind(session.game_session_id.as_bytes().as_slice())
            .bind(session.continuity.connection_generation.to_string())
            .bind(command_id.to_string())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipt, (request, expected_reply.clone()), "{boss_key}");
        }
        let effects: (bool, bool, bool) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM game_boss_difficulty_contexts WHERE character_id=encode($1,'hex')::uuid),
                    EXISTS (SELECT 1 FROM game_boss_room_participants WHERE character_id=encode($1,'hex')::uuid),
                    EXISTS (SELECT 1 FROM game_boss_room_reward_receipts WHERE character_id=encode($1,'hex')::uuid)",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(effects, (false, false, false));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!("W16 PARITY_PENDING Make Believe boss windows: three exact refusal receipts");
        Ok(())
    })
}
