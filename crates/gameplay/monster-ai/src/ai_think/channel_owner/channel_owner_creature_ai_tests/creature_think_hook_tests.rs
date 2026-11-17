use super::*;
use crate::ai_think::creature_think_hook::{CreatureThinkCall, CreatureThinkHooks};

#[test]
fn a_registered_think_hook_runs_once_after_every_committed_think_of_its_creature() {
    let mut channel = Channel::new();
    let a = channel.admit(at(100, 100), 0);
    let b = channel.admit(at(99, 104), 0);
    channel.table.entry_mut(a).expect("a").state.creature_key =
        Some("oteryn:creature.hook_probe".into());
    channel.table.entry_mut(b).expect("b").state.creature_key = Some("oteryn:creature.rat".into());
    assert_eq!(
        channel
            .table
            .state(a)
            .and_then(CreatureAiState::creature_key),
        Some("oteryn:creature.hook_probe")
    );
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = calls.clone();
    assert!(
        channel
            .table
            .think_hooks_mut()
            .register(
                "oteryn:creature.hook_probe",
                move |call: &CreatureThinkCall<'_>| {
                    sink.lock().expect("lock").push((
                        call.actor,
                        call.creature_key.to_owned(),
                        call.report.sequence,
                        call.now_us,
                    ));
                },
            )
            .is_none()
    );
    assert!(
        channel
            .table
            .think_hooks()
            .is_registered("oteryn:creature.hook_probe")
    );
    let mut expected = Vec::new();
    let record = |reports: Vec<(ExactActorRef, ThinkReport)>, now: u64, expected: &mut Vec<_>| {
        for (actor, report) in reports {
            if actor == a {
                expected.push((
                    a,
                    "oteryn:creature.hook_probe".to_owned(),
                    report.sequence,
                    now,
                ));
            }
        }
    };
    record(channel.thinks(0, true), 0, &mut expected);
    let mut now = 2 * SECOND;
    for x in (107..=114).rev() {
        channel.walk(at(x, 100), now);
        now += 100_000;
    }
    for k in 0..3 {
        let t = now + k * SECOND;
        record(channel.thinks(t, true), t, &mut expected);
    }
    let calls = calls.lock().expect("lock").clone();
    assert_eq!(calls, expected);
    assert!(
        calls.len() >= 4,
        "idle think plus at least three awake ticks"
    );
    assert!(calls.iter().all(|(actor, ..)| *actor == a));
}

fn think_report_scenario(with_hook: bool) -> (Vec<ThinkReport>, bool) {
    let mut channel = Channel::new();
    let a = channel.admit(at(100, 100), 0);
    let b = channel.admit(at(99, 104), 0);
    if with_hook {
        channel.table.entry_mut(a).expect("a").state.creature_key =
            Some("oteryn:creature.hook_probe".into());
        channel.table.entry_mut(b).expect("b").state.creature_key =
            Some("oteryn:creature.rat".into());
        assert!(
            channel
                .table
                .think_hooks_mut()
                .register("oteryn:creature.hook_probe", |_: &CreatureThinkCall<'_>| {})
                .is_none()
        );
    }

    let mut reports = channel.thinks(0, true);
    let mut now = 2 * SECOND;
    for x in (107..=114).rev() {
        channel.walk(at(x, 100), now);
        now += 100_000;
    }
    for k in 0..3 {
        reports.extend(channel.thinks(now + k * SECOND, true));
    }
    let b_was_awake = reports
        .iter()
        .any(|(actor, report)| *actor == b && !report.idle);
    (
        reports.into_iter().map(|(_, report)| report).collect(),
        b_was_awake,
    )
}

#[test]
fn think_hooks_never_change_any_think_report() {
    let (hooked, b_was_awake) = think_report_scenario(true);
    let (plain, _) = think_report_scenario(false);
    assert!(!hooked.is_empty());
    assert_eq!(hooked, plain);
    assert!(b_was_awake, "the unregistered creature also really thinks");
}

#[test]
fn think_hook_registry_replaces_unregisters_and_ignores_unknown_keys() {
    let mut hooks = CreatureThinkHooks::default();
    assert!(hooks.is_empty());
    assert!(
        hooks
            .register("k", |_: &CreatureThinkCall<'_>| {})
            .is_none()
    );
    assert!(
        hooks
            .register("k", |_: &CreatureThinkCall<'_>| {})
            .is_some()
    );
    assert_eq!(hooks.len(), 1);
    assert!(format!("{hooks:?}").contains("k"));
    assert!(hooks.unregister("k"));
    assert!(!hooks.unregister("k"));
    assert!(!hooks.is_registered("k"));

    let mut channel = Channel::new();
    let a = channel.admit(at(100, 100), 0);
    let report = channel
        .thinks(0, true)
        .into_iter()
        .find_map(|(actor, report)| (actor == a).then_some(report))
        .expect("a's real think report");
    assert!(!channel.table.run_think_hook(a, at(100, 100), 0, &report));
    channel.table.entry_mut(a).expect("a").state.creature_key = Some("unknown".into());
    assert!(!channel.table.run_think_hook(a, at(100, 100), 0, &report));
}
