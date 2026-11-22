use super::TestResult;
use super::party_xp_fixtures::{self, Case};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_foundation::scheduled_events::Schedule;

const START: i64 = 1_893_456_000;

fn schedule() -> TestResult<Schedule> {
    Schedule::parse(
        r#"{"events":[{"name":"Exaltation Overload","startdate":"01/01/2030",
            "enddate":"01/01/2030","ingame":{"exaltation_overload":"october_2026"}}]}"#,
    )
    .map_err(super::debug)
}

#[test]
fn exaltation_overload_awards_fifty_percent_base_xp_per_fiendish_stack_through_real_kill()
-> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    party_xp_fixtures::run(Case::ExaltationOverload)
}

#[test]
fn exaltation_overload_doubles_forge_population_rates_and_replaces_fiends_after_five_seconds()
-> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        use super::{ChannelRuntimeV1, Harness, debug};
        use crate::domain::forge_creature::ForgeCreature;
        use oteryn_gameplay_foundation::scheduled_events::test_clock;

        let harness =
            Harness::create_with_admission_seed(admin, "overload_spawns", false, false).await?;
        let mut runtime = event_runtime(&harness)?;
        let rat = "oteryn:creature.rat";
        let health = u64::try_from(runtime.companion_policy(rat).map_err(debug)?.maximum_health)?;
        runtime
            .realize_activation_spawns(vec![(
                (2..14).map(|x| (x, 0, 0)).collect(),
                60_000,
                5_000,
                rat.into(),
                health,
            )])
            .map_err(debug)?;

        let classify = |runtime: &ChannelRuntimeV1| {
            runtime
                .spawn_point_creatures()
                .into_iter()
                .filter_map(|(_, _, actor)| {
                    let actor = actor?;
                    Some((actor, runtime.companion_snapshot(actor).ok()?.forge?))
                })
                .collect::<Vec<_>>()
        };
        let _clock = test_clock::set(schedule()?, START - 1);
        runtime
            .drive_spawn_respawns(10_000_000, |_, _| false)
            .map_err(debug)?;
        assert_eq!(classify(&runtime).len(), 4);
        let _clock = test_clock::set(schedule()?, START);
        runtime
            .drive_spawn_respawns(20_000_000, |_, _| false)
            .map_err(debug)?;
        let classified = classify(&runtime);
        let fiends = classified
            .iter()
            .filter(|(_, kind)| *kind == ForgeCreature::Fiendish)
            .count();
        assert_eq!((fiends, classified.len() - fiends), (6, 3));

        let dead = classified
            .iter()
            .find(|(_, kind)| *kind == ForgeCreature::Fiendish)
            .ok_or("fiend")?
            .0;
        runtime.set_test_creature_health(dead, 0).map_err(debug)?;
        runtime
            .drive_spawn_respawns(21_000_000, |_, _| false)
            .map_err(debug)?;
        let live_fiends = |runtime: &ChannelRuntimeV1| {
            classify(runtime)
                .into_iter()
                .filter(|(_, kind)| *kind == ForgeCreature::Fiendish)
                .count()
        };
        runtime
            .drive_spawn_respawns(25_999_999, |_, _| false)
            .map_err(debug)?;
        assert_eq!(live_fiends(&runtime), 5);
        runtime
            .drive_spawn_respawns(26_000_000, |_, _| false)
            .map_err(debug)?;
        assert_eq!(live_fiends(&runtime), 6);
        harness.cleanup().await
    })
}
fn event_runtime(harness: &super::Harness) -> TestResult<super::ChannelRuntimeV1> {
    use super::{ChannelRuntimeV1, debug, id};
    use crate::foundation::{ChannelId, WorldId};

    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let (generation, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        harness.node.fact().node_id(),
        harness.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        32,
        pin.into_channel_pin(),
    )
    .map_err(debug)?;
    generation
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("active native content")?
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    Ok(runtime)
}

#[test]
fn exaltation_overload_january_profile_triggers_onslaught_on_an_untiered_swing() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        use super::{Harness, debug, id};
        use crate::foundation::{CommandId, CommandRef, GameSessionId};
        use crate::gameplay_transport::attack::equipped_offence::critical_swing;
        use oteryn_gameplay_combat_rules::resolution::{
            equipped_offence::EquippedOffence, swing_draw,
        };
        use oteryn_gameplay_foundation::scheduled_events::test_clock;
        use oteryn_simulation_determinism::GameplayDecisionRoot;

        let harness =
            Harness::create_with_admission_seed(admin, "overload_onslaught", false, false).await?;
        let mut runtime = event_runtime(&harness)?;
        let session = GameSessionId::decode(&id(70)).map_err(debug)?;
        let actor = runtime
            .reserve_fresh_session(session)
            .and_then(|r| runtime.commit_fresh_session(r))
            .map_err(debug)?;
        let command = CommandRef::new(session, CommandId::new(1).map_err(debug)?);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let ordinal = (0..u16::MAX)
            .find(|ordinal| {
                swing_draw(
                    &root,
                    actor.placement_identity(),
                    (*session.as_bytes(), 1),
                    *ordinal,
                    b"fatal-hit",
                    "attack.fatal-hit",
                    0,
                    10_000,
                )
                .is_some_and(|roll| roll < 400)
            })
            .ok_or("onslaught draw")?;
        let swing = || {
            critical_swing(
                &EquippedOffence::default(),
                &root,
                actor,
                command,
                ordinal,
                100,
                10_000,
            )
        };
        {
            let _clock = test_clock::set(schedule()?, START);
            assert_eq!(swing(), Some(100));
        }
        {
            let schedule = Schedule::parse(
                r#"{"events":[{"name":"Exaltation Overload",
                "startdate":"01/01/2030","enddate":"01/01/2030",
                "ingame":{"exaltation_overload":"january_2026"}}]}"#,
            )
            .map_err(debug)?;
            let _clock = test_clock::set(schedule, START);
            assert_eq!(swing(), Some(160));
        }
        harness.cleanup().await
    })
}

