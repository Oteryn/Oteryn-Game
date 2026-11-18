use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::GameSessionId;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::num::NonZeroU64;

pub(super) const NAME: &str = "qengine-rewards-goshnar-fluid";
const TARGET: Position = Position { x: 101, y: 100, floor: -7 };
pub(super) const STAGING: Position = Position { y: 101, ..TARGET };
const DESTINATION: Position = Position { x: 104, y: 100, ..TARGET };
const VIAL: &str = "oteryn:item.tibia.i2874";

#[test]
fn goshnar_blood_use_with_moves_two_staged_players_and_empties_one_vial() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback(position: Position) -> TestResult<Callback> {
    // Crystal 201189f8 data-global/scripts/actions/other/fluids.lua:36-47,157-184.
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/goshnar-fluid-fixture",
        "quest":"oteryn:quest.the_pits_of_inferno_quest", "trigger":"use",
        "selector":{"position":position,"ordinal":1,"action":2023},
        "reversible":false, "passages":[], "storage":[], "offerings":[], "return_on_failure":false,
        "effects":{"fluid_relocation":{
            "subtype":5,"staging_dx":0,"staging_dy":1,"destination":DESTINATION,
            "source_effect":1,"destination_effect":18,"speech":"Muahahahaha.."
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/fluids.lua:151-184"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    assert!(owner.states.quest_callbacks.fixture.set(vec![callback(TARGET)?]).is_ok());
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = owner.current_quest_fence(session).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let member_session = GameSessionId::decode(&id(51)).map_err(debug)?;
    let member = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(member_session).map_err(debug)?;
        let member = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let staging = super::quest_world_callbacks::local(STAGING);
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), staging);
        runtime.initialize_pinned_test_position(member, staging).map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.initialize(
            &runtime, member, member_session,
            crate::gameplay_transport::actor_spell::tests::FACTS,
            (3, 7_000_000), owner.owner_now()
        ).is_some());
        member
    };
    super::quest_world_callbacks::input_item(
        owner, fence.item_fence(),
        &Offering { position:TARGET,item:VIAL.into(),count:1 },
        956,true
    ).await?;
    {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        let inserted = sqlx::query(
            "INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id)
             SELECT item_instance_id,world_id,5,game_character_uuid_v7() FROM game_item_instances
             WHERE definition_production_key=$1 AND lifecycle=1",
        ).bind(VIAL).execute(&mut *tx).await?;
        assert_eq!(inserted.rows_affected(),1);
        tx.commit().await?;
    }
    let reference = owner.active_generation.and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item",VIAL,"definition-r1"))
        .ok_or("vial definition")?;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor,TARGET.x,TARGET.y,1).ok_or("target")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::new(3).ok_or("handle")?),
        use_with:None,
        use_with_map_item:Some(MapUseTarget::Object(WorldObjectTarget {
            placement,expected_revision:map.facts.object_revision(key),
        })),
        use_with_position:None,
    }).map_err(debug)?;
    let (next, disposition, _) = super::item_craft::exchange(
        owner,*admitted,admitted.continuity.next_command_id,&payload,Some(reference)
    ).await?;
    *admitted=next;
    assert_eq!(disposition,UseDisposition::Committed);
    {
        let runtime = owner.runtime.lock().await;
        for who in [actor, member] {
            assert_eq!(runtime.read_actor_position(who).map_err(debug)?.position(), super::quest_world_callbacks::local(DESTINATION));
        }
    }
    let consumed: (i64,i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_fluid_states),
                (SELECT count(*) FROM game_item_fluid_splashes)"
    ).fetch_one(&h.pool).await?;
    assert_eq!(consumed,(0,0));
    Ok(())
}
