use super::*;
use crate::carrier::soul_war_player_source::{
    CurrentSoulWarPlayerSourceProof, SoulWarPlayerSourceFacts, SoulWarPlayerSourceOrigin,
    source_proof_seal,
};
use oteryn_gameplay_content::content::spell_data::Vocation;
use oteryn_gameplay_foundation::foundation::CommandId;

fn id(tag: u8) -> [u8; 16] {
    [
        tag, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, tag,
    ]
}
static NATIVE_CATALOG: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
pub(in crate::carrier) fn policies() -> runtime_actor_companion::CompiledCreaturePolicies {
    static POLICIES: std::sync::OnceLock<runtime_actor_companion::CompiledCreaturePolicies> =
        std::sync::OnceLock::new();
    POLICIES.get_or_init(|| {
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native=oteryn_gameplay_content::content::native_gameplay::callback_native_policy_fixture_at(&path).expect("actual manifest-pinned native policy fixture");
        let catalog=native.catalog_digest();
        NATIVE_CATALOG.set(catalog).expect("single immutable fixture source catalog");
        let mut policies=native.policies().clone();
        policies.bind_qualified_outer_artifact([1;32]).expect("unit outer pin retains exact decoded native bodies");
        assert!(policies.soul_war_health_policy("oteryn:creature.mirror_image","definition-r1").is_some(),"actual bounded source witness must positively admit this native source image");
        policies
    }).clone()
}
struct Source;
impl source_proof_seal::Sealed for Source {}
impl CurrentSoulWarPlayerSourceProof for Source {
    fn current_facts(
        &self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: GameSessionId,
        _: Option<CommandRef>,
        _: u64,
    ) -> Option<SoulWarPlayerSourceFacts> {
        Some(SoulWarPlayerSourceFacts {
            vocation: Vocation::Druid,
            taints: 0,
            realm: None,
            before_revision: 1,
            origin: SoulWarPlayerSourceOrigin::Combat,
        })
    }
}
type NativeFixture = (
    ChannelRuntimeV1,
    ExactActorRef,
    GameSessionId,
    ExactActorRef,
    CommandRef,
);
fn fixture(capacity: usize) -> NativeFixture {
    fixture_with_scope(capacity, false)
}
fn fixture_with_scope(capacity: usize, instance: bool) -> NativeFixture {
    let world = WorldId::decode(&id(1)).expect("world");
    let scope = if instance {
        RuntimeScopeRefV1::instance(world, id(2)).expect("actual private fixture scope")
    } else {
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(2)).expect("channel"))
    };
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        NodeId::decode(&id(3)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        capacity,
        ChannelContentPin::test(world),
    )
    .expect("real fixed carrier");
    runtime
        .install_companion_policies(policies())
        .expect("actual native policy owner");
    let session = GameSessionId::decode(&id(4)).expect("session");
    let reserved = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reserved).expect("commit");
    runtime
        .initialize_first_entry_position(player)
        .expect("current position");
    runtime
        .bind_attacker_lease(
            player,
            session,
            CharacterLease::new(CharacterId::decode(&id(5)).expect("character"), 1).expect("lease"),
        )
        .expect("bound current lease");
    let policy = runtime
        .companion_policy("oteryn:creature.mirror_image")
        .expect("actual native image");
    let at = runtime
        .read_actor_position(player)
        .expect("position")
        .position();
    let image = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("native source identity");
    runtime
        .install_creature_policy(image, &policy.definition_key)
        .expect("sealed native callback");
    let command = CommandRef::new(session, CommandId::new(1).expect("command"));
    runtime
        .install_current_soul_war_player_source(player, session, Some(command), 200_000, &Source)
        .expect("current-pass caster facts");
    (runtime, player, session, image, command)
}
#[test]
fn srx3_mirror_direct_native_owner_creates_fresh_full_health_and_loses_original_hit() {
    let (mut runtime, player, _, image, command) = fixture(4);
    let maximum = runtime
        .creature_combat_facts(image, 100)
        .expect("native health")
        .maximum_health;
    runtime
        .borrow_exact_actor_commit_at(100)
        .commit_damage(
            image,
            OwnerDamageCommand {
                target: b"oteryn:creature.mirror_image",
                occurrence: b"wound",
                binding: b"wound\0source",
                damage: maximum - 1,
            },
        )
        .expect("canonical source nonplayer floor setup");
    assert_eq!(
        runtime
            .creature_combat_facts(image, 100)
            .expect("wounded image")
            .health,
        1
    );
    let result = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage_for_bound_attacker(
            image,
            player,
            command,
            0,
            OwnerDamageCommand {
                target: b"oteryn:creature.mirror_image",
                occurrence: &[],
                binding: b"mirror.actual.native.direct",
                damage: 500_000,
            },
        )
        .expect("source birth and retirement complete in the canonical owner");
    assert_eq!(
        (result.health_before, result.health_after),
        (1, 1),
        "the original hit is lost, not applied to the fresh apparition"
    );
    assert!(!runtime.contains_live_creature(image));
    let children: Vec<_> = runtime
        .positioned_actor_census()
        .expect("actual census")
        .into_iter()
        .filter_map(|(actor, _, session)| session.is_none().then_some(actor))
        .collect();
    assert_eq!(children.len(), 1);
    let child = children[0];
    assert_ne!(
        child, image,
        "fresh physical incarnation, not setType on the source image"
    );
    let snapshot = runtime
        .companion_snapshot(child)
        .expect("actual native apparition");
    assert!(
        rule::Apparition::ALL
            .into_iter()
            .any(|a| a.creature_key() == snapshot.state.policy.definition_key)
    );
    assert_eq!(snapshot.health, snapshot.state.policy.maximum_health);
    assert_eq!(
        snapshot.maximum_health,
        snapshot.state.policy.maximum_health
    );
    assert!(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(image)
            .is_err(),
        "retirement creates no lethal/corpse/loot receipt"
    );
    assert!(
        runtime
            .borrow_exact_actor_commit_at(200)
            .commit_damage_for_bound_attacker(
                image,
                player,
                command,
                0,
                OwnerDamageCommand {
                    target: b"oteryn:creature.mirror_image",
                    occurrence: &[],
                    binding: b"mirror.actual.native.direct",
                    damage: 500_000
                }
            )
            .is_err()
    );
    assert_eq!(
        runtime
            .positioned_actor_census()
            .expect("replay census")
            .into_iter()
            .filter(|(_, _, session)| session.is_none())
            .count(),
        1
    );
}
#[test]
fn srx3_mirror_direct_native_owner_capacity_failure_still_removes_source_without_loot() {
    let (mut runtime, player, _, image, command) = fixture(2);
    runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage_for_bound_attacker(
            image,
            player,
            command,
            0,
            OwnerDamageCommand {
                target: b"oteryn:creature.mirror_image",
                occurrence: &[],
                binding: b"mirror.full.capacity",
                damage: 1,
            },
        )
        .expect("source create fails then source image still retires");
    assert!(!runtime.contains_live_creature(image));
    assert_eq!(
        runtime
            .positioned_actor_census()
            .expect("actual census")
            .len(),
        1
    );
    assert!(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(image)
            .is_err()
    );
}

