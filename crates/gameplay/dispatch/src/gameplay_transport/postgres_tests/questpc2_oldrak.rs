//! Oldrak's source NPC steps only; the defender encounter is not qualified here.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const PREFIX: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/";

#[test]
fn questpc2_oldrak_source_confirmation_decline_and_finish_are_durable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC2_OLDRAK_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc2-oldrak-npc", || {
        super::quest_world_callbacks::scenario(
            "questpc2-oldrak-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC2_OLDRAK_POSTGRES_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn unoffered_yes(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
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
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC2_OLDRAK_POSTGRES_SERVER_VERSION={version}");
    let [oldrak] = install(owner, admitted, ["oteryn:npc.oldrak"]).await?;
    talk(owner, admitted, oldrak, 29000, "hi").await?;
    unoffered_yes(owner, admitted, oldrak, 29001).await?;
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}progress")).await?,
        0
    );
    let offer = talk(owner, admitted, oldrak, 29002, "mission").await?;
    assert!(offer.iter().any(|line| line.contains("infested area")));
    talk(owner, admitted, oldrak, 29003, "no").await?;
    unoffered_yes(owner, admitted, oldrak, 29004).await?;
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}progress")).await?,
        0
    );
    talk(owner, admitted, oldrak, 29005, "demon oak").await?;
    let accepted = talk(owner, admitted, oldrak, 29006, "yes").await?;
    assert!(accepted.iter().any(|line| line.contains("hallowed axe")));
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}progress")).await?,
        2
    );
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}done")).await?,
        0
    );
    talk(owner, admitted, oldrak, 29007, "mission").await?;
    talk(owner, admitted, oldrak, 29008, "yes").await?;
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-npc/demon_oak/oldrak/%'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        grants, 1,
        "temporary Lua Progress1 then2 is one final-value receipt"
    );

    // Explicit finish-branch precondition through the canonical writer. This does
    // not simulate or qualify the missing defender-wave encounter.
    let catalogue = std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/quest/u8_2/the_demon_oak/done/action_1"
                    .into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(29020).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let secret = talk(owner, admitted, oldrak, 29021, "mission").await?;
    assert_eq!(secret.len(), 3);
    assert!(secret.iter().any(|line| line.contains("Yesim Adeit")));
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}done")).await?,
        2
    );
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}squares")).await?,
        0
    );
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-npc/demon_oak/oldrak/%'")
        .fetch_one(&h.pool).await?;
    assert_eq!(grants, 2, "one start and one finish, never a repeat grant");
    Ok(())
}
