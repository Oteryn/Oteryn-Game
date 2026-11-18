//! Crystal 201189f8 world bindings through dispatched STEP and USE.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position, Trigger};
use oteryn_protocol_oteryn::refusal_detail::{
    CAPABILITY_REFUSAL_DETAIL_V1, RefusalDetail, split_refusal_detail,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_result, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, decode_step_result, encode_step_intent,
};

pub(super) const SATCHEL: &str = "galthen-satchel";
pub(super) const COOLDOWN: &str = "galthen-yselda-shortcut-1-cooldown";
pub(super) const COOLDOWN_EXPIRES_NOW: &str = "galthen-yselda-shortcut-1-cooldown-expires-now";
pub(super) const TARGETED_USE: &str = "-targeted-use";
pub(super) const HOTKEY_TARGETED_USE: &str = "-hotkey-targeted-use";
pub(super) const LOW_LEVEL: &str = "-low-level";
const SATCHEL_RECLAIM_SECONDS: i64 = 30 * 24 * 60 * 60;

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

// Literal coordinates from the travel registrations in the pinned donor.
const ROUTES: [(&str, Position, Position); 7] = [
    (
        "galthen-forest-1",
        at(32424, 32500, -10),
        at(32405, 32497, -10),
    ),
    (
        "galthen-forest-2",
        at(32405, 32498, -10),
        at(32424, 32498, -10),
    ),
    (
        "galthen-iksupan-entrance",
        at(32728, 32878, -7),
        at(34015, 31890, -8),
    ),
    (
        "galthen-sphere-1",
        at(34038, 31726, -10),
        at(34000, 31733, -9),
    ),
    (
        "galthen-sphere-2",
        at(34000, 31732, -9),
        at(34038, 31727, -10),
    ),
    (
        "galthen-yselda-shortcut-1",
        at(32419, 32458, -7),
        at(32528, 32489, -11),
    ),
    (
        "galthen-yselda-shortcut-2",
        at(32528, 32488, -11),
        at(32418, 32458, -7),
    ),
];

#[test]
fn galthen_forest_entry_and_shortcut_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (name, source) in [
        (ROUTES[0].0, ROUTES[0].1),
        (ROUTES[1].0, ROUTES[1].1),
        (ROUTES[5].0, ROUTES[5].1),
        (ROUTES[6].0, ROUTES[6].1),
        (COOLDOWN, ROUTES[5].1),
        (COOLDOWN_EXPIRES_NOW, ROUTES[5].1),
        ("galthen-forest-1-low-level", ROUTES[0].1),
    ] {
        travel(name, source)?;
    }
    Ok(())
}

#[test]
fn galthen_iksupan_direct_use_level_gate_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (name, source) in [
        (ROUTES[2].0, ROUTES[2].1),
        ("galthen-iksupan-entrance-low-level", ROUTES[2].1),
        ("galthen-iksupan-entrance-targeted-use", ROUTES[2].1),
        ("galthen-iksupan-entrance-hotkey-targeted-use", ROUTES[2].1),
    ] {
        travel(name, source)?;
    }
    Ok(())
}

#[test]
fn galthen_iksupan_ungated_spheres_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (name, source, _) in &ROUTES[3..5] {
        travel(name, *source)?;
    }
    Ok(())
}

fn travel(name: &'static str, source: Position) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, source, false))
        .map_err(|error| format!("{name}: {error}").into())
}

#[test]
fn galthen_live_satchel_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(SATCHEL, || {
        scenario(SATCHEL, at(32366, 32542, -8), false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    actor: ExactActorRef,
    name: &str,
) -> TestResult {
    let source = callback.selector.position.ok_or("source tile")?;
    let refused = name.ends_with(LOW_LEVEL) || name == COOLDOWN;
    if matches!(name, COOLDOWN | COOLDOWN_EXPIRES_NOW) {
        const EPOCH: i64 = 1_700_000_000;
        h.root.set_test_quest_clock(EPOCH)?;
        let ready_at = EPOCH * 1000 + if name == COOLDOWN { 1000 } else { 0 };
        sqlx::query(
            "INSERT INTO game_character_boss_cooldowns
               (character_id,boss_key,available_at,admission,reset_epoch)
             VALUES(encode($1,'hex')::uuid,'oteryn:boss.megasylvan_yselda',$2,
               encode($3,'hex')::uuid,0)",
        )
        .bind(super::fixture::id(41).as_slice())
        .bind(ready_at)
        .bind(super::fixture::id(81).as_slice())
        .execute(&h.pool)
        .await?;
    }
    let start = at(source.x - 1, source.y, source.floor);
    let expected = if refused {
        start
    } else if name == SATCHEL {
        at(32396, 32520, -7)
    } else if name.ends_with(TARGETED_USE) {
        start
    } else if name == COOLDOWN_EXPIRES_NOW {
        ROUTES[5].2
    } else {
        ROUTES
            .iter()
            .find(|(key, _, _)| *key == name)
            .ok_or("Galthen route")?
            .2
    };
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)?
            .position(),
        local(start)
    );
    let map = owner.bundle_map().ok_or("map")?;
    let effect_before = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |row| row.0);
    if callback.trigger == Trigger::StepIn {
        let reply = wire(
            owner,
            admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        decode_step_result(&reply).map_err(debug)?;
    } else {
        let key = oteryn_world_bundle::bundle::placement_key(source.floor, source.x, source.y, 1)
            .ok_or("source object")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        if name == SATCHEL {
            return exercise_satchel(owner, admitted, h, actor, start, expected, target, key).await;
        }
        if name.ends_with(TARGETED_USE) {
            return exercise_targeted_use(
                owner,
                admitted,
                h,
                actor,
                start,
                target,
                name.ends_with(HOTKEY_TARGETED_USE),
            )
            .await;
        }
        let outcomes =
            super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(1, &target)])
                .await?;
        assert_eq!(
            outcomes.first().map(|(_, result)| *result),
            Some(if refused {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            })
        );
    }
    assert!(
        map.facts
            .movement_effects_since(effect_before)
            .iter()
            .any(|(_, at, effect)| *effect == 11
                && (at.x, at.y, at.floor) == (expected.x, expected.y, expected.floor)),
        "{name} must emit its teleport effect at the player's final position"
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)?
            .position(),
        local(expected)
    );
    Ok(())
}

