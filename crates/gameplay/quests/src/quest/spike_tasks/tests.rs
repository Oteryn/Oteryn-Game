use super::*;

fn pos(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

#[test]
fn offer_level_ranges_are_inclusive_and_cooldown_equality_is_still_blocked() -> Result<(), String> {
    let tasks = load()?;
    let upper = &tasks[0];
    assert!(upper.can_offer(-1, 99, 25, 100));
    assert!(upper.can_offer(-1, 99, 49, 100));
    assert!(!upper.can_offer(-1, 99, 24, 100));
    assert!(!upper.can_offer(-1, 99, 50, 100));
    assert!(!upper.can_offer(-1, 100, 30, 100));
    assert!(!upper.can_offer(0, 99, 30, 100));
    let middle = &tasks[4];
    assert!(middle.can_offer(-1, -1, 50, 100));
    assert!(middle.can_offer(-1, -1, 79, 100));
    assert!(!middle.can_offer(-1, -1, 80, 100));
    assert!(tasks[8].can_offer(-1, -1, 80, 100));
    Ok(())
}

#[test]
fn report_has_exact_threshold_and_twenty_hour_cooldown_from_claim_time() -> Result<(), String> {
    let tasks = load()?;
    let task = &tasks[0];
    assert!(task.complete(6, 10, 100).is_none());
    assert!(task.complete(8, 10, 100).is_none());
    let reward = task.complete(7, -1, 100).ok_or("reward")?;
    assert_eq!(reward.main, -1);
    assert_eq!(reward.daily, 72100);
    assert_eq!(reward.fame, 1);
    assert_eq!(reward.experience, 1000);
    assert_eq!(
        tasks[4].complete(3, 8, 100).ok_or("middle")?.experience,
        2000
    );
    assert_eq!(
        tasks[8].complete(4, 8, 100).ok_or("lower")?.experience,
        3500
    );
    Ok(())
}

#[test]
fn shovel_preserves_the_inclusive_ninety_roll_and_one_success_threshold() -> Result<(), &'static str>
{
    let mut counts = [0; 5];
    for roll in 1..=100 {
        let outcome = shovel(0, 19211, roll).ok_or("valid roll")?;
        let slot = match outcome.monster {
            None => 0,
            Some("Zombie") => 1,
            Some("Demon Skeleton") => 2,
            Some("Ghost") => 3,
            Some("Enraged Soul") => 4,
            _ => return Err("source monster"),
        };
        counts[slot] += 1;
        assert_eq!(outcome.consume_tool, slot == 4);
        assert_eq!(outcome.main, i64::from(slot == 4));
    }
    assert_eq!(counts, [49, 20, 10, 10, 11]);
    assert!(shovel(-1, 19211, 100).is_none());
    assert!(shovel(1, 19211, 100).is_none());
    assert!(shovel(0, 19212, 100).is_none());
    Ok(())
}

#[test]
fn pacifier_consumes_the_fork_only_on_seven() {
    assert_eq!(
        pacifier(5, 19208),
        Some(CounterChange {
            main: 6,
            consume_tool: false
        })
    );
    assert_eq!(
        pacifier(6, 19208),
        Some(CounterChange {
            main: 7,
            consume_tool: true
        })
    );
    assert!(pacifier(7, 19208).is_none());
    assert!(pacifier(-1, 19208).is_none());
    assert!(pacifier(0, 19213).is_none());
}

#[test]
fn detector_accepts_four_tiles_in_each_axis_only_on_the_same_floor() {
    let target = pos(32200, 32600, -8);
    assert!(detector_found(pos(32204, 32604, -8), target));
    assert!(!detector_found(pos(32205, 32600, -8), target));
    assert!(!detector_found(pos(32200, 32600, -9), target));
}

#[test]
fn detector_progress_clears_target_and_consumes_only_after_three_floors() -> Result<(), &'static str>
{
    let xy = (32200, 32600);
    let first = detector_advance(0, xy).ok_or("first")?;
    assert_eq!(first.target, Some(pos(32200, 32600, -9)));
    let second = detector_advance(1, xy).ok_or("second")?;
    assert_eq!(second.target, Some(pos(32200, 32600, -10)));
    let third = detector_advance(2, xy).ok_or("third")?;
    assert_eq!(third.target, None);
    assert!(third.consume_tool);
    assert_eq!(third.main, 3);
    assert!(detector_advance(3, xy).is_none());
    assert!(detector_advance(0, (32170, 32600)).is_none());
    Ok(())
}

#[test]
fn detector_direction_distance_and_floor_messages_match_the_donor() {
    let at = pos(32200, 32600, -9);
    assert_eq!(detector_hint(at, pos(32200, 32600, -8)), "above you");
    assert_eq!(detector_hint(at, pos(32200, 32600, -10)), "below you");
    assert_eq!(detector_hint(at, pos(32204, 32604, -9)), "next to you");
    assert_eq!(detector_hint(at, pos(32220, 32600, -9)), "to the east");
    assert_eq!(
        detector_hint(at, pos(32180, 32580, -8)),
        "on a higher level to the north-west"
    );
    assert_eq!(detector_hint(at, pos(32300, 32600, -9)), "far to the east");
    assert_eq!(
        detector_hint(at, pos(32475, 32600, -9)),
        "very far to the east"
    );
}

#[test]
fn monolith_first_tool_can_reset_progress_and_unstarted_charged_tool_is_allowed()
-> Result<(), &'static str> {
    let reset = monolith(2, 19207, 19217, 60).ok_or("reset")?;
    assert_eq!(reset.main, 1);
    assert_eq!(reset.tool_replacement, Some(19216));
    assert!(!reset.scatter);
    assert_eq!(monolith(-1, 19216, 19217, 61).ok_or("unstarted")?.main, 1);
    assert!(monolith(3, 19216, 19217, 100).is_none());
    Ok(())
}

