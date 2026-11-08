//! Shared source switch activation uses literal USE, without invented Quest progress.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
#[test]
fn l6_shared_switch_12121_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12121")
}
#[test]
fn l6_shared_switch_12122_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12122")
}
#[test]
fn l6_shared_switch_12123_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12123")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "shared switch fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_shared_switch version={version}");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("switch placement")?;
    let map = owner.bundle_map().ok_or("switch map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed,
        "exact source switch must commit its world lease"
    );
    let replay = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
    assert_eq!(reply, replay);
    let rows:Vec<(i64,bool,bool)>=sqlx::query_as("SELECT expires_at_unix_ms-occurred_at_unix_ms,r.created_xact_id=a.created_xact_id,a.envelope_sha256=sha256(a.envelope) FROM game_shared_switch_leases r JOIN game_shared_switch_audit a USING(transaction_id,event_id)").fetch_all(&h.pool).await?;
    assert_eq!(
        rows,
        vec![(1_200_000, true, true)],
        "one source20min lease/audit after original wire replay"
    );
    // The transport duplicate above is cached. Replay the durable owner itself
    // after its source lever projection has advanced, preserving the original due.
    let (transaction,recipe,placement,revision,context,ordinal,expires):(Vec<u8>,String,Vec<u8>,String,serde_json::Value,String,i64)=sqlx::query_as("SELECT uuid_send(transaction_id),recipe,placement,object_revision::text,context,cause_ordinal::text,expires_at_unix_ms FROM game_shared_switch_leases").fetch_one(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "shared replay fence")?
        .ok_or("shared replay fence")?;
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("shared replay lane")?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(ordinal.parse()?).map_err(debug)?,
    );
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("shared replay native")?;
    let request = crate::durability::quest_world_callback::shared_switch::Request::new(
        recipe.clone(),
        placement,
        revision.parse()?,
        serde_json::from_value(context)?,
    );
    let durable = owner
        .root
        .activate_shared_switch(
            &permit,
            owner.character,
            owner.holder,
            fence,
            command,
            native,
            request.clone(),
        )
        .await?
        .ok_or("original shared lease replay")?;
    assert!(durable.replayed);
    assert_eq!(durable.transaction_id.as_slice(), transaction.as_slice());
    assert_eq!(
        durable.expires_at_unix_ms, expires,
        "replay never renews the source20min deadline"
    );
    assert_eq!(durable.active_count, 1);
    let mut wrong = request;
    wrong.recipe = oteryn_gameplay_quests::quest::shared_switches::RECIPES
        .iter()
        .find(|r| r.key != recipe)
        .ok_or("other source switch")?
        .key
        .into();
    assert!(
        owner
            .root
            .activate_shared_switch(
                &permit,
                owner.character,
                owner.holder,
                fence,
                command,
                native,
                wrong
            )
            .await?
            .is_none(),
        "same cause with a different source intent cannot count a switch"
    );
    let final_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(final_count, 1);
    eprintln!("SHARED_SWITCH_DURABLE_REPLAY_VERIFIED {recipe} original_due={expires}");
    drop(permit);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "shared world state must not invent personal quest progress"
    );
    Ok(())
}
