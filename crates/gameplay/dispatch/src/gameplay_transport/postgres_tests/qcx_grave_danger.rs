//! TibiaWiki Grave Danger Quest/Spoiler, curid 88737, oldid 1176165.
use super::fixture::{Harness, debug, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    CharacterId, CharacterLease, MovementFacing, MovementLocalPosition, NpcPlacement,
    NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::{grave_danger as grave, loader};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn jack_starts_twelve_graves_and_answers_an_early_report() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-grave-danger", scenario);
    eprintln!(
        "QCX_GRAVE_DANGER duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx_grave_danger", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let admitted = dependencies.admitted;
        let mut owner = dependencies.owner(&h, &authority);
        let lowered = loader::load_embedded_quest_state("content-1").map_err(debug)?;
        owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
        owner.admit_quest_session(&admitted).await;
        let jack = place_jack(&owner, &admitted).await?;
        talk(&mut owner, &admitted, jack, 100, "hi").await?;
        let lines = talk(&mut owner, &admitted, jack, 101, "late").await?;
        assert!(
            lines
                .iter()
                .any(|line| line.contains("very potent holy water"))
        );
        let stage: i64 = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE track_key = $1",
        )
        .bind(grave::QUESTLINE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stage, 1);
        let initialized: Vec<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks
             WHERE track_key LIKE 'oteryn:quest-progress/quest/u12_20/grave_danger/graves/%'",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(initialized, vec![1; 12]);

        let lines = talk(&mut owner, &admitted, jack, 102, "report").await?;
        assert_eq!(
            lines,
            [
                "Sadly, I have no news yet. But I can give you information about the {locations} of the graves that we learned about. If you need more holy water just ask me for a {trade}."
            ]
        );
        talk(&mut owner, &admitted, jack, 103, "late").await?;
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key = $1",
        )
        .bind(grave::START)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 1);
        Ok(())
    })
}

async fn place_jack(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<EntityRef> {
    let model = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?
    .service_model_for_project()
    .map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model,
            &[grave::NPC],
        )));
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: grave::NPC.into(),
                    placements: vec![NpcPlacement {
                        x: u16::try_from(at.x)?,
                        y: u16::try_from(at.y + 1)?,
                        floor: -i8::try_from(at.floor)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let jack = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some(grave::NPC))
        .ok_or("placed Jack Springer")?;
    Ok(EntityRef {
        identity: jack.actor.placement_identity(),
        generation: jack.generation,
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc_actor: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<Vec<String>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
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
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    assert!(!outcome.unknown, "{text}");
    Ok(outcome
        .conversation
        .map(|conversation| conversation.lines)
        .unwrap_or_default())
}
