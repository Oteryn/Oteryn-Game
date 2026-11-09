use super::super::{apply_result, augment_heading, augment_rows};
use super::*;
use oteryn_client_runtime::cyclopedia_character::{
    combat::CombatStats,
    offence::{Augment, Effect, Snapshot, Source, SourceKind},
};
fn reply(page: Page) -> Result<GatedCommandResult, &'static str> {
    Ok(GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: page.encode().map_err(|_| "encode")?,
    })
}
#[test]
fn spell_modifier_pages_validate_target_filters_revisions_and_unknowns() -> Result<(), &'static str>
{
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        target: Some([2; 16]),
        ..Default::default()
    };
    let query = Query {
        target: Some([2; 16]),
        ..Default::default()
    };
    let command = request(&mut p, AugmentIntent::Refresh).ok_or("request")?;
    assert_eq!(Query::decode(&command.payload).map_err(|_| "query")?, query);
    let stats = CombatStats::new(CharacterTab::Offence, 123, 4, 5).map_err(|_| "stats")?;
    let facts = Snapshot {
        known_sources: 5,
        sources: vec![
            Source {
                kind: SourceKind::Equipment,
                slot: 5,
                key: "oteryn:item/weapon".into(),
            },
            Source {
                kind: SourceKind::Wheel,
                slot: 0,
                key: "Current Wheel".into(),
            },
        ],
        rows: vec![
            Augment {
                source: 0,
                spell: "A spell".into(),
                effect: Effect::BaseDamage,
                value: 1500,
            },
            Augment {
                source: 1,
                spell: "B spell".into(),
                effect: Effect::Cooldown,
                value: 4000,
            },
        ],
    };
    let page = Page::from_snapshot([2; 16], query, stats.clone(), Some(facts.clone()))
        .map_err(|_| "page")?;
    assert!(apply_result(&mut p, &reply(page)?));
    assert_eq!(p.combat, Some(stats.clone()));
    let rows = augment_rows(&p);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].label.contains("15.00%"));
    assert!(rows[1].detail.contains("half-base"));
    assert!(augment_heading(&p).contains("Proficiency unavailable"));
    assert!(request(&mut p, AugmentIntent::Next).is_none());
    let cmd = request(&mut p, AugmentIntent::Sources(4)).ok_or("filter")?;
    let filtered = Query::decode(&cmd.payload).map_err(|_| "filtered")?;
    assert_eq!(filtered.sources, 4);
    assert!(p.augments.page.is_none());
    let page =
        Page::from_snapshot([2; 16], filtered, stats.clone(), Some(facts)).map_err(|_| "page")?;
    assert!(apply_result(&mut p, &reply(page)?));
    assert_eq!(augment_rows(&p).len(), 1);
    request(&mut p, AugmentIntent::Refresh).ok_or("refresh")?;
    let wrong = Page::from_snapshot([3; 16], filtered, stats.clone(), None).map_err(|_| "wrong")?;
    assert!(!apply_result(&mut p, &reply(wrong)?));
    assert!(augment_rows(&p).is_empty());
    request(&mut p, AugmentIntent::Refresh).ok_or("refresh")?;
    let unknown = Page::from_snapshot([2; 16], filtered, stats, None).map_err(|_| "unknown")?;
    assert!(apply_result(&mut p, &reply(unknown)?));
    assert!(augment_heading(&p).contains("unavailable"));
    p.change_target(Some([4; 16]));
    assert!(!p.augments.open);
    assert!(p.augments.page.is_none());
    Ok(())
}
