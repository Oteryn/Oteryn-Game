//! Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:5-10,30-55.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::gameplay_transport::{
    connection::UseCommand,
    item_move::{LoopMove, LoopMoveReplay, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
    loop_move,
};
use oteryn_gameplay_quests::quest::world_callbacks::{
    Offering, Position,
    coal_basins::{self, COIN, CoalBasin, Treasure},
};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

pub(super) const START: Position = Position { x: 33072, y: 32589, floor: -13 };

#[test]
fn coal_basin_sacrifices_only_the_arriving_split_and_replays_the_other_players_landing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::coal_basin_arrival(admin)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    pool: &sqlx::PgPool,
) -> TestResult {
    use super::{l6_shared_gate_native::relocate, quest_world_callbacks::input_item};
    use std::sync::atomic::Ordering;

    let document = serde_json::json!({
        "schema": 1, "callbacks": [],
        "coal_basins": [{
            "source_uid": 9035,
            "basin": { "position": {"x":33073,"y":32589,"floor":-13},
                       "ordinal":1, "appearance":2114 },
            "flame": {"x":33073,"y":32590,"floor":-13},
            "destination": {"x":33080,"y":32588,"floor":-13},
            "treasure":"dipthrah",
        }],
    });
    let basins = coal_basins::load(&document.to_string())?;
    let CoalBasin { basin, flame, destination, .. } = basins[0].clone();
    assert!(owner.states.quest_callbacks.coal_basins_fixture.set(basins).is_ok());
    let [depositor, guest] = members else { return Err("coal basin requires two characters".into()); };
    let depositor_actor = depositor.runtime_actor.ok_or("depositor actor")?;
    let guest_actor = guest.runtime_actor.ok_or("flame occupant")?;
    let donor = owner.current_quest_fence(depositor.game_session_id).await
        .map_err(|_| "depositor fence")?.ok_or("depositor session")?;
    let recipient = owner.current_quest_fence(guest.game_session_id).await
        .map_err(|_| "guest fence")?.ok_or("guest session")?;
    relocate(owner, guest_actor, guest.game_session_id, (flame.x, flame.y, 13)).await?;

    // Seed custody separately so the actual arriving split must leave the old
    // stack on the basin and the undispatched remainder in the depositor's bag.
    input_item(owner, donor.item_fence(), &Offering { position: basin.position, item:COIN.into(), count:1 }, 960, false).await?;
    input_item(owner, donor.item_fence(), &Offering { position: START, item:COIN.into(), count:6 }, 961, true).await?;
    let item: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(e.item_instance_id)
           FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
          WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.quantity=6",
    ).bind(donor.character_id.as_bytes().as_slice()).bind(COIN).fetch_one(pool).await?;
    let item: [u8;16] = item.try_into().map_err(|_| "coin stack ID")?;
    let target = LoopTarget::Ground(WorldTilePosition {
        x:i32::from(basin.position.x), y:i32::from(basin.position.y), floor:i16::from(basin.position.floor),
    });
    let command = UseCommand {
        game_session_id: depositor.game_session_id, command_id: 4507, item_fence: Some(donor.item_fence()),
    };
    owner.states.quest_callbacks.interrupt_after_commit.store(true, Ordering::Release);
    assert_eq!(
        loop_move::move_loop_item(owner,depositor_actor,command,LoopMove {
            item:ItemKey::Instance(item), count:Some(2), target,
        }).await,
        LoopMoveResult::Unknown
    );
    let position = |p: Position| (i32::from(p.x), i32::from(p.y), -i16::from(p.floor));
    async fn at(owner: &ComposedFreshAdmission<'_, '_, '_>, actor: crate::foundation::ExactActorRef)
        -> TestResult<(i32,i32,i16)>
    {
        let p = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
        Ok((p.x,p.y,p.floor))
    }
    assert_eq!(at(owner, guest_actor).await?,position(flame),"initial acknowledgement was interrupted");
    let quest = owner.root.read_character_quest_state(owner.character,recipient.character_id).await.map_err(debug)?;
    assert_eq!(quest.tracks().get(coal_basins::START),Some(&1));
    assert_eq!(quest.tracks().get(Treasure::Dipthrah.track()),Some(&1));
    let paid = owner.root.read_character_quest_state(owner.character,donor.character_id).await.map_err(debug)?;
    assert_eq!(paid.tracks().get(coal_basins::START),None);
    assert_eq!(loop_move::committed_loop_move(owner,command).await,Some(LoopMoveReplay::Moved));
    assert_eq!(at(owner,guest_actor).await?,position(destination));
    assert_eq!(at(owner,depositor_actor).await?,position(START));
    assert_eq!(loop_move::committed_loop_move(owner,command).await,Some(LoopMoveReplay::Moved));
    let sacrificed: Vec<(i64,bool,bool)> = sqlx::query_as(
        "SELECT (before_item->>'quantity')::bigint, recipient=encode($1,'hex')::uuid, quest_changed
           FROM game_coal_basin_arrivals",
    ).bind(recipient.character_id.as_bytes().as_slice()).fetch_all(pool).await?;
    assert_eq!(sacrificed,[(2,true,true)]);
    let custody: Vec<(i64,i16,bool)> = sqlx::query_as(
        "SELECT quantity,lifecycle,EXISTS(SELECT 1 FROM game_item_ground_locations g
            WHERE g.item_instance_id=i.item_instance_id)
           FROM game_item_instances i WHERE definition_production_key=$1 ORDER BY quantity",
    ).bind(COIN).fetch_all(pool).await?;
    assert_eq!(custody,[(0,2,false),(1,1,true),(4,1,false)]);
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_pending_arrivals WHERE source_coal_basin",
    ).fetch_one(pool).await?;
    assert_eq!(pending,0);
    Ok(())
}
