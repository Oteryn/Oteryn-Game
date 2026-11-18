//! Crystal 201189f8 data-global/scripts/quests/targuna/creaturescripts_herald_of_fire.lua:4-7,51-59.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
use crate::gameplay_transport::kill_reward::{self, KillSink};
use oteryn_gameplay_quests::quest::{
    source_deaths::DeathPortal,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

pub(super) const NAME: &str = "qewrath-herald-escape";
const PORTAL: Position = Position {
    x: 32496,
    y: 32661,
    floor: -15,
};
const DESTINATION: Position = Position {
    x: 32418,
    y: 32686,
    floor: -13,
};
const APPEARANCE: u16 = 25051;
const HERALD: &str = "oteryn:creature.herald_of_fire";

#[test]
fn herald_death_reveals_one_traversable_escape_portal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PORTAL, false)
    })
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_objects(
        callback,
        PORTAL,
        &[(DESTINATION, 100)],
        &[APPEARANCE],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let binding: DeathPortal = serde_json::from_value(serde_json::json!({
        "creature":HERALD,
        "death_minimum":{"x":32487,"y":32649,"floor":-15},
        "death_maximum":{"x":32504,"y":32664,"floor":-15},
        "position":PORTAL,"destination":DESTINATION,"appearance":APPEARANCE,"effect":11,
    }))?;
    let bindings = [binding];
    let tile = |p: Position| crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let map = owner.bundle_map().ok_or("map")?;
    let queue = kill_reward::KillSettlementQueue::default();
    let timestamp = owner.owner_now();
    let now = timestamp.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        )
        .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        let at = super::quest_world_callbacks::local(Position {
            x: PORTAL.x + 1,
            ..PORTAL
        });
        let health = runtime
            .companion_policy(HERALD)
            .map_err(debug)?
            .maximum_health;
        let boss = runtime
            .admit_encounter_actor(HERALD, health, at)
            .map_err(debug)?;
        runtime
            .install_creature_policy(boss, HERALD)
            .map_err(debug)?;
        let hit = |runtime: &mut crate::foundation::ChannelRuntimeV1, id, damage| -> TestResult {
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    boss,
                    actor,
                    CommandRef::new(session, CommandId::new(id).map_err(debug)?),
                    0,
                    OwnerDamageCommand {
                        target: HERALD.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"qewrath-portal-death",
                        damage,
                    },
                )
                .map_err(debug)?;
            Ok(())
        };
        let credit = crate::combat::CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        };
        let sink = KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: owner.reward_table(),
            charms: None,
        };
        hit(&mut runtime, 9100, 1)?;
        kill_reward::record_creature_kill_with_portals(
            &mut runtime,
            sink,
            boss,
            credit,
            now,
            Some(timestamp),
            Some((map, &bindings)),
        );
        assert_eq!(map.facts.quest_portal_destination(tile(PORTAL), 0), None);
        hit(&mut runtime, 9101, health - 1)?;
        for _ in 0..2 {
            kill_reward::record_creature_kill_with_portals(
                &mut runtime,
                sink,
                boss,
                credit,
                now,
                Some(timestamp),
                Some((map, &bindings)),
            );
        }
    }
    assert_eq!(
        map.facts.quest_portal_destination(tile(PORTAL), now * 1000),
        Some(tile(DESTINATION))
    );
    let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile(PORTAL))
        .map_err(debug)?
        .ok_or("portal tile")?;
    assert_eq!(
        stack
            .entries
            .iter()
            .filter(|entry| entry.facts.appearance_id == APPEARANCE)
            .count(),
        1
    );
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    let landing = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(landing, super::quest_world_callbacks::local(DESTINATION));
    Ok(())
}
