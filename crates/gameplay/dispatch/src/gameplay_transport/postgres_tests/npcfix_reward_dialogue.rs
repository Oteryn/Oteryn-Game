//! Actual carried-item effects through the production NPC and fenced quest owners.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn npcfix_reward_postgres_fury_cage_is_atomic_and_not_duplicated() -> TestResult {
    scenario("npcfix-reward-fury")
}
#[test]
fn npcfix_reward_postgres_al_dee_costed_pick_repeats() -> TestResult {
    scenario("npcfix-reward-al-dee")
}
#[test]
fn npcfix_reward_postgres_wandering_crystal_requires_mission_and_exact_words() -> TestResult {
    scenario("npcfix-reward-wandering")
}

async fn count(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    eprintln!("POSTGRES_STARTED NPCFIX reward dialogue {name}");
    let npc = match name {
        "npcfix-reward-fury" => "oteryn:npc.a_weakened_forest_fury",
        "npcfix-reward-al-dee" => "oteryn:npc.al_dee",
        _ => "oteryn:npc.a_wandering_soul",
    };
    let [actor] = install(owner, admitted, [npc]).await?;
    talk(owner, admitted, actor, 100, "hi").await?;
    if name == "npcfix-reward-fury" {
        talk(owner, admitted, actor, 101, "yes").await?;
        assert_eq!(count(h, 23812).await?, 0);
        let offered = talk(owner, admitted, actor, 102, "mission").await?;
        assert!(offered.iter().any(|line| line.contains("guardian")));
        talk(owner, admitted, actor, 103, "yes").await?;
        assert_eq!(count(h, 23812).await?, 1);
        assert_eq!(
            progress(
                owner,
                admitted,
                authority,
                "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/bird_cage"
            )
            .await?,
            1
        );
        talk(owner, admitted, actor, 104, "cages").await?;
        talk(owner, admitted, actor, 105, "yes").await?;
        assert_eq!(
            count(h, 23812).await?,
            1,
            "replacement dialogue cannot mint another free cage"
        );
        return Ok(());
    }
    let input = if name == "npcfix-reward-al-dee" {
        3462
    } else {
        8453
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for occurrence in [700, 701] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: format!("oteryn:item.tibia.i{input}"),
                count: 1,
            },
            occurrence,
            true,
        )
        .await?;
    }
    if name == "npcfix-reward-al-dee" {
        let time = talk(owner, admitted, actor, 109, "time").await?;
        assert_eq!(time.len(), 1);
        assert!(!time[0].contains("|TIME|"));
        assert!(time[0].contains(':'));
        for (id, left) in [(110, 1), (120, 0)] {
            talk(owner, admitted, actor, id, "pick").await?;
            talk(owner, admitted, actor, id + 1, "yes please").await?;
            assert_eq!(
                count(h, 3462).await?,
                left + 1,
                "only exact grant keywords consume an axe"
            );
            if id == 110 {
                let physical = owner.spell_lane_permit().await.ok_or("physical lane")?;
                let mut paid = Box::pin(talk(owner, admitted, actor, id + 2, "yes"));
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(500), paid.as_mut())
                        .await
                        .is_err(),
                    "a reward hand-in must wait for the existing physical lane"
                );
                assert_eq!(
                    count(h, 3462).await?,
                    left + 1,
                    "held physical lane preserves the inputs"
                );
                assert_eq!(
                    count(h, 3456).await?,
                    1 - left,
                    "held physical lane cannot mint"
                );
                drop(physical);
                paid.await?;
            } else {
                talk(owner, admitted, actor, id + 2, "yes").await?;
            }
            assert_eq!(count(h, 3462).await?, left);
            assert_eq!(count(h, 3456).await?, 2 - left);
        }
        talk(owner, admitted, actor, 130, "pick").await?;
        attempt(
            owner,
            admitted,
            actor,
            131,
            "yes",
            NpcIntentDisposition::Rejected,
        )
        .await?;
        assert_eq!(count(h, 3462).await?, 0);
        assert_eq!(count(h, 3456).await?, 2, "an unpaid attempt writes no gift");
        attempt(owner, admitted, actor, 122, "yes", NpcIntentDisposition::Ok).await?;
        assert_eq!(
            count(h, 3456).await?,
            2,
            "exact replay cannot mint a third pick"
        );
    } else {
        talk(owner, admitted, actor, 110, "blood crystal").await?;
        assert_eq!(
            count(h, 8453).await?,
            2,
            "mission refusal preserves both crystals"
        );
        assert_eq!(count(h, 8225).await?, 0);
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_4/blood_brothers/mission05/npc_2".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(111).map_err(debug)?,
                    )),
                },
                std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        talk(owner, admitted, actor, 112, "bye").await?;
        let greeting = talk(owner, admitted, actor, 113, "hi").await?;
        assert!(greeting.iter().any(|line| line.contains("I saw you...")));
        talk(owner, admitted, actor, 114, "a blood crystal please").await?;
        assert_eq!(count(h, 8453).await?, 2);
        for (id, left) in [(115, 1), (118, 0)] {
            if id == 115 {
                let physical = owner.spell_lane_permit().await.ok_or("physical lane")?;
                let mut paid = Box::pin(talk(owner, admitted, actor, id, "blood crystal"));
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(500), paid.as_mut())
                        .await
                        .is_err(),
                    "a non-yes reward must wait for the physical lane"
                );
                assert_eq!(
                    count(h, 8453).await?,
                    2,
                    "held lane preserves both crystals"
                );
                assert_eq!(
                    count(h, 8225).await?,
                    0,
                    "held lane cannot mint a charged crystal"
                );
                drop(physical);
                paid.await?;
            } else {
                talk(owner, admitted, actor, id, "blood crystal").await?;
            }
            assert_eq!(count(h, 8453).await?, left);
            assert_eq!(count(h, 8225).await?, 2 - left);
        }
        talk(owner, admitted, actor, 119, "bye").await?;
        let greeting = talk(owner, admitted, actor, 120, "hi").await?;
        assert!(
            greeting
                .iter()
                .any(|line| line.contains("The pain is slightly less"))
        );
        attempt(
            owner,
            admitted,
            actor,
            121,
            "blood crystal",
            NpcIntentDisposition::Rejected,
        )
        .await?;
        attempt(
            owner,
            admitted,
            actor,
            118,
            "blood crystal",
            NpcIntentDisposition::Ok,
        )
        .await?;
        assert_eq!(
            count(h, 8225).await?,
            2,
            "refusal and exact replay cannot create another crystal"
        );
        let deadlines:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_temporal_schedules p JOIN game_reward_claim_mint_receipts m ON m.transaction_id=p.source_transaction_id WHERE p.definition_production_key='oteryn:item.tibia.i8225' AND p.duration_millis=604800000 AND p.target_production_key='oteryn:item.tibia.i8453' AND p.expires_at_unix_ms=m.occurred_at+604800000")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            deadlines, 2,
            "both paid conversions retain the exact source lifetime and decay target"
        );
        let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/ghostwhisperer'").fetch_one(&h.pool).await?;
        assert_eq!(
            achievements, 1,
            "repeatable conversion earns the achievement once"
        );
    }
    Ok(())
}

async fn attempt(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    actor: EntityRef,
    id: u64,
    text: &str,
    expected: NpcIntentDisposition,
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
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    assert_eq!(outcome.disposition, expected, "{id}: {text}");
    Ok(())
}
