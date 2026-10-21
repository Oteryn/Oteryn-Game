use super::*;
use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
// Isolated proof: real current carrier, owner damage commits and corpse projections.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn bone_phase_actual_sealed_owner_deaths_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
        NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed");
    fn actor(r: &mut ChannelRuntimeV1, key: &[u8], hp: i64) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(0),
                std::str::from_utf8(key).expect("valid native fixture operation must succeed"),
                hp,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    fn kill(r: &mut ChannelRuntimeV1, a: ExactActorRef, key: &[u8]) {
        r.borrow_exact_actor_commit()
            .commit_damage(
                a,
                OwnerDamageCommand {
                    target: key,
                    occurrence: b"source-cage-death",
                    binding: b"source-cage-death\0lethal",
                    damage: 120000,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut d = r.borrow_combat_death();
        let receipt = d
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        d.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    let cages = BONE_CAGE_KEYS.map(|key| actor(&mut r, key, 120000));
    let phylactery = actor(&mut r, b"oteryn:creature.bonelord_s_phylactery", 50000);
    assert!(
        r.bind_bone_overlord_cage_phase(cages, phylactery, "wrong", [1; 32])
            .is_err()
    );
    assert!(
        r.bind_bone_overlord_cage_phase(cages, phylactery, BONE_CASE_FINGERPRINT, [9; 32])
            .is_err()
    );
    let mut bad = cages;
    bad[0] = cages[1];
    assert!(
        r.bind_bone_overlord_cage_phase(bad, phylactery, BONE_CASE_FINGERPRINT, [1; 32])
            .is_err()
    );
    let mut state = r
        .bind_bone_overlord_cage_phase(cages, phylactery, BONE_CASE_FINGERPRINT, [1; 32])
        .expect("valid native fixture operation must succeed");
    assert!(
        r.commit_bone_overlord_projected_death(&mut state, cages[0])
            .is_err()
    );
    assert_eq!(state.dead_mask, 0);
    let unrelated = actor(&mut r, b"oteryn:creature.cat", 10);
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, unrelated),
        Err(BonePhaseError::WrongParticipant)
    );
    for i in 0..4 {
        kill(&mut r, cages[i], BONE_CAGE_KEYS[i]);
        let x = r
            .commit_bone_overlord_projected_death(&mut state, cages[i])
            .expect("valid native fixture operation must succeed");
        assert!(x.newly_committed);
        assert_eq!(x.cages_destroyed, (i + 1) as u8);
        assert_eq!(
            x.phase,
            if i == 3 {
                BoneOverlordPhase::Phylactery
            } else {
                BoneOverlordPhase::Soulcages
            }
        );
        let duplicate = r
            .commit_bone_overlord_projected_death(&mut state, cages[i])
            .expect("valid native fixture operation must succeed");
        assert!(!duplicate.newly_committed);
        assert_eq!(duplicate.cages_destroyed, x.cages_destroyed);
    }
    kill(&mut r, phylactery, b"oteryn:creature.bonelord_s_phylactery");
    let victory = r
        .commit_bone_overlord_projected_death(&mut state, phylactery)
        .expect("valid native fixture operation must succeed");
    assert_eq!(victory.phase, BoneOverlordPhase::Completed);
    assert!(victory.newly_committed);
    assert!(
        !r.commit_bone_overlord_projected_death(&mut state, phylactery)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    // An old exact member cannot advance a new instance after its slot is reused.
    r.remove_encounter_actor(cages[0])
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, BONE_CAGE_KEYS[0], 120000);
    assert_ne!(replacement, cages[0]);
    assert!(
        r.commit_bone_overlord_projected_death(&mut state, cages[0])
            .is_err()
    );
    // A source token is fenced by native content even when actor references look plausible.
    state.server = [9; 32];
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, cages[1]),
        Err(BonePhaseError::ContentChanged)
    );
    println!(
        "PASS Bone native4distinct sealed deaths/phase/duplicate/source/current-pin/stale-generation; full sharedHP/spawn/gates NOT CLAIMED"
    );
    // Exercise the relocated hooks through the real owner damage/heal/projection
    // dispatch as well. This qualifies the carrier integration; retained source
    // membership remains covered by the separate actual-source registry test.
    bone_shared_actual_owner_harness(world, [1; 32], |runtime, cages, phylactery| {
        runtime
            .bind_bone_overlord_cage_phase(cages, phylactery, BONE_CASE_FINGERPRINT, [1; 32])
            .expect("native owner phase binding")
    });
}

