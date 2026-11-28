use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::ExactActorRef;

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
    let replay = super::quest_world_callbacks_fixtures::wire(
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
                success: None,
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

pub(super) async fn control_loss(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    destination: oteryn_gameplay_quests::quest::world_callbacks::Position,
) -> TestResult {
    use crate::gameplay_transport::connection::{ControlLossResult, GraceExpiryResult};
    use crate::gameplay_transport::last_position::AdmissionPosition;
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    use super::quest_world_callbacks_fixtures::{local, wire};

    let session = admitted.game_session_id;
    let controller = admitted.controller.ok_or("carried STEP controller")?;
    assert!(owner.track_restored(session, actor, &AdmissionPosition::FirstLogin, fence));
    owner.states.quest_callbacks.interrupt_after_commit.store(
        true, std::sync::atomic::Ordering::Release,
    );
    let reply = wire(
        owner,
        admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    ).await?;
    // STEP acknowledges entering the trigger cell. Its paid follow-up is still unresolved.
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let charges = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_carried_step_receipts WHERE input_before IS NOT NULL",
        ).fetch_one(&h.pool).await
    };
    assert_eq!(charges().await?, 1);
    assert!(owner.has_pending_quest_callback(session));
    let loss = owner.commit_control_loss(session, actor, controller).await;
    assert!(
        !owner.has_pending_quest_callback(session),
        "control loss must settle the charged STEP before releasing the actor ({loss:?})",
    );
    assert_eq!(
        owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
        local(destination),
    );
    assert_eq!(loss, ControlLossResult::Recorded);
    let maintain_readiness = async {
        loop {
            super::maintain_ready(&h.root).await?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    };
    let released = tokio::select! {
        release = tokio::time::timeout(
            std::time::Duration::from_secs(70),
            owner.expire_session_control_loss(*admitted),
        ) => release?,
        ready = maintain_readiness => return ready,
    };
    assert_eq!(released, GraceExpiryResult::Released);
    assert!(owner.runtime.lock().await.read_actor_position(actor).is_err());
    assert_eq!(charges().await?, 1);
    Ok(())
}
