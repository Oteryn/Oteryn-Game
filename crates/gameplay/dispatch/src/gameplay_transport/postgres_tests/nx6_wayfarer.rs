//! NX-6: the actual NPC route must commit the dream and Wayfarer together.
use super::classic_quests::{progress, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, world_callbacks::Position,
    wrath_sleeping_dragon as quest,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

#[test]
fn nx6_wayfarer_real_npc_atomic_finale_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("nx6-wayfarer", || {
        super::quest_world_callbacks::scenario(
            "nx6-wayfarer",
            Position {
                x: 33243,
                y: 31214,
                floor: -10,
            },
            false,
        )
    })
}

fn advance(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC rate clock")?;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/wayfarer'), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1)")
        .bind(quest::COMPLETE).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let started = std::time::Instant::now();
    let dragon = super::questpc8_wrath_source_dragon::install(owner, admitted).await?;
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let keys = [
        quest::QUESTLINE,
        quest::MISSION,
        quest::POTION,
        super::questpc8_wrath_dream::STAGE,
    ];
    // Only prerequisites are seeded. The real NPC must earn the dream receipt.
    let setup_key = "oteryn:quest-transition/nx6-fixture-dream-ready";
    let catalogue = Arc::new(
        canonical
            .with_fixture_transition(QuestTransition {
                world_change: None,
                key: setup_key.into(),
                quest: quest::QUEST.into(),
                effects: keys
                    .into_iter()
                    .zip([27, 1, 1, 6])
                    .map(|(key, value)| QuestEffect {
                        track: key.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(value),
                    })
                    .collect(),
                completes: false,
                experience: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: setup_key.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(6000).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::questpc8_wrath_dream::before(owner, admitted, dragon).await?;
    talk(owner, admitted, dragon, 6003, "hi").await?;
    talk(owner, admitted, dragon, 6004, "SOLOSARASATIQUARIUM").await?;
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/dialogues/quests-2/wrath-sleeping-dragon.json"
    ))?;
    for (index, step) in source["steps"]
        .as_array()
        .ok_or("steps")?
        .iter()
        .take(31)
        .enumerate()
    {
        let lines = talk(
            owner,
            admitted,
            dragon,
            6010 + u64::try_from(index)?,
            step["keyword"].as_str().ok_or("word")?,
        )
        .await?;
        assert!(!lines.is_empty());
    }
    assert_eq!(counts(h).await?, (0, 0));
    let actor = admitted.runtime_actor.ok_or("actor")?;
    for connection in [false, true] {
        advance(owner)?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let mut stale = fence.item_fence();
        if connection {
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(stale.connection_generation.get() + 1)
                    .map_err(debug)?;
        } else {
            stale.character_lease_generation += 1;
        }
        let result = owner
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 6100 + u64::from(connection),
                    item_fence: Some(stale),
                },
                NpcTalkIntent {
                    npc_actor: dragon,
                    text: "use mirror".into(),
                },
            )
            .await;
        assert_eq!(result.disposition, NpcIntentDisposition::Rejected);
        assert_eq!(counts(h).await?, (0, 0));
    }
    // Force the achievement owner to fail after the quest writes. The physical
    // transaction must leave neither the source stores nor its receipt behind.
    sqlx::query("CREATE FUNCTION nx6_refuse_grant() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'NX6 grant rollback probe'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER nx6_refuse_grant BEFORE INSERT ON game_account_achievements FOR EACH ROW EXECUTE FUNCTION nx6_refuse_grant()")
        .execute(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let failed_command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 6150,
        item_fence: Some(fence.item_fence()),
    };
    let failed_intent = NpcTalkIntent {
        npc_actor: dragon,
        text: "use mirror".into(),
    };
    advance(owner)?;
    let failed = owner.npc_talk(actor, failed_command, failed_intent).await;
    assert_eq!(failed.disposition, NpcIntentDisposition::Rejected);
    assert_eq!(counts(h).await?, (0, 0));
    assert_eq!(
        progress(owner, admitted, authority, quest::QUESTLINE).await?,
        27
    );
    assert_eq!(
        progress(owner, admitted, authority, quest::MISSION).await?,
        1
    );
    sqlx::query("DROP TRIGGER nx6_refuse_grant ON game_account_achievements")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION nx6_refuse_grant()")
        .execute(&h.pool)
        .await?;
    if failed.unknown {
        let reconciled = owner
            .resume_npc_bank(actor, failed_command)
            .await
            .ok_or("failed receipt reconciliation")?;
        assert!(!reconciled.unknown);
        assert_eq!(reconciled.disposition, NpcIntentDisposition::Rejected);
    }
    owner
        .states
        .npc_conversations
        .fixture_interrupt_quest_dialogue_commit(admitted.game_session_id, 6200)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 6200,
        item_fence: Some(fence.item_fence()),
    };
    advance(owner)?;
    let lost = owner
        .npc_talk(
            actor,
            command,
            NpcTalkIntent {
                npc_actor: dragon,
                text: "use mirror".into(),
            },
        )
        .await;
    assert!(
        lost.unknown,
        "lose the acknowledgement after the SQL commit: {:?}",
        lost.disposition
    );
    assert_eq!(counts(h).await?, (1, 1));
    // The transport's unknown-command recovery calls this production seam,
    // rather than dispatching the word as a new ordinary conversation.
    let recovered = owner
        .resume_npc_bank(actor, command)
        .await
        .ok_or("retained dream reply")?;
    assert_eq!(recovered.disposition, NpcIntentDisposition::Ok);
    assert!(!recovered.unknown);
    let lines = recovered.conversation.ok_or("final speech")?.lines;
    assert_eq!(
        lines.len(),
        7,
        "the finale must reach the live durable caller"
    );
    assert_eq!(counts(h).await?, (1, 1));
    assert_eq!(
        progress(owner, admitted, authority, quest::QUESTLINE).await?,
        28
    );
    assert_eq!(
        progress(owner, admitted, authority, quest::MISSION).await?,
        2
    );
    talk(owner, admitted, dragon, 6200, "use mirror").await?;
    talk(owner, admitted, dragon, 6201, "use mirror").await?;
    assert_eq!(counts(h).await?, (1, 1));
    let facts = h
        .root
        .read_account_achievements(
            authority,
            admitted.controller.ok_or("controller")?.account_id,
            1000,
        )
        .await
        .map_err(debug)?;
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].achievement_key, "oteryn:achievement/wayfarer");
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "notice fence")?
        .ok_or("notice fence")?;
    let mut observed_session = *admitted;
    observed_session.item_fence = Some(current.item_fence());
    let observed = owner
        .observe_achievement_notice_facts(observed_session)
        .await
        .ok_or("production achievement notice")?;
    let notices = observed.facts.ok_or("complete notice facts")?;
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].name, "Wayfarer");
    let payload =
        oteryn_protocol_oteryn::achievement_notices::encode_achievement_earned(&notices[0])
            .map_err(debug)?;
    assert_eq!(
        payload.as_slice(),
        b"\x0a\x1boteryn:achievement/wayfarer\x12\x08Wayfarer\x18\x01\x20\x03",
        "these exact server bytes are decoded by the native client play test"
    );
    super::questpc8_wrath_dream::after(owner, admitted, dragon, h, authority).await?;
    eprintln!(
        "NX6_POSTGRES_CASE_COMPLETED elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
