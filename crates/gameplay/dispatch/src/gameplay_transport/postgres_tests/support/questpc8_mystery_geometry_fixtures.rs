use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::map::{overlay::TilePos, view::MapFacts};

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

use super::questpc8_authored_programmes_fixtures::step;

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::questpc8_native_slice::world(
        native,
        &[
            (33832, 32350, 33837, 32354, -4),
            (32461, 31090, 32465, 31094, -5),
            (33185, 31189, 33189, 31193, -6),
            (33834, 31674, 33838, 31679, -4),
            (33080, 32668, 33084, 32672, -3),
            (32365, 31057, 32368, 31059, -7),
            (32407, 31065, 32410, 31067, -7),
        ],
        &[],
        TilePos {
            x: 33835,
            y: 32352,
            floor: -4,
        },
    )
}

/// Arrival between distant regions is fixture setup, not a quest travel reward.
/// Every destination uses the real B3 cells and unchanged native collision index.
pub(super) async fn arrive(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: TilePos,
) -> TestResult {
    use crate::content::{CollisionClass, LogicalCell};
    let local = super::quest_world_callbacks_fixtures::local(Position {
        x: at.x,
        y: at.y,
        floor: at.floor,
    });
    let cells = &owner.movement_cells;
    assert_eq!(
        cells.index().lookup(
            cells.scope(),
            LogicalCell {
                x: local.x,
                y: local.y,
                z: i32::from(local.floor)
            }
        ),
        Ok(CollisionClass::Walkable),
        "native fixture landing {at:?}"
    );
    let mut runtime = owner.runtime.lock().await;
    assert_eq!(
        cells.scope().generation_digest,
        runtime.content_pin().server_artifact_digest()
    );
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    // Reuse the existing exact-snapshot physical relocation owner for fixture
    // arrival only. It changes neither quest state nor inventory.
    runtime
        .relocate_arena_player(actor, admitted.game_session_id, before, local)
        .map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_mystery_geometry after PostgreSQL fixture initialization"
    );
    super::questpc8_mystery_fixtures::exercise_intro(owner, admitted, h, authority).await?;
    for (at, approach, appearance, ordinal) in [
        (
            TilePos {
                x: 32463,
                y: 31092,
                floor: -5,
            },
            TilePos {
                x: 32462,
                y: 31092,
                floor: -5,
            },
            10225,
            1,
        ),
        (
            TilePos {
                x: 33187,
                y: 31191,
                floor: -6,
            },
            TilePos {
                x: 33188,
                y: 31191,
                floor: -6,
            },
            10225,
            1,
        ),
        (
            TilePos {
                x: 33836,
                y: 31677,
                floor: -4,
            },
            TilePos {
                x: 33836,
                y: 31676,
                floor: -4,
            },
            10225,
            1,
        ),
        (
            TilePos {
                x: 33082,
                y: 32671,
                floor: -3,
            },
            TilePos {
                x: 33081,
                y: 32670,
                floor: -3,
            },
            1084,
            2,
        ),
    ] {
        arrive(owner, admitted, approach).await?;
        let map = owner.bundle_map().ok_or("map")?;
        let tile = map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .ok_or("native tile")?;
        let id = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))
            .map(|(id, _)| *id)
            .ok_or("native ordinal")?;
        let facts = map
            .facts
            .base_entry(at, ordinal, id)
            .ok_or("native facts")?;
        assert_eq!(facts.appearance_id, appearance, "native landmark {at:?}");
    }
    exercise_sled(owner, admitted, h).await?;
    let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/mystery-of-the-valley/stage'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1, "fixture arrival earns no quest progress");
    Ok(())
}

async fn exercise_sled(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let nibelor = TilePos {
        x: 32366,
        y: 31058,
        floor: -7,
    };
    arrive(owner, admitted, nibelor).await?;
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(actor_position(owner, admitted).await?, local_tile(nibelor));

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: nibelor.x,
                y: nibelor.y,
                floor: nibelor.floor,
            },
            item: "oteryn:item.tibia.i3582".into(),
            count: 2,
        },
        5141,
        true,
    )
    .await?;
    step(owner, admitted, 2, StepDirection::East).await?;
    assert_eq!(
        actor_position(owner, admitted).await?,
        local_tile(TilePos {
            x: 32407,
            y: 31067,
            floor: -7,
        })
    );
    assert_eq!(carried_ham(h).await?, 1);
    step(owner, admitted, 3, StepDirection::East).await?;
    step(owner, admitted, 4, StepDirection::North).await?;
    step(owner, admitted, 5, StepDirection::East).await?;
    assert_eq!(
        actor_position(owner, admitted).await?,
        local_tile(TilePos {
            x: 32365,
            y: 31059,
            floor: -7,
        })
    );
    assert_eq!(carried_ham(h).await?, 0);
    Ok(())
}

fn local_tile(at: TilePos) -> crate::foundation::MovementLocalPosition {
    super::quest_world_callbacks_fixtures::local(Position {
        x: at.x,
        y: at.y,
        floor: at.floor,
    })
}

async fn actor_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::foundation::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}

async fn carried_ham(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i
         JOIN game_item_container_entries e USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid
           AND i.definition_production_key='oteryn:item.tibia.i3582' AND i.lifecycle=1",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
