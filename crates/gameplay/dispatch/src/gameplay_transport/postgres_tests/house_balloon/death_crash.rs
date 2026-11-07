//! Test-only storage fault after genuine HouseExit COMMIT, before actual placement.
use super::*;
pub(super) async fn install_fault(h: &Harness) -> TestResult {
    sqlx::raw_sql("CREATE FUNCTION srv7_test_refuse_pending_placement() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'SRV7_TEST_PLACEMENT_CRASH' USING ERRCODE='23514'; END $$; CREATE TRIGGER srv7_test_refuse_pending_placement BEFORE DELETE ON game_character_pending_respawns FOR EACH ROW EXECUTE FUNCTION srv7_test_refuse_pending_placement();").execute(&h.pool).await?;
    Ok(())
}
pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    capture: &std::sync::Arc<std::sync::Mutex<super::wire::HouseWireCapture>>,
    disconnected: bool,
) -> TestResult {
    // Remove only the test fault. The production pending row and old checkpoint remain untouched.
    sqlx::raw_sql("DROP TRIGGER srv7_test_refuse_pending_placement ON game_character_pending_respawns; DROP FUNCTION srv7_test_refuse_pending_placement();").execute(&h.pool).await?;
    let (exit_status, missing_ack, saw_zero) = {
        let outcome = capture.lock().map_err(|_| "crash transfer outcome")?;
        (
            outcome.exit_status,
            outcome.exit_ack.is_none(),
            outcome
                .vitals
                .iter()
                .filter_map(|v| oteryn_protocol_oteryn::actor_spell::decode_actor_vitals(v).ok())
                .any(|v| v.health == 0),
        )
    };
    assert!(
        exit_status != Some(oteryn_protocol_oteryn::CommandStatus::Accepted) && missing_ack,
        "the interrupted placement cannot acknowledge a completed transfer"
    );
    let _ = disconnected; // EOF can retain the last admitted source in the mature wire helper.
    assert!(saw_zero, "genuine native HP0 precedes real Exit102");
    let character = source.item_fence.ok_or("crash source fence")?.character_id;
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_house_transfer_receipts WHERE source_game_session_id=encode($2,'hex')::uuid AND state=2 AND direction=2)")
        .bind(character.as_bytes().as_slice()).bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (1, 1, 1),
        "the real death and exit committed while placement remains pending"
    );
    let retained = owner
        .root
        .house_transfer_recovery(source.game_session_id)
        .await?
        .ok_or("committed crash exit")?;
    assert!(retained.ack.house.is_none());
    assert!(
        retained.house_position.is_none(),
        "receipt-proven pending death must project Temple rather than old House metadata"
    );
    assert_eq!(
        [
            retained.placement.record.cell.x,
            retained.placement.record.cell.y,
            i32::from(retained.placement.record.cell.floor)
        ],
        retained.ack.position
    );
    super::death_recovery_negatives::qualify(h, owner, source, &retained).await?;
    let token = RecoverySource::token(191)?;
    let resumed = crate::gameplay_transport::connection::FreshAdmissionAuthority::resume(
        owner,
        crate::gameplay_transport::connection::ResumeAttempt {
            game_session_id: source.game_session_id,
            recovery_material: token.as_bytes(),
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[191; 16])
                .map_err(debug)?,
            last_applied_server_sequence: 0,
            supported_capabilities: source.continuity.selected_capabilities.as_slice(),
        },
    )
    .await
    .map_err(|e| format!("actual pending death recovery: {e:?}"))?;
    assert_eq!(resumed.game_session_id.as_bytes(), &retained.ack.session);
    assert!(
        resumed.continuity.connection_generation > 1,
        "real recovery advances the native connection fence"
    );
    let actor = resumed.runtime_actor.ok_or("recovered Temple actor")?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        crate::foundation::MovementLocalPosition {
            x: retained.ack.position[0],
            y: retained.ack.position[1],
            floor: i16::try_from(retained.ack.position[2])?
        }
    );
    let states = owner.states.spell_states.lock().await;
    assert!(
        states
            .get(&runtime, actor, resumed.game_session_id)
            .ok_or("restored Temple pools")?
            .vitals()
            .health
            > 0
    );
    drop(states);
    drop(runtime);
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)").bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (1, 0),
        "real recovery consumes placement once without repricing death"
    );
    eprintln!("SRV7_HOUSE_DEATH_ACTUAL_CRASH_RECOVERY_EXECUTED");
    Ok(())
}
