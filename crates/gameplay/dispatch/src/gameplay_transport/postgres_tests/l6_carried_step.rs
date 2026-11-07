//! Exact carried-key absence selects the source post-boss fallback through real STEP.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_tomb_carried_missing_3950_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3950",
        Position {
            x: 33231,
            y: 32705,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_carried_missing_3951_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3951",
        Position {
            x: 33282,
            y: 32744,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_carried_missing_3952_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3952",
        Position {
            x: 33250,
            y: 32832,
            floor: -8,
        },
    )
}

#[test]
fn l6_tomb_carried_present_3950_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3950",
        Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_carried_present_3951_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3951",
        Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
    )
}
#[test]
fn l6_tomb_carried_present_3952_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3952",
        Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
    )
}

pub(super) async fn verify(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    consumed: bool,
    native_portal: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(super::fixture::debug)?;
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(if native_portal {
            StepDirection::South
        } else {
            StepDirection::East
        }),
    )
    .await?;
    assert_eq!(
        decode_step_result(&replay).map_err(super::fixture::debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?,
        position,
        "duplicate original STEP must not move twice"
    );
    // Exercise the durable replay itself after the input has been retired and
    // the actor has left the source tile, beyond the transport's result cache.
    let (transaction,recipe,placement,revision,context,ordinal):(Vec<u8>,String,Vec<u8>,String,serde_json::Value,String)=
        sqlx::query_as("SELECT uuid_send(transaction_id),recipe,placement,object_revision::text,context,cause_ordinal::text FROM game_carried_step_receipts")
        .fetch_one(&h.pool).await?;
    let permit = owner.spell_lane_permit().await.ok_or("replay lane")?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    let durable = owner
        .root
        .commit_carried_step(
            &permit,
            owner.character,
            owner.holder,
            fence,
            crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(ordinal.parse()?)?,
            ),
            owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native replay")?,
            crate::durability::quest_world_callback::carried_step::Request {
                recipe,
                placement,
                object_revision: revision.parse()?,
                context: serde_json::from_value(context)?,
                success_prepared: false,
                missing_prepared: false,
            },
        )
        .await?
        .ok_or("original durable branch replay")?;
    assert!(durable.replayed);
    assert_eq!(durable.transaction_id.as_slice(), transaction.as_slice());
    assert_eq!(
        durable.consumed, consumed,
        "retired input cannot change the historical branch"
    );
    drop(permit);

    let rows:Vec<(bool,bool,bool,bool)> = sqlx::query_as("SELECT r.input_before IS NOT NULL,r.created_xact_id=a.created_xact_id,a.envelope_sha256=sha256(a.envelope),convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(r),'recipe',to_jsonb(p)) FROM game_carried_step_receipts r JOIN game_carried_step_audit a USING(transaction_id,event_id) JOIN game_carried_step_recipes p USING(recipe)").fetch_all(&h.pool).await?;
    assert_eq!(
        rows,
        vec![(consumed, true, true, true)],
        "one exact branch/audit, no replay duplication"
    );
    let balance:bool=sqlx::query_scalar("SELECT CASE WHEN r.input_before IS NULL THEN NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.last_transaction_id=r.transaction_id) ELSE EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_audit_outbox a ON a.transaction_id=r.transaction_id AND a.item_instance_id=i.item_instance_id WHERE i.item_instance_id=(r.input_before->>'item_instance_id')::uuid AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id AND a.envelope_sha256=r.item_envelope_sha256 AND a.created_xact_id=r.created_xact_id) END FROM game_carried_step_receipts r").fetch_one(&h.pool).await?;
    assert!(
        balance,
        "same-XID physical debit or proven no Item mutation"
    );
    Ok(())
}

fn run(name: &'static str, fallback: Position) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::stateless_scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                fallback,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "carried STEP scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
