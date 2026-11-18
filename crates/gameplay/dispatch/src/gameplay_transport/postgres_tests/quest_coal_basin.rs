//! Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:5-10,30-55.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::gameplay_transport::{
    connection::UseCommand,
    item_move::{ItemMoveStep, item_move},
    item_view::{ItemKey, ItemViewContinuity, ItemViewError, SessionItemView},
};
use oteryn_gameplay_quests::quest::world_callbacks::{
    Offering, Position,
    coal_basins::{self, COIN, CoalBasin, Treasure},
};
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    WorldTilePosition, encode_item_move_intent_for,
};

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
    let target = WorldTilePosition {
        x:i32::from(basin.position.x), y:i32::from(basin.position.y), floor:i16::from(basin.position.floor),
    };
    let source = ItemKey::Instance(item);
    let mut view = SessionItemView::resume(ItemViewContinuity::default())
        .with_map_view().with_equip_drop();
    let handle = view.map_view(&[source], |handles| {
        handles.handle(&source).ok_or(ItemViewError::Exhausted)
    }).map_err(debug)?;
    let selection = ItemMoveSelection {equip_drop:true, container_tree:false, world_interaction:true};
    let mut payload = encode_item_move_intent_for(
        &ItemMoveIntent {source:handle, destination:ItemMoveDestination::Ground(target)},
        selection,
    ).map_err(debug)?;
    payload.extend_from_slice(&[0x30,2]); // Command 9 field 6 under ITEM_STACK_COUNT_V1.
    let command = UseCommand {
        game_session_id: depositor.game_session_id, command_id: 4507, item_fence: Some(donor.item_fence()),
    };
    // An existing landing makes the recipient write fail after the original
    // move and track updates. Both must roll back before this command retries.
    sqlx::query(
        "INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor)
         VALUES(encode($1,'hex')::uuid,game_character_uuid_v7(),encode($2,'hex')::uuid,$3,$4,$5)",
    ).bind(recipient.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice())
      .bind(i32::from(flame.x)).bind(i32::from(flame.y)).bind(13_i16).execute(pool).await?;
    let rolled_back = item_move(owner,depositor_actor,command,&payload,selection,true,&mut view).await;
    assert_eq!(rolled_back, ItemMoveStep::Disconnect);
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(pool).await?;
    assert_eq!(quantity,6);
    let quest = owner.root.read_character_quest_state(owner.character,recipient.character_id).await.map_err(debug)?;
    assert_eq!(quest.tracks().get(coal_basins::START),None);
    sqlx::query("DELETE FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid")
        .bind(recipient.character_id.as_bytes().as_slice()).execute(pool).await?;
    owner.states.quest_callbacks.interrupt_after_commit.store(true, Ordering::Release);
    assert_eq!(
        item_move(owner,depositor_actor,command,&payload,selection,true,&mut view).await,
        ItemMoveStep::Disconnect
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
    assert_eq!(quest.tracks().get("oteryn:quest-track/ancient-tombs/diprath-treasure"),Some(&1));
    let paid = owner.root.read_character_quest_state(owner.character,donor.character_id).await.map_err(debug)?;
    assert_eq!(paid.tracks().get(coal_basins::START),None);
    let replay = item_move(owner,depositor_actor,command,&payload,selection,true,&mut view).await;
    assert!(matches!(replay,ItemMoveStep::Result(ItemMoveOutcome::Moved,_)),"{replay:?}");
    assert_eq!(at(owner,guest_actor).await?,position(destination));
    assert_eq!(at(owner,depositor_actor).await?,position(START));
    let replay = item_move(owner,depositor_actor,command,&payload,selection,true,&mut view).await;
    assert!(matches!(replay,ItemMoveStep::Result(ItemMoveOutcome::Moved,_)),"{replay:?}");
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
    let map = owner.bundle_map().ok_or("coal basin map")?;
    let fire_cues = map.facts.movement_effects_since(0).into_iter().filter(|(_, at, effect)| {
        (at.x, at.y, at.floor) == (basin.position.x, basin.position.y, basin.position.floor)
            && *effect == crate::durability::item_loop_move::COIN_FIRE_EFFECT
    }).count();
    assert_eq!(fire_cues, 1, "rollback and reply replay emit no second fire cue");

    // Crystal uses the true top creature (lines 44-45). The first-admitted
    // depositor enters the occupied flame last and is now the recipient.
    relocate(
        owner,
        guest_actor,
        guest.game_session_id,
        (flame.x, flame.y, 13),
    )
    .await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let expected = runtime
            .read_actor_position(depositor_actor)
            .map_err(debug)?;
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let proof = crate::map::floor::prepare_quest_encounter_ejection(
            &runtime,
            owner.movement_cells(),
            depositor_actor,
            depositor.game_session_id,
            expected,
            super::quest_world_callbacks::local(flame),
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        assert_eq!(
            runtime
                .current_top_actor_on_cell(super::quest_world_callbacks::local(flame))
                .map_err(debug)?,
            Some(depositor_actor),
        );
    }
    let second_drop = item_move(
        owner,
        depositor_actor,
        UseCommand {
            command_id: 4508,
            ..command
        },
        &payload,
        selection,
        true,
        &mut view,
    )
    .await;
    assert!(
        matches!(second_drop, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
        "{second_drop:?}"
    );
    assert_eq!(at(owner, depositor_actor).await?, position(destination));
    assert_eq!(at(owner, guest_actor).await?, position(flame));
    let paid = owner
        .root
        .read_character_quest_state(owner.character, donor.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(paid.tracks().get(coal_basins::START), Some(&1));
    Ok(())
}