#[test]
fn srx3_mirror_paid_batch_retains_real_spare_across_unknown_and_replays_once() {
    let (mut runtime, player, session, image, command) = fixture(4);
    let batch = runtime_actor_spell_types::OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(&id(5)).expect("character"),
        current_lease_generation: 1,
        command,
        occurrence: runtime_actor_spell_types::SpellOccurrenceBinding {
            id: "soul-war.actual-mirror.paid".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"source-mirror-paid-bound-current".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 200,
        effects: vec![runtime_actor_spell_types::OwnerCombatEffect {
            target: image,
            sub_ordinal: 0,
            change: runtime_actor_spell_types::OwnerCombatChange::Damage {
                target_atom: "oteryn:creature.mirror_image".into(),
                magnitude: 500_000,
            },
        }],
        deferred: None,
    };
    let mut abandoned = runtime
        .stage_spell_batch(&batch)
        .expect("prospective source stage");
    runtime
        .reserve_spell_batch(&mut abandoned)
        .expect("prospective real spare");
    runtime
        .release_definitely_uncommitted_spell_batch(&abandoned)
        .expect("definite SQL rollback releases only source reservations");
    assert!(runtime.contains_live_creature(image));
    assert!(!runtime.actor_spell_reserved(player));
    let mut staged = runtime
        .stage_spell_batch(&batch)
        .expect("actual native source stage");
    assert!(staged.has_source_native_reaction());
    assert!(
        runtime.contains_live_creature(image),
        "staging publishes nothing"
    );
    runtime
        .reserve_spell_batch(&mut staged)
        .expect("reserve actual spare before SQL");
    assert!(runtime.actor_spell_reserved(player));
    assert!(
        runtime.contains_live_creature(image),
        "unknown SQL outcome retains old actor"
    );
    drop(staged);
    let retained = runtime
        .stage_spell_batch(&batch)
        .expect("same command retains original real spare");
    assert!(retained.has_source_native_reaction());
    assert!(
        runtime
            .commit_spell_batch(retained)
            .expect("resolve original commit")
            .applied
    );
    assert!(!runtime.contains_live_creature(image));
    assert!(!runtime.actor_spell_reserved(player));
    let children = runtime
        .positioned_actor_census()
        .expect("actual census")
        .into_iter()
        .filter(|(_, _, actor_session)| actor_session.is_none())
        .count();
    assert_eq!(children, 1);
    let replay = runtime.stage_spell_batch(&batch).expect("canonical replay");
    assert!(
        !runtime
            .commit_spell_batch(replay)
            .expect("replay receipt")
            .applied
    );
    assert_eq!(
        runtime
            .positioned_actor_census()
            .expect("actual census")
            .into_iter()
            .filter(|(_, _, actor_session)| actor_session.is_none())
            .count(),
        1
    );
    assert!(runtime.player_control_facts(player, session).is_ok());
}

struct PeriodicSource;
impl source_proof_seal::Sealed for PeriodicSource {}
impl CurrentSoulWarPlayerSourceProof for PeriodicSource {
    fn current_facts(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now: u64,
    ) -> Option<SoulWarPlayerSourceFacts> {
        let mut facts = Source.current_facts(runtime, actor, session, command, now)?;
        facts.origin = SoulWarPlayerSourceOrigin::Condition;
        Some(facts)
    }
}
#[test]
fn srx3_periodic_source_data_accepts_visible_grace_without_regranting_combat_input() {
    let (mut runtime, player, session, _, command) = fixture(4);
    runtime
        .record_control_loss(
            player,
            session,
            crate::ControlLossMark {
                epoch: 1,
                grace_deadline: 1_000_000,
            },
        )
        .expect("actual retained visible avatar");
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .expect("read-only current condition source facts survive loss of input control");
    assert_eq!(
        runtime
            .current_soul_war_player_source(player, session, None, 201_000)
            .expect("current immutable facts")
            .expect("condition context")
            .origin,
        SoulWarPlayerSourceOrigin::Condition
    );
    assert!(
        runtime
            .install_current_soul_war_player_source(
                player,
                session,
                Some(command),
                201_000,
                &Source
            )
            .is_err(),
        "a presence read must not authorize a fresh combat command"
    );
}

