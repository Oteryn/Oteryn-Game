//! M56: framed self USE consumes exactly the selected wax and commits ear protection.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget, encode_item_use_intent};

const KEY: &str = "oteryn:quest-reward/cursed-crystal/protect-ears";
const TRACK: &str = "oteryn:quest-progress/quest/u10_70/the_cursed_crystal/oneeyedjoe";

#[test]
fn m56_cursed_crystal_exact_self_wax_consumption_and_protection_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m56-wax")
}

#[test]
fn m56_cursed_crystal_wax_outside_crystal_area_is_preserved_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m56-wax-outside")
}

#[test]
fn m56_cursed_crystal_wax_at_area_corner_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m56-wax-corner")
}

#[test]
fn m56_cursed_crystal_wax_one_tile_beyond_area_is_preserved_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m56-wax-edge-outside")
}

pub(super) fn trigger(name: &str) -> Position {
    // Crystal uses abs(delta) < 5 on each axis, including the diagonal corner.
    // The shared scenario places the player one tile west of this trigger.
    Position {
        x: if name == "qengine-rewards-m56-wax-edge-outside" { 31948 } else { 31947 },
        y: 32940,
        floor: -10,
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    outside: bool,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter().find(|c| c.key == KEY).ok_or("M56 wax callback is not wired")?;
    assert_eq!(callback.storage[0].track, TRACK);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (position, target) = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let player = visible.players.iter().find(|p| p.actor == actor).ok_or("self entity")?;
        (runtime.read_actor_position(actor).map_err(debug)?.position(),
         oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
             identity: actor.placement_identity(), generation: player.generation,
         })
    };
    assert_eq!(callback.self_target.as_ref().ok_or("area")?.contains(position.x, position.y, position.floor), !outside);
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(), &[4, 6, 11, 15, 18],
        ).ok_or("USE capabilities")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|_| "fence")?.ok_or("fence")?.item_fence();
    // The synthetic runtime must install the same fenced lease as normal admission.
    owner.runtime.lock().await.bind_attacker_lease(
        actor, admitted.game_session_id,
        crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        ).map_err(debug)?,
    ).map_err(debug)?;
    for n in [580, 581] {
        super::quest_world_callbacks::input_item(owner, fence, &Offering {
            position: Position { x: position.x.try_into()?, y: position.y.try_into()?, floor: (-position.floor).try_into()? },
            item: "oteryn:item.tibia.i9106".into(), count: 1,
        }, n, true).await?;
    }
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
    let reference = native.item_index().ok_or("index")?
        .definition_ref("Item", "oteryn:item.tibia.i9106", "definition-r1").ok_or("wax")?;
    let inventory = owner.root.read_item_use_inventory(owner.character, owner.holder, fence).await.map_err(debug)?;
    let selected = inventory.backpack.iter().flat_map(|bag| &bag.entries)
        .rfind(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i9106").ok_or("selected wax")?.item.item_instance_id;
    for bad in [None, Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        generation: target.generation + 1, ..target
    }), Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: super::fixture::id(99), ..target
    })] {
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
            use_with: bad, use_with_map_item: None, use_with_position: None,
        }).map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let (next, result, _) = super::rust_lottery::exchange_latest_source(owner, *admitted, command, &payload, Some(reference)).await?;
        *admitted = next;
        assert_ne!(result, UseDisposition::Committed, "bare, stale or other target must refuse");
        assert_eq!(super::qengine_voucher_stories::quantity(h, 9106).await?, 2);
    }
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: Some(target), use_with_map_item: None, use_with_position: None,
    }).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = super::rust_lottery::exchange_latest_source(owner, *admitted, command, &payload, Some(reference)).await?;
    *admitted = next;
    if outside {
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 9106).await?, 2);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(KEY).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 0);
        return Ok(());
    }
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 9106).await?, 1);
    let consumed: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(KEY).fetch_one(&h.pool).await?;
    assert_eq!(consumed.as_bytes(), &selected, "consume the selected instance, retain the other wax");
    let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1").bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(value, 2, "source ear-protection condition commits with consumption");
    let (next, result, _) = super::rust_lottery::exchange(owner, *admitted, command, &actual, None, None).await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed, "receipt replay after selected item retires");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 9106).await?, 1);
    let command = admitted.continuity.next_command_id;
    let (next, result, _) = super::rust_lottery::exchange_latest_source(owner, *admitted, command, &payload, Some(reference)).await?;
    *admitted = next;
    assert_ne!(result, UseDisposition::Committed, "already protected cannot consume surplus wax");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 9106).await?, 1);
    Ok(())
}
