fn wvar_gate_case(run: fn()) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(run)
        .expect("owner test thread")
        .join()
        .expect("owner test completed");
}

fn wvar_gate_definitions() -> Vec<SpawnDefinition> {
    let cells = (2..8).map(|x| (x, 0, 7)).collect::<Vec<_>>();
    let guards = SpawnDefinition::new(
        cells.clone(),
        6,
        SPAWN_DELAY,
        5_000_000,
        "oteryn:creature/rat".to_string(),
        20,
    )
    .expect("guards");
    let boss = SpawnDefinition::new(
        vec![(10, 0, 7)],
        1,
        1_768_000_000,
        5_000_000,
        "oteryn:creature/bat".to_string(),
        35,
    )
    .expect("boss")
    .with_kill_prerequisites(
        cells
            .into_iter()
            .map(|cell| (cell, "oteryn:creature/rat".to_string()))
            .collect(),
    )
    .expect("six distinct prerequisites");
    vec![guards, boss]
}

fn wvar_project_guard(runtime: &mut ChannelRuntimeV1, ordinal: usize) -> ExactActorRef {
    let actor = kill_spawn_cell(
        &runtime.continuity,
        &mut runtime.carrier,
        SpawnSourceId(0),
        ordinal,
    );
    wvar_project_dead_actor(runtime, actor);
    actor
}

fn wvar_project_dead_actor(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
    let receipt = runtime
        .carrier
        .committed_lethal_receipt_inner(&runtime.continuity, actor.0)
        .expect("actual committed lethal receipt");
    let replay = CommittedLethalReceipt {
        projection: receipt.projection.clone(),
    };
    runtime
        .carrier
        .project_committed_lethal_inner(&runtime.continuity, receipt, false, false)
        .expect("actual committed death projection");
    runtime
        .carrier
        .project_committed_lethal_inner(&runtime.continuity, replay, false, false)
        .expect("projection replay");
}

#[test]
fn wvar_spawn_gate_requires_six_distinct_committed_guard_deaths() {
    wvar_gate_case(|| {
        let mut runtime = runtime(16);
        assert_eq!(
            runtime.begin_spawn_definitions(wvar_gate_definitions()),
            Ok(7)
        );
        runtime
            .realize_activation_spawn_window(0)
            .expect("activate");
        let boss = spawn::SpawnPoint {
            source: 1,
            ordinal: 0,
        };
        assert_eq!(
            runtime
                .spawn_point_creatures()
                .iter()
                .filter(|(_, _, a)| a.is_some())
                .count(),
            6
        );
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("closed drive");
        assert_eq!(
            runtime.pending_respawn(boss),
            None,
            "no boot activation or timer"
        );

        let first = wvar_project_guard(&mut runtime, 0);
        runtime
            .drive_spawn_respawns(50_000, reference_view)
            .expect("guard gone");
        runtime
            .drive_spawn_respawns(50_000 + SPAWN_DELAY, reference_view)
            .expect("guard warning");
        let respawned = runtime
            .drive_spawn_respawns(
                50_000 + SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS,
                reference_view,
            )
            .expect("guard respawn");
        assert_eq!(respawned.admitted.len(), 1);
        assert_ne!(respawned.admitted[0], first);
        wvar_project_guard(&mut runtime, 0);
        for ordinal in 1..5 {
            wvar_project_guard(&mut runtime, ordinal);
        }
        runtime
            .drive_spawn_respawns(65_000_000, reference_view)
            .expect("five unique guards");
        assert_eq!(
            runtime.pending_respawn(boss),
            None,
            "farming and replay cannot replace sixth guard"
        );
        wvar_project_guard(&mut runtime, 5);
        let opened = runtime
            .drive_spawn_respawns(65_050_000, reference_view)
            .expect("gate opens");
        assert_eq!(
            opened.warnings,
            vec![MovementLocalPosition {
                x: 10,
                y: 0,
                floor: 7
            }]
        );
        let realized = runtime
            .drive_spawn_respawns(65_050_000 + spawn::SPAWN_WARNING_MICROS, reference_view)
            .expect("existing respawn admission");
        assert_eq!(realized.admitted.len(), 1);
        assert_eq!(
            spawned_creature(&runtime, realized.admitted[0]),
            Some((b"oteryn:creature/bat".to_vec(), 35))
        );
        assert_eq!(runtime.pending_respawn(boss), None);
    });
}