#[test]
fn srx3_mirror_periodic_health_event_changes_same_actor_type_preserving_old_maximum() {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    let (mut runtime, player, session, image, _) = fixture(4);
    let before = runtime
        .companion_snapshot(image)
        .expect("actual native image");
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .expect("fresh actual condition caster data");
    let field = runtime_actor_periodic::CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "source-field.mirror.periodic",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 1000,
                total_max: 1000,
                per_tick: 1000,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .expect("actual condition owner recipe"),
        source: Some(player),
        lineage: Some(
            oteryn_gameplay_foundation::foundation::condition::ConditionLineage {
                source_character: id(5),
                source_session: *session.as_bytes(),
                source_lease_generation: 1,
                source_actor_placement: player.placement_identity(),
                source_scope_generation: player.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: *NATIVE_CATALOG.get().expect("same decoded native catalog"),
                source_creation_command: 1,
            },
        ),
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([8; 16]),
    };
    runtime
        .commit_creature_field_entry(image, 201_000, Some(&field))
        .expect("existing field condition owner");
    let receipt = runtime
        .apply_creature_periodic_turn_with_sources(
            image,
            201_000,
            TickFacts::default(),
            &mut |r, source, _| Ok(r.bound_attacker_lease(source, session).ok().map(|_| 100)),
        )
        .expect("canonical actual native periodic health owner");
    let after = runtime
        .companion_snapshot(image)
        .expect("same physical actor survives");
    assert_eq!(
        after.state.policy.definition_key,
        "oteryn:creature.druid_s_apparition"
    );
    assert_eq!(
        after.actor, image,
        "DOT uses setType, without fresh birth or removal"
    );
    assert_eq!(
        after.maximum_health, before.maximum_health,
        "restoreHealth=false retains existing maximum"
    );
    assert!(
        after.health < before.health,
        "the original DOT hit continues after setType"
    );
    assert_eq!(receipt.health_after, after.health);
}

#[test]
fn srx3_fourth_taint_null_periodic_killer_consumes_tick_without_death() {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    let (mut runtime, player, _, _, _) = fixture(4);
    let policy = runtime
        .companion_policy("oteryn:creature.bony_sea_devil")
        .expect("actual source participant");
    let at = runtime
        .read_actor_position(player)
        .expect("actual source frame")
        .position();
    let target = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual admitted native body");
    runtime
        .install_creature_policy(target, &policy.definition_key)
        .expect("sealed source prepare-death event");
    assert!(
        runtime
            .soul_war_health_callback(target)
            .expect("source token")
            .is_some()
    );
    runtime
        .set_test_creature_health(target, 1)
        .expect("bounded pre-hit injury");
    let field = runtime_actor_periodic::CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "source-field.fourth.null",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 1000,
                total_max: 1000,
                per_tick: 1000,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .expect("condition owner recipe"),
        source: None,
        lineage: None,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([9; 16]),
    };
    runtime
        .commit_creature_field_entry(target, 201_000, Some(&field))
        .expect("actual native field owner");
    let staged = runtime
        .stage_creature_periodic_turn(target, 201_000, TickFacts::default())
        .expect("actual existing source periodic stage");
    let receipt = runtime
        .commit_creature_periodic_turn(staged)
        .expect("source null-killer cancellation is a completed tick");
    assert_eq!(
        receipt.health_after, 1,
        "literal null PrepareDeath error cancels would-be lethal hit"
    );
    assert!(
        runtime
            .companion_snapshot(target)
            .expect("still alive")
            .state
            .conditions
            .instances()
            .is_empty(),
        "cancelled damage must still consume its original due tick"
    );
    assert!(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(target)
            .is_err()
    );
}

#[test]
fn srx3_cloak_actual_committed_hp_event_retains_source_after_removal_and_replays_once() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("actual native Cloak");
    let at = runtime
        .read_actor_position(player)
        .expect("actual physical player")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Cloak actor");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("sealed source body");
    assert!(
        runtime
            .soul_war_health_callback(cloak)
            .expect("source health flag")
            .is_some(),
        "full52 witness positive before actual HP event"
    );
    // Crystal raw Lua health components are signed: healing is positive
    // before creatureevent.cpp473 normalizes the callback's return values.
    let batch = cloak_health_batch(
        player,
        session,
        cloak,
        2,
        runtime_actor_spell_types::OwnerCombatChange::Heal {
            target_atom: "oteryn:creature.cloak_of_terror".into(),
            magnitude: 1000,
        },
    );
    let prepared = runtime
        .stage_spell_batch(&batch)
        .expect("actual positive native healing callback");
    assert!(
        runtime
            .commit_spell_batch(prepared)
            .expect("known canonical HP event")
            .applied
    );
    let events = runtime
        .pending_cloak_blood_events(64)
        .expect("current native source event owner");
    assert_eq!(
        events.len(),
        1,
        "actual committed onHealthChange queues one bounded blood source receipt"
    );
    let event = &events[0];
    assert_eq!(event.actor(), cloak);
    assert_eq!(event.position(), at);
    assert!(event.current_for(&runtime));
    let replay = runtime
        .stage_spell_batch(&batch)
        .expect("known healing occurrence replay");
    assert!(
        !runtime
            .commit_spell_batch(replay)
            .expect("replay does not reapply")
            .applied
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("retained source event")
            .len(),
        1
    );
    runtime
        .remove_encounter_actor(cloak)
        .expect("source parent retires without fake death");
    assert!(
        event.current_for(&runtime),
        "known historical HP callback outlives its source actor"
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("historical event")
            .len(),
        1
    );
    runtime.retire_owner_cycle();
    assert!(!event.current_for(&runtime));
}

