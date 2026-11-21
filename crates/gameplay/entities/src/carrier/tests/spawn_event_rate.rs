/// W05d: a scheduled event spawn rate scales the respawn delay the real drive schedules
/// (`delay * 100 / rate`, Crystal `spawn_monster.cpp:443-458`) only inside its window.
#[test]
fn w05d_an_event_spawn_rate_scales_the_scheduled_respawn_only_inside_its_window() {
    use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
    // 2030-01-01T00:00:00Z and 2030-01-02T23:59:59Z.
    const START: i64 = 1_893_456_000;
    const END: i64 = 1_893_628_799;
    let due_at = |now: Option<i64>| {
        let schedule = Schedule::parse(
            r#"{"events":[{"name":"w05d","startdate":"01/01/2030","enddate":"01/02/2030",
                "ingame":{"spawnrate":200}}]}"#,
        )
        .expect("schedule");
        let _clock = now.map(|now| test_clock::set(schedule, now));
        let mut runtime = spawned_runtime(den_definition());
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        runtime
            .drive_spawn_respawns(1_000, reference_view)
            .expect("drive");
        let point = runtime.spawn_point_creatures()[0].0;
        runtime.pending_respawn(point).map(|p| p.due_micros)
    };
    assert_eq!(due_at(None), Some(1_000 + SPAWN_DELAY));
    for now in [START - 1, END + 1] {
        assert_eq!(due_at(Some(now)), Some(1_000 + SPAWN_DELAY), "outside");
    }
    for now in [START, END] {
        assert_eq!(due_at(Some(now)), Some(1_000 + SPAWN_DELAY / 2), "edge");
    }
}

#[test]
fn initial_blocked_and_skipped_respawns_compose_all_rates() {
    use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
    let schedule = Schedule::parse(r#"{"events":[{"name":"boost","startdate":"01/01/2030","enddate":"01/02/2030","ingame":{"spawnrate":200}}]}"#).expect("schedule");
    let _clock = test_clock::set(schedule, 1_893_456_000);
    for blockable in [true, false] {
        let mut runtime = spawned_runtime(den_definition().with_blockable(blockable));
        runtime
            .install_daily_boost(
                oteryn_gameplay_content::content::daily_boost::DailyBoostSelection {
                    reset_epoch: 0,
                    creature: "oteryn:creature/rat".into(),
                    boss: "test:boss".into(),
                },
            )
            .expect("boost");
        runtime.apply_cyclopedia_respawn(|_, _| true).expect("area");
        let dead = den_creature(&runtime).expect("rat");
        kill_creature(&mut runtime, dead);
        if blockable {
            place_player(
                &mut runtime,
                40,
                MovementLocalPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
            );
        } else {
            place_creature(&mut runtime, DEN);
        }
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        let point = runtime.spawn_point_creatures()[0].0;
        let delay = SPAWN_DELAY / 8;
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(delay)
        );
        let mut now = delay;
        let report = runtime
            .drive_spawn_respawns(now, reference_view)
            .expect("due");
        if blockable {
            assert_eq!(report.blocked, 1);
        } else {
            assert_eq!(report.warnings, vec![DEN]);
            now += spawn::SPAWN_WARNING_MICROS;
            for _ in 0..spawn::OCCUPANCY_RETRIES_MAX {
                assert_eq!(
                    runtime
                        .drive_spawn_respawns(now, reference_view)
                        .expect("occupied")
                        .retried,
                    1
                );
                now += SPAWN_RETRY;
                runtime
                    .drive_spawn_respawns(now, reference_view)
                    .expect("retry");
                now += spawn::SPAWN_WARNING_MICROS;
            }
            assert_eq!(
                runtime
                    .drive_spawn_respawns(now, reference_view)
                    .expect("skip")
                    .skipped,
                1
            );
        }
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(now + delay)
        );
    }
}
