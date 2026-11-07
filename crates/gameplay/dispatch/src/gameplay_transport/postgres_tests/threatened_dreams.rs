//! Source NPC decisions through the actual dispatch trait and fenced QuestState writer.
//! The story test follows source NPC writes. Inventory cases seed only preceding story
//! requirements and demand actual physical hand-ins, rewards and receipts through dispatch.
#[path = "threatened_dreams/inventory.rs"]
mod inventory;
#[path = "threatened_dreams/maps.rs"]
mod maps;

use super::classic_quests::{install, progress, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{threatened_dreams as quest, world_callbacks::Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const MAELYRRA: &str = "oteryn:npc.maelyrra";
const AURITA: &str = "oteryn:npc.aurita";
const FAIRY: &str = "oteryn:npc.dancing_fairy";
const IKASSIS: &str = "oteryn:npc.ikassis";
const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const COUPLE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1";
const COUPLE_PROGRESS: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple";
const START: &str = "oteryn:quest-transition/threatened-dreams/maelyrra-start";

#[test]
fn threatened_dreams_real_npc_source_start_fences_topics_and_recovers_lost_ack() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_DREAMS_NPC_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("threatened-dreams-npc", || {
        super::quest_world_callbacks::scenario(
            "threatened-dreams-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [maelyrra, aurita, fairy, ikassis] =
        install(owner, admitted, [MAELYRRA, AURITA, FAIRY, IKASSIS]).await?;
    talk(owner, admitted, maelyrra, 2200, "hi").await?;
    talk(owner, admitted, maelyrra, 2201, "yes").await?;
    assert_eq!(
        progress(owner, admitted, authority, NIGHTMARE).await?,
        0,
        "no confirmation before the source offer"
    );
    let offered = talk(owner, admitted, maelyrra, 2202, "mission").await?;
    assert_eq!(
        offered.as_slice(),
        quest::reply(MAELYRRA, 63).ok_or("source mission reply")?
    );

    // The former Maelyrra offer must not survive a different NPC conversation.
    talk(owner, admitted, aurita, 2203, "hi").await?;
    talk(owner, admitted, aurita, 2204, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, COUPLE).await?, 0);
    talk(owner, admitted, maelyrra, 2205, "hi").await?;
    talk(owner, admitted, maelyrra, 2206, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 0);
    talk(owner, admitted, maelyrra, 2207, "mission").await?;

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut stale = fence.item_fence();
    stale.character_lease_generation += 1;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let refused = owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 2208,
                item_fence: Some(stale),
            },
            NpcTalkIntent {
                npc_actor: maelyrra,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(refused.disposition, NpcIntentDisposition::Rejected);
    assert!(!refused.unknown);
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 0);

    // Lose the acknowledgement after the actual SQL commit. The exact original
    // request/reply remain pending and must reconcile through this adapter.
    owner
        .states
        .npc_conversations
        .fixture_interrupt_threatened_commit(admitted.game_session_id, 2209)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 2209,
        item_fence: Some(current.item_fence()),
    };
    let lost = owner
        .npc_talk(
            actor,
            command,
            NpcTalkIntent {
                npc_actor: maelyrra,
                text: "yes".into(),
            },
        )
        .await;
    assert!(lost.unknown, "the fault discards only the acknowledgement");
    assert!(lost.conversation.is_none());
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 1);
    assert_eq!(
        progress(owner, admitted, authority, quest::QUESTLINE).await?,
        2
    );
    for (track, expected) in [
        (
            "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_access",
            1,
        ),
        (
            "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count",
            0,
        ),
        (
            "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count",
            0,
        ),
        (
            "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission02_1",
            1,
        ),
    ] {
        assert_eq!(progress(owner, admitted, authority, track).await?, expected);
    }
    let wrong = UseCommand {
        command_id: 2210,
        ..command
    };
    assert!(
        owner.resume_npc_bank(actor, wrong).await.is_none(),
        "another command cannot take this receipt"
    );
    let replay = owner
        .resume_npc_bank(actor, command)
        .await
        .ok_or("retained Threatened reply")?;
    assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
    assert!(!replay.unknown);
    assert!(!replay.inventory_changed);
    assert_eq!(
        replay
            .conversation
            .ok_or("source replay speech")?
            .lines
            .as_slice(),
        quest::reply(MAELYRRA, 86).ok_or("source accepted speech")?
    );
    assert!(owner.resume_npc_bank(actor, command).await.is_none());
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(START)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts, 1,
        "all six writes share one occurrence, including replay"
    );

    // Incomplete kills are a source refusal, never a fabricated success flag.
    talk(owner, admitted, maelyrra, 2211, "mission").await?;
    let incomplete = talk(owner, admitted, maelyrra, 2212, "yes").await?;
    assert_eq!(
        incomplete.as_slice(),
        quest::reply(MAELYRRA, 105).ok_or("source refusal")?
    );
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 1);

    // Two real NPCs write two distinct source tracks; a repeated spell is read-only.
    talk(owner, admitted, aurita, 2213, "hi").await?;
    talk(owner, admitted, aurita, 2214, "mission").await?;
    talk(owner, admitted, aurita, 2215, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, COUPLE).await?, 1);
    assert_eq!(
        progress(owner, admitted, authority, COUPLE_PROGRESS).await?,
        1
    );
    talk(owner, admitted, fairy, 2216, "hi").await?;
    let spell = talk(owner, admitted, fairy, 2217, "spell").await?;
    assert_eq!(
        spell.as_slice(),
        quest::reply(FAIRY, 58).ok_or("source spell")?
    );
    assert_eq!(progress(owner, admitted, authority, COUPLE).await?, 1);
    assert_eq!(
        progress(owner, admitted, authority, COUPLE_PROGRESS).await?,
        2
    );
    let repeat = talk(owner, admitted, fairy, 2218, "spell").await?;
    assert_eq!(
        repeat.as_slice(),
        quest::reply(FAIRY, 66).ok_or("repeat spell")?
    );

    // The inventory-dependent Ikassis branch is not reachable merely by saying yes.
    talk(owner, admitted, ikassis, 2219, "hi").await?;
    let silence = talk(owner, admitted, ikassis, 2220, "yes").await?;
    assert!(silence.is_empty());
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        0
    );
    let total:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/threatened-dreams/%'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        total, 3,
        "start, Aurita and Dancing Fairy only; no static, stale or duplicate receipt"
    );
    Ok(())
}
