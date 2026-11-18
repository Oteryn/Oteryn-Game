//! Fenced PostgreSQL USE and STEP through the world callback dispatcher.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data-global/scripts/quests:
//! the_pits_of_inferno_quest/actions_fountain.lua:3-15;
//! lions_rock/actions_lions_rock.lua:115-139;
//! the_first_dragon/actions_rewards.lua:43-72;
//! the_great_dragon_hunt_quest/actions_treasure.lua:19-40 and
//! the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua:12-25.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings,
    world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) const PREFIX: &str = "qengine-rewards-qonce-";
const WELL: &str = "qengine-rewards-qonce-well";
const WELL_LOST_ACK: &str = "qengine-rewards-qonce-well-lost-ack";
const DAILY: &str = "qengine-rewards-qonce-daily";
const ANNUAL: &str = "qengine-rewards-qonce-annual";
const HOARD: &str = "qengine-rewards-qonce-hoard";
const STEP: &str = "qengine-rewards-qonce-step";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const CLAIM: &str = "oteryn:quest-callback/qonce-dispatch-claim";
const LEDGER: &str = "oteryn:quest-progress/qonce-dispatch/ledger";
const CHOICE: &str = "oteryn:quest-progress/qonce-dispatch/choice";
const HUNT_GATE: &str = "oteryn:quest-progress/qonce-dispatch/hunt-gate";
const HUNT_KILL: &str = "oteryn:quest-transition/native-kills/qonce-hoard-dragon";
const DRAGON: &str = "oteryn:creature.dragon";
const COUNT: &str = "oteryn:quest-progress/qonce-dispatch/count";
const WELL_ACHIEVEMENT: &str = "oteryn:achievement/fountain_of_life";
const HOARD_ACHIEVEMENT: &str = "oteryn:achievement/hoard_of_the_dragon";
const DAILY_REFUSAL: &str = "At the moment there is neither a treasure nor anything else in the fountain. Perhaps you might return later.";
const ANNUAL_REFUSAL: &str = "The chest is empty.";
const HOARD_REFUSAL: &str = "You try to pick a treasure, but you hear further dragons approaching. You should kill some more before picking out something.";
const AT: Position = Position { x: 33294, y: 31677, floor: -15 };

#[test]
fn once_only_well_restores_saved_and_live_vitals_and_grants_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL)
}

#[test]
fn daily_claim_serializes_double_use_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(DAILY)
}

#[test]
fn annual_claim_delivers_backpack_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(ANNUAL)
}

#[test]
fn hoard_bundle_resets_eligibility_and_awards_tenth_claim_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(HOARD)
}

#[test]
fn interrupted_well_use_recovers_committed_heal_without_second_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL_LOST_ACK)
}

#[test]
fn step_claim_credits_the_same_achievement_only_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(STEP)
}

fn storage(track: &str, minimum: i64, maximum: i64, set: i64) -> Value {
    json!({"track": track, "minimum": minimum, "maximum": maximum, "set": set})
}

fn track(key: &str, initial: i64, minimum: i64, maximum: i64) -> Value {
    json!({"key": key, "initial": initial, "minimum": minimum, "maximum": maximum})
}

async fn track_value(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn achievements(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(CLAIM).fetch_one(&h.pool).await?)
}

fn advance_interaction(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1))
        .ok_or("world interaction clock")?;
    owner.states.world_gate.lock().map_err(|_| "world interaction gate")?.begin_turn();
    Ok(())
}

fn claim_target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("claim map")?;
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("claim placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) })
}

