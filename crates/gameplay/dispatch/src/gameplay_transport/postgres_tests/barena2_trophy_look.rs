use super::TestResult;
use super::arena_match::{NativeScenario, command, scenario_route};
use super::fixture::{Harness, debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView, inventory_from_backpack};
use oteryn_protocol_oteryn::arena::{ArenaAction as A, ArenaDisposition as D};
use oteryn_protocol_oteryn::item_look::ItemLookResult;

#[test]
fn arena_goblet_look_projects_the_awarded_character() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::TrophyLook)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "arena goblet LOOK scenario panicked")??;
    Ok(())
}

pub(super) async fn check_trophy_look(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    seed_completion(harness, admitted).await?;
    let session = admitted.game_session_id;
    let fence = admitted.item_fence.ok_or("item fence")?;

    // The wire claim also checks that the character stands in the reward room.
    let (x, y, floor) = crate::durability::arena_match::donor::REWARD_ROOM;
    {
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .relocate_arena_player(
                actor,
                session,
                before,
                crate::foundation::MovementLocalPosition { x, y, floor },
            )
            .map_err(debug)?;
    }

    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&harness.pool)
    .await?;
    let index = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index()).ok_or("native Item index")?;
    for (rank, item_id, difficulty, punctuation) in [
        (1, 5807, "greenhorn", ""),
        (2, 5806, "scrapper", "."),
        (3, 5805, "warlord", "."),
    ] {
        assert_eq!(
            command(owner, admitted, A::ClaimReward { rank, category: 4, choice: 1 })
                .await?.disposition,
            D::Applied
        );
        let backpack = harness.root.read_character_backpack(owner.character, fence.character_id).await?;
        let inventory = inventory_from_backpack(backpack.as_ref(), |definition| {
            index.definition_ref(&definition.family, &definition.production_key, &definition.revision_ref)
        }).ok_or("backpack projection")?;
        let mut views = SessionItemView::resume(ItemViewContinuity::default());
        views.snapshot(inventory, None).map_err(debug)?;
        let key = format!("oteryn:item.tibia.i{item_id}");
        let item: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
        ).bind(key).fetch_one(&harness.pool).await?;
        let handle = views.table().handle(&ItemKey::Instance(item.as_slice().try_into()?))
            .ok_or("arena trophy handle")?;
        let looked = crate::gameplay_transport::item_look::look_item(
            owner, actor, session, Some(fence), &views, handle,
        ).await;
        let ItemLookResult::Text(text) = looked else {
            return Err("LOOK refused the arena trophy".into());
        };
        assert!(text.ends_with(&format!("For victory in the Barbarian Arena at {difficulty} difficulty. Awarded to {name}{punctuation}")), "{text}");
    }
    Ok(())
}


#[test]
fn arena_trophy_step_places_each_goblet_one_tile_north() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::TrophyStep)).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "arena trophy STEP scenario panicked")??;
    Ok(())
}

pub(super) async fn check_trophy_step(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    seed_completion(harness, admitted).await?;
    let (x, y, floor) = crate::durability::arena_match::donor::REWARD_ROOM;
    for (offset, item) in [(0, 5807), (1, 5806), (2, 5805)] {
        let x = x + offset;
        {
            let mut runtime = owner.runtime.lock().await;
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            runtime.relocate_arena_player(actor, admitted.game_session_id, before,
                crate::foundation::MovementLocalPosition { x, y: y + 1, floor }).map_err(debug)?;
        }
        let moved = crate::gameplay_transport::connection::regression_routed_step(
            owner, *admitted, actor, StepDirection::North,
        ).await?;
        assert_eq!(moved.disposition, StepDisposition::Moved);
        let positions: Vec<Vec<u8>> = sqlx::query_scalar(
            "SELECT g.spatial_position FROM game_item_ground_locations g
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key=$1 AND i.lifecycle=1",
        ).bind(format!("oteryn:item.tibia.i{item}")).fetch_all(&harness.pool).await?;
        let mut north = Vec::new();
        north.extend_from_slice(&x.to_be_bytes());
        north.extend_from_slice(&(y - 1).to_be_bytes());
        north.extend_from_slice(&floor.to_be_bytes());
        assert_eq!(positions, vec![north], "trophy {item} placement");
    }
    Ok(())
}

async fn seed_completion(harness: &Harness, admitted: &AdmittedSession) -> TestResult {
    // Import completed arena progress to isolate the trophy and LOOK callers.
    let session = admitted.game_session_id;
    let mut tx = harness.pool.begin().await?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let completed = crate::durability::arena_match::MatchState {
        completed: 3,
        rank: 3,
        phase: 3,
        session: *session.as_bytes(),
        scope_generation: fence.scope_ownership_generation.get(),
        ..Default::default()
    };
    let state = serde_json::to_value(&completed)?;
    sqlx::query(
        "INSERT INTO game_arena_match_receipts
           (character_id,game_session_id,command_id,payload,before_state,after_state,reply)
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,999999,$3,$4,$5,$6)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(session.as_bytes().as_slice())
    .bind(
        oteryn_protocol_oteryn::arena::encode_match_request(A::Status)
            .map_err(super::fixture::debug)?,
    )
    .bind(serde_json::to_value(
        crate::durability::arena_match::MatchState::default(),
    )?)
    .bind(&state)
    .bind(
        oteryn_protocol_oteryn::arena::encode_match_result(
            &oteryn_protocol_oteryn::arena::ArenaResult {
                disposition: D::Applied,
                state: completed.wire(0, [0; 3]),
            },
        )
        .map_err(super::fixture::debug)?,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_arena_matches(character_id,body,game_session_id,command_id)
         VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,999999)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&state)
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}