#[test]
fn srx3_cloak_actual_healing_callback_records_blood_before_full_health_clamp() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("actual native Cloak");
    let at = runtime
        .read_actor_position(player)
        .expect("actual position")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Cloak");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("actual source health flag");
    let before = runtime.companion_snapshot(cloak).expect("actual full HP");
    let batch = runtime_actor_spell_types::OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(&id(5)).expect("actual Character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(2).expect("real command")),
        occurrence: runtime_actor_spell_types::SpellOccurrenceBinding {
            id: "source.cloak.actual.heal".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"source.cloak.actual.heal".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 200,
        effects: vec![runtime_actor_spell_types::OwnerCombatEffect {
            target: cloak,
            sub_ordinal: 0,
            change: runtime_actor_spell_types::OwnerCombatChange::Heal {
                target_atom: "oteryn:creature.cloak_of_terror".into(),
                magnitude: 1000,
            },
        }],
        deferred: None,
    };
    let mut staged = runtime
        .stage_spell_batch(&batch)
        .expect("actual native heal staging");
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("current source queue")
            .len(),
        0,
        "staging cannot publish source events"
    );
    runtime
        .reserve_spell_batch(&mut staged)
        .expect("reserve actual touched slots and event capacity before paid SQL");
    drop(staged);
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("unknown result is unpublished")
            .len(),
        0
    );
    let staged = runtime
        .stage_spell_batch(&batch)
        .expect("reconcile retained exact source successor after uncertain outcome");
    assert!(
        runtime
            .commit_spell_batch(staged)
            .expect("actual native heal commit")
            .applied
    );
    assert_eq!(
        runtime
            .companion_snapshot(cloak)
            .expect("same current native actor")
            .health,
        before.health
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("committed health callback")
            .len(),
        1,
        "literal primaryDamage positive healing qualifies before full-health clamp"
    );
    let replay = runtime
        .stage_spell_batch(&batch)
        .expect("real native heal replay");
    assert!(
        !runtime
            .commit_spell_batch(replay)
            .expect("replay retained")
            .applied
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("single callback event")
            .len(),
        1
    );
}
#[test]
fn srx3_cloak_null_attacker_does_not_emit_native_blood_callback() {
    let (mut runtime, player, _, _, _) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("actual native Cloak");
    let at = runtime
        .read_actor_position(player)
        .expect("actual position")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Cloak");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("source health flag");
    let hit = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage(
            cloak,
            OwnerDamageCommand {
                target: b"oteryn:creature.cloak_of_terror",
                occurrence: b"null-source",
                binding: b"null-source\0actual.cloak",
                damage: 1,
            },
        )
        .expect("actual null-source HP owner");
    assert!(hit.applied);
    assert!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("current native queue")
            .is_empty()
    );
}

#[test]
fn srx3_cloak_actual_player_condition_tick_records_original_health_callback_once() {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionLineage, ConditionSourceKind, TickFacts,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let (mut runtime, player, session, _, command) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("actual native Cloak");
    let at = runtime
        .read_actor_position(player)
        .expect("current source position")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("native source actor");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("actual full callback seal");
    let before = runtime
        .companion_snapshot(cloak)
        .expect("current native target");
    let mut after = before.state.clone();
    let definition = ConditionDefinition::new(
        "test.cloak.current.player.dot",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: 1000,
            total_max: 1000,
            per_tick: 1000,
            interval_ms: 1000,
            delayed: true,
        },
    )
    .expect("canonical condition definition");
    let root = GameplayDecisionRoot::from_bytes([17; 32]);
    let lineage = ConditionLineage {
        source_character: id(5),
        source_session: *session.as_bytes(),
        source_lease_generation: 1,
        source_actor_placement: player.placement_identity(),
        source_scope_generation: player.scope_generation().get(),
        source_content_digest: runtime.content_pin().server_artifact_digest(),
        source_catalog_digest: *NATIVE_CATALOG.get().expect("actual source catalogue"),
        source_creation_command: command.command_id().get(),
    };
    after
        .conditions
        .apply_with_lineage(
            &definition,
            Some(player),
            ConditionSourceKind::Field,
            &[],
            &ApplicationFacts {
                now: 0,
                base_speed: policy.base_speed.try_into().expect("native speed"),
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: false,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes(id(116)),
            },
            &lineage,
        )
        .expect("actual attributed condition store");
    runtime
        .compare_companion_state(&before, after)
        .expect("canonical condition publication");
    let mut source = |current: &ChannelRuntimeV1, actor, _now| {
        current.player_control_facts(actor, session)?;
        Ok(Some(100))
    };
    let receipt = runtime
        .apply_creature_periodic_turn_with_sources(
            cloak,
            1_000_000,
            TickFacts::default(),
            &mut source,
        )
        .expect("real current-owner condition tick");
    assert!(
        receipt.health_after < receipt.health_before,
        "positive finalized periodic component before blood assertion"
    );
    let events = runtime
        .pending_cloak_blood_events(64)
        .expect("current native source queue");
    assert_eq!(
        events.len(),
        1,
        "accepted authoring Q6a: actual Player condition hit leaves one blood pool intent"
    );
    runtime
        .apply_creature_periodic_turn_with_sources(
            cloak,
            1_000_000,
            TickFacts::default(),
            &mut source,
        )
        .expect("same owner time replay");
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("retained queue")
            .len(),
        1
    );
}