// Test-only native owner harness. Registration callback supplied by root test reads actual project.
// No Content dependency is introduced into standalone Foundation path includes.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
pub fn bone_shared_actual_owner_harness(
    world: WorldId,
    artifact_digest: [u8; 32],
    mut register: impl FnMut(
        &ChannelRuntimeV1,
        [ExactActorRef; 4],
        ExactActorRef,
    ) -> BoneOverlordCagePhase,
) {
    fn id(t: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = t;
        b
    }
    fn runtime(
        world: oteryn_gameplay_foundation::foundation::WorldId,
        artifact_digest: [u8; 32],
    ) -> ChannelRuntimeV1 {
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            32,
            ChannelContentPin::from_activation(
                world,
                1,
                artifact_digest,
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn actor(r: &mut ChannelRuntimeV1, k: &[u8], hp: i64) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(0),
                std::str::from_utf8(k).expect("valid native fixture operation must succeed"),
                hp,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    fn hp(r: &ChannelRuntimeV1, a: ExactActorRef) -> i64 {
        let i = r
            .encounter_actor_index(a)
            .expect("valid native fixture operation must succeed");
        match r.encounter_slot(i) {
            Slot::CreatureOccupied { health, .. } => Some(*health),
            _ => None,
        }
        .expect("fixture must contain a native creature")
    }
    fn damage(
        r: &mut ChannelRuntimeV1,
        a: ExactActorRef,
        key: &[u8],
        occ: &[u8],
        binding: &[u8],
        n: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        r.borrow_exact_actor_commit().commit_damage(
            a,
            OwnerDamageCommand {
                target: key,
                occurrence: occ,
                binding,
                damage: n,
            },
        )
    }
    let mut r = runtime(world, artifact_digest);
    let cages = BONE_CAGE_KEYS.map(|k| actor(&mut r, k, 120000));
    let phyl = actor(&mut r, b"oteryn:creature.bonelord_s_phylactery", 50000);
    let mut state = register(&r, cages, phyl);
    let scope = oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
        world,
        r.binding().channel_id(),
    );
    let generation = r.binding().scope_generation();
    let mut fence = oteryn_gameplay_foundation::foundation::scoped_fence_fixture(scope, generation);
    let ordinal = fence
        .accept_input(generation)
        .expect("valid native fixture operation must succeed");
    let stamp = fence.stamp(ordinal);

    let mut wrong = [0; 16];
    wrong[6] = 0x70;
    wrong[8] = 0x80;
    wrong[15] = 99;
    let wrong = oteryn_gameplay_foundation::foundation::scoped_fence_fixture(
        oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
            world,
            ChannelId::decode(&wrong).expect("valid native fixture operation must succeed"),
        ),
        generation,
    );
    assert!(r.register_bone_shared_hp(&state, &wrong, stamp).is_err());
    r.register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert_eq!(bone_groups(&r).len(), 1);
    r.register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert_eq!(bone_groups(&r).len(), 1);
    assert_eq!(
        damage(
            &mut r,
            phyl,
            b"oteryn:creature.bonelord_s_phylactery",
            b"premature",
            b"premature\0hit",
            50000
        ),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(hp(&r, phyl), 50000);
    let first = damage(&mut r, cages[1], BONE_CAGE_KEYS[1], b"A", b"A\0hit400", 400)
        .expect("valid native fixture operation must succeed");
    assert!(first.applied);
    for a in cages {
        assert_eq!(hp(&r, a), 119600)
    }
    assert!(
        !damage(&mut r, cages[1], BONE_CAGE_KEYS[1], b"A", b"A\0hit400", 400)
            .expect("valid native fixture operation must succeed")
            .applied
    );
    assert!(
        damage(
            &mut r,
            cages[1],
            BONE_CAGE_KEYS[1],
            b"A",
            b"A\0changed",
            500
        )
        .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119600)
    }
    let heals = vec![
        (
            cages[2],
            std::str::from_utf8(BONE_CAGE_KEYS[2])
                .expect("valid native fixture operation must succeed")
                .to_owned(),
            120000,
            200,
        ),
        (
            cages[0],
            std::str::from_utf8(BONE_CAGE_KEYS[0])
                .expect("valid native fixture operation must succeed")
                .to_owned(),
            120000,
            100,
        ),
    ];
    let healed = r
        .commit_source_creature_heal_batch(artifact_digest, &heals)
        .expect("valid native fixture operation must succeed");
    assert_eq!(healed.len(), 2);
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    } // source-roster lowest index chosen, oncecast
    let duplicate = vec![heals[0].clone(), heals[0].clone()];
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &duplicate)
            .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    }
    let mut map_revision_digest = r.content_pin().map_revision_digest();
    map_revision_digest[0] ^= 1;
    let entry_start = r.content_pin().entry_start();
    r.replace_encounter_fixture_content(ChannelContentPin::from_activation(
        r.content_pin().world_id(),
        r.content_pin().activation_sequence(),
        r.content_pin().server_artifact_digest(),
        r.content_pin().client_artifact_digest(),
        r.content_pin().frame_binding_digest(),
        map_revision_digest,
        (entry_start.x, entry_start.y, entry_start.floor),
    ));
    assert!(
        damage(
            &mut r,
            cages[0],
            BONE_CAGE_KEYS[0],
            b"generic-map",
            b"generic-map\0hit",
            100
        )
        .is_err()
    );
    assert!(
        damage(
            &mut r,
            phyl,
            b"oteryn:creature.bonelord_s_phylactery",
            b"generic-phyl-map",
            b"generic-phyl-map\0hit",
            1
        )
        .is_err()
    );
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &heals)
            .is_err()
    );
    assert!(
        r.commit_bone_shared_damage(
            &state,
            &fence,
            stamp,
            cages[0],
            OwnerDamageCommand {
                target: BONE_CAGE_KEYS[0],
                occurrence: b"map",
                binding: b"map\0hit",
                damage: 100
            },
            None,
            false
        )
        .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    }
    let mut map_revision_digest = r.content_pin().map_revision_digest();
    map_revision_digest[0] ^= 1;
    let entry_start = r.content_pin().entry_start();
    r.replace_encounter_fixture_content(ChannelContentPin::from_activation(
        r.content_pin().world_id(),
        r.content_pin().activation_sequence(),
        r.content_pin().server_artifact_digest(),
        r.content_pin().client_artifact_digest(),
        r.content_pin().frame_binding_digest(),
        map_revision_digest,
        (entry_start.x, entry_start.y, entry_start.floor),
    ));
    damage(
        &mut r,
        cages[0],
        BONE_CAGE_KEYS[0],
        b"lethal",
        b"lethal\0hp",
        120000,
    )
    .expect("valid native fixture operation must succeed");
    for a in cages {
        assert_eq!(hp(&r, a), 0)
    }
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &heals)
            .is_err()
    );
    for (i, a) in cages.into_iter().enumerate() {
        let mut owner = r.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
        let projected = owner
            .projected_death(a)
            .expect("valid native fixture operation must succeed")
            .0;
        assert_eq!(projected.actor_local_id(), a.0.actor_local_id.0);
        let phase = r
            .commit_bone_overlord_projected_death(&mut state, a)
            .expect("valid native fixture operation must succeed");
        assert_eq!(phase.cages_destroyed, (i + 1) as u8);
        if i < 3 {
            assert!(
                damage(
                    &mut r,
                    phyl,
                    b"oteryn:creature.bonelord_s_phylactery",
                    b"early",
                    b"early\0hit",
                    1
                )
                .is_err()
            );
        }
    }
    assert_eq!(state.phase, BoneOverlordPhase::Phylactery);
    assert!(bone_groups(&r)[0].cage_deaths.iter().all(Option::is_some));
    damage(
        &mut r,
        phyl,
        b"oteryn:creature.bonelord_s_phylactery",
        b"victory",
        b"victory\0hit",
        50000,
    )
    .expect("valid native fixture operation must succeed");
    {
        let mut owner = r.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(phyl)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, phyl)
            .expect("valid native fixture operation must succeed")
            .phase,
        BoneOverlordPhase::Completed
    );
    assert_eq!(r.encounter_fixture_death_counts().0, 5);
    assert!(r.encounter_fixture_death_counts().1 == 0);
    // Late fourth-slot conflict: three prepared native slots must remain unpublished.
    let mut q = runtime(world, artifact_digest);
    let c = BONE_CAGE_KEYS.map(|k| actor(&mut q, k, 120000));
    let p = actor(&mut q, b"oteryn:creature.bonelord_s_phylactery", 50000);
    let st = register(&q, c, p);
    for i in 0..3 {
        damage(&mut q, c[i], BONE_CAGE_KEYS[i], b"seed", b"seed\0hit", 400)
            .expect("valid native fixture operation must succeed");
    }
    damage(
        &mut q,
        c[3],
        BONE_CAGE_KEYS[3],
        b"late",
        b"late\0hit400",
        400,
    )
    .expect("valid native fixture operation must succeed");
    q.register_bone_shared_hp(&st, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    let before = q.encounter_fixture_snapshot();
    assert_eq!(
        damage(
            &mut q,
            c[0],
            BONE_CAGE_KEYS[0],
            b"late",
            b"late\0hit200",
            200
        ),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(q.encounter_fixture_snapshot(), before);
    q.remove_encounter_actor(c[1])
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut q, BONE_CAGE_KEYS[1], 120000);
    let before = q.encounter_fixture_snapshot();
    assert!(
        damage(
            &mut q,
            c[0],
            BONE_CAGE_KEYS[0],
            b"stale",
            b"stale\0hit",
            200
        )
        .is_err()
    );
    assert_eq!(q.encounter_fixture_snapshot(), before);
    assert_ne!(replacement, c[1]);
    // Same source species and identical physical HP: only exact local generation can refuse fanout.
    let mut replacement_case = runtime(world, artifact_digest);
    let rc = BONE_CAGE_KEYS.map(|k| actor(&mut replacement_case, k, 120000));
    let rp = actor(
        &mut replacement_case,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let rs = register(&replacement_case, rc, rp);
    replacement_case
        .register_bone_shared_hp(&rs, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    damage(
        &mut replacement_case,
        rc[0],
        BONE_CAGE_KEYS[0],
        b"before-replacement",
        b"before-replacement\0damage400",
        400,
    )
    .expect("valid native fixture operation must succeed");
    replacement_case
        .remove_encounter_actor(rc[1])
        .expect("valid native fixture operation must succeed");
    let reused = actor(&mut replacement_case, BONE_CAGE_KEYS[1], 119600);
    assert_eq!(reused.0.actor_local_id, rc[1].0.actor_local_id);
    assert_ne!(
        reused.0.actor_local_generation,
        rc[1].0.actor_local_generation
    );
    let before = replacement_case.encounter_fixture_snapshot();
    assert_eq!(
        damage(
            &mut replacement_case,
            rc[0],
            BONE_CAGE_KEYS[0],
            b"replacement-fanout",
            b"replacement-fanout\0damage200",
            200
        ),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(replacement_case.encounter_fixture_snapshot(), before);
    let heal = vec![(
        rc[0],
        std::str::from_utf8(BONE_CAGE_KEYS[0])
            .expect("valid native fixture operation must succeed")
            .to_owned(),
        120000,
        100,
    )];
    assert_eq!(
        replacement_case.commit_source_creature_heal_batch(artifact_digest, &heal),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(replacement_case.encounter_fixture_snapshot(), before);
    assert_eq!(hp(&replacement_case, reused), 119600);
    // Retired exact generations must not consume the16-group bound forever. New same-species
    // occupants in reused indexes are not the retired group's members. Keep a partially live group.
    let mut pruning = runtime(world, artifact_digest);
    let live = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
    let live_phyl = actor(
        &mut pruning,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let live_state = register(&pruning, live, live_phyl);
    pruning
        .register_bone_shared_hp(&live_state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    for turn in 0..20 {
        let c = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
        let p = actor(
            &mut pruning,
            b"oteryn:creature.bonelord_s_phylactery",
            50000,
        );
        let state = register(&pruning, c, p);
        pruning
            .register_bone_shared_hp(&state, &fence, stamp)
            .expect("valid native fixture operation must succeed");
        assert_eq!(bone_groups(&pruning).len(), 2);
        assert!(bone_groups(&pruning).iter().any(|g| g.cages == live));
        for a in c.into_iter().chain(std::iter::once(p)) {
            pruning
                .remove_encounter_actor(a)
                .expect("valid native fixture operation must succeed");
        }
        assert_eq!(hp(&pruning, live[0]), 120000);
        assert!(turn < 20);
    }
    // Remove four cages, retaining only the exact HP0/present or live Phyl member still holds group.
    for a in live {
        pruning
            .remove_encounter_actor(a)
            .expect("valid native fixture operation must succeed");
    }
    let c = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
    let p = actor(
        &mut pruning,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let state = register(&pruning, c, p);
    pruning
        .register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert!(
        bone_groups(&pruning)
            .iter()
            .any(|g| g.phylactery == live_phyl)
    );
    assert_eq!(bone_groups(&pruning).len(), 2);
    println!(
        "PASS actual5 source roster/bindings; native generic4slot shareddamage/heal coalescence; fullpin/map/fence guards; replay/conflict/late-fourth atomicity; real4sealed cage keys openPhyl only after4; distinct finaldeath; no reward mint; stale replacement untouched. PROJECT120000, no spirit/map Global claims."
    );
}

#[allow(clippy::expect_used)]
fn bone_groups(runtime: &ChannelRuntimeV1) -> &[NativeBoneSharedGroup] {
    &runtime
        .encounter_hooks()
        .expect("registered hooks")
        .as_any()
        .downcast_ref::<BoneEncounterHooks>()
        .expect("Bone hooks")
        .groups
}
