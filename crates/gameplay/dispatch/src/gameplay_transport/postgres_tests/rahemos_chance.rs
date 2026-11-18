use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::{FreshAdmissionAuthority, connection::UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "qengine-rewards-rahemos-hat";
const TARGET: Position = Position { x: 101, y: 100, floor: -7 };
pub(super) const DOOR: Position = Position { x: 103, ..TARGET };
const SUCCESS: &str = "You found the carrot! The door is open!";
const FAILURE: &str = "You guessed wrong! Take this! Carrot changed now the Hat!";

#[test]
fn rahemos_lever_roll_leaves_hat_closed_on_damage_and_opens_on_success() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback(at: Position) -> TestResult<Callback> {
    // Crystal 201189f8 actions_oasis_lever_door.lua:22-44.
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/rahemos-hat-fixture",
        "quest":"oteryn:quest.the_ancient_tombs_quest","trigger":"use",
        "selector":{"position":at,"ordinal":1,"appearance":2772},
        "lever":[2772,2773],"reversible":false,"passages":[{"position":DOOR,"appearance":1662,"replacement":1663,"if_present":true}],"storage":[],"offerings":[],
        "return_on_failure":false,
        "success_message":SUCCESS,
        "effects":{"reset_after_ms":4000,"magic":{"position":DOOR,"effect":15},"use_chance":{
            "faces":3,"winning_faces":1,"pickup":{"appearance":3595,"dx":-1,"dy":0},"success_cue":{"dx":-1,"dy":0,"effect":15},
            "failure_hit":{"kind":"physical","amount":200,"effect":3},
            "failure_message":FAILURE
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/actions_oasis_lever_door.lua:22-45"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let row = callback(TARGET)?;
    assert!(owner.states.quest_callbacks.fixture.set(vec![row.clone()]).is_ok());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.states.chat.admit(session, "Rahemos", owner.owner_now());
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("source key")?;
    let (loss, win) = {
        let runtime = owner.runtime.lock().await;
        owner.states.spell_states.lock().await
            .get_mut(&runtime, actor, session).ok_or("state")?
            .install_equipped_element_protection(ReferenceResistanceKind::Physical,
                Some(ReferenceRationalPercent::new(95,1).map_err(debug)?));
        let chance = row.effects.use_chance.as_ref().ok_or("chance")?;
        let mut loss = None;
        let mut win = None;
        for id in 20..256 {
            let command = CommandRef::new(session, CommandId::new(id).map_err(debug)?);
            let wins = crate::gameplay_transport::quest_callbacks::use_chance::wins(
                &runtime, actor, command, &row.key, key, chance
            ).ok_or("decision")?;
            if wins && loss.is_some() {
                win = Some(id);
                break;
            }
            if !wins {
                loss.get_or_insert(id);
            }
        }
        (loss.ok_or("losing occurrence")?, win.ok_or("winning occurrence")?)
    };
    let appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>, at: Position| -> TestResult<u16> {
        let map = owner.bundle_map().ok_or("map")?;
        let tile = crate::map::overlay::TilePos { x: at.x, y: at.y, floor: at.floor };
        let id = *map.overlay.base().tile(tile.x,tile.y,tile.floor).ok_or("tile")?
            .ids().get(1).ok_or("lever")?;
        Ok(map.facts.base_entry(tile,1,id).ok_or("entry")?.appearance_id)
    };
    let before = super::underwater_drowning::health(owner, actor, session).await?;
    for (command_id, expected, door, feedback) in [
        (loss,2772,1662,FAILURE),
        (win,2773,1663,SUCCESS),
    ] {
        owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement=map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let result = owner.use_object(actor, UseCommand {
            game_session_id:session, command_id, item_fence:admitted.item_fence,
        }, WorldObjectTarget {
            placement, expected_revision:map.facts.object_revision(key),
        }).await;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(appearance(owner, TARGET)?, expected);
        assert_eq!(appearance(owner, DOOR)?, door);
        assert!(owner.states.chat.drain(session).iter().any(|line|
            matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == feedback)));
        let remaining = super::underwater_drowning::health(owner, actor, session).await?;
        assert!((1..=10).contains(&before.saturating_sub(remaining)));
    }
    let carrot = {
        let map = owner.bundle_map().ok_or("map")?;
        map.facts.movement_item_entries().into_iter().find_map(|(at, entry)| {
            use crate::map::view::EntrySource;
            if at.x == TARGET.x - 1 && at.y == TARGET.y && entry.facts.appearance_id == 3595
                && let EntrySource::Added(id) = entry.source {
                Some(id.get())
            } else { None }
        }).ok_or("temporary carrot")?
    };
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
    let movement = LoopMove {
        item: ItemKey::MapAdded(carrot),
        count: Some(1),
        target: LoopTarget::MainBackpack,
    };
    let moved = crate::gameplay_transport::loop_move::move_loop_item_with_house_capacity(
        owner, actor, UseCommand { game_session_id:session, command_id:win+1, item_fence:Some(fence.item_fence()) },
        movement, false,
    ).await;
    assert_eq!(moved, LoopMoveResult::Committed);
    let carrots: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_temporary_pickups r JOIN game_item_container_entries b USING(item_instance_id)
         WHERE r.definition_key='oteryn:item.tibia.i3595'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(carrots,1);
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.quest_pickup(carrot, owner.owner_now().get()).is_none());
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(5)).ok_or("clock")?;
    owner.tick_timed_items().await;
    assert_eq!(appearance(owner, TARGET)?,2772);
    assert_eq!(appearance(owner, DOOR)?,1663);

    let chance = row.effects.use_chance.as_ref().ok_or("chance")?;
    let next_win = {
        let runtime = owner.runtime.lock().await;
        (win + 2..win + 256).find(|id| {
            CommandId::new(*id).ok().and_then(|id| {
                crate::gameplay_transport::quest_callbacks::use_chance::wins(
                    &runtime,actor,CommandRef::new(session,id),&row.key,key,chance,
                )
            }) == Some(true)
        }).ok_or("second winning occurrence")?
    };
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = owner.use_object(actor, UseCommand {
        game_session_id:session, command_id:next_win,item_fence:Some(fence.item_fence()),
    }, WorldObjectTarget {
        placement,expected_revision:map.facts.object_revision(key),
    }).await;
    assert_eq!(result.disposition,UseDisposition::Committed);
    let spawned = |owner: &ComposedFreshAdmission<'_, '_, '_>| {
        owner.bundle_map().is_some_and(|map| map.facts.movement_item_entries().iter()
            .any(|(_,entry)|entry.facts.appearance_id==3595))
    };
    assert!(spawned(owner));
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(5)).ok_or("clock")?;
    owner.tick_timed_items().await;
    assert!(!spawned(owner));
    assert_eq!(appearance(owner, TARGET)?,2772);
    assert_eq!(appearance(owner, DOOR)?,1663);
    Ok(())
}
