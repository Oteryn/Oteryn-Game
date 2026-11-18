//! Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::rotten_blood::putrefactory::{
    CHARGE_TRACK as CHARGE, PASSIVE_CLOCK, PASSIVE_TICK, RESERVOIR_ACTION,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use serde_json::json;
use std::sync::Arc;

pub(super) const LEGACY: &str = "putrefactory-reservoir-legacy";
pub(super) const CAP: &str = "putrefactory-reservoir-cap";
pub(super) const PASSIVE: &str = "putrefactory-passive-charge";
const QUEST: &str = "oteryn:quest/rotten-blood";
const CLOCK: &str = "oteryn:quest-progress/rotten-blood/putrefactory/reservoir-1-used-at";
const STARTED_AT: i64 = 1_800_000_000;
const SAVED_CHARGE: i64 = 12;
// Crystal creaturescripts_putrefactory-x.lua:195-199.
const EXHAUSTED: &str = "You already exhausted this reservoir.";
const AT: Position = Position {
    x: 34081,
    y: 31704,
    floor: -14,
};

#[test]
fn putrefactory_reservoir_recharges_unset_counter_and_cools_down_on_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(LEGACY, || scenario(LEGACY, AT, false))
}

#[test]
fn putrefactory_reservoir_caps_charge_without_restarting_clock_when_full() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(CAP, || scenario(CAP, AT, false))
}

