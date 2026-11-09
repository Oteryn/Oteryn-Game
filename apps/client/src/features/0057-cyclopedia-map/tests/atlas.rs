use super::*;
pub(super) async fn round_trip(link: &PlayLink, exchanges: &Arc<Script>) -> Result<(), Error> {
    let atlas = wire::region_atlas::Page {
        next: Some(128),
        areas: vec![
            wire::region_atlas::Area {
                id: 23,
                parent: 0,
                name: "Hrodmir".into(),
            },
            wire::region_atlas::Area {
                id: 109,
                parent: 23,
                name: "Nibelor".into(),
            },
        ],
        runs: vec![wire::region_atlas::Run {
            area: 109,
            x: 100,
            end: 110,
            y: 200,
            floor: -7,
        }],
    };
    let (kind, bytes) = wire::request(5, 0).checked()?;
    send(
        link,
        exchanges,
        Intent::RegionAtlas,
        kind,
        bytes,
        CommandStatus::Accepted,
        wire::region_atlas::encode(&atlas).checked()?,
    )
    .await
    .checked()?;
    assert_eq!(
        link.cyclopedia_map_panel().region_atlas,
        Some(atlas.clone())
    );
    assert!(
        oteryn_client_ui::cyclopedia_map::panel_text(&link.cyclopedia_map_panel())
            .contains("Hrodmir > Nibelor")
    );
    let last = wire::region_atlas::Page {
        next: None,
        areas: atlas.areas.clone(),
        runs: vec![wire::region_atlas::Run {
            area: 109,
            x: 100,
            end: 110,
            y: 201,
            floor: -7,
        }],
    };
    let (kind, bytes) = wire::request(5, 128).checked()?;
    send(
        link,
        exchanges,
        Intent::AtlasNext,
        kind,
        bytes,
        CommandStatus::Accepted,
        wire::region_atlas::encode(&last).checked()?,
    )
    .await
    .checked()?;
    assert_eq!(
        link.cyclopedia_map_panel()
            .region_atlas
            .as_ref()
            .map(|p| p.runs.len()),
        Some(2)
    );
    link.cyclopedia_map_request(Intent::AtlasNext);
    assert!(!link.cyclopedia_map_panel().pending);
    Ok(())
}

#[test]
fn atlas_activity_scope_clears_geometry_and_visible_exploration_points() {
    let (link, _incoming, _outbox) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    let panel = &mut mailbox.properties.cyclopedia_map;
    panel.region_atlas = Some(wire::region_atlas::Page {
        next: Some(64),
        areas: vec![],
        runs: vec![],
    });
    panel.atlas_after = 32;
    panel.map = Some(wire::MapResult {
        code: 0,
        revision: 1,
        discovered: false,
        found: 0,
        area_percent: 0,
        pool: 0,
        epoch: 0,
        improved: false,
        visible: vec![(100, 200, -7)],
    });
    panel.summary = "Source map".into();
    mailbox.cancel_activity_requests();
    let panel = &mailbox.properties.cyclopedia_map;
    assert!(panel.region_atlas.is_none());
    assert!(panel.map.is_none());
    assert_eq!(panel.atlas_after, 0);
    assert!(panel.summary.is_empty());
}
