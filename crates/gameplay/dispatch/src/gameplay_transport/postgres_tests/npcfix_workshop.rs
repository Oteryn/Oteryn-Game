//! Source workshop rewards compose paid custody and database-time quest effects.
use super::TestResult;
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use std::sync::Arc;

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
fn npcfix_workshop_postgres_fading_memory_clock_gates_tear() -> TestResult {
    scenario("npcfix-reward-workshop-fading")
}
#[test]
fn npcfix_workshop_postgres_altar_requires_all_costs_atomically() -> TestResult {
    scenario("npcfix-reward-workshop-altar")
}
#[test]
fn npcfix_workshop_postgres_funded_missing_parts_never_charge() -> TestResult {
    scenario("npcfix-reward-workshop-sweaty-missing")
}

#[test]
fn npcfix_workshop_postgres_sweaty_preserves_delayed_craft() -> TestResult {
    scenario("npcfix-reward-workshop-sweaty")
}

#[test]
fn npcfix_workshop_postgres_smithing_exchanges_require_friendship_and_exact_acceptance()
-> TestResult {
    scenario("npcfix-reward-workshop-smithing")
}

#[test]
fn npcfix_workshop_postgres_soul_orbs_trade_all_carried_inputs() -> TestResult {
    scenario("npcfix-reward-workshop-soul-orbs")
}

#[test]
fn npcfix_workshop_postgres_failed_costs_clear_source_confirmation_topics() -> TestResult {
    scenario("npcfix-reward-workshop-refusal-topics")
}

const READY: &str = "oteryn:quest-transition/npcfix-workshop-fixture-ready";
const ELAPSED: &str = "oteryn:quest-transition/npcfix-workshop-fixture-elapsed";

fn install_clock_fixture(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    callbacks: &[RewardCallback],
    elapsed: i64,
) -> TestResult {
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut keys = std::collections::BTreeSet::new();
    for callback in callbacks {
        keys.extend(callback.storage.iter().map(|s| s.track.clone()));
        keys.extend(callback.clock_effects().map(|c| c.track));
    }
    let tracks = keys
        .iter()
        .map(|key| canonical.track(key).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut transitions = callbacks
        .iter()
        .map(|c| canonical.transition(&c.key).cloned().ok_or("transition"))
        .collect::<Result<Vec<_>, _>>()?;
    let quest = callbacks[0].quest.clone();
    transitions.push(QuestTransition {
        key: READY.into(),
        quest: quest.clone(),
        effects: callbacks[0]
            .storage
            .iter()
            .map(|s| QuestEffect {
                track: s.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(0),
            })
            .collect(),
        completes: false,
        experience: None,
    });
    let clock = callbacks
        .iter()
        .flat_map(|c| c.clock_effects())
        .next()
        .ok_or("clock")?
        .track;
    transitions.push(QuestTransition {
        key: ELAPSED.into(),
        quest,
        effects: vec![QuestEffect {
            track: clock,
            from: QuestComparison::Ge(elapsed),
            effect: QuestEffectKind::Add(-elapsed),
        }],
        completes: false,
        experience: None,
    });
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    ));
    Ok(())
}

async fn seed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    ordinal: u64,
) -> TestResult {
    let catalogue = Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "fixture: {outcome:?}"
    );
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