fn cloak_health_batch(
    player: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    command: u64,
    change: runtime_actor_spell_types::OwnerCombatChange,
) -> runtime_actor_spell_types::OwnerCombatBatch {
    runtime_actor_spell_types::OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(&id(5)).expect("actual Character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(command).expect("actual command")),
        occurrence: runtime_actor_spell_types::SpellOccurrenceBinding {
            id: format!("source.cloak.signed.health.{command}"),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: format!("source.cloak.signed.health.{command}").into_bytes(),
        anchor: None,
        now_us: None,
        now_ms: 200,
        effects: vec![runtime_actor_spell_types::OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change,
        }],
        deferred: None,
    }
}
#[test]
fn srx3_cloak_accepted_q6a_normal_player_hits_record_each_callback_once() {
    let (mut runtime, player, session, _, command) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("full native source");
    let at = runtime
        .read_actor_position(player)
        .expect("actual current actor")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Cloak");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("full sealed source callback");
    let damage = OwnerDamageCommand {
        target: b"oteryn:creature.cloak_of_terror",
        occurrence: &[],
        binding: b"source.cloak.signed.loss",
        damage: 1,
    };
    let hit = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage_for_bound_attacker(cloak, player, command, 0, damage)
        .expect("real canonical Player loss");
    assert!(
        hit.applied && hit.health_before - hit.health_after == 1,
        "real HP loss must execute before accepted Q6a blood assertion"
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("canonical source queue")
            .len(),
        1,
        "authoring13.7 Q6a explicitly overrides the signed donor guard for Player hits"
    );
    let swing = runtime
        .borrow_exact_actor_commit_at(201)
        .commit_swing_damage_for_bound_attacker(
            cloak,
            player,
            CommandRef::new(session, CommandId::new(2).expect("actual attack command")),
            0,
            damage,
        )
        .expect("canonical actual swing");
    assert!(swing.applied && swing.health_before - swing.health_after == 1);
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("actual swing queue")
            .len(),
        2
    );
    let batch = cloak_health_batch(
        player,
        session,
        cloak,
        3,
        runtime_actor_spell_types::OwnerCombatChange::Damage {
            target_atom: "oteryn:creature.cloak_of_terror".into(),
            magnitude: 1,
        },
    );
    let prepared = runtime
        .stage_spell_batch(&batch)
        .expect("canonical signed damage batch");
    assert!(
        runtime
            .commit_spell_batch(prepared)
            .expect("known loss")
            .applied
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("actual batch queue")
            .len(),
        3
    );
    let replay = runtime
        .stage_spell_batch(&batch)
        .expect("same actual source batch");
    assert!(
        !runtime
            .commit_spell_batch(replay)
            .expect("known callback replay")
            .applied
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("retained exact callbacks")
            .len(),
        3
    );
}

#[test]
fn srx3_cloak_canonical_absorption_heal_has_null_attacker_and_no_blood() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let policy = runtime
        .companion_policy("oteryn:creature.cloak_of_terror")
        .expect("actual native source Cloak");
    let at = runtime
        .read_actor_position(player)
        .expect("actual owner position")
        .position();
    let cloak = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Cloak");
    runtime
        .install_creature_policy(cloak, &policy.definition_key)
        .expect("sealed callback policy");
    runtime
        .set_test_creature_health(cloak, policy.maximum_health - 1000)
        .expect("canonical pre-event injury");
    let before = runtime
        .companion_snapshot(cloak)
        .expect("current before")
        .health;
    // This is the canonical composed-HP boundary, not a new gameplay spell or
    // a claim that pinned Cloak has an absorbing resistance. Crystal game.cpp
    // 7400-7401 calls the positive absorption heal with nullptr, then any
    // residual loss has its original negative sign and actual attacker.
    let batch = cloak_health_batch(
        player,
        session,
        cloak,
        1,
        runtime_actor_spell_types::OwnerCombatChange::DamageWithHealing {
            target_atom: "oteryn:creature.cloak_of_terror".into(),
            damage: 100,
            healing: 200,
        },
    );
    let prepared = runtime
        .stage_spell_batch(&batch)
        .expect("actual composed HP stage");
    assert!(
        runtime
            .commit_spell_batch(prepared)
            .expect("known HP commit")
            .applied
    );
    assert_eq!(
        runtime
            .companion_snapshot(cloak)
            .expect("after composed HP")
            .health,
        before + 100,
        "both canonical heal and residual loss must execute before checking source blood"
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("actual source queue")
            .len(),
        1,
        "Q6a residual Player hit emits one event; NULL absorption healing must not emit a second"
    );
    let heal_only = cloak_health_batch(
        player,
        session,
        cloak,
        2,
        runtime_actor_spell_types::OwnerCombatChange::DamageWithHealing {
            target_atom: "oteryn:creature.cloak_of_terror".into(),
            damage: 0,
            healing: 200,
        },
    );
    let prepared = runtime
        .stage_spell_batch(&heal_only)
        .expect("actual NULL absorption heal");
    assert!(
        runtime
            .commit_spell_batch(prepared)
            .expect("known NULL heal")
            .applied
    );
    assert_eq!(
        runtime
            .companion_snapshot(cloak)
            .expect("after NULL heal")
            .health,
        before + 300
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("NULL heal queue")
            .len(),
        1
    );
    let replay_heal = runtime
        .stage_spell_batch(&heal_only)
        .expect("same NULL heal occurrence");
    assert!(
        !runtime
            .commit_spell_batch(replay_heal)
            .expect("known NULL heal replay")
            .applied
    );
    let replay = runtime
        .stage_spell_batch(&batch)
        .expect("same canonical occurrence");
    assert!(
        !runtime
            .commit_spell_batch(replay)
            .expect("same known receipt")
            .applied
    );
    assert_eq!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("replayed source queue")
            .len(),
        1
    );
}

