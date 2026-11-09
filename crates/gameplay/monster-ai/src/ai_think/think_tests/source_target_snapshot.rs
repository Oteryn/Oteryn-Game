//! A source onThink retarget cannot redirect an already selected native melee cast.
use super::*;

#[test]
fn source_retarget_preserves_the_committed_native_melee_recipient() {
    let source = behavior(rat_targeting(), vec![entry("bite", 1000, 8)], vec![], None);
    let profile = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native profile");
    let actor = rat();
    let mut state = CreatureAiState::new(actor, profile, at(100, 100));
    let facts = Facts::new();
    let players = [player(10, at(101, 100))];
    let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    let mut table = CreatureAiTable::default();
    table.admit(state, 0);
    table.entry_mut(actor).expect("entry").sequence = report.sequence;
    assert_eq!(
        table
            .native_melee_selection(actor, 0)
            .expect("selected native entry")
            .target,
        Some(players[0].actor)
    );
    table.entry_mut(actor).expect("entry").state.target = Some(player(11, at(100, 101)).actor);
    assert_eq!(
        table
            .native_melee_selection(actor, 0)
            .expect("same native entry")
            .target,
        Some(players[0].actor),
        "source state affects future thinks, not a committed cast"
    );
    assert_eq!(
        table.entry(actor).expect("entry").state.last.as_ref(),
        Some(&report)
    );
}