#[test]
fn exaltation_overload_october_profile_dodges_a_native_attack_without_armor_tier() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    use crate::gameplay_transport::attack::{
        ChannelAttackStates, charm_combat::source_vitals::CharmSourceVitals,
    };
    use crate::player_lethal::PlayerLethalVitals;
    use oteryn_gameplay_foundation::scheduled_events::test_clock;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
    use sha2::{Digest, Sha256};

    let (mut fixture, source) =
        super::all_charms_contracts_fixtures::enrage_fixture_with_identity(1000, None)?;
    let occurrence = (0_u32..10_000)
        .map(|ordinal| format!("overload-incoming-{ordinal}"))
        .find(|text| {
            let hash = Sha256::new()
                .chain_update(b"oteryn.charm.source.v1")
                .chain_update(text.as_bytes())
                .finalize();
            let mut identity = [0; 16];
            identity.copy_from_slice(&hash[..16]);
            deterministic_decision_u64(
                &fixture.root,
                DecisionOccurrenceId::from_bytes(identity),
                "attack.ruse",
                0,
            )
            .is_ok_and(|draw| draw % 10_001 < 400)
        })
        .ok_or("ruse draw")?;
    let mut attack = ChannelAttackStates::default();
    let vitals = CharmSourceVitals {
        states: &mut fixture.states,
        attack: &mut attack,
        catalogue: &fixture.catalogue,
        root: fixture.root,
    };
    let dodges = || {
        vitals.dodge_source_attack(
            &fixture.runtime,
            source,
            fixture.actor,
            fixture.session,
            &occurrence,
        )
    };
    {
        let _clock = test_clock::set(schedule()?, START - 1);
        assert!(!dodges());
    }
    {
        let _clock = test_clock::set(schedule()?, START);
        assert!(dodges());
    }
    Ok(())
}

#[test]
fn exaltation_overload_october_transcendence_uses_the_avatar_owner_and_expires() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    use crate::gameplay_transport::attack::ChannelAttackStates;
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_foundation::scheduled_events::test_clock;
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};

    let mut f = super::all_charms_contracts_fixtures::fixture()?;
    let digest = f.runtime.content_pin().server_artifact_digest();
    let root = GameplayDecisionRoot::from_bytes(digest);
    let slot = (1_u64..10_000)
        .find(|slot| {
            let hash = Sha256::new()
                .chain_update(b"oteryn:forge-avatar:v1")
                .chain_update(f.session.as_bytes())
                .chain_update(f.actor.placement_identity())
                .chain_update(slot.to_be_bytes())
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&hash[..16]);
            deterministic_decision_u64(
                &root,
                DecisionOccurrenceId::from_bytes(occurrence),
                "attack.transcendence",
                0,
            )
            .is_ok_and(|draw| (100..150).contains(&(draw % 10_001)))
        })
        .ok_or("October-only Transcendence roll")?;
    let now = slot * 2_000_000;
    let timing = SemanticTimeMicros::from_micros(now);
    let state = f
        .states
        .get_mut(&f.runtime, f.actor, f.session)
        .ok_or("player")?;
    let attack = ChannelAttackStates::default();
    let _clock = test_clock::set(schedule()?, START);
    attack.trigger_forge_avatar(state, f.actor, f.session, timing, &digest);
    assert_eq!(
        state.owned_conditions().outfit_at(now),
        None,
        "no aggression"
    );
    state.record_forge_spell_attack(now);
    attack.trigger_forge_avatar(state, f.actor, f.session, timing, &digest);
    assert!(state.owned_conditions().outfit_at(now).is_some());
    assert!(matches!(
        state.owned_conditions().attributes_at(now),
        Some(ConditionValues::Attributes {
            critical_chance_percent: 100,
            critical_extra_percentage_points: 15,
            incoming_reduction_percent: 15,
            ..
        })
    ));
    let repeat = SemanticTimeMicros::from_micros(now + 1_000);
    attack.trigger_forge_avatar(state, f.actor, f.session, repeat, &digest);
    assert!(
        state
            .owned_conditions()
            .outfit_at(now + 6_999_999)
            .is_some()
    );
    assert_eq!(state.owned_conditions().outfit_at(now + 7_000_000), None);
    Ok(())
}