#[test]
fn srx3_cruelty_literal_health_event_reads_current_drain_on_world_creature() {
    let (mut runtime, player, _, _, _) = fixture(4);
    let mut at = runtime
        .read_actor_position(player)
        .expect("current player pose")
        .position();
    at.x += 2;
    let boss = runtime
        .spawn_arena_opponent("oteryn:creature.goshnar_s_cruelty", at, |_, _| Some(true))
        .expect("actual qualified native Cruelty, no invented stats");
    let snapshot = runtime
        .companion_snapshot(boss)
        .expect("current native health owner");
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .expect("native callback token")
            .is_some_and(|policy| policy.is_cruelty())
    );
    assert_eq!(
        runtime.soul_war_cruelty_drain(&snapshot),
        Ok(None),
        "literal onHealthChange has no scene predicate; readonly current drain must work on the World"
    );
    assert_eq!(
        runtime.increase_soul_war_cruelty(&snapshot, None),
        Err(CarrierError::WrongScope),
        "reading native health facts grants no encounter mutation authority"
    );
}

#[test]
fn srx3_cruelty_actual_player_condition_scales_health_component_once() {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{
        ConditionDefinition, ConditionLineage, TickFacts,
    };
    let (mut runtime, player, session, _, _) = fixture(8);
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .expect("current real Player condition origin");
    let origin = runtime
        .read_actor_position(player)
        .expect("physical player position")
        .position();
    let mut losses = Vec::new();
    for (index, attributed) in [false, true].into_iter().enumerate() {
        let at = MovementLocalPosition {
            x: origin.x + 2 + index as i32,
            y: origin.y,
            floor: origin.floor,
        };
        let boss = runtime
            .spawn_arena_opponent("oteryn:creature.goshnar_s_cruelty", at, |_, _| Some(true))
            .expect("actual native source Cruelty");
        let field = runtime_actor_periodic::CreatureFieldEntry {
            definition: ConditionDefinition::new(
                "source-field.cruelty.bleeding",
                1,
                ConditionValues::DamageOverTime {
                    element: DotElement::Bleeding,
                    total_min: 1000,
                    total_max: 1000,
                    per_tick: 1000,
                    interval_ms: 1000,
                    delayed: false,
                },
            )
            .expect("canonical condition recipe"),
            source: attributed.then_some(player),
            lineage: attributed.then(|| ConditionLineage {
                source_character: id(5),
                source_session: *session.as_bytes(),
                source_lease_generation: 1,
                source_actor_placement: player.placement_identity(),
                source_scope_generation: player.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: *NATIVE_CATALOG.get().expect("actual source catalogue"),
                source_creation_command: 1,
            }),
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(
                121 + index as u8
            )),
        };
        runtime
            .commit_creature_field_entry(boss, 201_000, Some(&field))
            .expect("real condition publication");
        let receipt = runtime
            .apply_creature_periodic_turn_with_sources(
                boss,
                201_000,
                TickFacts::default(),
                &mut |r, source, _| Ok(r.bound_attacker_lease(source, session).ok().map(|_| 100)),
            )
            .expect("actual canonical source-aware condition tick");
        let loss = receipt.health_before - receipt.health_after;
        assert!(
            loss > 0,
            "same actual source native physical response precedes callback"
        );
        losses.push(loss);
    }
    assert_eq!(
        losses[1],
        (losses[0] as f64 * 0.98).trunc() as i64,
        "literal Cruelty event scales Player-origin periodic damage once; NULL-origin control retains native mitigation"
    );
}

#[test]
fn srx3_soul_war_reflection_stream_keeps_actual_player_source_without_health_event() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let mut at = runtime
        .read_actor_position(player)
        .expect("current reflector position")
        .position();
    at.x += 2;
    let target = runtime
        .spawn_arena_opponent("oteryn:creature.bony_sea_devil", at, |_, _| Some(true))
        .expect("actual native Soul War hunting monster");
    assert_eq!(runtime.soul_war_health_callback(target).expect("actual source policy")
        .expect("FourthTaint callback").kind(),
        oteryn_gameplay_content::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::FourthTaint);
    let before = runtime
        .companion_snapshot(target)
        .expect("current target health")
        .health;
    let applied=runtime.commit_source_damage(target,player,session,b"srx3-native-reflection",1,100,201)
        .expect("real credited reflection stream must accept nonlethal damage to source-qualified hunting monster");
    assert!(applied.applied);
    assert_eq!(applied.health_before - applied.health_after, 100);
    assert_eq!(
        runtime
            .companion_snapshot(target)
            .expect("current target health")
            .health,
        before - 100
    );
    let replay = runtime
        .commit_source_damage(
            target,
            player,
            session,
            b"srx3-native-reflection",
            1,
            100,
            202,
        )
        .expect("same autonomous source stream replays without a fabricated client command");
    assert!(!replay.applied);
    assert_eq!(
        runtime
            .companion_snapshot(target)
            .expect("current replay health")
            .health,
        before - 100
    );
}

struct ReflectionSource;

