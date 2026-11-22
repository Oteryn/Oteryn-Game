//! Catalogue-level witness for the Sanctuary of the Lizard God egg altar.
use super::*;

#[test]
fn lizard_sanctuary_altar_halves_take_the_snake_eggs_once() -> Result<(), String> {
    let steps: Vec<_> = load()?
        .into_iter()
        .filter(|c| {
            c.key
                .starts_with("oteryn:quest-callback/qc-lizard-sanctuary-altar-")
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
        assert!(input.selected_instance);
        assert!(step.requires_world_admission);
        assert!(step.sources.iter().any(|s| s.contains("oldid=856177")));
        assert!(
            step.sources
                .iter()
                .any(|s| s.starts_with("canary:666a60f6fef8"))
        );
    }
    Ok(())
}

#[test]
fn lizard_sanctuary_portal_reads_the_same_discovery_track() -> Result<(), String> {
    let callbacks = load()?;
    let altar = callbacks
        .iter()
        .find(|c| c.key.ends_with("qc-lizard-sanctuary-altar-1"))
        .ok_or("sacrifice")?;
    let portal = callbacks
        .iter()
        .find(|c| c.key.ends_with("qc-lizard-sanctuary-portal"))
        .ok_or("portal")?;
    assert_eq!(portal.trigger, Trigger::StepIn);
    assert!(portal.is_read_only());
    assert!(portal.return_on_failure);
    assert_eq!(portal.storage[0].track, altar.storage[0].track);
    assert_eq!(portal.storage[0].minimum, altar.storage[0].set);
    assert_eq!(
        portal.teleport,
        Some(Position {
            x: 32124,
            y: 31938,
            floor: -8
        })
    );
    Ok(())
}