async fn count(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn declined(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    ordinal: u64,
    word: &str,
) -> TestResult {
    let result = super::qengine_rewards::talk(owner, admitted, npc, ordinal, word).await?;
    assert_eq!(
        result,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    eprintln!("POSTGRES_STARTED NPCFIX workshop {name}");
    let npc = match name {
        "npcfix-reward-workshop-fading" => "oteryn:npc.a_fading_memory",
        "npcfix-reward-workshop-altar" => "oteryn:npc.altar",
        _ => "oteryn:npc.a_sweaty_cyclops",
    };
    let callbacks: Vec<_> = oteryn_gameplay_quests::quest::reward_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .filter(|c| c.npc == npc)
        .collect();
    assert!(!callbacks.is_empty());
    let [actor] = install(owner, admitted, [npc]).await?;
    if name.ends_with("refusal-topics") {
        talk(owner, admitted, actor, 100, "hi").await?;
        talk(owner, admitted, actor, 101, "melt").await?;
        declined(owner, admitted, actor, 102, "yes").await?;
        let position = Position {
            x: 33293,
            y: 31677,
            floor: -15,
        };
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position,
                item: "oteryn:item.tibia.i9058".into(),
                count: 1,
            },
            200,
            true,
        )
        .await?;
        super::qengine_rewards::talk(owner, admitted, actor, 103, "yes").await?;
        assert_eq!(
            count(h, 9058).await?,
            1,
            "failed melt must cancel its old confirmation"
        );
        assert_eq!(count(h, 12804).await?, 0);
        talk(owner, admitted, actor, 104, "melt").await?;
        talk(owner, admitted, actor, 105, "yes").await?;
        assert_eq!(count(h, 9058).await?, 0);
        assert_eq!(count(h, 12804).await?, 1);
        talk(owner, admitted, actor, 106, "amulet").await?;
        declined(owner, admitted, actor, 107, "yes").await?;
        let callback = callbacks.iter().find(|c| c.fee.is_some()).ok_or("fee")?;
        for (i, input) in callback.take.iter().enumerate() {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position,
                    item: input.item.clone(),
                    count: input.count,
                },
                300 + u64::try_from(i)?,
                true,
            )
            .await?;
        }
        super::guild_bank::bank_seed::balance(
            h,
            super::fixture::id(40),
            super::fixture::id(41),
            5000,
            150,
        )
        .await?;
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
        super::qengine_rewards::talk(owner, admitted, actor, 108, "yes").await?;
        assert_eq!(
            bank(h).await?,
            5000,
            "failed craft must cancel its old payment confirmation"
        );
        for input in &callback.take {
            let quantity:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(&input.item).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);
        }
        talk(owner, admitted, actor, 109, "amulet").await?;
        talk(owner, admitted, actor, 110, "yes").await?;
        assert_eq!(bank(h).await?, 0);
        assert_eq!(
            count(h, 7532).await?,
            0,
            "reconfirmed craft still retains its24h wait"
        );
        return Ok(());
    }
    if name.ends_with("smithing") || name.ends_with("soul-orbs") {
        talk(owner, admitted, actor, 100, "hi").await?;
        for (i, word) in ["uth'lokr", "yes", "yes"].iter().enumerate() {
            talk(owner, admitted, actor, 101 + u64::try_from(i)?, word).await?;
        }
        talk(owner, admitted, actor, 104, "uth'lokr").await?;
        declined(owner, admitted, actor, 105, "yes").await?;
        let position = Position {
            x: 33293,
            y: 31677,
            floor: -15,
        };
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        for offset in 0..3 {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position,
                    item: "oteryn:item.tibia.i3560".into(),
                    count: 1,
                },
                200 + offset,
                true,
            )
            .await?;
        }
        talk(owner, admitted, actor, 106, "uth'lokr").await?;
        talk(owner, admitted, actor, 107, "yes").await?;
        assert_eq!(count(h, 3560).await?, 0);
        if name.ends_with("soul-orbs") {
            talk(owner, admitted, actor, 108, "soul orb").await?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position,
                    item: "oteryn:item.tibia.i5944".into(),
                    count: 2,
                },
                250,
                true,
            )
            .await?;
            super::qengine_rewards::talk(owner, admitted, actor, 109, "yes please").await?;
            assert_eq!(
                count(h, 5944).await?,
                2,
                "inexact acceptance cannot consume any orb"
            );
            assert_eq!(count(h, 6528).await?, 0);
            talk(owner, admitted, actor, 110, "yes").await?;
            assert_eq!(
                count(h, 5944).await?,
                0,
                "exact acceptance consumes every carried orb"
            );
            let bolts = count(h, 6528).await?;
            assert!(
                [200, 300, 400].contains(&bolts),
                "two independently rolled orb rewards"
            );
            return Ok(());
        }
        for (i, (word, input, output)) in [
            ("uth'kean", 3381, 5887),
            ("uth'lokr", 3416, 5889),
            ("za'ralator", 3356, 5888),
            ("uth'prta", 3281, 5892),
            ("melt", 9058, 12804),
            ("gear whell", 5880, 8775),
        ]
        .into_iter()
        .cycle()
        .take(12)
        .enumerate()
        {
            let ordinal = 300 + u64::try_from(i)? * 10;
            talk(owner, admitted, actor, ordinal, word).await?;
            declined(owner, admitted, actor, ordinal + 1, "yes").await?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position,
                    item: format!("oteryn:item.tibia.i{input}"),
                    count: 1,
                },
                ordinal + 2,
                true,
            )
            .await?;
            talk(owner, admitted, actor, ordinal + 5, word).await?;
            super::qengine_rewards::talk(owner, admitted, actor, ordinal + 3, "yes please").await?;
            assert_eq!(count(h, input).await?, 1, "inexact acceptance keeps input");
            assert_eq!(count(h, output).await?, i64::try_from(i / 6)?);
            talk(owner, admitted, actor, ordinal + 4, "yes").await?;
            assert_eq!(count(h, input).await?, 0);
            assert_eq!(count(h, output).await?, i64::try_from(i / 6 + 1)?);
        }
        let deadlines: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_temporal_schedules p JOIN game_reward_claim_mint_receipts m ON m.transaction_id=p.source_transaction_id WHERE p.definition_production_key='oteryn:item.tibia.i12804' AND p.duration_millis=36000000 AND p.target_production_key IS NULL AND p.expires_at_unix_ms=m.occurred_at+36000000").fetch_one(&h.pool).await?;
        assert_eq!(
            deadlines, 2,
            "both molten gold gifts retain the source ten-hour decay"
        );
        return Ok(());
    }
    if name.ends_with("sweaty-missing") {
        let callback = callbacks.iter().find(|c| c.fee.is_some()).ok_or("fee")?;
        super::guild_bank::bank_seed::balance(
            h,
            super::fixture::id(40),
            super::fixture::id(41),
            5000,
            150,
        )
        .await?;
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
        talk(owner, admitted, actor, 100, "hi").await?;
        talk(owner, admitted, actor, 101, "amulet").await?;
        for (i, input) in callback.take.iter().take(3).enumerate() {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position: Position {
                        x: 33293,
                        y: 31677,
                        floor: -15,
                    },
                    item: input.item.clone(),
                    count: input.count,
                },
                200 + u64::try_from(i)?,
                true,
            )
            .await?;
            talk(owner, admitted, actor, 120 + u64::try_from(i)?, "amulet").await?;
            declined(owner, admitted, actor, 110 + u64::try_from(i)?, "yes").await?;
            assert_eq!(
                bank(h).await?,
                5000,
                "missing fourth part must never burn funded bank gold"
            );
        }
        let fees: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_fees")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(fees, 0);
        assert_eq!(count(h, 7532).await?, 0);
        for input in callback.take.iter().take(3) {
            let quantity:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                .bind(&input.item).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1, "refusal preserves every supplied part");
        }
        return Ok(());
    }
    if name.ends_with("fading") {
        let callback = &callbacks[0];
        let clock = callback.cooldown.as_ref().ok_or("cooldown")?.track.clone();
        install_clock_fixture(owner, &callbacks, 72002)?;
        seed(owner, admitted, READY, 80).await?;
        talk(owner, admitted, actor, 100, "hi").await?;
        for (i, word) in ["kala", "diary", "yes", "vampire", "free soul"]
            .iter()
            .enumerate()
        {
            talk(owner, admitted, actor, 101 + u64::try_from(i)?, word).await?;
        }
        super::qengine_rewards::talk(owner, admitted, actor, 106, "awaken please").await?;
        assert_eq!(count(h, 8746).await?, 0, "grant requires an exact match");
        talk(owner, admitted, actor, 107, "awaken").await?;
        assert_eq!(count(h, 8746).await?, 1);
        let (clock_value,db_now):(i64,i64)=sqlx::query_as("SELECT value,floor(extract(epoch FROM now()))::bigint FROM game_character_quest_tracks WHERE track_key=$1").bind(&clock).fetch_one(&h.pool).await?;
        assert!(
            (db_now - 2..=db_now).contains(&clock_value),
            "clock is database Unix seconds"
        );
        for (i, word) in ["kala", "diary", "yes", "vampire", "free soul"]
            .iter()
            .enumerate()
        {
            talk(owner, admitted, actor, 110 + u64::try_from(i)?, word).await?;
        }
        let waiting = talk(owner, admitted, actor, 115, "awaken").await?;
        assert!(
            waiting
                .iter()
                .any(|line| line.contains("my tears are not ready yet"))
        );
        assert_eq!(
            count(h, 8746).await?,
            1,
            "20-hour cooldown survives fresh command"
        );
        seed(owner, admitted, ELAPSED, 120).await?;
        for (i, word) in ["kala", "diary", "yes", "vampire", "free soul"]
            .iter()
            .enumerate()
        {
            talk(owner, admitted, actor, 121 + u64::try_from(i)?, word).await?;
        }
        talk(owner, admitted, actor, 126, "awaken").await?;
        assert_eq!(count(h, 8746).await?, 2);
        return Ok(());
    }
    if name.ends_with("sweaty") {
        let amulet: Vec<_> = callbacks
            .iter()
            .filter(|c| c.quest == "oteryn:quest.koshei_the_deathless_quest")
            .cloned()
            .collect();
        install_clock_fixture(owner, &amulet, 86402)?;
    }
    let callback = callbacks
        .iter()
        .find(|c| c.fee.is_some())
        .ok_or("fee callback")?
        .clone();
    let at = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    talk(owner, admitted, actor, 100, "hi").await?;
    let words: &[&str] = if name.ends_with("altar") {
        &["kneel", "offer"]
    } else {
        &["amulet"]
    };
    for (i, word) in words.iter().enumerate() {
        talk(owner, admitted, actor, 101 + u64::try_from(i)?, word).await?;
    }
    declined(owner, admitted, actor, 110, "yes").await?;
    for (i, input) in callback.take.iter().enumerate() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: at,
                item: input.item.clone(),
                count: input.count,
            },
            200 + u64::try_from(i)?,
            true,
        )
        .await?;
    }
    for (i, word) in words.iter().enumerate() {
        talk(owner, admitted, actor, 300 + u64::try_from(i)?, word).await?;
    }
    declined(owner, admitted, actor, 111, "yes").await?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        receipts, 0,
        "missing money cannot consume inputs or complete a craft"
    );
    let fee = i64::try_from(callback.fee.as_ref().ok_or("fee")?.gold)?;
    super::guild_bank::bank_seed::balance(
        h,
        super::fixture::id(40),
        super::fixture::id(41),
        fee,
        150,
    )
    .await?;
    for (i, word) in words.iter().enumerate() {
        talk(owner, admitted, actor, 310 + u64::try_from(i)?, word).await?;
    }
    declined(owner, admitted, actor, 112, "yes").await?;
    assert_eq!(
        bank(h).await?,
        fee,
        "V1 must preserve the existing bank-fee boundary"
    );
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&h.pool)
        .await?;
    talk(owner, admitted, actor, 319, "yes").await?;
    assert_eq!(
        bank(h).await?,
        fee,
        "a failed offer needs a fresh confirmation topic"
    );
    assert_eq!(
        count(h, if name.ends_with("altar") { 43895 } else { 7532 }).await?,
        0
    );
    for (i, word) in words.iter().enumerate() {
        talk(owner, admitted, actor, 320 + u64::try_from(i)?, word).await?;
    }
    super::qengine_rewards::talk(owner, admitted, actor, 113, "yes please").await?;
    assert_eq!(bank(h).await?, fee, "exact confirmation required");
    talk(owner, admitted, actor, 114, "yes").await?;
    assert_eq!(bank(h).await?, 0);
    let fees: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_fees")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(fees, 1);
    if name.ends_with("altar") {
        assert_eq!(count(h, 43895).await?, 1);
        talk(owner, admitted, actor, 115, "kneel").await?;
        talk(owner, admitted, actor, 116, "offer").await?;
        declined(owner, admitted, actor, 117, "yes").await?;
        assert_eq!(
            count(h, 43895).await?,
            1,
            "repeat requires fresh full costs"
        );
    } else {
        assert_eq!(
            count(h, 7532).await?,
            0,
            "paid hand-in does not finish the one-day craft"
        );
        let waiting = talk(owner, admitted, actor, 115, "amulet").await?;
        assert!(waiting.iter().any(
            |line| line.starts_with("Big Ben still hammering! Come back in ")
                && line.ends_with('!')
        ));
        assert_eq!(count(h, 7532).await?, 0);
        seed(owner, admitted, ELAPSED, 120).await?;
        talk(owner, admitted, actor, 121, "amulet").await?;
        assert_eq!(count(h, 7532).await?, 1);
        talk(owner, admitted, actor, 122, "amulet").await?;
        assert_eq!(
            count(h, 7532).await?,
            1,
            "completed craft cannot grant again"
        );
    }
    Ok(())
}

async fn bank(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT balance FROM game_account_bank_balances")
            .fetch_one(&h.pool)
            .await?,
    )
}

#[test]
fn npcfix_workshop_postgres_native_fading_memory() -> TestResult {
    super::npcfix_physical_endpoints::threaded_anchor((
        "oteryn:npc.a_fading_memory",
        "A Fading Memory",
        32952,
        31440,
        -3,
    ))
}
#[test]
fn npcfix_workshop_postgres_native_sweaty_cyclops() -> TestResult {
    super::npcfix_physical_endpoints::threaded_anchor((
        "oteryn:npc.a_sweaty_cyclops",
        "A Sweaty Cyclops",
        32699,
        31674,
        -3,
    ))
}
#[test]
fn npcfix_workshop_postgres_native_altar() -> TestResult {
    super::npcfix_physical_endpoints::threaded_anchor((
        "oteryn:npc.altar",
        "Altar",
        34070,
        32009,
        -14,
    ))
}
