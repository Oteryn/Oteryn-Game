//! T187: persisted completion -> actual STEP router -> one runtime contribution.
use super::*;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

async fn step(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: AdmittedSession,
    origin: MovementLocalPosition,
    direction: StepDirection,
) -> TestResult {
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
    let outcome = crate::gameplay_transport::connection::regression_routed_step(
        owner, admitted, deps.actor, direction,
    )
    .await?;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    Ok(())
}
async fn bonus(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: AdmittedSession,
) -> TestResult<i64> {
    owner
        .runtime
        .lock()
        .await
        .actor_active_speed_delta(
            deps.actor,
            Some(admitted.game_session_id),
            owner.owner_now(),
        )
        .map_err(debug)
}

pub(super) async fn verify(
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

    step(
        owner,
        deps,
        *admitted,
        MovementLocalPosition {
            x: 20,
            y: 7,
            floor: 7,
        },
        StepDirection::South,
    )
    .await?;
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        0,
        "T187: leaving completed area removes contribution"
    );
    step(
        owner,
        deps,
        *admitted,
        MovementLocalPosition {
            x: 20,
            y: 8,
            floor: 7,
        },
        StepDirection::North,
    )
    .await?;
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        195,
        "T187: entering completed area applies contribution"
    );
    step(
        owner,
        deps,
        *admitted,
        MovementLocalPosition {
            x: 20,
            y: 7,
            floor: 7,
        },
        StepDirection::East,
    )
    .await?;
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        195,
        "T187: repeated movement does not stack with itself"
    );
    owner
        .runtime
        .lock()
        .await
        .set_player_discovery_speed(deps.actor, admitted.game_session_id, 0)
        .map_err(debug)?;
    step(
        owner,
        deps,
        *admitted,
        MovementLocalPosition {
            x: 21,
            y: 7,
            floor: 7,
        },
        StepDirection::East,
    )
    .await?;
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        195,
        "T187: contribution reloads from PostgreSQL"
    );
    let query_id = admitted.continuity.next_command_id;
    let (status, value) = support::exchange(owner, admitted, query_id, 6178, &[]).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(value, 195u16.to_le_bytes());
    let reset_id = admitted.continuity.next_command_id;
    let (_, reset) = support::exchange(owner, admitted, reset_id, 73, &request(2, 0)).await?;
    assert_eq!(reset[0], 0);
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        0,
        "T187: reset immediately removes bonus"
    );
    let stored: bool = sqlx::query_scalar(
        "SELECT (progress->'subareas'->'2'->>'discovered')::boolean FROM game_character_cyclopedia",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(!stored, "T187: reset eligibility loss persists");
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
            *admitted,
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
    assert_eq!(
        bonus(owner, deps, *admitted).await?,
        195,
        "T187: seventh POI restores completed-area bonus through STEP"
    );
    Ok(())
}
