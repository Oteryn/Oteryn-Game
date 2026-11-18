//! Real StepIn and durable credit for both attackers in the Plagirath room.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const NAME: &str = "bosscred-ferumbras-plagirath";
pub(super) const PORTAL: Position = Position {
    x: 33174,
    y: 31511,
    floor: -13,
};
const WEST: Position = Position { x: 33173, ..PORTAL };
const WAIT: Position = Position { y: 31512, ..PORTAL };
const BOSS: Position = Position {
    x: 33172,
    y: 31501,
    floor: -13,
};
const EXIT: Position = Position {
    x: 33204,
    y: 31510,
    floor: -13,
};
const CREATURE: &str = "oteryn:creature.plagirath";
const CREDIT: &str = "oteryn:quest-progress/ferumbras-ascension/defeated-at/plagirath";

#[test]
fn bosscred_ferumbras_plagirath_credits_both_attackers_and_expires_exit() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PORTAL, false)
    })
}

pub(super) fn fixture_callback(mut callback: Callback) -> Callback {
    callback.selector.position = Some(PORTAL);
    callback.selector.unique = None;
    callback.selector.appearance = Some(1949);
    callback.teleport = Some(EXIT);
    callback.effects.transform = Some(22761);
    callback
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_cells(callback, PORTAL, &[WEST, BOSS, EXIT, WAIT])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    first: &mut AdmittedSession,
    deps: &Dependencies,
    harness: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::local;
    super::questpc7_native_deaths::prepare_native_hunter(owner, first).await?;
    let (actor, mut second) =
        super::grizzly_boss::vacancy::second_player(owner, deps, harness, local(WAIT)).await?;
    assert!(
        owner
            .refresh_source_party_presence(actor, second.game_session_id)
            .await
    );
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let hp = runtime
            .companion_policy(CREATURE)
            .map_err(debug)?
            .maximum_health;
        let target = runtime
            .admit_encounter_actor(CREATURE, hp, local(BOSS))
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, CREATURE)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(owner.owner_now().get() / 1000)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(
                    second.game_session_id,
                    CommandId::new(31_600).map_err(debug)?,
                ),
                0,
                OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: second.game_session_id.as_bytes(),
                    binding: b"bosscred-plagirath-damage",
                    damage: 1,
                },
            )
            .map_err(debug)?;
        target
    };
    super::questpc7_native_deaths::lethal_creature(owner, first, target, 31_601, CREATURE).await?;
    let first_session = first.game_session_id;
    for player in [&mut *first, &mut second] {
        assert!(super::questpc7_ferumbras::value(owner, player, CREDIT).await? > 0);
        let actor = player.runtime_actor.ok_or("player actor")?;
        super::grizzly_boss::vacancy::teleport(owner, actor, player.game_session_id, local(WEST))
            .await?;
        super::questpc7_ferumbras::step(owner, player, StepDirection::East).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
        super::grizzly_boss::vacancy::teleport(
            owner,
            actor,
            player.game_session_id,
            local(if player.game_session_id == first_session {
                BOSS
            } else {
                WAIT
            }),
        )
        .await?;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(60))
        .ok_or("clock origin")?;
    let map = owner.bundle_map().ok_or("portal map")?;
    assert!(
        map.facts
            .quest_portal_destination(
                crate::map::overlay::TilePos {
                    x: PORTAL.x,
                    y: PORTAL.y,
                    floor: PORTAL.floor,
                },
                owner.owner_now().get()
            )
            .is_none()
    );
    for (player, parking) in [(&mut *first, BOSS), (&mut second, WAIT)] {
        let actor = player.runtime_actor.ok_or("player actor")?;
        super::grizzly_boss::vacancy::teleport(owner, actor, player.game_session_id, local(WEST))
            .await?;
        super::questpc7_ferumbras::step(owner, player, StepDirection::East).await?;
        assert_ne!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
        super::grizzly_boss::vacancy::teleport(
            owner,
            actor,
            player.game_session_id,
            local(parking),
        )
        .await?;
    }
    Ok(())
}
