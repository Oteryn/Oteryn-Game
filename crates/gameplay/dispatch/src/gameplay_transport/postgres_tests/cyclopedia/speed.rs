//! W12: persisted discovery completion leaves the routed STEP pace unchanged.
use super::*;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
    decode_step_result_receipt, encode_step_intent,
};

async fn step(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &mut AdmittedSession,
    origin: MovementLocalPosition,
    direction: StepDirection,
) -> TestResult<u32> {
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        let mut at = movement.read(deps.actor).map_err(debug)?;
        while at.position() != origin {
            let current = at.position();
            let next = if current.x != origin.x {
                MovementLocalPosition {
                    x: current.x + (origin.x - current.x).signum(),
                    ..current
                }
            } else {
                MovementLocalPosition {
                    y: current.y + (origin.y - current.y).signum(),
                    ..current
                }
            };
            at = movement.commit_cardinal(at, next).map_err(debug)?;
        }
    }
    owner.clock_origin -= std::time::Duration::from_secs(2);
    deps.initialize(owner).await;
    super::super::maintain_ready(owner.root).await?;
    let command_id = admitted.continuity.next_command_id;
    let (status, reply) = support::exchange(
        owner,
        admitted,
        command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &encode_step_intent(direction),
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let receipt = decode_step_result_receipt(&reply, true).map_err(debug)?;
    assert_eq!(receipt.disposition, StepDisposition::Moved);
    assert!(receipt.step_wait_ms > 0, "STEP must return a negotiated pacing receipt");
    Ok(receipt.step_wait_ms)
}

pub(super) async fn completed_discovery_keeps_step_pace_and_speed_query_refuses(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    generation: &crate::content::ActiveGeneration,
    frame: [u8; 32],
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_cyclopedia SET progress='{\"legacy\":{\"discovered\":false,\"exploration\":null},\"subareas\":{\"2\":{\"discovered\":true,\"exploration\":null},\"3\":{\"discovered\":true,\"exploration\":null}}}'")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            authority,
            &h.node,
            &admitted.item_fence.ok_or("fence")?,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;

    owner.test_step_wait_receipts = Some(true);
    let origin = MovementLocalPosition {
        x: 20,
        y: 7,
        floor: 7,
    };
    let completed_pace = step(owner, deps, admitted, origin, StepDirection::East).await?;
    let reset_id = admitted.continuity.next_command_id;
    let (_, reset) = support::exchange(owner, admitted, reset_id, 73, &request(2, 0)).await?;
    assert_eq!(reset[0], 0);
    let incomplete_pace = step(owner, deps, admitted, origin, StepDirection::East).await?;
    assert_eq!(
        completed_pace, incomplete_pace,
        "Crystal discovery completion contributes no speed to STEP"
    );
    let query_id = admitted.continuity.next_command_id;
    let (status, value) = support::exchange(owner, admitted, query_id, 6178, &[]).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(value, b"Sorry, not possible.");
    let stored: bool = sqlx::query_scalar(
        "SELECT (progress->'subareas'->'2'->>'discovered')::boolean FROM game_character_cyclopedia",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(!stored, "reset clears the durable subarea completion");
    let points: serde_json::Value = sqlx::query_scalar(
        "SELECT progress->'subareas'->'2'->'exploration' FROM game_character_cyclopedia",
    )
    .fetch_one(&h.pool)
    .await?;
    let exploration: crate::domain::cyclopedia::Exploration = serde_json::from_value(points)?;
    for point in exploration.points {
        let left = point.x > 0
            && owner
                .bundle_map()
                .ok_or("map")?
                .overlay
                .base()
                .tile((point.x - 1) as u16, point.y as u16, point.floor as i8)
                .is_some_and(|tile| tile.walkable());
        step(
            owner,
            deps,
            admitted,
            MovementLocalPosition {
                x: point.x + if left { -1 } else { 1 },
                y: point.y,
                floor: -point.floor,
            },
            if left {
                StepDirection::East
            } else {
                StepDirection::West
            },
        )
        .await?;
    }
    let stored: bool = sqlx::query_scalar(
        "SELECT (progress->'subareas'->'2'->>'discovered')::boolean FROM game_character_cyclopedia",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(stored, "the seventh POI still restores subarea completion through STEP");
    assert_eq!(
        step(owner, deps, admitted, origin, StepDirection::East).await?,
        incomplete_pace,
        "rediscovery through STEP leaves the pacing receipt unchanged"
    );
    Ok(())
}
