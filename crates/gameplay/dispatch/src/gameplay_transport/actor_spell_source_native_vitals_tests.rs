#![allow(
    clippy::panic,
    reason = "test-only native outcome destructuring asserts impossible fixture variants; same scope as adjacent canonical owner tests"
)]
use super::tests::{runtime_with_player, wound};
use super::*;
use crate::player_lethal::PlayerLethalVitals;
fn ready(
    tag: u8,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
) {
    let (mut runtime, actor, session) = runtime_with_player(tag);
    assert!(
        runtime
            .initialize_movement_test_position(
                actor,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7
                }
            )
            .is_ok()
    );
    let mut states = ChannelSpellStates::default();
    let facts = CharacterCastFacts {
        vocation: crate::spell::Vocation::Druid,
        level: 30,
        magic_level: 20,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    };
    assert!(
        states
            .initialize(
                &runtime,
                actor,
                session,
                facts,
                (0, 0),
                SemanticTimeMicros::from_micros(0)
            )
            .is_some()
    );
    (runtime, states, actor, session)
}
#[test]
fn source_lethal_receipt_reuses_native_death_and_respawn_clears_source_replay() {
    let (mut r, mut s, a, g) = ready(0x41);
    wound(&mut s, a, g, 8);
    let Some(hit) = s.apply_attack_damage(
        &mut r,
        a,
        g,
        8,
        "source:lethal:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("native source hit")
    };
    let Some(native) = s.player_death(&r, a, g) else {
        panic!("native death")
    };
    let Some(receipt) = hit.death else {
        panic!("receipt")
    };
    assert_eq!(receipt.occurrence_bytes(), *native.occurrence.as_bytes());
    assert_eq!(receipt.position(), native.cell);
    assert_eq!(hit.health_after, 0);
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "source:lethal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(hit)
    );
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            9,
            "source:lethal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.deaths.len(), 1);
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            g,
            1,
            "heal:dead",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            1,
            "mana:dead",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert!(
        s.respawn(
            &r,
            a,
            g,
            native.occurrence,
            SemanticTimeMicros::from_micros(1)
        )
        .is_some()
    );
    assert!(s.source_damage.is_empty());
    assert!(s.source_mana.is_empty());
    assert!(s.source_heal.is_empty());
    assert!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "source:lethal:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        )
        .is_some()
    );
    assert!(!s.is_dead(a));
}
#[test]
fn source_failed_native_mint_and_wrong_session_have_no_hp_or_memo_write() {
    let (mut r, mut s, a, g) = ready(0x42);
    wound(&mut s, a, g, 8);
    s.mint_death = || None;
    let before = s.get(&r, a, g).cloned();
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "mint:failed",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.get(&r, a, g).cloned(), before);
    assert!(s.source_damage.is_empty());
    assert!(s.deaths.is_empty());
    let (_, _, other) = runtime_with_player(0x43);
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.get(&r, a, g).cloned(), before);
}
#[test]
fn source_heal_uses_current_maximum_native_credit_noop_revision_and_replay() {
    let (mut r, mut s, a, g) = ready(0x44);
    wound(&mut s, a, g, 100);
    let before = s.get(&r, a, g).cloned();
    let Some(heal) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        u32::MAX,
        "heal:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("native credit")
    };
    assert_eq!(
        (
            heal.applied,
            heal.health_after,
            heal.max_health,
            heal.vitals_revision
        ),
        (85, 185, 185, 2)
    );
    let Some(next) = s.get(&r, a, g).cloned() else {
        panic!("current")
    };
    let Some(before) = before else {
        panic!("before")
    };
    assert_eq!(
        (before.vitals().mana, before.vitals().soul),
        (next.vitals().mana, next.vitals().soul)
    );
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            g,
            u32::MAX,
            "heal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(heal)
    );
    assert_eq!(s.get(&r, a, g), Some(&next));
    let Some(full) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        1,
        "heal:1",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("full credit")
    };
    assert_eq!((full.applied, full.vitals_revision), (0, 2));
    let Some(zero) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        0,
        "heal:2",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("zero")
    };
    assert_eq!((zero.applied, zero.vitals_revision), (0, 2));
}
#[test]
fn source_mp_only_drain_clamps_current_pool_replays_and_exhaustion_keeps_revision() {
    let (mut r, mut s, a, g) = ready(0x45);
    let Some(before) = s.get(&r, a, g).cloned() else {
        panic!("before")
    };
    let Some(drain) = s.apply_attack_mana_drain(
        &mut r,
        a,
        g,
        u32::MAX,
        "mana:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("MP debit")
    };
    assert_eq!(
        (
            drain.applied,
            drain.mana_before,
            drain.mana_after,
            drain.vitals_revision
        ),
        (90, 90, 0, 2)
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            u32::MAX,
            "mana:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(drain)
    );
    let Some(next) = s.get(&r, a, g) else {
        panic!("current")
    };
    assert_eq!(
        (before.vitals().health, before.vitals().soul),
        (next.vitals().health, next.vitals().soul)
    );
    assert!(!s.is_dead(a));
    let Some(empty) = s.apply_attack_mana_drain(
        &mut r,
        a,
        g,
        1,
        "mana:1",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("empty MP")
    };
    assert_eq!((empty.applied, empty.vitals_revision), (0, 2));
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            2,
            "mana:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
}