async fn exercise_targeted_use(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    start: Position,
    target: WorldObjectTarget,
    hotkey: bool,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("Iksupan item fence")?;
    let rope = Offering {
        position: start,
        item: "oteryn:item.tibia.i3003".into(),
        count: 1,
    };
    input_item(owner, fence, &rope, 1, true).await?;
    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", &rope.item, "definition-r1"))
        .ok_or("Iksupan rope definition")?;

    let map = owner.bundle_map().ok_or("Iksupan map")?;
    let effect_before = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |row| row.0);
    let capabilities: &[u32] = if hotkey {
        &[4, 6, 11, 15, 18, CAPABILITY_REFUSAL_DETAIL_V1]
    } else {
        &[4, 6, 11, 15, 18]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), capabilities)
            .ok_or("Iksupan targeted USE capabilities")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let command_id = admitted.continuity.next_command_id;
    let outcome = if hotkey {
        let reply = wire(
            owner,
            admitted,
            command_id,
            COMMAND_TYPE_USE_INTENT,
            payload,
        )
        .await?;
        let (result, detail) = split_refusal_detail(&reply).map_err(debug)?;
        assert_eq!(
            detail.map(RefusalDetail::reference_message),
            Some("Sorry, not possible."),
        );
        decode_use_result(result).map_err(debug)?
    } else {
        let (next, outcome, _) =
            super::item_craft::exchange(owner, *admitted, command_id, &payload, Some(reference))
                .await?;
        *admitted = next;
        outcome
    };
    assert_eq!(
        outcome,
        UseDisposition::NotPossible,
        "Crystal registers only direct USE for the Iksupan entrance"
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)?
            .position(),
        local(start)
    );
    assert!(map.facts.movement_effects_since(effect_before).is_empty());
    let quantity: i64 = sqlx::query_scalar(
        "SELECT item.quantity FROM game_item_container_entries entry
           JOIN game_item_instances item USING (item_instance_id)
          WHERE entry.character_id=encode($1,'hex')::uuid
            AND item.definition_production_key=$2 AND item.lifecycle=1",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&rope.item)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 1, "refused Iksupan USE retains the carried item");
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the fixture supplies the exact actor, map placement and two tiles"
)]
async fn exercise_satchel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    start: Position,
    exit: Position,
    mut target: WorldObjectTarget,
    key: u64,
) -> TestResult {
    const EPOCH: i64 = 1_700_000_000;
    let map = owner.bundle_map().ok_or("map")?;
    h.root.set_test_quest_clock(EPOCH)?;
    for (id, advance, rewards, last_claim) in [
        (1, 0, 1, EPOCH),
        (2, SATCHEL_RECLAIM_SECONDS, 1, EPOCH),
        (3, 1, 2, EPOCH + SATCHEL_RECLAIM_SECONDS + 1),
    ] {
        if advance > 0 {
            h.root.advance_test_quest_clock(advance)?;
        }
        target.expected_revision = map.facts.object_revision(key);
        let cursor = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
        let outcomes =
            super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(id, &target)])
                .await?;
        assert_eq!(
            outcomes.as_slice(),
            &[(
                oteryn_protocol_oteryn::CommandStatus::Accepted,
                UseDisposition::Committed
            )],
        );
        let splashes = map.facts.movement_effects_since(cursor);
        assert_eq!(splashes.len(), 1, "one watersplash per tree USE");
        let (_, at, effect) = splashes[0];
        assert_eq!(
            (at.x, at.y, at.floor, effect),
            (exit.x, exit.y, exit.floor, 53)
        );
        {
            let mut runtime = owner.runtime.lock().await;
            let at = runtime.read_actor_position(actor)?;
            assert_eq!(at.position(), local(exit));
            if id < 3 {
                let doors = crate::map::floor::BundleDoors(map.facts.doors());
                let return_to_tree = crate::map::floor::prepare_quest_callback_teleport_facing(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    admitted.game_session_id,
                    at,
                    local(start),
                    false,
                    &doors,
                    None,
                )
                .map_err(debug)?;
                runtime.commit_source_step(return_to_tree).map_err(debug)?;
            }
        }
        let nested: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (
                SELECT item_instance_id, parent_item_instance_id FROM game_item_container_entries
                UNION ALL
                SELECT item_instance_id, parent_item_instance_id FROM game_item_nested_entries
             ) edges
             JOIN game_item_instances bag ON bag.item_instance_id=edges.parent_item_instance_id
             JOIN game_item_instances item ON item.item_instance_id=edges.item_instance_id
             WHERE bag.definition_production_key='oteryn:item.tibia.i36813'
                AND item.definition_production_key='oteryn:item.tibia.i36810' AND item.quantity=1",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(nested, rewards);
        let claim: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind("oteryn:quest-progress/galthen/satchel-at")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(claim, last_claim);
    }
    Ok(())
}
