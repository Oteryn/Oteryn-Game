//! Crystal 201189f8 data-global/scripts/quests/wrath_of_the_emperor/creaturescripts_zalamon_kill.lua:1-42,
//! actions_mission11_payback_time_lever.lua:1-48.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{debug, id},
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, GameSessionId, OwnerDamageCommand,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::kill_reward::{self, KillSink};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::json;

pub(super) const PREFIX: &str = "qewrath-payback-";
const LEVER: Position = Position {
    x: 33358,
    y: 31404,
    floor: -9,
};
const TEAM_PAD: Position = Position {
    x: 33358,
    y: 31405,
    floor: -9,
};
const LANDING: Position = Position {
    x: 33359,
    y: 31406,
    floor: -10,
};
const FIGHT: Position = Position {
    x: 33365,
    y: 31407,
    floor: -10,
};
const GUARD: &str = "oteryn:creature.plaguethrower";
const FORMS: [&str; 4] = [
    "oteryn:creature.snake_god_essence",
    "oteryn:creature.snake_thing",
    "oteryn:creature.lizard_abomination",
    "oteryn:creature.mutated_zalamon",
];

#[test]
fn payback_advances_exact_deaths_and_releases_the_room_on_victory_or_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("qewrath-payback", || {
        for name in ["qewrath-payback-victory", "qewrath-payback-timeout"] {
            super::quest_world_callbacks::scenario(name, LEVER, false)?;
        }
        Ok(())
    })
}

