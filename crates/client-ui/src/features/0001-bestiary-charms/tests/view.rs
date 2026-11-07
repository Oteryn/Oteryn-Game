use super::*;

#[test]
fn bestiary_rows_derive_stage_and_progress_and_apply_deltas() {
    let mut view = cyclopedia();
    let rows: Vec<_> = view.bestiary_rows().collect();
    assert_eq!(
        rows.iter()
            .map(|row| (row.race.get(), row.unlocked_stage, row.next_threshold))
            .collect::<Vec<_>>(),
        [(1, 1, Some(10)), (2, 2, Some(25)), (3, 3, None)]
    );
    // A delta upserts only its races.
    view.apply_bestiary_delta(vec![race(1, 24), race(7, 1)]);
    let rows: Vec<_> = view
        .bestiary_rows()
        .map(|row| (row.race.get(), row.kill_count))
        .collect();
    assert_eq!(rows, [(1, 24), (2, 10), (3, 25), (7, 1)]);
    // A snapshot replaces every race.
    view.apply_bestiary_snapshot(vec![race(9, 1)]);
    assert_eq!(view.bestiary_rows().count(), 1);
}

#[test]
fn charm_rows_show_what_can_be_done() {
    let view = cyclopedia();
    let rows: Vec<_> = view
        .charm_rows()
        .map(|row| {
            (
                row.state.charm.get(),
                row.can_unlock,
                row.can_assign,
                row.effect_note(),
            )
        })
        .collect();
    // Charm 2 is minor and 50 echoes do not cover 100; charm 4 has no next stage. Charm 2's
    // effect is not active on the server; it stays unlockable and assignable.
    let not_active = Some(CHARM_EFFECT_NOT_ACTIVE_TEXT);
    assert_eq!(
        rows,
        [
            (1, true, false, None),
            (2, false, true, not_active),
            (3, true, false, None),
            (4, false, true, None)
        ]
    );
    assert_eq!(view.charm_points_available(), 150);
    assert_eq!(view.minor_charm_echoes_available(), 50);
    assert_eq!(view.assignment_slots_in_use(), 1);
    assert_eq!(view.assignment_slot_limit(), Some(index(2)));
}
