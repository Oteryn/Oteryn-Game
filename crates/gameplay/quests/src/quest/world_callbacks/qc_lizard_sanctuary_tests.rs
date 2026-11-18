//! Catalogue-level witness for the Sanctuary of the Lizard God egg altar.
use super::*;

#[test]
fn lizard_sanctuary_altar_halves_take_the_snake_eggs_once() -> Result<(), String> {
    let steps: Vec<_> = load()?
        .into_iter()
        .filter(|c| {
            c.key
                .starts_with("oteryn:quest-callback/qc-lizard-sanctuary-")
        })
        .collect();
    assert_eq!(steps.len(), 2);
    for step in &steps {
        assert_eq!(step.quest, "oteryn:quest.sanctuary_of_the_lizard_god");
        let input = step.item_on_target.as_ref().ok_or("eggs")?;
        assert_eq!(
            (input.item.as_str(), input.count),
            ("oteryn:item.tibia.i21469", 1)
        );
        assert!(!input.reusable);
        assert!(step.sources.iter().any(|s| s.contains("oldid=856177")));
        assert!(
            step.sources
                .iter()
                .any(|s| s.starts_with("canary:666a60f6fef8"))
        );
    }
    Ok(())
}
