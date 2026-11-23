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
    prepare_steps(admitted, h, authority, generation, frame).await?;

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

pub(super) async fn prepare_steps(
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    generation: &crate::content::ActiveGeneration,
    frame: [u8; 32],
) -> TestResult {
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

    Ok(())
}

type TilePos = crate::map::overlay::TilePos;
type World = crate::map::boot::BundleWorld;

pub(super) struct Boundary {
    outside: TilePos,
    inside: TilePos,
    entry: StepDirection,
    exit: StepDirection,
    area: u32,
}

impl Boundary {
    pub(super) fn start(&self) -> (i32, i32, i16) {
        let at = self.outside;
        (i32::from(at.x), i32::from(at.y), -i16::from(at.floor))
    }
}

// Adjacent Ground cells at the western edge of the shipped Svargrond discovery area.
const INSIDE: TilePos = TilePos {
    x: 32064,
    y: 31089,
    floor: -7,
};
const OUTSIDE: TilePos = TilePos { x: 32065, ..INSIDE };

pub(super) fn shipped_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<World> {
    super::super::source_world_crop::compiled_region(INSIDE, OUTSIDE, INSIDE, native)
}

pub(super) fn boundary(world: &World) -> TestResult<Boundary> {
    assert!(world.enterable(OUTSIDE) && world.enterable(INSIDE));
    let areas = &world.base().subareas;
    let (area, _) = areas
        .resolve(INSIDE.x, INSIDE.y, INSIDE.floor)
        .ok_or("shipped discovery entry has no area membership")?;
    assert_eq!(area, 23);
    assert!(
        areas
            .resolve(OUTSIDE.x, OUTSIDE.y, OUTSIDE.floor)
            .is_none_or(|(outside, _)| outside != area)
    );
    Ok(Boundary {
        outside: OUTSIDE,
        inside: INSIDE,
        entry: StepDirection::West,
        exit: StepDirection::East,
        area,
    })
}

pub(super) async fn seed_shipped_completion(
    h: &Harness,
    world: &World,
    edge: &Boundary,
) -> TestResult {
    let subareas: serde_json::Map<String, serde_json::Value> = world
        .base()
        .subareas
        .areas
        .iter()
        .filter(|subarea| subarea.parent == Some(edge.area))
        .map(|subarea| {
            (
                subarea.id.to_string(),
                serde_json::json!({"discovered": true, "exploration": null}),
            )
        })
        .collect();
    assert!(!subareas.is_empty());
    let progress = serde_json::json!({
        "legacy": {"discovered": false, "exploration": null},
        "subareas": subareas,
    });
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,catalogue_digest,progress,occurrence) VALUES(encode($1,'hex')::uuid,$2,$3,$4,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice())
        .bind(world.base().digest().as_slice())
        .bind(crate::domain::cyclopedia::BasicArea::content_digest().as_slice())
        .bind(progress)
        .bind(id(81).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn verify_shipped(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &mut AdmittedSession,
    edge: Boundary,
) -> TestResult {
    let command = oteryn_protocol_oteryn::cyclopedia::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY;
    let id = admitted.continuity.next_command_id;
    assert_eq!(
        support::exchange(owner, admitted, id, command, &[]).await?,
        (CommandStatus::Accepted, 0u16.to_le_bytes().to_vec())
    );
    // One completed main area out of the shipped catalogue, which has several.
    for (direction, at, expected) in [
        (edge.entry, edge.inside, 100u16),
        (edge.exit, edge.outside, 0),
    ] {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        deps.initialize(owner).await;
        super::super::maintain_ready(owner.root).await?;
        let outcome = crate::gameplay_transport::connection::regression_routed_step(
            owner, *admitted, deps.actor, direction,
        )
        .await?;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
        let position = outcome
            .moved_to
            .ok_or("shipped step position")?
            .actor_position;
        assert_eq!(
            (position.x, position.y, position.floor),
            (i32::from(at.x), i32::from(at.y), -i16::from(at.floor))
        );
        assert_eq!(bonus(owner, deps, *admitted).await?, i64::from(expected));
        let id = admitted.continuity.next_command_id;
        assert_eq!(
            support::exchange(owner, admitted, id, command, &[]).await?,
            (CommandStatus::Accepted, expected.to_le_bytes().to_vec())
        );
    }
    eprintln!(
        "W12_SHIPPED speed=0,100,0 area={} outside={:?} inside={:?}",
        edge.area, edge.outside, edge.inside
    );
    Ok(())
}