#[test]
fn srx3_autonomous_reflection_damage_map_keeps_actual_player_and_consumed_stream() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let mut at = runtime
        .read_actor_position(player)
        .expect("actual reflector pose")
        .position();
    at.x += 2;
    let target = runtime
        .spawn_arena_opponent("oteryn:creature.rat", at, |_, _| Some(true))
        .expect("actual native target");
    let first = runtime
        .commit_source_damage(target, player, session, b"srx3-reflector-map", 1, 5, 201)
        .expect("actual autonomous Player source");
    assert!(first.applied);
    assert_eq!(first.health_before - first.health_after, 5);
    assert_eq!(
        runtime
            .borrow_combat_death()
            .current_session_damage_shares(target)
            .expect("existing ordinary credit owner")
            .len(),
        1
    );
    let shares = runtime
        .source_player_damage_shares(target)
        .expect("same canonical damage map");
    assert_eq!(
        shares.len(),
        1,
        "ORIGIN_NONE skips healthChange, not the actual Player damage-map entry"
    );
    assert_eq!(shares[0].actor, player);
    assert_eq!(shares[0].total, 5);
    assert_eq!(shares[0].last_damage_at_ms, Some(201));
    let replay = runtime
        .commit_source_damage(target, player, session, b"srx3-reflector-map", 1, 5, 202)
        .expect("same consumed stream");
    assert!(!replay.applied);
    let after = runtime
        .source_player_damage_shares(target)
        .expect("retained source map");
    assert_eq!(after[0].total, 5);
    assert_eq!(after[0].last_damage_at_ms, Some(201));
}

impl source_proof_seal::Sealed for ReflectionSource {}
impl CurrentSoulWarPlayerSourceProof for ReflectionSource {
    fn current_facts(
        &self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: GameSessionId,
        _: Option<CommandRef>,
        _: u64,
    ) -> Option<SoulWarPlayerSourceFacts> {
        Some(SoulWarPlayerSourceFacts {
            vocation: Vocation::Knight,
            taints: 0,
            realm: None,
            before_revision: 1,
            origin: SoulWarPlayerSourceOrigin::Reflection,
        })
    }
}

#[test]
fn srx3_autonomous_player_source_keeps_prepare_death_and_bypasses_mirror_health_event() {
    let (mut runtime, player, session, image, _) = fixture(8);
    runtime
        .install_current_soul_war_player_source(player, session, None, 200_000, &ReflectionSource)
        .expect("current autonomous Player DATA without a fabricated command");
    let mut at = runtime
        .read_actor_position(player)
        .expect("reflector pose")
        .position();
    at.x += 2;
    let fourth = runtime
        .spawn_arena_opponent("oteryn:creature.bony_sea_devil", at, |_, _| Some(true))
        .expect("native FourthTaint creature");
    for target in [fourth, image] {
        let snapshot = runtime
            .companion_snapshot(target)
            .expect("actual source target");
        let magnitude =
            u32::try_from(snapshot.health).expect("native health fits combat component");
        let result = runtime
            .commit_source_damage(
                target,
                player,
                session,
                b"srx3-autonomous-lethal",
                1,
                magnitude,
                200,
            )
            .expect("real Player prepareDeath facts independent of HEALTHCHANGE invocation");
        assert!(result.applied);
        assert_eq!(
            result.health_after, 0,
            "FourthTaint outside its realm permits the Player kill; ORIGIN_NONE skips Mirror's health callback"
        );
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(target)
            .expect("canonical lethal receipt, never absence inference");
        let replay = runtime
            .commit_source_damage(
                target,
                player,
                session,
                b"srx3-autonomous-lethal",
                1,
                magnitude,
                200,
            )
            .expect("retained autonomous stream receipt");
        assert!(!replay.applied);
    }
}

#[path = "tests/absorption_source.rs"]
mod absorption_source;

#[test]
fn srx3_actual_player_damage_map_retains_source_actor_and_replay_without_master_credit_alias()
-> Result<(), String> {
    let (mut runtime, player, session, image, command) = fixture(8);
    runtime
        .remove_encounter_actor(image)
        .expect("remove healthy image fixture");
    let policy = runtime
        .companion_policy("oteryn:creature.rat")
        .expect("actual native Rat");
    let target = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                "oteryn:creature.rat",
                policy.maximum_health,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("actual canonical native target"),
    );
    runtime
        .install_creature_policy(target, "oteryn:creature.rat")
        .expect("native policy");
    let commit = |runtime: &mut ChannelRuntimeV1| {
        runtime
            .borrow_exact_actor_commit_at(200)
            .commit_damage_for_bound_attacker(
                target,
                player,
                command,
                0,
                OwnerDamageCommand {
                    target: b"oteryn:creature.rat",
                    occurrence: &[],
                    binding: b"source.actual-player.map\0direct",
                    damage: 5,
                },
            )
            .expect("actual current bound Player damage owner")
    };
    assert!(commit(&mut runtime).applied);
    let ordinary = runtime
        .borrow_combat_death()
        .reward_damage_shares(target)
        .expect("ordinary contribution positive control");
    assert_eq!(ordinary.len(), 1);
    assert_eq!(ordinary[0].total, 5);
    assert_eq!(ordinary[0].session, Some(session));
    let source = runtime
        .source_player_damage_shares(target)
        .expect("source data projection");
    assert_eq!(
        source.len(),
        1,
        "literal getKillers(true) must retain the actual Player actor, not merely credit Character"
    );
    assert_eq!(source[0].actor, player);
    assert_eq!(source[0].total, 5);
    assert_eq!(source[0].last_damage_at_ms, Some(200));
    assert!(!commit(&mut runtime).applied);
    assert_eq!(
        runtime
            .source_player_damage_shares(target)
            .map_err(|error| format!("{error:?}"))?,
        source
    );
    Ok(())
}

