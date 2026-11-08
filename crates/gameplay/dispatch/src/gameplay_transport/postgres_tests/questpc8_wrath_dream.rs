//! Chosen Wrath stage7 reuses the actual fenced Sleeping Dragon receipt.
use super::classic_quests::{progress, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
pub(super) const STAGE: &str = "oteryn:quest-progress/chosen/wrath/stage";
const REPORT: &str = "oteryn:quest-reward/chosen/wrath/dream-report";

async fn refuse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    dragon: EntityRef,
    command_id: u64,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: dragon,
                text: "dream journey".into(),
            },
        )
        .await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
    assert!(!outcome.unknown);
    Ok(())
}

pub(super) async fn before(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    dragon: EntityRef,
) -> TestResult {
    eprintln!("POSTGRES_CASE_STARTED questpc8_wrath_dream_report");
    assert!(
        owner.premium_coordinator.is_some(),
        "chosen Wrath needs native Premium authority"
    );
    assert!(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("catalogue")?
            .transition(REPORT)
            .is_some(),
        "WRATH_DREAM_REPORT_RED: chosen stage has no receipt-bound native report"
    );
    talk(owner, admitted, dragon, 6001, "hi").await?;
    refuse(owner, admitted, dragon, 6002).await
}

pub(super) async fn after(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    dragon: EntityRef,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 6);
    talk(owner, admitted, dragon, 6300, "hi").await?;
    talk(owner, admitted, dragon, 6301, "dream journey").await?;
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 7);
    talk(owner, admitted, dragon, 6301, "dream journey").await?;
    refuse(owner, admitted, dragon, 6302).await?;
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    talk(owner, admitted, dragon, 6303, "hi").await?;
    refuse(owner, admitted, dragon, 6304).await?;
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 7);
    let receipt_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2").bind(id(41).as_slice()).bind(REPORT).fetch_one(&h.pool).await?;
    assert_eq!(receipt_count, 1);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind("oteryn:quest.wrath_of_the_emperor_quest").fetch_one(&h.pool).await?;
    assert!(
        !completed,
        "dream report must not complete the whole Wrath quest"
    );
    eprintln!("POSTGRES_WRATH_DREAM_RECEIPT_STAGE_REPLAY_AND_COLD_REFUSAL_PASS");
    Ok(())
}
