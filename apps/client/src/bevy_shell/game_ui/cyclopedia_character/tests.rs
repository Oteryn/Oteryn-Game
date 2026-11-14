use super::super::GameModal;
use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_character::{gallery, items, protection},
    raw_feature::{CommandStatus, GatedCommandResult},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn cyclopedia_character_window_shows_server_data_only() -> Result {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_cyclopedia_character_initialize();
    let panel = link.cyclopedia_character_panel();
    assert!(body(&panel).contains("No server character view yet"));
    assert!(link.cyclopedia_character_bound(&panel, Action::Toggle));
    let forecast = protection::Forecast {
        observed_at_unix_ms: 1000,
        level: 123,
        experience: 10000,
        regular_blessings: 3,
        twist: false,
        promotion_benefit: true,
        red_or_black: false,
        adventurer: false,
        pve: protection::Scenario {
            experience_lost: 1234,
            equipment_per_mille: 100,
            container_per_mille: 200,
            consumption: protection::Consumption::Regular,
            amulet_consumed: false,
        },
        pvp: None,
    };
    for (intent, payload, needle) in [
        (
            Intent::Items,
            items::Page {
                offset: 0,
                total: 1,
                rows: vec![items::Row {
                    definition: "confirmed sword".into(),
                    quantities: [1, 2, 3, 4],
                }],
            }
            .encode()
            .map_err(|_| "items")?,
            "confirmed sword",
        ),
        (
            Intent::Appearances,
            gallery::Page {
                offset: 0,
                total: 1,
                entries: vec![gallery::Entry {
                    kind: gallery::Kind::Outfit,
                    key: "confirmed outfit".into(),
                    look_type: 128,
                }],
            }
            .encode()
            .map_err(|_| "gallery")?,
            "confirmed outfit",
        ),
        (
            Intent::Protection,
            forecast.encode().ok_or("forecast")?,
            "Level 123",
        ),
    ] {
        let panel = link.cyclopedia_character_panel();
        assert!(!body(&panel).contains(needle));
        assert!(link.cyclopedia_character_bound(&panel, Action::Request(intent)));
        assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
        assert!(!body(&link.cyclopedia_character_panel()).contains(needle));
        outbox.bevy_cyclopedia_character_result(&GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        });
        assert!(body(&link.cyclopedia_character_panel()).contains(needle));
    }
    Ok(())
}
#[test]
fn cyclopedia_character_window_registered() -> Result {
    assert!(include_str!("../mod.rs").contains("cyclopedia_character::install(app)"));
    assert!(include_str!("../../ui.rs").contains("shell.cyclopedia_character"));
    assert!(include_str!("../../ui.rs").contains("cyclopedia_character::toggle"));
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_cyclopedia_character_initialize();
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    toggle(&owner);
    assert!(
        owner
            .play
            .as_ref()
            .is_some_and(|p| p.link.cyclopedia_character_panel().open)
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    toggle(&owner);
    assert!(
        owner
            .play
            .as_ref()
            .is_some_and(|p| !p.link.cyclopedia_character_panel().open)
    );
    assert!(commands.try_recv().is_err());
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, keyboard);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::F9);
    }
    let modal = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .is_some_and(|p| !p.link.cyclopedia_character_panel().open)
    );
    app.world_mut().despawn(modal);
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .is_some_and(|p| p.link.cyclopedia_character_panel().open)
    );
    Ok(())
}
