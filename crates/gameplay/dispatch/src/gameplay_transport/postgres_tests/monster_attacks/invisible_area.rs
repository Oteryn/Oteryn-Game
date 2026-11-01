//! Invisible recipients remain eligible for a visible target's source area.
use super::*;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_gameplay_foundation::foundation::condition::{ApplicationFacts, ConditionDefinition};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult {
    let native = owner
        .active_generation
        .ok_or("active source generation")?
        .native_gameplay()
        .ok_or("native source generation")?;
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    // Crystal 201189f, data/scripts/spells/support/invisible.lua:5-7.
    // This existing test-only self-use CAS installs the actual owned condition;
    // it does not bypass the spell owner's visibility or damage policy.
    let definition = ConditionDefinition::new(
        "fixture:source.invisibility",
        1,
        ConditionValues::Invisible {
            duration_ms: 200_000,
        },
    )
    .ok_or("qualified source invisibility")?;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let now = owner.owner_now().get();
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let base_speed = u16::try_from(
        states
            .get(&runtime, deps.actor, session)
            .ok_or("current invisible player")?
            .owned_base_speed(),
    )?;
    let protected = owner
        .states
        .attack
        .lock()
        .await
        .creature_target_protection(
            &runtime,
            deps.actor,
            session,
            SemanticTimeMicros::from_micros(now),
        )
        .ok_or("current invisible player protection")?;
    assert!(
        !protected,
        "the fixture does not replace re-entry protection facts"
    );
    assert!(states.install_owned_player_self_use_test_condition(
        &runtime,
        runtime.owner_fence().map_err(debug)?,
        stamp,
        deps.actor,
        session,
        &definition,
        &ApplicationFacts {
            now,
            base_speed,
            mana_shield_capacity: 0,
            target_reentry_protected: protected,
            source_reentry_protected: protected,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(deps.actor.placement_identity()),
        },
    ));
    assert_eq!(
        states
            .get(&runtime, deps.actor, session)
            .ok_or("owned invisible player")?
            .owned_invisible_at(now),
        Ok(true)
    );
    Ok(())
}

pub(super) async fn assert_current(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    monster: ExactActorRef,
    visible: (ExactActorRef, GameSessionId),
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let attack = owner.states.attack.lock().await;
    let now = owner.owner_now().get();
    assert_eq!(
        states
            .monster_melee
            .creature_ai
            .state(monster)
            .ok_or("current source area think")?
            .target(),
        Some(visible.0),
        "Fire Devil acquires the visible player, never the invisible area recipient",
    );
    for (actor, session, invisible) in [
        (deps.actor, deps.admitted.game_session_id, true),
        (visible.0, visible.1, false),
    ] {
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("current source area player")?
                .owned_invisible_at(now),
            Ok(invisible),
            "the owned invisibility must still be current when the area resolves"
        );
        assert_eq!(
            attack.creature_target_protection(
                &runtime,
                actor,
                session,
                SemanticTimeMicros::from_micros(now),
            ),
            Some(false),
            "source area damage still uses current re-entry protection"
        );
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let tile = deps
            .room
            .movement_cells()
            .spell_tiles()
            .lookup(
                deps.room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
            .map_err(debug)?;
        assert!(
            !tile.flags().protection_zone,
            "the source tile has no protection override"
        );
    }
    Ok(())
}
