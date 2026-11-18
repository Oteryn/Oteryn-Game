//! The source lever admits the four exact death generations and reopens the arena.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn wrath_payback_real_lever_chains_native_deaths_and_expires_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::engenc2_cults::cults_scenario(false, true)
}

pub(super) fn callback() -> TestResult<Callback> {
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|row| row.key == "oteryn:quest-callback/wrath/payback")
        .ok_or("production Payback arena binding")?;
    assert_eq!(callback.selector.unique, Some(3198));
    callback.selector.position = Some(Position {
        x: 100,
        y: 100,
        floor: -7,
    });
    callback.selector.appearance = Some(2772);
    let encounter = callback
        .effects
        .encounter
        .as_mut()
        .ok_or("Payback encounter")?;
    for member in &mut encounter.players {
        member.from = Position {
            x: member.from.x - 33357 + 100,
            y: member.from.y - 31404 + 100,
            floor: -7,
        };
        member.to = Position {
            x: 120,
            y: 120,
            floor: -7,
        };
    }
    for monster in &mut encounter.monsters {
        monster.position = Position {
            x: monster.position.x - 33349 + 110,
            y: monster.position.y - 31396 + 110,
            floor: -7,
        };
    }
    encounter.room_min = Position {
        x: 110,
        y: 110,
        floor: -7,
    };
    encounter.room_max = Position {
        x: 130,
        y: 130,
        floor: -7,
    };
    Ok(callback)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    placement: &[u8],
    key: u64,
    partner: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let partner_actor = partner.runtime_actor.ok_or("partner")?;
    for member in [actor, partner_actor] {
        let runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(member)
            .map_err(debug)?
            .position();
        assert_eq!((at.x, at.y, at.floor), (120, 120, 7));
    }
    let forms = [
        "oteryn:creature.snake_god_essence",
        "oteryn:creature.snake_thing",
        "oteryn:creature.lizard_abomination",
        "oteryn:creature.mutated_zalamon",
    ];
    for (ordinal, form) in forms.into_iter().enumerate() {
        let target = {
            let runtime = owner.runtime.lock().await;
            let actors = runtime.current_live_creature_census().map_err(debug)?;
            let matching = actors
                .into_iter()
                .filter(|actor| runtime.matches_live_creature_identity(*actor, form.as_bytes()))
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), 1, "{form}");
            matching[0]
        };
        super::engenc2_cults::kill(owner, admitted, target, form, 100 + u64::try_from(ordinal)?)
            .await?;
    }
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert!(
            states
                .encounter_phases
                .payback
                .lock()
                .map_err(debug)?
                .is_none()
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        for (seat, y) in [(admitted, 100), (&partner, 101)] {
            let member = seat.runtime_actor.ok_or("member")?;
            let before = runtime.read_actor_position(member).map_err(debug)?;
            runtime
                .relocate_arena_player(
                    member,
                    seat.game_session_id,
                    before,
                    crate::foundation::MovementLocalPosition {
                        x: 100,
                        y,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        }
    }
    let map = owner.bundle_map().ok_or("map")?;
    let outcome = owner
        .use_object(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 200,
                item_fence: admitted.item_fence,
            },
            WorldObjectTarget {
                placement: placement.to_vec(),
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(601))
        .ok_or("encounter deadline clock")?;
    owner.drain_quest_encounter_deadlines().await;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert!(
        states
            .encounter_phases
            .payback
            .lock()
            .map_err(debug)?
            .is_none()
    );
    assert!(
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    Ok(())
}
