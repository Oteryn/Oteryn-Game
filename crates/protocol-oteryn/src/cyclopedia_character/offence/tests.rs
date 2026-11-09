use super::*;
use crate::cyclopedia_character::{CharacterTab, combat::CombatStats};
#[test]
fn offence_augments_pages_preserve_sources_sort_units_and_unknowns() -> Result<(), Error> {
    let mut facts = Snapshot {
        known_sources: 7,
        sources: vec![
            Source {
                kind: SourceKind::Equipment,
                slot: 5,
                key: "oteryn:item/weapon".into(),
            },
            Source {
                kind: SourceKind::Wheel,
                slot: 0,
                key: "Active Wheel".into(),
            },
        ],
        rows: Vec::new(),
    };
    for i in 0..70 {
        facts.rows.push(Augment {
            source: if i < 65 { 0 } else { 1 },
            spell: format!("Spell {i:02}"),
            effect: Effect::Cooldown,
            value: 4000,
        });
    }
    let bytes = facts.encode()?;
    assert_eq!(Snapshot::decode(&bytes)?, facts);
    let stats = CombatStats::new(CharacterTab::Offence, 1, 2, 3).map_err(|_| Error::Malformed)?;
    let first = Page::from_snapshot(
        [2; 16],
        Query::default(),
        stats.clone(),
        Some(facts.clone()),
    )?;
    assert_eq!(first.total, 70);
    assert!(first.has_more());
    assert_eq!(first.facts.as_ref().map(|f| f.rows.len()), Some(64));
    let last = Page::from_snapshot(
        [2; 16],
        Query {
            page: 1,
            ..Default::default()
        },
        stats.clone(),
        Some(facts.clone()),
    )?;
    assert!(!last.has_more());
    assert_eq!(last.facts.as_ref().map(|f| f.rows.len()), Some(6));
    let wheel = Page::from_snapshot(
        [2; 16],
        Query {
            sources: 4,
            ..Default::default()
        },
        stats.clone(),
        Some(facts.clone()),
    )?;
    assert_eq!(wheel.total, 5);
    for page in [first, last, wheel] {
        let bytes = page.encode()?;
        assert_eq!(Page::decode(&bytes)?, page);
        for n in 0..bytes.len() {
            assert!(Page::decode(&bytes[..n]).is_err());
        }
    }
    let unknown = Page::from_snapshot([2; 16], Query::default(), stats.clone(), None)?;
    let empty = Page::from_snapshot([2; 16], Query::default(), stats, Some(Snapshot::default()))?;
    assert_ne!(unknown.encode()?, empty.encode()?);
    facts.rows.push(facts.rows[0].clone());
    assert!(facts.encode().is_err());
    for bytes in [
        vec![],
        vec![SELECTOR, 0, 0],
        vec![SELECTOR, 0, 16],
        vec![SELECTOR; 20],
    ] {
        assert!(Query::decode(&bytes).is_err());
    }
    Ok(())
}

#[test]
fn merged_character_request_selectors_are_distinct() -> Result<(), Error> {
    use crate::cyclopedia_character::{achievements, gallery, items, offence, profile, protection};
    let selectors = [
        achievements::SELECTOR,
        gallery::SELECTOR,
        items::SELECTOR,
        protection::SELECTOR,
        profile::SELECTOR,
        offence::SELECTOR,
    ];
    let distinct: std::collections::BTreeSet<_> = selectors.into_iter().collect();
    assert_eq!(
        distinct.len(),
        selectors.len(),
        "each Character query must reach its own handler"
    );
    Ok(())
}
