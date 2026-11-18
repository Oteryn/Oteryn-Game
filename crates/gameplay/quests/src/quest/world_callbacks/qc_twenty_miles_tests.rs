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
    assert_eq!(steps.len(), 25);
    let mut positions = BTreeSet::new();
    for step in &steps {
        assert_eq!(step.quest, QUEST);
        assert!(
            !step.completes,
            "only the Guardian's compass turn completes"
        );
        let at = step.selector.position.ok_or("placement")?;
        assert!(positions.insert((at.x, at.y, at.floor)), "{}", step.key);
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
    assert_eq!(turns.len(), 18);
    let completing: Vec<_> = turns.iter().filter(|c| c.completes).collect();
    assert_eq!(completing.len(), 1);
    assert_eq!(
        completing[0].key,
        "oteryn:quest-reward/qc-twenty-miles-guardian-compass"
    );
    Ok(())
}