#[test]
fn pubproj_creature_dot_keeps_frozen_identity_after_source_removal_and_hides_summon_species()
-> Result<(), String> {
    use crate::durability::character_death::PublicDeathCause;
    use oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    for summoned in [false, true] {
        let (mut runtime, mut states, actor, session) = ready(if summoned { 0x7a } else { 0x79 });
        let policy = CompiledCreaturePolicy {
            definition_key: "oteryn:creature.dragon".into(),
            definition_revision: "pubproj-source-1".into(),
            display_name: "Dragon".into(),
            maximum_health: 20,
            forge_difficulty: None,
            base_speed: 100,
            outfit_look_type: 1,
            object_look_type: None,
            summonable: true,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(0),
            armor: Some(0),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    runtime.content_pin().server_artifact_digest(),
                    vec![policy],
                )
                .map_err(|error| format!("trusted loaded policy: {error:?}"))?,
            )
            .map_err(|error| format!("active content pin: {error:?}"))?;
        let source = runtime
            .admit_pinned_monster_lab_creature(
                MovementLocalPosition {
                    x: 11,
                    y: 10,
                    floor: 7,
                },
                "oteryn:creature.dragon",
                20,
            )
            .map_err(|error| format!("native creature: {error:?}"))?;
        runtime
            .install_creature_policy(source, "oteryn:creature.dragon")
            .map_err(|error| format!("same exact native source: {error:?}"))?;
        if summoned {
            let current = runtime
                .companion_snapshot(source)
                .map_err(|error| format!("native source snapshot: {error:?}"))?;
            runtime
                .compare_assign_companion(actor, session, &current)
                .map_err(|error| format!("qualified player master: {error:?}"))?;
        }
        wound(&mut states, actor, session, 5);
        let root = GameplayDecisionRoot::from_bytes([19; 32]);
        let facts = crate::foundation::ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([19; 16]),
        };
        let definition = crate::foundation::ConditionDefinition::new(
            "pubproj.poison",
            1,
            crate::foundation::ConditionValues::DamageOverTime {
                element: crate::foundation::DotElement::Poison,
                total_min: 5,
                total_max: 5,
                per_tick: 5,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .ok_or_else(|| "bounded native damage condition".to_owned())?;
        assert!(states.install_owned_source_condition_fixture(
            &runtime,
            actor,
            session,
            source,
            &[definition],
            &facts
        ));
        let expected = if summoned {
            PublicDeathCause::PlayerSession {
                game_session_id: *session.as_bytes(),
            }
        } else {
            PublicDeathCause::creature("oteryn:creature.dragon", "Dragon")
                .ok_or_else(|| "native identity".to_owned())?
        };
        assert_eq!(native_public_death_cause(&runtime, source), Some(expected));
        runtime
            .remove_test_actor(source)
            .map_err(|error| format!("source generation removed before tick: {error:?}"))?;
        assert!(
            native_public_death_cause(&runtime, source).is_none(),
            "no death-time source resolution is possible"
        );
        let receipt = states
            .tick_source_player_conditions(
                &mut runtime,
                actor,
                session,
                DecisionOccurrenceId::from_bytes([20; 16]),
                oteryn_gameplay_foundation::foundation::condition::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            )
            .ok_or_else(|| "native delayed damage owner".to_owned())?;
        assert!(receipt.death.is_some());
        let death = states
            .player_death(&runtime, actor, session)
            .ok_or_else(|| "retained native death".to_owned())?;
        assert_eq!(death.public_cause, Some(expected));
        assert_eq!(
            death
                .public_cause
                .as_ref()
                .and_then(PublicDeathCause::killer_key),
            Some(if summoned {
                "player"
            } else {
                "oteryn:creature.dragon"
            })
        );
        if summoned {
            assert!(
                !matches!(death.public_cause, Some(PublicDeathCause::Creature { .. })),
                "player-owned source must not escape PvP privacy as a public creature"
            );
        }
    }
    Ok(())
}