fn fixture() -> TestResult<Callback> {
    let row = json!({
        "key":"oteryn:quest-callback/wrath/payback-forms", "quest":"oteryn:quest.wrath_of_the_emperor_quest",
        "trigger":"use", "selector":{"position":LEVER,"appearance":2772},
        "lever":[2772,2773], "reversible":true, "passages":[], "offerings":[],
        "storage":[], "return_on_failure":false,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/creaturescripts_zalamon_kill.lua:1-42"],
        "effects":{"encounter":{
            "minimum_level":0,
            "players":[
                {"from":{"x":33357,"y":31404,"floor":-9},"to":LANDING},
                {"from":TEAM_PAD,"to":LANDING}
            ],
            "monsters":[
                {"creature":FORMS[0],"position":FIGHT},
                {"creature":GUARD,"position":{"x":33355,"y":31403,"floor":-10}},
            ],
            "room_min":{"x":33349,"y":31396,"floor":-10},
            "room_max":{"x":33369,"y":31416,"floor":-10},
            "phase":"linear_death_chain",
            "linear_phases":{"forms":FORMS,"speeches":[
                "IT'S NOT THAT EASY MORTALS! FEEL THE POWER OF THE GOD!",
                "NOOO! NOW YOU HERETICS WILL FACE MY GODLY WRATH!",
                "YOU ... WILL ... PAY WITH ETERNITY ... OF AGONY!"
            ],"reservation_ms":600000},
        }}
    });
    use oteryn_gameplay_quests::quest::authored_bindings::{Bindings, Trigger};
    use sha2::{Digest, Sha256};
    let recipe = serde_json::to_vec(&row)?;
    let digest = Sha256::digest(&recipe)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{
            "quest":row["quest"],"recipe_sha256":digest,"sources":row["sources"],
            "tracks":[{"key":"oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline",
                "initial":0,"minimum":0,"maximum":34}],
            "stages":[{"key":"payback","completes":false,"trigger":{"kind":"map_use","callback":row}}]
        }]
    }))?)?;
    match &bindings.quests[0].stages[0].trigger {
        Trigger::MapUse { callback } => Ok(*callback.clone()),
        _ => Err("payback binding".into()),
    }
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world(&fixture()?, LEVER)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    timeout: bool,
) -> TestResult {
    let callback = fixture()?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 1)
        .ok_or("lever key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .is_ok()
    );
    let actor = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let lease = CharacterLease::new(
        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    let partner_session = GameSessionId::decode(&id(51)).map_err(debug)?;
    let partner = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(partner_session)
            .map_err(debug)?;
        let partner = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_source_pinned_lab_player_position(
                partner,
                partner_session,
                super::quest_world_callbacks::local(TEAM_PAD),
            )
            .map_err(debug)?;
        let partner_lease =
            CharacterLease::new(CharacterId::decode(&id(52)).map_err(debug)?, 1).map_err(debug)?;
        runtime
            .bind_attacker_lease(partner, partner_session, partner_lease)
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                partner,
                partner_session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("partner spell state")?;
        partner
    };
    let admitted_at = owner.owner_now().get();
    let outcome = owner
        .use_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 1,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: 0,
            },
        )
        .await;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    let queue = kill_reward::KillSettlementQueue::default();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let find = |runtime: &crate::foundation::ChannelRuntimeV1, key: &str| {
        runtime.positioned_melee_census().ok().and_then(|actors| {
            actors.into_iter().find(|(actor, _, _)| {
                runtime.matches_live_creature_identity(*actor, key.as_bytes())
            })
        })
    };
    for member in [actor, partner] {
        assert_eq!(
            runtime
                .read_actor_position(member)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(LANDING)
        );
    }
    assert!(find(&runtime, FORMS[0]).is_some());
    let original_guard = find(&runtime, GUARD).ok_or("opening guard")?.0;
    let original_boss = find(&runtime, FORMS[0]).ok_or("opening boss")?.0;
    if !timeout {
        for (index, creature) in FORMS.iter().enumerate() {
            let (target, position, _) = find(&runtime, creature).ok_or("current form")?;
            assert_eq!(
                position.position(),
                super::quest_world_callbacks::local(FIGHT)
            );
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            let time = owner.owner_now();
            let now = time.get() / 1_000;
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(
                        session,
                        CommandId::new(10 + u64::try_from(index)?).map_err(debug)?,
                    ),
                    0,
                    OwnerDamageCommand {
                        target: creature.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"qewrath-payback",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            let sink = KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            };
            for _ in 0..2 {
                kill_reward::record_creature_kill_with_portals(
                    &mut runtime,
                    sink,
                    target,
                    crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session,
                        actor,
                    },
                    now,
                    Some(time),
                    kill_reward::portal_context(Some(map)),
                );
            }
        }
        assert_eq!(
            map.facts.movement_speech_since(0).len(),
            3,
            "one arrival cue per successor"
        );
    }
    let reset_at = if timeout {
        admitted_at + 601_000_000
    } else {
        owner.owner_now().get()
    };
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &states, reset_at);
    assert!(runtime.contains_live_creature(original_guard));
    assert_eq!(runtime.contains_live_creature(original_boss), timeout);
    for (member, member_session) in [(actor, session), (partner, partner_session)] {
        let at = runtime
            .read_actor_position(member)
            .map_err(debug)?
            .position();
        assert!(!states.encounter_phases.blocks_channel_switch(
            &runtime,
            member,
            member_session,
            at
        ));
    }
    assert!(
        states
            .encounter_phases
            .active
            .lock()
            .map_err(|_| "phase registry")?
            .is_empty()
    );

    // Crystal's spectator scan gathers both characters on the same source tile.
    // The next USE must retire the old room and admit both stacked players.
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let pad = super::quest_world_callbacks::local(Position { x: 33357, ..LEVER });
    for (member, member_session) in [(actor, session), (partner, partner_session)] {
        let expected = runtime.read_actor_position(member).map_err(debug)?;
        let teleport = crate::map::floor::prepare_quest_encounter_ejection(
            &runtime,
            owner.movement_cells(),
            member,
            member_session,
            expected,
            pad,
            &closed,
        )
        .map_err(debug)?;
        runtime.commit_source_step(teleport).map_err(debug)?;
    }
    drop(states);
    drop(runtime);
    let restarted = owner
        .use_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 99,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: 1,
            },
        )
        .await;
    assert_eq!(restarted.disposition, UseDisposition::Committed);
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(original_guard));
    assert!(!runtime.contains_live_creature(original_boss));
    assert!(find(&runtime, FORMS[0]).is_some());
    assert!(find(&runtime, GUARD).is_some());
    for member in [actor, partner] {
        assert_eq!(
            runtime.read_actor_position(member).map_err(debug)?.position(),
            super::quest_world_callbacks::local(LANDING)
        );
    }
    Ok(())
}
