use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::{
    charm_progression::{self as progression, catalogue as wire},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn command(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("feature command required".into());
    };
    Ok(command)
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == kind)
        .map(|(entity, _)| entity)
        .ok_or("control missing")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn labels(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn bestiary_catalogue_selection_details_and_close_use_retained_owner() -> Result {
    let mut owner = HostOwner::new(None)?;
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_bestiary_initialize();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .charm_progression_open();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    assert!(labels(&mut app).contains("Bestiary"));
    let query = command(&mut receiver)?;
    assert_eq!((query.capability, query.kind), (67, 84));
    // End the progress query, then let the Bevy system request the catalogue.
    outbox.bevy_bestiary_result(
        &query,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: progression::encode_charm_progression_reply(
                &progression::CharmProgressionReply {
                    outcome: progression::CharmProgressionOutcome::Query,
                    fee_gold: 0,
                    bestiary: vec![progression::bestiary::BestiaryRaceProgress {
                        race: std::num::NonZeroU32::new(9).ok_or("race")?,
                        kill_count: 12,
                        kill_thresholds: [10, 20, 30],
                    }],
                    charms: Default::default(),
                },
            )
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    app.update();
    let catalogue = command(&mut receiver)?;
    assert_eq!((catalogue.capability, catalogue.kind), (67, 84));
    let mut page = wire::Page {
        next: Some(9),
        categories: vec!["Amphibic".into()],
        cards: vec![
            wire::Card {
                race: 7,
                name: "Toad".into(),
                category: "Amphibic".into(),
                outfit_look_type: 226,
                object_look_type: None,
                detail: String::new(),
                echo_warden_completed: None,
            },
            wire::Card {
                race: 9,
                name: "Frog".into(),
                category: "Amphibic".into(),
                outfit_look_type: 226,
                object_look_type: None,
                detail: String::new(),
                echo_warden_completed: None,
            },
        ],
    };
    let publish = |command: &FeatureCommand, page: &wire::Page| -> Result {
        outbox.bevy_bestiary_result(
            command,
            &GatedCommandResult {
                command_id: 2,
                server_sequence: 2,
                status: oteryn_session::CommandStatus::Accepted,
                payload: wire::encode_page(page).map_err(|e| format!("{e:?}"))?,
            },
        );
        Ok(())
    };
    publish(&catalogue, &page)?;
    app.update();
    assert!(labels(&mut app).contains("Toad [Amphibic]"));
    assert!(labels(&mut app).contains("Progress not reported"));
    click(&mut app, Kind::Select(9))?;
    let detail = command(&mut receiver)?;
    assert_eq!(
        wire::decode_request(&detail.payload)
            .map_err(|e| format!("{e:?}"))?
            .race,
        9
    );
    page.cards.remove(0);
    page.cards[0].detail = "Health: 20\r\nLoot:\r\nGold coin".into();
    page.next = Some(1);
    publish(&detail, &page)?;
    app.update();
    assert!(labels(&mut app).contains("Health: 20"));
    assert!(labels(&mut app).contains("Stage 1: 10/10 complete"));
    assert!(labels(&mut app).contains("Stage 2: 12/20 in progress"));
    click(&mut app, Kind::Catalogue(3))?;
    let loot = command(&mut receiver)?;
    let request = wire::decode_request(&loot.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.race, request.after), (9, 1));
    page.next = None;
    publish(&loot, &page)?;
    app.update();
    click(&mut app, Kind::Catalogue(5))?;
    let category = command(&mut receiver)?;
    assert_eq!(
        wire::decode_request(&category.payload)
            .map_err(|e| format!("{e:?}"))?
            .category,
        1
    );
    app.update();
    let focused = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == Kind::Close)
        .map(|(entity, _)| entity)
        .ok_or("close")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, bevy::input_focus::FocusCause::Navigated);
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<InputFocus>().get().is_none());
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .next()
            .is_none()
    );
    Ok(())
}
