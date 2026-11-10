use super::*;

#[test]
fn srx3_soul_war_reflection_uses_post_resistance_damage_before_overkill_clamp_and_replays_once() {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates, commit_owner_batch, stage_player_batch_with_magnitude,
    };
    use crate::spell::combat_batch::{OwnerCombatBatch, SpellOccurrenceBinding};
    use oteryn_gameplay_entities::carrier::soul_war::SoulWarCreatureChange;

    let mut f = fixture_with_source_kind(0, true, true, Some("oteryn:creature.goshnar_s_greed"));
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, Some(1)));
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(0);
    let target = f.bindings[1].actor;
    let current = f
        .runtime
        .companion_snapshot(target)
        .expect("current source actor");
    f.runtime
        .change_soul_war_creature(
            &current,
            SoulWarCreatureChange::Buff {
                defense: 10,
                reflection: 20,
                maximum_health: 0,
                health: 0,
            },
        )
        .expect("source reflection modifier");
    f.runtime
        .set_test_creature_health(target, 1)
        .expect("one remaining target HP");
    let mut plan = plan();
    let NativeCombatPlan::Combat(combat) = &mut plan else {
        panic!("Combat plan")
    };
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 2_000;
    combat.hits[0].side_percent = None;
    combat.block_armor = true;
    let mut magnitude = qualify(&f, &plan).expect("real source magnitude owner");
    let batch = OwnerCombatBatch {
        caster: f.caster,
        attacker: f.attributes.binding.character,
        current_lease_generation: 1,
        command: CommandRef::new(f.session, CommandId::new(80).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "srx3:reflection:overkill".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"srx3:reflection:actual-magnitude".to_vec(),
        anchor: None,
        now_us: Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100_123)),
        now_ms: 100,
        effects: Vec::new(),
        deferred: None,
    };
    let lowered = crate::spell::combat_batch::lower_native(
        &f.runtime,
        batch,
        1,
        &plan,
        &f.bindings,
        0,
        &mut |plan, hit, base| {
            magnitude.finish_with_damage_healing(plan, hit, base, &mut |low, _| low)
        },
    )
    .expect("final post-block and resistance source damage");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &f.runtime,
            f.caster,
            f.session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("caster owner");
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        f.caster,
        f.session,
        f.state.clone()
    ));
    let before = f.state.vitals();
    let preflight = stage_player_batch_with_magnitude(
        &f.runtime,
        &states,
        &lowered.batch,
        None,
        Some(&magnitude),
    )
    .expect("reflection joins sealed source successor");
    let staged = f
        .runtime
        .stage_spell_batch(&lowered.batch)
        .expect("actual creature damage staging");
    let receipt = commit_owner_batch(&mut f.runtime, &mut states, staged, Some(preflight))
        .expect("one physical and caster commit");
    assert!(receipt.applied);
    assert_eq!(
        f.runtime
            .companion_snapshot_including_dead(target)
            .expect("dead source target")
            .health,
        0
    );
    let after = states
        .read_owned_player_state_test_snapshot(&f.runtime, f.caster, f.session)
        .expect("current caster")
        .vitals();
    let reflected = before.health - after.health;
    assert!(
        reflected > 0,
        "using the clamped one HP loss would incorrectly reflect zero"
    );
    assert!(
        reflected <= before.max_health.div_ceil(100),
        "source cap is 1% before caster defenses"
    );
    let replay = f
        .runtime
        .stage_spell_batch(&lowered.batch)
        .expect("physical receipt replay");
    let receipt = commit_owner_batch(&mut f.runtime, &mut states, replay, None)
        .expect("replay joins no new caster successor");
    assert!(!receipt.applied);
    assert_eq!(
        states
            .read_owned_player_state_test_snapshot(&f.runtime, f.caster, f.session)
            .expect("retained caster")
            .vitals(),
        after
    );
}

