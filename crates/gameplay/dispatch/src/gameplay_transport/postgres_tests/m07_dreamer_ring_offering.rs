//! Literal UID2244 on synthetic geometry: a real item drop followed by UID9234 STEP.
use super::TestResult;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    dreamer_death_ring,
    world_callbacks::{Offering, Position},
};
#[test]
fn m07_ring_real_additem_consumes_once_then_step_resets_shared_trees() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-ring-offering", || {
        super::quest_world_callbacks::scenario(
            "m07-ring-offering",
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
) -> TestResult {
    use oteryn_protocol_oteryn::{
        item_view::{ItemMoveOutcome, WorldTilePosition},
        world_spatial::{StepDirection, StepDisposition, decode_step_result, encode_step_intent},
    };
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                12,
                14,
                15,
                18,
                oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
            ],
        )
        .ok_or("drop capabilities")?;
    let drop = Position {
        x: 99,
        y: 101,
        floor: -11,
    };
    let destination = WorldTilePosition {
        x: 99,
        y: 101,
        floor: -11,
    };
    let keys = dreamer_death_ring::TREES.map(|p| {
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("tree key")
    });
    let keys = [keys[0]?, keys[1]?, keys[2]?];
    for (key, command) in [
        ("oteryn:item.tibia.i3595", 1),
        ("oteryn:item.tibia.i6299", 2),
        ("oteryn:item.tibia.i6299", 3),
    ] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: drop,
                item: key.into(),
                count: 1,
            },
            59 + command,
            true,
        )
        .await?;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let reference = native
            .item_index()
            .ok_or("index")?
            .definition_ref("Item", key, "definition-r1")
            .ok_or("reference")?;
        let cursor = owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |e| e.0);
        let (next, result, payload) = super::map_seed_planting::drop_exchange_at(
            owner,
            *admitted,
            command,
            None,
            reference,
            1,
            false,
            destination,
        )
        .await?;
        *admitted = next;
        assert_eq!(result, Some(ItemMoveOutcome::Moved));
        let map = owner.bundle_map().ok_or("map")?;
        for k in keys {
            assert_eq!(map.facts.base_hidden(k), command >= 2);
        }
        let cues = map.facts.movement_effects_since(cursor);
        assert_eq!(
            cues.iter().filter(|e| e.2 == 13).count(),
            if command == 2 { 3 } else { 0 }
        );
        if command >= 2 {
            let rows:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_dreamer_ring_offerings),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6299' AND lifecycle=1)").fetch_one(&h.pool).await?;
            assert_eq!(rows, if command == 2 { (1, 0) } else { (1, 1) });
        }
        if command == 2 {
            let cursor = cues.last().map_or(cursor, |e| e.0);
            let (next, replay, _) = super::map_seed_planting::drop_exchange_at(
                owner,
                *admitted,
                command,
                Some(payload),
                reference,
                1,
                false,
                destination,
            )
            .await?;
            *admitted = next;
            assert_eq!(replay, result);
            assert!(
                owner
                    .bundle_map()
                    .ok_or("map")?
                    .facts
                    .movement_effects_since(cursor)
                    .is_empty()
            );
        }
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let payload = encode_step_intent(StepDirection::East);
    let reply = super::quest_world_callbacks::wire(owner, admitted, 4, 1, payload).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(super::fixture::debug)?,
        StepDisposition::Moved
    );
    for key in keys {
        assert!(!owner.bundle_map().ok_or("map")?.facts.base_hidden(key));
    }
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?
            .position(),
        super::quest_world_callbacks::local(dreamer_death_ring::DESTINATION)
    );
    let atomic:bool=sqlx::query_scalar("SELECT r.created_xact_id=m.created_xact_id AND r.created_xact_id=a.created_xact_id AND a.envelope_sha256=sha256(a.envelope) FROM game_dreamer_ring_offerings r JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id JOIN game_item_audit_outbox a ON a.transaction_id=r.transaction_id").fetch_one(&h.pool).await?;
    assert!(atomic);
    // A live ring on a historical ordinary drop is not a fresh ADDITEM cause.
    let forged=sqlx::query("INSERT INTO game_dreamer_ring_offerings(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,trees_before,map_revision,content_digest,occurred_at,envelope_sha256) SELECT game_character_uuid_v7(),game_character_uuid_v7(),m.transaction_id,m.game_session_id,m.command_id,m.character_id,m.world_id,m.channel_id,i.item_instance_id,to_jsonb(i),to_jsonb(g),m.intent,r.placement,r.reset_epoch,r.object_revision,r.trees_before,r.map_revision,r.content_digest,r.occurred_at,r.envelope_sha256 FROM game_dreamer_ring_offerings r CROSS JOIN game_item_loop_move_receipts m JOIN game_item_loop_move_lines l ON l.transaction_id=m.transaction_id JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id WHERE m.command_id=3 AND l.to_kind=3 AND i.definition_production_key='oteryn:item.tibia.i6299' AND i.lifecycle=1").execute(&h.pool).await;
    assert!(
        matches!(forged,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514")),
        "historical drop forged offering: {forged:?}"
    );

    let tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(tracks, 0);
    eprintln!(
        "M07 POSTGRES_CASE_COMPLETED Death Ring real_ADDITEM wrong_input preserved absent_trees preserved atomic_ring_debit replay real_STEP shared_reset"
    );
    Ok(())
}
