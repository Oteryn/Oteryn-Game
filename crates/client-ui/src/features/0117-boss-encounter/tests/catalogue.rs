use super::*;
use oteryn_client_runtime::{
    boss_encounter::catalogue::{Entry, Page},
    raw_feature::{CommandStatus, GatedCommandResult},
};
#[test]
fn boss_gallery_refusal_bad_cursor_and_category_mismatch_preserve_confirmed_cards() {
    let mut p = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    let first = Page {
        next: Some(64),
        points: 50,
        entries: vec![Entry {
            key: "oteryn:creature.boss".into(),
            name: "Boss".into(),
            category: 1,
            kills: 25,
            thresholds: [25, 100, 300],
            points: [5, 20, 50],
            look: 21,
            object: false,
        }],
    };
    assert!(request(&mut p, Intent::Catalogue).is_some());
    let result = |status, payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    };
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert!(request(&mut p, Intent::CatalogueNext).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Rejected, vec![])
    ));
    assert_eq!(p.catalogue, Some(first.clone()));
    assert!(request(&mut p, Intent::CatalogueNext).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert_eq!(p.catalogue, Some(first.clone()));
    p.browser_category = BossCategory::Archfoe;
    assert!(request(&mut p, Intent::Catalogue).is_some());
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Accepted, first.encode().unwrap_or_default())
    ));
    assert_eq!(p.catalogue, Some(first));
    assert_eq!(
        [
            BossCategory::Bane.server_filter(),
            BossCategory::Archfoe.server_filter(),
            BossCategory::Nemesis.server_filter()
        ],
        [1, 2, 3]
    );
}

#[test]
fn boss_refresh_reconciles_gallery_progress_and_stages() -> Result<(), &'static str> {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    panel.catalogue = Some(Page {
        next: None,
        points: 0,
        entries: vec![Entry {
            key: "a:b".into(),
            name: "Boss".into(),
            category: 1,
            kills: 2,
            thresholds: [3, 10, 20],
            points: [5, 20, 50],
            look: 21,
            object: false,
        }],
    });
    request(&mut panel, Intent::Refresh).ok_or("refresh")?;
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![8, 100, 34, 7, 10, 3, b'a', b':', b'b', 16, 3, 40, 5],
        }
    ));
    let page = panel.catalogue.as_ref().ok_or("gallery")?;
    assert_eq!(page.points, 5);
    assert_eq!(page.entries[0].kills, 3);
    assert_eq!(page.entries[0].stage(), 1);
    Ok(())
}

#[test]
fn boss_gallery_next_restarts_when_current_filters_change() -> Result<(), &'static str> {
    use oteryn_client_runtime::boss_encounter::catalogue::Query;
    for category_change in [false, true] {
        let mut panel = BossEncounterPanel {
            selected: true,
            catalogue_query: Some(Query {
                after: 0,
                category: 0,
                search: "old".into(),
            }),
            catalogue: Some(Page {
                next: Some(64),
                points: 0,
                entries: vec![],
            }),
            browser_search: "new".into(),
            ..Default::default()
        };
        if category_change {
            panel.browser_search = "old".into();
            panel.browser_category = BossCategory::Archfoe;
        }
        let command = request(&mut panel, Intent::CatalogueNext).ok_or("next")?;
        let expected = Query {
            after: 0,
            category: panel.browser_category.server_filter(),
            search: panel.browser_search.clone(),
        };
        assert_eq!(Some(command.payload), expected.encode());
    }
    Ok(())
}

#[test]
fn boss_refresh_requeries_the_loaded_gallery_page_including_progress_outside_snapshot()
-> Result<(), &'static str> {
    use oteryn_client_runtime::boss_encounter::catalogue::Query;
    let query = Query {
        after: 64,
        category: 0,
        search: String::new(),
    };
    let mut panel = BossEncounterPanel {
        selected: true,
        catalogue_query: Some(query.clone()),
        catalogue: Some(Page {
            next: None,
            points: 0,
            entries: vec![],
        }),
        ..Default::default()
    };
    let refresh = request(&mut panel, Intent::CatalogueRefresh).ok_or("refresh")?;
    assert_eq!(Some(refresh.payload), query.encode());
    assert_eq!(panel.pending_catalogue, Some(query));
    Ok(())
}

#[test]
fn boss_cooldown_refresh_after_gallery_updates_snapshot_and_clock() -> Result<(), &'static str> {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    request(&mut panel, Intent::Catalogue).ok_or("gallery request")?;
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: Page {
                next: None,
                points: 0,
                entries: vec![]
            }
            .encode()
            .ok_or("page")?
        }
    ));
    let command = request(&mut panel, Intent::Refresh).ok_or("cooldown request")?;
    assert!(!command.payload.starts_with(b"BCT1"));
    assert!(panel.pending_catalogue.is_none());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: vec![8, 100]
        }
    ));
    assert!(panel.snapshot.is_some());
    assert!(panel.cooldown_received_at.is_some());
    Ok(())
}
