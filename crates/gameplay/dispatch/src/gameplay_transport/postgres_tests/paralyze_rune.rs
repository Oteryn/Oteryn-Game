//! Rune 3165 through the admitted USE dispatcher, native payment and condition owners.
use super::fixture::{Harness, debug, id, run};
use super::rune_use_bridge::{Case, run_case};
use super::{Dependencies, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_domain::domain::condition::{ConditionType, ConditionValues, SpeedRange};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionStore,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

#[test]
fn rune_3165_dispatch_paralyzes_monster_and_replaces_haste() -> TestResult {
    super::item_look::large_stack(|| {
        run(async |admin| run_case(admin, Case::ParalyzeMonster).await)
    })
}

#[test]
fn rune_3165_dispatch_paralyzes_player_and_replaces_haste() -> TestResult {
    super::item_look::large_stack(|| run(async |admin| run_case(admin, Case::ParalyzePlayer).await))
}

pub(super) async fn initialize_player(
    dependencies: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<EntityRef> {
    let actor = super::appearance_change_postgres::observer_visibility_tests::initialize(
        dependencies,
        owner,
    )
    .await?;
    let mut admitted = dependencies.admitted;
    admitted.game_session_id = GameSessionId::decode(&id(110)).map_err(debug)?;
    admitted.runtime_actor = Some(actor);
    owner.admit_progression_session(&mut admitted).await;
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(before, super::rune_use_bridge::VICTIM)
        .map_err(debug)?;
    let player = runtime
        .visible_entities()
        .players
        .into_iter()
        .find(|p| p.actor == actor)
        .ok_or("player entity")?;
    Ok(EntityRef {
        identity: actor.placement_identity(),
        generation: player.generation,
    })
}

fn target(
    owner: &crate::foundation::ChannelRuntimeV1,
    victim: EntityRef,
) -> TestResult<ExactActorRef> {
    owner
        .positioned_actor_census()
        .map_err(debug)?
        .into_iter()
        .find(|(actor, _, _)| actor.placement_identity() == victim.identity)
        .map(|(actor, _, _)| actor)
        .ok_or_else(|| "live target".into())
}

fn haste<S: Clone>(store: &mut ConditionStore<S>, base_speed: u16, now: u64) -> TestResult {
    let definition = ConditionDefinition::new(
        "fixture.haste",
        1,
        ConditionValues::Speed {
            paralysis: false,
            duration_ms: 30_000,
            range: SpeedRange {
                a_min: 0,
                a_max: 0,
                b_min: i32::from(base_speed) + 50,
                b_max: i32::from(base_speed) + 50,
            },
        },
    )
    .ok_or("haste definition")?;
    store
        .apply(
            &definition,
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now,
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: false,
                decision_root: &GameplayDecisionRoot::from_bytes([1; 32]),
                occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
            },
        )
        .map_err(debug)?;
    assert_eq!(store.active_speed_delta(now), 50);
    Ok(())
}

pub(super) async fn prepare_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    victim: EntityRef,
    case: Case,
) -> TestResult {
    // Global captures: docs/reference/spells/r22-audit/player-references/player-reference-coverage.json:267020
    // (level 54, ML 18), :166869 (1400 mana), :166903 (3 soul for conjuring),
    // :267255 (Fandom target-rule provenance; library text in
    // docs/reference/spells/r21-local-candidate/bundles/rune-paralyze_rune/spell.json:11).
    // Optional-PvP targets require war. Captures do not state effect speed/duration;
    // retain Crystal data/scripts/runes/paralyze_rune.lua:5 (-1,0,-1,0; 6000 ms).
    // The captured 6000 ms cooldown at :267173 is not the condition lifetime.
    let now = owner.owner_now();
    let mut runtime = owner.runtime.lock().await;
    let actor = target(&runtime, victim)?;
    if case == Case::ParalyzePlayer {
        let mut states = owner.states.spell_states.lock().await;
        let session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("target spell state")?;
        let before = state.owned_conditions().clone();
        let mut next = before.clone();
        haste(
            &mut next,
            u16::try_from(state.owned_base_speed())?,
            now.get(),
        )?;
        state
            .apply_batch_conditions(&before, &next)
            .map_err(debug)?;
    } else {
        let before = runtime.companion_snapshot(actor).map_err(debug)?;
        let mut next = before.state.clone();
        haste(
            &mut next.conditions,
            u16::try_from(next.policy.base_speed)?,
            now.get(),
        )?;
        runtime
            .compare_companion_state(&before, next)
            .map_err(debug)?;
    }
    drop(runtime);
    if case == Case::ParalyzePlayer {
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(101).as_slice()).execute(&harness.pool).await?;
    }
    // Open PvP permits the independently admitted player target, without war membership.
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'paralyze-fixture')")
        .bind(id(42).as_slice()).execute(&harness.pool).await?;
    if case == Case::ParalyzePlayer {
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                let ready: bool = sqlx::query_scalar("SELECT r.authorization_decided_at<=floor(extract(epoch FROM clock_timestamp()))::bigint-10 FROM game_durability_fresh_admission_receipts r JOIN game_durability_reconnect_sessions s ON COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key)=r.replay_key WHERE s.game_session_id=encode($1,'hex')::uuid")
                    .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
                if ready { return Ok::<_, Box<dyn std::error::Error>>(()); }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }).await??;
    }
    Ok(())
}

fn assert_speed<S: Clone>(store: &ConditionStore<S>) -> TestResult {
    let instance = store
        .instances()
        .iter()
        .find(|c| c.definition().condition_type() == ConditionType::Paralysis)
        .ok_or("paralysis installed by rune 3165")?;
    assert!(
        !store
            .instances()
            .iter()
            .any(|c| c.definition().key() == "fixture.haste")
    );
    assert!(instance.application_base_speed() > 40);
    assert!(instance.speed_delta() < 0);
    assert_eq!(
        i64::from(instance.application_base_speed()) + instance.speed_delta(),
        40
    );
    let start = instance.started_at();
    assert_eq!(instance.remaining_duration_ms(start), Some(6000));
    assert_eq!(
        store.active_speed_delta(start + 5_999_999),
        instance.speed_delta()
    );
    assert_eq!(store.active_speed_delta(start + 6_000_000), 0);
    Ok(())
}

pub(super) async fn assert_paralyzed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    victim: EntityRef,
    case: Case,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let actor = target(&runtime, victim)?;
    if case == Case::ParalyzePlayer {
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(
                &runtime,
                actor,
                GameSessionId::decode(&id(110)).map_err(debug)?,
            )
            .ok_or("target spell state")?;
        assert_speed(state.owned_conditions())?;
    } else {
        let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_speed(&snapshot.state.conditions)?;
    }
    Ok(())
}
