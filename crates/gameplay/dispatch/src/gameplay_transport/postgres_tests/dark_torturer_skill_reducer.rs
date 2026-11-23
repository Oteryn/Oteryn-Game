//! W01c: Dark Torturer's scheduled cone and its retained spell occurrence.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::creature_damage_spell::{SpellCombatFacts, SpellSource, SpellWorldReader};
use crate::foundation::{
    AttributeModifier, ChannelContentPin, ChannelRuntimeV1, ExactActorRef, GameSessionId,
    MovementLocalPosition, RuntimeWorkStamp, WorldId,
};
use crate::gameplay_transport::actor_spell::tests::FACTS;
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::{Duration, Instant};

const CREATURE: &str = "oteryn:creature.dark_torturer";
const ABILITY: &str = "oteryn:ability.spell.dark_torturer_skill_reducer";
const DURATION_US: u64 = 4_000_000;

#[test]
fn dark_torturer_cone_applies_one_defence_roll_per_cast() -> TestResult {
    run_case("m8_dark_torturer_cone", false)
}

#[test]
fn dark_torturer_replay_does_not_extend_debuff() -> TestResult {
    run_case("m8_dark_torturer_replay", true)
}

fn run_case(tag: &'static str, replay: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name(tag.into())
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            run(async |admin| {
                let started = Instant::now();
                let harness = Harness::create_with_admission_seed(admin, tag, false, false).await?;
                let result = scenario(&harness, replay).await;
                let cleanup = harness.cleanup().await;
                eprintln!(
                    "DARK_TORTURER_POSTGRES_EXECUTED {tag} elapsed={:?} passed={}",
                    started.elapsed(),
                    result.is_ok() && cleanup.is_ok()
                );
                result?;
                cleanup
            })
            .map_err(|error| format!("{error:?}"))
        })?
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        .map_err(Into::into)
}

