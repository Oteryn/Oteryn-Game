//! Literal UID2241 on synthetic source geometry; real STEP and fenced carried inputs.
//! This qualifies the owner, not a recovered production-map UID correspondence.
use super::TestResult;
use oteryn_gameplay_quests::quest::{dreamer_carrot, world_callbacks::Position};

#[test]
fn m07_carrot_premium_entry_consumes_exactly_one_and_replays() -> TestResult {
    trial("m07-carrot-premium")
}
#[test]
fn m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal() -> TestResult {
    trial("m07-carrot-free")
}
fn trial(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-carrot", move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 100,
                y: 100,
                floor: -9,
            },
            false,
        )
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the common scenario supplies the admitted actor and all current fences"
)]
pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    origin: crate::foundation::MovementLocalPosition,
    premium: bool,
) -> TestResult {
    use super::fixture::debug;
    use oteryn_protocol_oteryn::{
        chat::{ChatLine, ChatSpeechMode},
        world_spatial::{StepDirection, StepDisposition, decode_step_result, encode_step_intent},
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Dreamer", owner.owner_now());
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let hp = health(owner, admitted).await?;
    let payload = encode_step_intent(StepDirection::East);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    let missing_hp = health(owner, admitted).await?;
    assert!(
        hp > missing_hp && hp - missing_hp <= 20,
        "native mitigation may reduce the source 10..20 fire roll"
    );
    assert_eq!(counts(h).await?, (0, 0, 0));
    assert!(!owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
        matches!(line,ChatLine::Local{text,..} if text == dreamer_carrot::PREMIUM_SPEECH)),"no carrot is checked before Premium speech");
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        health(owner, admitted).await?,
        missing_hp,
        "replay must not punish twice"
    );
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 100,
                y: 100,
                floor: -9,
            },
            item: dreamer_carrot::ITEM.into(),
            count: 2,
        },
        60,
        true,
    )
    .await?;
    let before_second = health(owner, admitted).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    if premium {
        assert_eq!(
            position,
            super::quest_world_callbacks::local(dreamer_carrot::DESTINATION)
        );
        assert_eq!(health(owner, admitted).await?, before_second);
        assert_eq!(counts(h).await?, (1, 1, 0));
        let consumed:(i64,i64,bool)=sqlx::query_as("SELECT i.quantity,i.lifecycle::bigint,c.proof_sha256=sha256(c.proof) FROM game_quest_callback_consumptions c JOIN game_item_instances i USING(item_instance_id) WHERE c.transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(consumed, (1, 1, true));
        let shape:(i32,bool)=sqlx::query_as("SELECT cardinality(track_keys),completes FROM game_character_quest_receipts WHERE transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(shape, (0, false));
    } else {
        assert_eq!(position, origin);
        let after = health(owner, admitted).await?;
        assert!(before_second > after && before_second - after <= 20);
        assert_eq!(counts(h).await?, (0, 0, 0));
        let quantity: i64 = sqlx::query_scalar(
            "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1",
        )
        .bind(dreamer_carrot::ITEM)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(quantity, 2);
    }
    let speech=owner.states.chat.drain(admitted.game_session_id).into_iter().filter(|line|
        matches!(line,ChatLine::Local{mode:ChatSpeechMode::MonsterSay,text,..} if text == dreamer_carrot::PREMIUM_SPEECH)).count();
    assert_eq!(speech, usize::from(!premium));
    let hp = health(owner, admitted).await?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 1, payload).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(health(owner, admitted).await?, hp);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        position
    );
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    eprintln!(
        "M07 POSTGRES_CASE_COMPLETED carrot real_STEP premium={premium} missing_before_premium carried_atomic_debit origin_fire replay_no_extra_effects no_progress"
    );
    Ok(())
}
async fn counts(h: &super::fixture::Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_character_quest_tracks)").fetch_one(&h.pool).await?)
}
async fn health(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
) -> TestResult<u32> {
    let probe = async {
        loop {
            let runtime = owner.runtime.lock().await;
            if let Ok(states) = owner.states.spell_states.try_lock() {
                return states
                    .get(
                        &runtime,
                        admitted.runtime_actor.ok_or("actor")?,
                        admitted.game_session_id,
                    )
                    .map(|state| state.vitals().health)
                    .ok_or_else(|| "health state".into());
            }
            drop(runtime);
            drop(owner.states.spell_states.lock().await);
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), probe)
        .await
        .map_err(|_| "health probe timeout")?
}
