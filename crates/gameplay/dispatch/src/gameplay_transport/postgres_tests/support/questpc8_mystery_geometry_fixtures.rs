use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::map::{overlay::TilePos, view::MapFacts};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world_id, "content-1")?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::questpc8_native_slice::world(
        native,
        &[
            (32568, 32503, 32574, 32509, -7),
            (32456, 31090, 32462, 31096, -6),
            (33195, 31181, 33201, 31187, -6),
            (33831, 31694, 33837, 31700, -6),
            (33077, 32701, 33083, 32708, -5),
        ],
        &[],
        TilePos {
            x: 32570,
            y: 32506,
            floor: -7,
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
    for (at, appearance, ordinal) in [
        (
            TilePos {
                x: 32459,
                y: 31093,
                floor: -6,
            },
            799,
            0,
        ),
        (
            TilePos {
                x: 33198,
                y: 31184,
                floor: -6,
            },
            469,
            0,
        ),
        (
            TilePos {
                x: 33834,
                y: 31697,
                floor: -6,
            },
            4408,
            0,
        ),
        (
            TilePos {
                x: 33080,
                y: 32704,
                floor: -5,
            },
            231,
            0,
        ),
        (
            TilePos {
                x: 33080,
                y: 32705,
                floor: -5,
            },
            4448,
            1,
        ),
    ] {
        arrive(owner, admitted, at).await?;
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
    let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/mystery-of-the-valley/stage'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1, "fixture arrival earns no quest progress");
    Ok(())
}
