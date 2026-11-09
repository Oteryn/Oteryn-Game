//! M18: the selected s5 encounter has a bounded, source-qualified activity entry.
use oteryn_gameplay_content::content::bosses::activity_template::captured_boss_levers;

#[test]
fn m18_selected_first_dragon_is_registered_with_source_timers_and_party_bounds()
-> Result<(), String> {
    let candidates = captured_boss_levers().map_err(|error| format!("{error:?}"))?;
    let selected = candidates
        .iter()
        .find(|candidate| {
            candidate.creature_key.as_deref() == Some("oteryn:creature.the_first_dragon")
        })
        .ok_or("M18 selected First Dragon has no production activity candidate")?;
    assert_eq!(selected.cooldown_seconds, Some(20 * 60 * 60));
    assert_eq!(selected.time_limit_seconds, Some(20 * 60));
    assert_eq!(selected.minimum_players, Some(1));
    assert_eq!(selected.lever_positions.len(), 15);
    assert_eq!(selected.entry_positions.len(), 15);
    assert_eq!(selected.boss_position, Some([33617, 31023, 14]));
    assert_eq!(selected.entry_positions[0], [33621, 31023, 14]);
    assert!(selected.callbacks_known);
    // The distinct original source constructor retains its unknown custom closures.
    let original = candidates
        .iter()
        .find(|candidate| {
            candidate.path == "data-global/scripts/quests/the_first_dragon/actions_lever.lua"
        })
        .ok_or("original First Dragon source capture disappeared")?;
    assert!(!original.callbacks_known);
    assert!(!original.blockers.is_empty());
    Ok(())
}
