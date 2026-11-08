//! M38: reuse the existing native source program; a shared reward callback is unnecessary.
//! This proves the fenced dialogue step, not physical Mistrock access or the whole quest.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{npc_rules::NpcRuleTrigger, world_callbacks::Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const NPC: &str = "oteryn:npc.maris_mistrock";
const TRACK: &str = "oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/way_to_beregar";
const RULE: &str = "oteryn:quest-npc/the_hidden_city_of_beregar_quest/maris_mistrock/hidden-path";

#[test]
fn npcfix_m38_mistrock_source_dialogue_commits_once_and_replays_postgres() -> TestResult {
    run_case("npcfix-m38-mistrock")
}

#[test]
fn npcfix_m38_mistrock_requires_current_premium_postgres() -> TestResult {
    run_case("npcfix-m38-mistrock-free")
}

fn run_case(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 32641,
                y: 31436,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_MISTROCK case={name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    premium: bool,
) -> TestResult {
    eprintln!("POSTGRES_STARTED NPCFIX M38 existing Mistrock native source program");
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    let position = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| {
                n.actor.placement_identity() == npc.identity && n.generation == npc.generation
            })
            .ok_or("Mistrock actor")?
            .position
    };
    assert_eq!((position.x, position.y, position.floor), (32640, 31437, 7));
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        0
    );
    super::classic_quests::talk(owner, admitted, npc, 38800, "hi").await?;
    if !premium {
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
                    command_id: 38801,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "mistrock".into(),
                },
            )
            .await;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
            0
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(RULE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipts, 0,
            "free-account dialogue cannot commit the premium-gated stage"
        );
        return Ok(());
    }
    let reply = super::classic_quests::talk(owner, admitted, npc, 38801, "mistrock").await?;
    assert_eq!(reply.len(), 2);
    assert!(reply[0].contains("fire of a forge"));
    assert!(reply[1].contains("Fenrock"));
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(38801).map_err(debug)?,
        ),
        npc: NPC.into(),
        trigger: NpcRuleTrigger::Say,
        message: "mistrock".into(),
        topic: 0,
        transition_key: RULE.into(),
    };
    let replay = owner
        .root
        .reconcile_npc_program(authority, fence.character_id, request.clone())
        .await
        .map_err(debug)?
        .ok_or("source receipt")?;
    assert_eq!(replay.reply, reply);
    let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else {
        return Err("expected bound command replay".into());
    };
    assert_eq!(receipt.changes.len(), 1);
    assert_eq!(receipt.changes[0].after, 1);
    assert_eq!(
        receipt.committed_character_revision.get(),
        receipt.original_character_revision.get() + 1
    );
    let mut conflicting = request;
    conflicting.message = "fenrock".into();
    assert!(matches!(owner.root.reconcile_npc_program(authority, fence.character_id, conflicting).await,
        Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)));
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(RULE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(before, 1);
    let repeated = super::classic_quests::talk(owner, admitted, npc, 38802, "mistrock").await?;
    assert_eq!(repeated, reply);
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(RULE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        after, before,
        "repeat dialogue must not mint a second progress receipt"
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    Ok(())
}