async fn scenario(harness: &Harness, replay: bool) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == CREATURE)
        .ok_or("Dark Torturer profile")?;
    let definitions = record
        .attack_definitions
        .as_ref()
        .ok_or("attack definitions")?;
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let channel_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    // The map corridor has walls on three sides of the caster. Its one open
    // exit is the primary player's cell, with the second player farther west.
    let at = |x| MovementLocalPosition {
        x,
        y: 10,
        floor: -7,
    };
    let session = deps.admitted.game_session_id;
    let first = place_player(&mut runtime, session, at(9))?;
    deps.actor = first.0;
    let second = place_player(
        &mut runtime,
        GameSessionId::decode(&id(51)).map_err(debug)?,
        at(8),
    )?;
    let outside = place_player(
        &mut runtime,
        GameSessionId::decode(&id(52)).map_err(debug)?,
        at(12),
    )?;
    let maximum_health = u64::try_from(
        runtime
            .companion_policy(CREATURE)
            .map_err(debug)?
            .maximum_health,
    )?;
    runtime
        .begin_activation_spawn_sources(vec![(
            vec![((10, 10, -7), 60_000, CREATURE.into(), maximum_health, false)],
            5_000,
        )])
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        1
    );
    let caster = runtime.spawn_point_creatures()[0]
        .2
        .ok_or("spawned Dark Torturer")?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    owner.clock_origin -= Duration::from_secs(10);
    let now = owner.owner_now();
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        for player in [first, second, outside] {
            initialize_player(&runtime, &mut states, player, now)?;
        }
    }
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(|()| "current player fence")?
        .ok_or("current player session")?;
    assert!(owner.track_restored(
        session,
        first.0,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));

    for tick in 0..128 {
        let now = SemanticTimeMicros::from_micros(now.get() + tick * 1_000_000);
        owner.drain_monster_melee_at(now).await;
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let Some(proposal) = states
            .monster_melee
            .creature_ai
            .last_report_for_test(caster)
            .and_then(|report| report.proposals.iter().find(|p| p.ability.key == ABILITY))
            .cloned()
        else {
            continue;
        };
        let Some(cast_at) = states.monster_spells.retained_activity_time(
            caster,
            proposal.list,
            proposal.entry_index,
        ) else {
            continue;
        };
        let shielding = |states: &crate::gameplay_transport::actor_spell::ChannelSpellStates,
                         runtime: &ChannelRuntimeV1,
                         (actor, session),
                         now|
         -> TestResult<Option<AttributeModifier>> {
            Ok(states
                .native_effective_attributes(runtime, actor, session, now)
                .ok_or("current player attributes")?
                .shielding)
        };
        let source = SpellSource::from_native(
            &record.profile.target,
            proposal.entry_index,
            &definitions.records,
            &definitions.profiles,
            native.source_digest(),
        )
        .map_err(debug)?;
        let mut spells = std::mem::take(&mut states.monster_spells);
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        let replay_at = cast_at.saturating_add_micros(if replay { 2_000_000 } else { 0 });
        let outcome = spells.execute_current(
            &mut runtime,
            stamp,
            &mut *states,
            &source,
            &proposal,
            &mut ReplayOnly,
            replay_at,
        );
        assert_eq!(
            spells.retained_activity_time(caster, proposal.list, proposal.entry_index),
            Some(cast_at)
        );
        states.monster_spells = spells;
        let outcome = outcome.map_err(debug)?;
        let cue = outcome.presentation.ok_or("retained cone presentation")?;
        assert!(cue.source.effects.iter().any(|effect| matches!(effect,
            crate::content::ProjectV2AbilityEffect::Inline(effect)
                if effect.presentation.as_ref().and_then(|cue| cue.impact_asset_binding.as_deref())
                    == Some("canary.appearance:effect/sound_purple")
        )));
        assert_eq!(outcome.requested, 0);
        assert!(
            cue.tiles.contains(&(9, 10, -7)) && cue.tiles.contains(&(8, 10, -7)),
            "the cone must include both corridor players, got {:?}",
            cue.tiles
        );
        assert!(!cue.tiles.contains(&(12, 10, -7)));

        let modifier = shielding(&states, &runtime, first, now)?;
        let Some(AttributeModifier::PercentOfBase(percent @ 1..=40)) = modifier else {
            return Err(format!(
                "successful purple cone must install 1–40% defence, got {modifier:?}; targets={:?}",
                outcome.targets,
            )
            .into());
        };
        assert_eq!(shielding(&states, &runtime, second, now)?, modifier);
        assert_eq!(shielding(&states, &runtime, outside, now)?, None);
        assert_eq!(
            outcome.source_variant.ok_or("retained cast roll")?.index + 1,
            usize::try_from(percent)?
        );
        let deadline = cast_at.get() + DURATION_US;
        for target in [first, second] {
            assert_eq!(
                shielding(
                    &states,
                    &runtime,
                    target,
                    SemanticTimeMicros::from_micros(deadline - 1)
                )?,
                modifier
            );
            assert_eq!(
                shielding(
                    &states,
                    &runtime,
                    target,
                    SemanticTimeMicros::from_micros(deadline)
                )?,
                None
            );
        }
        return Ok(());
    }
    Err("the real monster dispatcher did not complete the scheduled Dark Torturer cone".into())
}

fn initialize_player(
    runtime: &ChannelRuntimeV1,
    states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
    (actor, session): (ExactActorRef, GameSessionId),
    now: SemanticTimeMicros,
) -> TestResult {
    states
        .initialize(
            runtime,
            actor,
            session,
            crate::spell::cast::CharacterCastFacts {
                max_health: 100_000,
                ..FACTS
            },
            (0, 0),
            now,
        )
        .ok_or("initialize test player")?;
    Ok(())
}

fn place_player(
    runtime: &mut ChannelRuntimeV1,
    session: GameSessionId,
    position: MovementLocalPosition,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_source_pinned_lab_player_position(actor, session, position)
        .map_err(debug)?;
    Ok((actor, session))
}

// The repeat must return the dispatcher's memo without preparing another cast.
struct ReplayOnly;

impl SpellWorldReader for ReplayOnly {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        None
    }

    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        None
    }

    fn tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: i32,
        _: i32,
        _: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }

    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        None
    }
}