fn native_lua_reflection_case(key: &str, element: Element, percent: u32) {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates, commit_owner_batch, stage_player_batch_with_magnitude,
    };
    use crate::spell::combat_batch::{OwnerCombatBatch, SpellOccurrenceBinding};
    let mut f = fixture_with_source_kind(0, true, true, Some(key));
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, Some(1)));
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(0);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = element;
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 500;
    combat.hits[0].side_percent = None;
    combat.block_armor = false;
    let mut magnitude = qualify(&f, &p).expect("actual native target post-mitigation owner");
    let batch = OwnerCombatBatch {
        caster: f.caster,
        attacker: f.attributes.binding.character,
        current_lease_generation: 1,
        command: CommandRef::new(f.session, CommandId::new(81).expect("actual command")),
        occurrence: SpellOccurrenceBinding {
            id: "source-lua-reflection.actual-native".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: format!("source-lua-reflection:{key}:{element:?}").into_bytes(),
        anchor: None,
        now_us: Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100_123)),
        now_ms: 100,
        effects: Vec::new(),
        deferred: None,
    };
    let lowered = crate::spell::combat_batch::lower_native(
        &f.runtime,
        batch,
        1,
        &p,
        &f.bindings,
        0,
        &mut |p, hit, base| magnitude.finish_with_damage_healing(p, hit, base, &mut |low, _| low),
    )
    .expect("actual native damage before health clamp");
    let damage = lowered
        .batch
        .effects
        .iter()
        .filter_map(|effect| match effect.change {
            OwnerCombatChange::Damage { magnitude, .. }
            | OwnerCombatChange::Heal { magnitude, .. } => {
                Some(u32::try_from(magnitude).expect("nonnegative actual component"))
            }
            _ => None,
        })
        .sum::<u32>();
    assert!(
        damage > 100,
        "meaningful source reflection above the C++ one-percent cap"
    );
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &f.runtime,
            f.caster,
            f.session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("canonical caster state");
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        f.caster,
        f.session,
        f.state.clone()
    ));
    let before = f.state.vitals();
    let joined = stage_player_batch_with_magnitude(
        &f.runtime,
        &states,
        &lowered.batch,
        None,
        Some(&magnitude),
    )
    .expect("source Lua effect joins original paid successor");
    let staged = f
        .runtime
        .stage_spell_batch(&lowered.batch)
        .expect("canonical target stage");
    assert!(
        commit_owner_batch(&mut f.runtime, &mut states, staged, Some(joined))
            .expect("joint target/caster mutation")
            .applied
    );
    let after = states
        .read_owned_player_state_test_snapshot(&f.runtime, f.caster, f.session)
        .expect("retained caster")
        .vitals();
    assert_eq!(
        after.health,
        before.health.saturating_sub(damage * percent / 100),
        "signed normal damage fails Lua positive-only guard; positive Cage healing keeps its literal10percent loss"
    );
    let replay = f
        .runtime
        .stage_spell_batch(&lowered.batch)
        .expect("actual physical replay");
    assert!(
        !commit_owner_batch(&mut f.runtime, &mut states, replay, None)
            .expect("replay")
            .applied
    );
    assert_eq!(
        states
            .read_owned_player_state_test_snapshot(&f.runtime, f.caster, f.session)
            .expect("caster replay")
            .vitals(),
        after
    );
}
#[test]
fn srx3_malice_actual_native_signed_damage_does_not_enter_positive_lua_reflection() {
    native_lua_reflection_case("oteryn:creature.goshnar_s_malice", Element::Physical, 0);
}
#[test]
fn srx3_cage_actual_native_signed_damage_does_not_enter_positive_lua_reflection() {
    native_lua_reflection_case("oteryn:creature.soul_cage", Element::Energy, 0);
}
#[test]
fn srx3_malice_actual_native_lua_leaves_energy_unreflected() {
    native_lua_reflection_case("oteryn:creature.goshnar_s_malice", Element::Energy, 0);
}

#[test]
fn srx3_cage_actual_native_lua_reflects_healing_before_full_health_clamp() {
    native_lua_reflection_case("oteryn:creature.soul_cage", Element::Healing, 10);
}

#[test]
fn srx3_cruelty_actual_native_health_callback_reduces_player_heal_component_before_hp_clamp() {
    let mut f = fixture_with_source_kind(0, true, true, Some("oteryn:creature.goshnar_s_cruelty"));
    let target = f.bindings[1].actor;
    let before = f
        .runtime
        .companion_snapshot(target)
        .expect("actual native Cruelty");
    assert_eq!(
        f.runtime
            .soul_war_cruelty_drain(&before)
            .expect("source qualified drain"),
        None
    );
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, Some(1)));
    f.attributes.wheel_flat_healing = Some(0);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("Combat plan")
    };
    combat.element = Element::Healing;
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 10_000;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    combat.block_armor = false;
    let hit = combat.hits[0].clone();
    let mut magnitude = qualify(&f, &p).expect("current actual native healing owner");
    let result = magnitude
        .finish(&p, &hit, &f.bindings[1], &mut |low, _| low)
        .expect("source health event after finalized healing");
    assert_eq!(
        result, 9_800,
        "absent source drain defaults to two, including healing before capped HP application"
    );
}

#[test]
fn srx3_cruelty_source_health_component_precedes_wheel_multiplier_and_flat() {
    let mut f = fixture_with_source_kind(0, true, true, Some("oteryn:creature.goshnar_s_cruelty"));
    let target = f.bindings[1].actor;
    let before = f
        .runtime
        .companion_snapshot(target)
        .expect("actual native Cruelty");
    assert_eq!(
        f.runtime
            .soul_war_cruelty_drain(&before)
            .expect("source qualified drain"),
        None
    );
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, Some(1)));
    f.attributes.wheel_flat_healing = Some(200);
    f.attributes.source_healing_multiplier_percent = Some(10);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("Combat plan")
    };
    combat.element = Element::Healing;
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 101;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    combat.block_armor = false;
    let hit = combat.hits[0].clone();
    let mut magnitude = qualify(&f, &p).expect("current actual native healing owner");
    let result = magnitude
        .finish(&p, &hit, &f.bindings[1], &mut |low, _| low)
        .expect("source health event after finalized healing");
    assert_eq!(
        result, 307,
        "source truncates101*0.98 to98 before Wheel10percent and flat200"
    );
}