async fn use_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    let map = owner.bundle_map().ok_or("claim map")?;
    let target = claim_target(owner)?;
    Ok(owner.use_quest_callback(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        map,
        &target,
    ).await.ok_or("claim callback did not dispatch")?.disposition)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let mut callback = serde_json::to_value(super::authored_world_rewards::placeholder(AT)?)?;
    callback["key"] = json!(CLAIM);
    callback["requires_world_admission"] = json!(true);
    let well = matches!(name, WELL | WELL_LOST_ACK);
    let mut tracks = vec![];
    let mut effects;
    let period = match name {
        DAILY => Some((86_401_i64, DAILY_REFUSAL)),
        ANNUAL => Some((31_536_001_i64, ANNUAL_REFUSAL)),
        _ => None,
    };
    if let Some((seconds, refusal)) = period {
        tracks.push(track(LEDGER, -1, -1, i64::MAX));
        tracks.push(track(CHOICE, 0, 0, 1));
        callback["storage"] = json!([storage(CHOICE, 0, 1, 0)]);
        callback["state_operations"] = json!([{
            "track": LEDGER, "from": {"kind":"elapsed_at_least","seconds":seconds},
            "effect":{"kind":"set_now"}
        }]);
        callback["failure_message"] = json!(refusal);
        let give = if name == ANNUAL {
            json!({"item":"oteryn:item.tibia.i2854","count":1,
                "contents":[{"item":"oteryn:item.tibia.i3031","count":2}]})
        } else {
            json!({"item":"oteryn:item.tibia.i3031","count":2})
        };
        effects = json!({"reset":{"kind":"elapsed","track":LEDGER,"seconds":seconds},
            "selection_track":CHOICE,"rewards":[{"give":give}]});
    } else if name == HOARD {
        tracks.extend([
            track(LEDGER, 49, 0, 50), track(COUNT, 9, 0, 100),
            track(CHOICE, 0, 0, 2), track(HUNT_GATE, 0, 0, 0),
        ]);
        let mut count = storage(COUNT, 0, 99, 1);
        count["increment"] = json!(true);
        callback["storage"] = json!([storage(LEDGER, 50, 50, 0), count, storage(CHOICE, 0, 2, 0)]);
        callback["failure_message"] = json!(HOARD_REFUSAL);
        effects = json!({"reset":{"kind":"progress","track":LEDGER,"minimum":50},
            "selection_track":CHOICE,
            "milestone":{"track":COUNT,"count":10,"achievement":HOARD_ACHIEVEMENT},
            "rewards":[
                {"give":{"item":"oteryn:item.tibia.i3031","count":2,"additional":[{"item":"oteryn:item.tibia.i3039","count":1}]}},
                {"give":{"item":"oteryn:item.tibia.i3031","count":2,"additional":[{"item":"oteryn:item.tibia.i3039","count":2}]}}
            ]});
    } else {
        assert!(well || name == STEP);
        tracks.push(track(LEDGER, 0, 0, 1));
        callback["storage"] = json!([storage(LEDGER, 0, 0, 1)]);
        callback["achievements"] = json!([WELL_ACHIEVEMENT]);
        callback["failure_message"] = json!("Sorry, not possible.");
        effects = json!({"reset":{"kind":"once","track":LEDGER}});
        if well {
            effects["restore_vitals"] = json!(true);
        } else {
            callback["trigger"] = json!("step_in");
            callback["selector"]["unique"] = Value::Null;
            callback["selector"]["appearance"] = Value::Null;
        }
    }
    callback["effects"]["claim"] = effects;
    let recipe_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&callback)?));
    let trigger = if name == STEP { "step_in" } else { "map_use" };
    let mut stages = vec![json!({
        "key":"claim","completes":false,"trigger":{"kind":trigger,"callback":callback}
    })];
    if name == HOARD {
        stages.push(json!({
            "key":"kill-dragon","completes":false,"trigger":{"kind":"kill","counter":{
                "key":HUNT_KILL,"creature":DRAGON,"track":LEDGER,
                "gate":HUNT_GATE,"gate_value":0,"maximum":50,
                "death_area":{"minimum":[AT.x-2, AT.y-2, -AT.floor],
                              "maximum":[AT.x+2, AT.y+2, -AT.floor]}
            }}
        }));
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":recipe_sha256,
        "sources":["synthetic dispatch tests exercising the Crystal claim shapes"],
        "tracks":tracks,
        "stages":stages
    }]}))?)?;
    let revision = owner.quest_catalogue.as_deref().ok_or("catalogue")?.content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner.states.chat.admit(admitted.game_session_id, "Claimant", owner.owner_now());

    if name == STEP {
        for (command, direction) in [(1, StepDirection::East), (2, StepDirection::West), (3, StepDirection::East)] {
            let reply = super::quest_world_callbacks::wire(owner, admitted, command, 1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction)).await?;
            assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        }
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    if well {
        let actor = admitted.runtime_actor.ok_or("well actor")?;
        let session = admitted.game_session_id;
        let full = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("well vitals")?;
            let full = state.vitals();
            assert!(full.health > 1 && full.mana > 0);
            state.restore_saved_vitals(1, 0);
            (full.health, full.mana)
        };
        if name == WELL_LOST_ACK {
            let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
                &claim_target(owner)?,
            ).map_err(debug)?;
            owner.states.quest_callbacks.interrupt_after_commit
                .store(true, std::sync::atomic::Ordering::Release);
            assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await.is_err());
            assert!(owner.states.quest_callbacks.has_pending_claim_vitals(session));
            assert_eq!(receipts(h).await?, 1);
            assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
            let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
                .fetch_one(&h.pool).await?;
            assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let vitals = states.get(&runtime, actor, session).ok_or("interrupted actor")?.vitals();
                assert_eq!((vitals.health, vitals.mana), (1, 0));
            }
            assert!(owner.reconcile_quest_callbacks(session).await);
            assert!(!owner.states.quest_callbacks.has_pending_claim_vitals(session));
        } else {
            assert_eq!(use_claim(owner, *admitted, 1).await?, UseDisposition::Committed);
        }
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("healed actor")?;
            let vitals = state.vitals();
            assert_eq!((vitals.health, vitals.mana), full);
            state.restore_saved_vitals(1, 0);
        }
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 2).await?, UseDisposition::Rejected);
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states.get(&runtime, actor, session).ok_or("refused actor")?.vitals();
            assert_eq!((vitals.health, vitals.mana), (1, 0));
        }
        let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
            .fetch_one(&h.pool).await?;
        assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    h.root.set_test_quest_clock(100_000_000).map_err(debug)?;
    if name == HOARD {
        owner.active_generation.and_then(|generation| generation.native_gameplay())
            .ok_or("native creature policies")?
            .install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
        super::qengine_kill_stories::death(owner, admitted, DRAGON, 88_001).await?;
        assert_eq!(track_value(h, LEDGER).await?, 50, "the death opens the hoard");
    }
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    let before_red_gems = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let (first, second) = tokio::join!(use_claim(owner, *admitted, 1), use_claim(owner, *admitted, 2));
    let committed = [first?, second?].into_iter().filter(|outcome| *outcome == UseDisposition::Committed).count();
    assert_eq!(committed, 1);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 2);
    advance_interaction(owner)?;
    assert_eq!(use_claim(owner, *admitted, 3).await?, UseDisposition::Rejected);
    let refusal = period.map_or(HOARD_REFUSAL, |(_, refusal)| refusal);
    let chat = owner.states.chat.drain(admitted.game_session_id);
    assert!(chat.iter().any(|line| matches!(
        line,
        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == refusal
    )));
    if let Some((seconds, _)) = period {
        h.root.advance_test_quest_clock(seconds - 1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 4).await?, UseDisposition::Rejected);
        h.root.advance_test_quest_clock(1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 5).await?, UseDisposition::Committed);
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 6).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 2);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 4);
    } else {
        assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 1);
        let (ledger, count): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1), \
                    (SELECT value FROM game_character_quest_tracks WHERE track_key=$2)",
        ).bind(LEDGER).bind(COUNT).fetch_one(&h.pool).await?;
        assert_eq!((ledger, count), (0, 10));
        assert!((1..=2).contains(&(super::qengine_voucher_stories::quantity(h, 3039).await? - before_red_gems)));
        super::qengine_kill_stories::death(owner, admitted, DRAGON, 88_002).await?;
        assert_eq!(track_value(h, LEDGER).await?, 1, "the next hunt can earn credit again");
        let deaths: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        ).bind(HUNT_KILL).fetch_one(&h.pool).await?;
        assert_eq!(deaths, 2, "duplicate death delivery earns one credit per death");
    }
    let atomic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts m \
         JOIN game_character_quest_receipts q ON q.character_id=m.character_id \
          AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id \
          AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1",
    ).bind(CLAIM).fetch_one(&h.pool).await?;
    assert_eq!(atomic, receipts(h).await?);
    Ok(())
}