#[test]
fn third_monolith_returns_before_scatter_and_final_magic_effect() -> Result<(), &'static str> {
    let final_step = monolith(2, 19216, 19217, 100).ok_or("third")?;
    assert_eq!(final_step.main, 3);
    assert_eq!(final_step.tool_replacement, Some(19218));
    assert!(!final_step.scatter);
    assert!(!final_step.magic_effect);
    assert!(monolith(1, 19216, 19217, 61).ok_or("second")?.scatter);
    Ok(())
}

#[test]
fn mushroom_rejects_an_already_used_position_without_changing_progress() {
    assert!(fertilizer(2, 19215, true).is_none());
    assert_eq!(
        fertilizer(2, 19215, false),
        Some(CounterChange {
            main: 3,
            consume_tool: false
        })
    );
    assert_eq!(
        fertilizer(3, 19215, false),
        Some(CounterChange {
            main: 4,
            consume_tool: true
        })
    );
    assert!(fertilizer(4, 19215, false).is_none());
}

#[test]
fn nests_require_temporary_look_type_and_stop_at_five() -> Result<(), &'static str> {
    assert!(nest(0, 0, 60, 1).is_none());
    let final_nest = nest(4, 307, 61, 4).ok_or("nest")?;
    assert_eq!(final_nest.main, 5);
    assert_eq!(final_nest.monster, Some("Tarantula"));
    assert!(nest(5, 307, 100, 1).is_none());
    assert_eq!(nest(0, 307, 60, 1).ok_or("no spawn")?.monster, None);
    Ok(())
}

#[test]
fn transformer_retains_the_distinct_eight_gate_and_charge_one_to_two() {
    assert_eq!(geomantic(-1, -1, false, 0), Geomantic::Bounce);
    assert_eq!(
        geomantic(-1, 1, false, 0),
        Geomantic::Apply {
            outfit_seconds: None,
            charge: Some(2)
        }
    );
    assert_eq!(
        geomantic(5, -1, false, 0),
        Geomantic::Apply {
            outfit_seconds: Some(120),
            charge: None
        }
    );
    assert_eq!(geomantic(8, -1, false, 0), Geomantic::Bounce);
    assert_eq!(geomantic(0, 1, true, 307), Geomantic::Bounce);
}

#[test]
fn hottest_pool_is_an_explicit_shared_selection_and_checks_floor_and_edges() {
    assert!(hottest_pool(0, 1, pos(32263, 32481, -13)));
    assert!(hottest_pool(0, 1, pos(32274, 32490, -13)));
    assert!(!hottest_pool(0, 1, pos(32275, 32490, -13)));
    assert!(!hottest_pool(0, 1, pos(32263, 32481, -14)));
    assert!(!hottest_pool(0, 2, pos(32263, 32481, -13)));
    assert!(!hottest_pool(1, 1, pos(32263, 32481, -13)));
    assert!(!hottest_pool(0, 10, pos(32263, 32481, -13)));
}

#[test]
fn kill_bounds_include_faces_and_must_not_replace_recipient_qualification() {
    assert!(kill_in_area("upper", pos(32008, 32522, -8)));
    assert!(kill_in_area("upper", pos(32365, 32759, -10)));
    assert!(!kill_in_area("upper", pos(32366, 32759, -10)));
    assert!(!kill_in_area("upper", pos(32008, 32522, -11)));
    assert!(kill_in_area("middle", pos(32100, 32470, -11)));
    assert!(kill_in_area("lower", pos(32345, 32710, -15)));
}

#[test]
fn distinct_contacts_do_not_consume_or_increment_on_repeated_instances() {
    assert_eq!(contact(0, 3, false), Some(1));
    assert_eq!(contact(2, 3, false), Some(3));
    assert_eq!(contact(0, 4, false), Some(1));
    assert!(contact(2, 3, true).is_none());
    assert!(contact(3, 3, false).is_none());
    assert!(contact(-1, 4, false).is_none());
}
