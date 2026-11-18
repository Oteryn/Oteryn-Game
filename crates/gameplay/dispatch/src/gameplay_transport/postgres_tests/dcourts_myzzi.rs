//! Court entry is written by the production dialogue dispatcher after Myzzi's confirmation.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, npc_rules::NpcRuleCatalog};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.myzzi";
const TRACK: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/main/questline";

#[test]
fn dcourts_myzzi_dialogue_commits_court_access_only_after_confirmation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(Box::pin(async move {
                    let h =
                        Harness::create_with_admission_seed(admin, "dcourts_myzzi", true, false)
                            .await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                }))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Myzzi PostgreSQL thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let seed = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        seed.admitted.world_id,
        "content-1",
    )?;
    let start = activation.entry_start();
    let room = activation.qualified_room().clone();
    let deps = Dependencies::with_content(activation.into_channel_pin(), room, seed.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(controller.active().ok_or("active NPC generation")?);
    let lowered = load_embedded_quest_state("content-1").map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence unavailable")?
        .ok_or("quest fence")?;
    admitted.item_fence = Some(fence.item_fence());
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let npc = {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .bind_attacker_lease(deps.actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .map_err(debug)?;
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: NPC.into(),
                        placements: vec![NpcPlacement {
                            x: u16::try_from(start.x)?,
                            y: u16::try_from(start.y + 1)?,
                            floor: -i8::try_from(start.floor)?,
                            direction: MovementFacing::North,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find_map(|npc| {
                (runtime.npc_key(npc.actor) == Some(NPC)).then_some(EntityRef {
                    identity: npc.actor.placement_identity(),
                    generation: npc.generation,
                })
            })
            .ok_or("Myzzi placement")?
    };
    let content = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = content.service_model_for_project().map_err(debug)?;
    let source = NpcRuleCatalog::runtime()?;
    let keys = source
        .programs()
        .iter()
        .flat_map(|program| program.document.rules.iter().map(|rule| rule.npc.as_str()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model, &keys,
        )));
    owner
        .states
        .chat
        .admit(session, "Court witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;

    for (ordinal, text) in [
        "hi",
        "yes",
        "good",
        "help",
        "threat",
        "courts",
        "entrances",
        "no",
        "yes",
        "yes",
    ]
    .into_iter()
    .enumerate()
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("dialogue clock")?;
        let reply = owner
            .npc_talk(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 9000 + u64::try_from(ordinal)?,
                    item_fence: admitted.item_fence,
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "{text}");
        assert!(!reply.unknown, "{text}");
        let value: Option<i64> =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_optional(&h.pool)
                .await?;
        assert_eq!(value, (ordinal >= 8).then_some(1), "{text}");
    }
    let writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE $1 = ANY(track_keys)",
    )
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(writes, 1);
    Ok(())
}