#[test]
fn srx3_actual_player_periodic_damage_map_retains_source_actor_and_consumed_tick_replay()
-> Result<(), String> {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    let (mut runtime, player, session, image, command) = fixture(9);
    runtime
        .remove_encounter_actor(image)
        .expect("healthy fixture replacement");
    let key = "oteryn:creature.rat";
    let policy = runtime.companion_policy(key).expect("actual native Rat");
    let target = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                key,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("actual native target"),
    );
    runtime
        .install_creature_policy(target, key)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .expect("actual current condition creator data");
    let field = runtime_actor_periodic::CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "source.actual-player.periodic",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                // Rat's real native mitigation truncates a one-point tick to
                // zero. Use a positive finalized component before attribution.
                total_min: 20,
                total_max: 20,
                per_tick: 10,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .ok_or("valid condition definition")?,
        source: Some(player),
        lineage: Some(
            oteryn_gameplay_foundation::foundation::condition::ConditionLineage {
                source_character: id(5),
                source_session: *session.as_bytes(),
                source_lease_generation: 1,
                source_actor_placement: player.placement_identity(),
                source_scope_generation: player.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: *NATIVE_CATALOG.get().expect("actual native catalog"),
                source_creation_command: command.command_id().get(),
            },
        ),
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(118)),
    };
    runtime
        .commit_creature_field_entry(target, 201_000, Some(&field))
        .map_err(|error| format!("{error:?}"))?;
    let receipt = runtime
        .apply_creature_periodic_turn_with_sources(
            target,
            201_000,
            TickFacts::default(),
            &mut |current, source, _| {
                Ok(current
                    .bound_attacker_lease(source, session)
                    .ok()
                    .map(|_| 100))
            },
        )
        .expect("actual source-attributed periodic owner");
    assert!(
        receipt.health_after < receipt.health_before,
        "source damage attribution is tested only after a real canonical tick"
    );
    let source = runtime
        .source_player_damage_shares(target)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        source.len(),
        1,
        "source getDamageMap includes direct Player-created condition damage"
    );
    assert_eq!(source[0].actor, player);
    assert_eq!(
        source[0].total,
        u64::try_from(receipt.health_before - receipt.health_after)
            .map_err(|error| format!("{error:?}"))?
    );
    assert_eq!(source[0].last_damage_at_ms, Some(201));
    runtime
        .apply_creature_periodic_turn_with_sources(
            target,
            201_000,
            TickFacts::default(),
            &mut |current, source, _| {
                Ok(current
                    .bound_attacker_lease(source, session)
                    .ok()
                    .map(|_| 100))
            },
        )
        .expect("consumed tick replay");
    assert_eq!(
        runtime
            .source_player_damage_shares(target)
            .map_err(|error| format!("{error:?}"))?,
        source
    );
    Ok(())
}

#[test]
fn srx3_autonomous_reflection_lethal_enqueues_actual_source_death_before_reward()
-> Result<(), String> {
    let (mut runtime, player, session, image, _) = fixture(10);
    runtime
        .remove_encounter_actor(image)
        .expect("healthy fixture replacement");
    let key = "oteryn:creature.goshnar_s_megalomania_blue";
    let policy = runtime
        .companion_policy(key)
        .expect("actual native source boss");
    let target = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                key,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("current native source target"),
    );
    runtime
        .install_creature_policy(target, key)
        .expect("qualified current native death metadata");
    let damage = u32::try_from(policy.maximum_health).map_err(|error| format!("{error:?}"))?;
    let receipt = runtime
        .commit_source_damage(
            target,
            player,
            session,
            b"source.reflection.lethal.queue",
            1,
            damage,
            201,
        )
        .expect("actual autonomous HP owner");
    assert!(receipt.applied);
    assert_eq!(receipt.health_after, 0);
    let original = runtime
        .borrow_combat_death()
        .committed_lethal_receipt(target)
        .expect("actual immutable lethal receipt")
        .projection
        .occurrence
        .death_key()
        .map_err(|error| format!("{error:?}"))?;
    let notice = runtime
        .next_source_creature_death()
        .map_err(|error| format!("{error:?}"))?
        .expect("manual autonomous Slot publication must retain the same source death FIFO notice");
    assert_eq!(notice.actor(), target);
    assert_eq!(notice.death_key(), original);
    let begin = runtime
        .begin_source_creature_death(&notice)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        begin.has_reward_damage_credit(),
        "an actual reflected Player contribution cannot become an uncredited corpse cause"
    );
    assert!(
        !runtime
            .commit_source_damage(
                target,
                player,
                session,
                b"source.reflection.lethal.queue",
                1,
                damage,
                202
            )
            .map_err(|error| format!("{error:?}"))?
            .applied
    );
    assert_eq!(
        runtime
            .next_source_creature_death()
            .map_err(|error| format!("{error:?}"))?,
        Some(notice)
    );
    Ok(())
}

#[path = "tests/hatred_periodic.rs"]
mod hatred_periodic;
#[path = "tests/spite_expiry.rs"]
mod spite_expiry;

#[test]
fn srx3_cloak_accepted_q6a_still_skips_origin_none_reflection() {
    let (mut runtime, player, session, _, _) = fixture(8);
    let mut at = runtime
        .read_actor_position(player)
        .expect("actual reflector pose")
        .position();
    at.x += 2;
    let cloak = runtime
        .spawn_arena_opponent("oteryn:creature.cloak_of_terror", at, |_, _| Some(true))
        .expect("actual source native Cloak");
    let applied = runtime
        .commit_source_damage(cloak, player, session, b"q6a.origin-none", 1, 100, 201)
        .expect("canonical autonomous reflection with real Player attribution");
    assert!(applied.applied);
    assert_eq!(applied.health_before - applied.health_after, 100);
    assert!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("same canonical queue")
            .is_empty(),
        "OriginNONE does not invoke HealthChange; Q6a does not authorize a reflection pool"
    );
    let replay = runtime
        .commit_source_damage(cloak, player, session, b"q6a.origin-none", 1, 100, 202)
        .expect("same source stream replay");
    assert!(!replay.applied);
    assert!(
        runtime
            .pending_cloak_blood_events(64)
            .expect("replay queue")
            .is_empty()
    );
}
