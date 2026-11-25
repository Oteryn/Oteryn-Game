//! Catalogue-level witness for the Twenty Miles Beneath the Sea chain.
use super::*;

const QUEST: &str = "oteryn:quest.twenty_miles_beneath_the_sea_quest";

#[test]
fn twenty_miles_world_steps_are_bound_to_distinct_placements() -> Result<(), String> {
    let steps: Vec<_> = load()?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-callback/qc-twenty-miles-"))
        .collect();
    // 5 skull piles, 9 nacre altars, 4 hoard tiles, 3 chests, 3 broken altars, 1 sundial.
    assert_eq!(steps.len(), 74);
    let mut uses = BTreeSet::new();
    for step in &steps {
        assert_eq!(step.quest, QUEST);
        assert!(
            !step.completes,
            "only the Guardian's compass turn completes"
        );
        let at = step
            .selector
            .position
            .or_else(|| step.selector.native.as_ref().map(|native| native.position))
            .ok_or("placement")?;
        if step.trigger == Trigger::Use && !step.is_read_only() {
            assert!(uses.insert((at.x, at.y, at.floor)), "{}", step.key);
        }
        if step.trigger == Trigger::Use && step.is_read_only() {
            assert_eq!(
                steps
                    .iter()
                    .filter(|peer| phase::hoard_reentry_pair(step, peer))
                    .count(),
                1
            );
        }
        if let Some(tool) = &step.item_on_target {
            assert!(tool.selected_instance, "{}", step.key);
        }
        assert!(step.sources.iter().any(|s| s.contains("oldid=1149608")));
    }
    Ok(())
}

#[test]
fn twenty_miles_npc_chain_has_one_completing_turn() -> Result<(), String> {
    let turns: Vec<_> = crate::quest::reward_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-reward/qc-twenty-miles-"))
        .collect();
    assert_eq!(turns.len(), 29);
    let completing: Vec<_> = turns.iter().filter(|c| c.completes).collect();
    assert_eq!(completing.len(), 1);
    assert_eq!(
        completing[0].key,
        "oteryn:quest-reward/qc-twenty-miles-guardian-compass"
    );
    Ok(())
}

#[test]
fn twenty_miles_shell_can_issue_an_ear_after_a_borrowed_one_is_reported() -> Result<(), String> {
    let shell = load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/qc-twenty-miles-sea-ear-shell")
        .ok_or("Sea Ear shell")?;
    let turns = crate::quest::reward_callbacks::load()?;
    let guardian = turns
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/qc-twenty-miles-guardian-sea-ear")
        .ok_or("Guardian's Sea Ear turn")?;
    assert_eq!(guardian.present[0].item, phase::SEA_EAR_ITEM);
    assert_eq!(guardian.storage[0].set, 20);
    for stage in 19..=21 {
        let tracks = BTreeMap::from([
            (guardian.storage[0].track.clone(), stage),
            (phase::SEA_EAR_TRACK.to_owned(), -1),
        ]);
        assert!(
            phase::matches(&shell, &tracks),
            "shell should work at stage {stage}"
        );
    }
    Ok(())
}