#[test]
fn wvar_spawn_gate_ignores_removal_and_failed_projection() {
    wvar_gate_case(|| {
        let mut runtime = runtime(16);
        runtime
            .begin_spawn_definitions(wvar_gate_definitions())
            .expect("begin");
        runtime
            .realize_activation_spawn_window(0)
            .expect("activate");
        let guard = runtime.spawn_point_creatures()[0].2.expect("guard");
        runtime
            .carrier
            .remove(&runtime.continuity, guard.0)
            .expect("administrative removal");
        let context = runtime.pinned_position_context();
        let impostor = runtime
            .carrier
            .admit_creature(
                &runtime.continuity,
                ActorState(0),
                "oteryn:creature/rat",
                20,
            )
            .expect("unbound creature at same guard home");
        assert_ne!(ExactActorRef(impostor), guard);
        runtime
            .carrier
            .initialize_position(
                &runtime.continuity,
                impostor,
                context,
                LocalPosition {
                    x: 2,
                    y: 0,
                    floor: 7,
                },
            )
            .expect("same position is not same spawn identity");
        runtime
            .carrier
            .current_owner_exact_commit(&runtime.continuity)
            .commit_damage(
                ExactActorRef(impostor),
                OwnerDamageCommand {
                    target: b"oteryn:creature/rat",
                    occurrence: b"kill",
                    binding: b"kill\0spawn-gate-impostor",
                    damage: 20,
                },
            )
            .expect("unbound lethal commit");
        wvar_project_dead_actor(&mut runtime, ExactActorRef(impostor));
        for ordinal in 1..6 {
            wvar_project_guard(&mut runtime, ordinal);
        }
        let boss = spawn::SpawnPoint {
            source: 1,
            ordinal: 0,
        };
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe removed guard");
        assert_eq!(
            runtime.pending_respawn(boss),
            None,
            "neither removal nor another same-creature/same-position death qualifies"
        );
        runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("warning");
        runtime
            .drive_spawn_respawns(SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS, reference_view)
            .expect("respawn");
        let dead = kill_spawn_cell(
            &runtime.continuity,
            &mut runtime.carrier,
            SpawnSourceId(0),
            0,
        );
        let receipt = runtime
            .carrier
            .committed_lethal_receipt_inner(&runtime.continuity, dead.0)
            .expect("receipt");
        let retry = CommittedLethalReceipt {
            projection: receipt.projection.clone(),
        };
        assert_eq!(
            runtime.carrier.project_committed_lethal_inner(
                &runtime.continuity,
                receipt,
                true,
                false
            ),
            Err(CarrierError::InjectedCorpseProjectionFailure)
        );
        runtime
            .drive_spawn_respawns(65_000_000, reference_view)
            .expect("failed projection drive");
        assert_eq!(
            runtime.pending_respawn(boss),
            None,
            "unprojected lethal state cannot open gate"
        );
        assert_eq!(
            runtime
                .carrier
                .project_committed_lethal_inner(&runtime.continuity, retry, false, true),
            Err(CarrierError::InjectedCorpseResponseFailure)
        );
        let opened = runtime
            .drive_spawn_respawns(65_050_000, reference_view)
            .expect("committed projection");
        assert_eq!(
            opened.warnings,
            vec![MovementLocalPosition {
                x: 10,
                y: 0,
                floor: 7
            }]
        );
    });
}

#[test]
fn wvar_spawn_gate_rejects_missing_ambiguous_and_self_dependencies_before_activation() {
    wvar_gate_case(|| {
        let mut definitions = wvar_gate_definitions();
        let boss = definitions.pop().expect("boss");
        for invalid in [
            vec![boss.clone()],
            vec![definitions[0].clone(), definitions[0].clone(), boss.clone()],
        ] {
            let mut runtime = runtime(32);
            assert_eq!(
                runtime.begin_spawn_definitions(invalid),
                Err(CarrierError::InvalidSpawnDefinition)
            );
            assert!(runtime.spawn_point_creatures().is_empty());
        }
        let self_gated = definitions[0]
            .clone()
            .with_kill_prerequisites(vec![((2, 0, 7), "oteryn:creature/rat".to_string())])
            .expect("structurally valid identity");
        let mut runtime = runtime(16);
        assert_eq!(
            runtime.begin_spawn_definitions(vec![self_gated]),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        assert!(runtime.spawn_point_creatures().is_empty());
        assert_eq!(
            boss.with_kill_prerequisites(vec![
                ((2, 0, 7), "oteryn:creature/rat".to_string()),
                ((2, 0, 7), "oteryn:creature/rat".to_string()),
            ]),
            Err(CarrierError::InvalidSpawnDefinition)
        );
    });
}

#[test]
fn wvar_spawn_gate_preparation_is_cleared_when_one_turn_activation_refuses() {
    wvar_gate_case(|| {
        let mut definitions = wvar_gate_definitions();
        for source in 0..64 {
            definitions.push(
                SpawnDefinition::new(
                    (0..64)
                        .map(|cell| (1_000 + source * 64 + cell, 0, 7))
                        .collect(),
                    64,
                    SPAWN_DELAY,
                    5_000_000,
                    "test:other".to_string(),
                    20,
                )
                .expect("extra source within per-source ceiling"),
            );
        }
        let mut runtime = runtime(5_000);
        assert_eq!(
            runtime.realize_spawn_definitions(definitions),
            Err(CarrierError::CapacityExceeded)
        );
        assert!(runtime.spawn_activation.is_none());
        assert!(runtime.carrier.spawns.is_empty());
        assert!(runtime.carrier.spawn_kill_gates.is_empty());
        runtime
            .begin_spawn_definitions(wvar_gate_definitions())
            .expect("clean retry");
        runtime
            .realize_activation_spawn_window(0)
            .expect("retry activation");
        assert_eq!(
            runtime
                .spawn_point_creatures()
                .iter()
                .filter(|(_, _, actor)| actor.is_some())
                .count(),
            6
        );
    });
}
