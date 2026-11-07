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
        install_clock_fixture(owner, &callbacks, 86402)?;
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
    declined(owner, admitted, actor, 112, "yes").await?;
    assert_eq!(
        bank(h).await?,
        fee,
        "V1 must preserve the existing bank-fee boundary"
    );
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&h.pool)
        .await?;
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