#[test]
fn putrefactory_passive_charge_ticks_from_the_channel_clock() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(PASSIVE, || scenario(PASSIVE, AT, false))
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/fixture/putrefactory-reservoir-1",
        "quest":QUEST,
        "trigger":"use",
        "actor_kind":"player",
        "selector":{"position":AT,"action":RESERVOIR_ACTION},
        "passages":[],
        "storage":[],
        "offerings":[],
        "return_on_failure":false,
        "reversible":false,
        "requires_world_admission":true,
        "sources":["Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    name: &str,
) -> TestResult {
    let cap = name == CAP;
    let mut reservoir = serde_json::to_value(callback()?)?;
    reservoir["effects"]["magic"] = json!({"position":AT,"effect":241});
    reservoir["effects"]["recharge_cooldown_message"] = json!(EXHAUSTED);
    reservoir["state_operations"] = json!([
        {"track":CLOCK,"from":{"kind":"elapsed_at_least","seconds":20},
            "effect":{"kind":"set_now"}},
        {"track":CHARGE,"from":{"kind":"between","minimum":-1,"maximum":99},
            "effect":{"kind":"recharge","amount":20,"empty":0,"capacity":100}}
    ]);
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":["Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224"],
            "tracks":[
                {"key":CHARGE,"initial":if cap { 93 } else { -1 },"minimum":-1,"maximum":2147483647},
                {"key":CLOCK,"initial":-1,"minimum":-1,"maximum":i64::MAX}
            ],
            "stages":[{"key":"reservoir-1","completes":false,
                "trigger":{"kind":"map_use","callback":reservoir}}]
        }]
    }))?)?;
    let base = if name == PASSIVE {
        let active = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
        QuestStateCatalogue::new(
            "content-1",
            [CHARGE, PASSIVE_CLOCK]
                .into_iter()
                .map(|key| active.track(key).cloned().ok_or("passive track"))
                .collect::<Result<Vec<_>, _>>()?,
            vec![active.transition(PASSIVE_TICK).cloned().ok_or("passive transition")?],
        ).map_err(debug)?
    } else {
        QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?
    };
    let attached = bindings.attach(&base)?;
    if let Some(previous) = base.definition_hash(QUEST) {
        assert_ne!(attached.definition_hash(QUEST), Some(previous));
        assert!(oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(&attached, QUEST, &previous).is_some());
        seed_previous_charge(h, previous).await?;
    }
    owner.quest_catalogue = Some(Arc::new(attached));
    h.root.set_test_quest_clock(STARTED_AT).map_err(debug)?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, true).await;
    owner.states.chat.admit(session, "Reservoir fixture", owner.owner_now());
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("reservoir placement")?;
    let mut placement = owner.bundle_map().ok_or("map")?.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let tracks = || async {
        sqlx::query_as::<_, (String, i64)>(
            "SELECT track_key,value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key",
        )
        .bind(QUEST)
        .fetch_all(&h.pool)
        .await
    };
    let reply = wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0,
        "a personal recharge must leave the shared reservoir usable at the same revision",
    );
    let values = tracks().await?;
    assert_eq!(values.len(), 2);
    let first_charge = match name {
        CAP => 100,
        PASSIVE => SAVED_CHARGE + 20,
        _ => 20,
    };
    assert_eq!(values[0], (CHARGE.to_owned(), first_charge));
    assert_eq!(values[1].0, CLOCK);
    assert_eq!(values[1].1, STARTED_AT);
    let view = owner.observe_channel_entities(actor).await.ok_or("entity view")?;
    let entry = view
        .entities
        .iter()
        .find(|entry| entry.identity == actor.placement_identity())
        .ok_or("player entity")?;
    assert!(entry.source_icons.iter().any(|icon| {
        icon.category == 0 && icon.id == 2 && i64::from(icon.count) == first_charge
    }));

    if name == PASSIVE {
        h.root.advance_test_quest_clock(19).map_err(debug)?;
        owner.tick_timed_items_at(19_000).await;
        assert_eq!(tracks().await?, values);
        h.root.advance_test_quest_clock(1).map_err(debug)?;
        owner.tick_timed_items_at(20_100).await;
        let charged = tracks().await?;
        assert_eq!(
            charged,
            vec![
                (CHARGE.to_owned(), first_charge + 1),
                (PASSIVE_CLOCK.to_owned(), STARTED_AT + 20),
                (CLOCK.to_owned(), STARTED_AT),
            ]
        );
        // A delayed pulse re-arms from 20.1 seconds, not the start of its bucket.
        h.root.advance_test_quest_clock(20).map_err(debug)?;
        owner.tick_timed_items_at(40_000).await;
        assert_eq!(tracks().await?, charged);
        // The containing item clock checks once per second.
        owner.tick_timed_items_at(41_000).await;
        assert_eq!(tracks().await?[0], (CHARGE.to_owned(), first_charge + 2));
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1 AND cause_kind='source_think'",
        )
        .bind(PASSIVE_TICK)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 2);
        return Ok(());
    }

    assert_eq!(wire(owner, admitted, 1, 2, payload.clone()).await?, reply);
    h.root.advance_test_quest_clock(19).map_err(debug)?;
    let reply = wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(tracks().await?, values, "refused use and replay preserve both tracks");
    assert_eq!(
        owner.states.chat.drain(session),
        vec![oteryn_protocol_oteryn::chat::ChatLine::Private {
            speaker_name: "Game".into(),
            text: EXHAUSTED.into(),
        }]
    );
    h.root.advance_test_quest_clock(1).map_err(debug)?;
    let reply = wire(owner, admitted, 3, 2, payload).await?;
    let disposition = decode_use_result(&reply).map_err(debug)?;
    assert!(owner.states.chat.drain(session).is_empty());
    if cap {
        assert_eq!(disposition, UseDisposition::Rejected);
        assert_eq!(tracks().await?, values, "full charge leaves the expired clock alone");
    } else {
        assert_eq!(disposition, UseDisposition::Committed);
        assert_eq!(
            tracks().await?,
            vec![(CHARGE.to_owned(), 40), (CLOCK.to_owned(), STARTED_AT + 20)]
        );
    }
    Ok(())
}

async fn seed_previous_charge(h: &Harness, definition_hash: [u8; 32]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let character = super::fixture::id(41);
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id, quest_key, pinned_content_revision,
         definition_hash, started_character_revision, committed_character_revision)
         SELECT character_id, $2, 'content-1', $3, character_revision, character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .bind(QUEST)
    .bind(definition_hash.as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id, track_key, quest_key, value,
         committed_character_revision)
         SELECT character_id, $2, $3, $4, character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .bind(CHARGE)
    .bind(QUEST)
    .bind(SAVED_CHARGE)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
