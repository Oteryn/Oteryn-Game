/// Real activation/respawn owner: one weighted lifetime plus one independent point.
#[test]
fn srv7_weighted_spawn_boot_and_respawn_keep_one_cell_one_cooldown_and_native_health() {
    const SENTINEL: &str = "oteryn:creature.silverfrost_sentinel";
    const ASURA: &str = "oteryn:creature.true_frost_flower_asura";
    const DELAY: u64 = 47_104_000_000;
    let mut runtime = runtime(8);
    let source = (
        vec![
            ((2, 0, 7), 47_104_000, ASURA.to_owned(), 200, false),
            ((3, 0, 7), 60_000, ASURA.to_owned(), 200, false),
        ],
        5_000,
    );
    let choices = vec![ActivationWeightedSpawnPoint {
        source: 0,
        point: 0,
        choices: vec![
            ActivationSpawnAlternative {
                creature: SENTINEL.into(),
                weight: 25,
                maximum_health: 100,
                blockable: true,
            },
            ActivationSpawnAlternative {
                creature: ASURA.into(),
                weight: 25,
                maximum_health: 200,
                blockable: false,
            },
        ],
    }];
    assert_eq!(
        runtime.begin_activation_spawn_plan(vec![source], choices),
        Ok(2)
    );
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .expect("real boot window")
            .realized,
        2
    );
    let points = runtime.spawn_point_creatures();
    let selected = points[0].0;
    let south = points[1].2.expect("independent south actor");
    let mut seen = std::collections::BTreeSet::new();
    let mut now = 50_000;
    for _ in 0..48 {
        let actor = runtime.spawn_point_creatures()[0]
            .2
            .expect("one weighted actor");
        let (key, health) = spawned_creature(&runtime, actor).expect("native actor body");
        assert_eq!(health, if key == SENTINEL.as_bytes() { 100 } else { 200 });
        seen.insert(key);
        assert_eq!(
            runtime.carrier.occupied.len(),
            2,
            "no overlapping weighted actors"
        );
        assert_eq!(
            runtime.carrier.spawns.len(),
            1,
            "source stays one runtime source"
        );
        assert_eq!(runtime.spawn_point_creatures()[1].2, Some(south));
        kill_creature(&mut runtime, actor);
        runtime
            .drive_spawn_respawns(now, reference_view)
            .expect("observe death");
        let due = runtime
            .pending_respawn(selected)
            .expect("one shared timer")
            .due_micros;
        assert_eq!(
            due,
            now + DELAY,
            "both identities use final XML child47104s"
        );
        assert!(
            runtime
                .drive_spawn_respawns(due - 1, reference_view)
                .expect("not due")
                .admitted
                .is_empty()
        );
        let due_report = runtime
            .drive_spawn_respawns(due, reference_view)
            .expect("source due step");
        if due_report.admitted.len() == 1 {
            assert!(due_report.warnings.is_empty());
            assert_eq!(
                spawned_creature(&runtime, due_report.admitted[0])
                    .expect("immediate body")
                    .0,
                SENTINEL.as_bytes()
            );
            now = due;
        } else {
            assert_eq!(due_report.warnings.len(), 1);
            now = due + spawn::SPAWN_WARNING_MICROS;
            let admitted = runtime
                .drive_spawn_respawns(now, reference_view)
                .expect("staged source respawn");
            assert_eq!(admitted.admitted.len(), 1);
            assert_eq!(
                spawned_creature(&runtime, admitted.admitted[0])
                    .expect("staged body")
                    .0,
                ASURA.as_bytes()
            );
        }
        now += 50_000;
    }
    assert_eq!(
        seen,
        std::collections::BTreeSet::from([SENTINEL.as_bytes().to_vec(), ASURA.as_bytes().to_vec()]),
        "production boot/respawn must make both source alternatives reachable"
    );
}

#[test]
fn srv7_weighted_spawn_malformed_plan_admits_nothing() {
    for case in 0..4 {
        let mut runtime = runtime(8);
        let source = (
            vec![(
                (2, 0, 7),
                47_104_000,
                "oteryn:creature.bat".into(),
                35,
                false,
            )],
            5_000,
        );
        let mut selection = ActivationWeightedSpawnPoint {
            source: 0,
            point: 0,
            choices: vec![
                ActivationSpawnAlternative {
                    creature: "oteryn:creature.bat".into(),
                    weight: 25,
                    maximum_health: 35,
                    blockable: false,
                },
                ActivationSpawnAlternative {
                    creature: "oteryn:creature.rat".into(),
                    weight: 25,
                    maximum_health: 20,
                    blockable: false,
                },
            ],
        };
        match case {
            0 => selection.choices[0].weight = 0,
            1 => selection.choices[0].weight = u32::MAX,
            2 => selection.source = 1,
            _ => selection.choices[0].maximum_health = 0,
        }
        assert!(
            runtime
                .begin_activation_spawn_plan(vec![source], vec![selection])
                .is_err()
        );
        assert!(runtime.carrier.occupied.is_empty());
        assert!(runtime.spawn_activation.is_none());
    }
}
