//! Crystal 201189f8 world bindings through dispatched STEP and USE.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position, Trigger};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, decode_step_result, encode_step_intent,
};

pub(super) const SATCHEL: &str = "galthen-satchel";
pub(super) const COOLDOWN: &str = "galthen-yselda-shortcut-1-cooldown";
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
fn galthen_live_travel_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (name, source, _) in ROUTES {
        super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, source, false))?;
    }
    for (name, source) in [
        (COOLDOWN, ROUTES[5].1),
        ("galthen-forest-1-low-level", ROUTES[0].1),
        ("galthen-iksupan-entrance-low-level", ROUTES[2].1),
    ] {
        super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, source, false))?;
    }
    Ok(())
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
    if name == COOLDOWN {
        sqlx::query(
            "INSERT INTO game_character_boss_cooldowns
               (character_id,boss_key,available_at,admission,reset_epoch)
             VALUES(encode($1,'hex')::uuid,'oteryn:boss.megasylvan_yselda',
               floor(extract(epoch FROM clock_timestamp())*1000)::bigint+600000,
               encode($2,'hex')::uuid,0)",
        )
        .bind(super::fixture::id(41).as_slice())
        .bind(super::fixture::id(81).as_slice())
        .execute(&h.pool)
        .await?;
    }
    let start = at(source.x - 1, source.y, source.floor);
    let expected = if refused {
        start
    } else if name == SATCHEL {
        at(32396, 32520, -7)
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
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        if name == SATCHEL {
            return exercise_satchel(owner, admitted, h, actor, start, expected, target, key).await;
        }
        let effect_before = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
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
        if refused {
            assert!(
                map.facts
                    .movement_effects_since(effect_before)
                    .iter()
                    .any(|(_, at, effect)| *effect == 11
                        && (at.x, at.y, at.floor) == (start.x, start.y, start.floor))
            );
        }
    }
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
