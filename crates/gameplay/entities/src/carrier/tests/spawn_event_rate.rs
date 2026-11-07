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
