use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::charm_progression as wire;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn bestiary_window_shows_server_progress_only() -> TestResult {
    let (link, mut rx, out) = play_channel();
    link.use_shared_frontend();
    out.bevy_bestiary_selected();
    link.charm_progression_open();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { link, view });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let text = |app: &mut App| {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(text(&mut app).contains("Query"));
    assert!(!text(&mut app).contains("321 kills"));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.bestiary_dispatch_bound(
        &link.bestiary_binding(),
        Kind::Request(Intent::Query),
        false
    ));
    let PlayCommand::Feature(query) = rx.try_recv()? else {
        return Err("query".into());
    };
    let id = std::num::NonZeroU32::new(7).ok_or("id")?;
    let reply = wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Query,
        fee_gold: 0,
        bestiary: vec![wire::bestiary::BestiaryRaceProgress {
            race: id,
            kill_count: 321,
            kill_thresholds: [1, 20, 321],
        }],
        charms: wire::charm::CharmView {
            charm_points_available: 900,
            charms: vec![wire::charm::CharmState {
                charm: std::num::NonZeroU32::new(1).ok_or("charm")?,
                kind: wire::charm::CharmKind::Major,
                unlocked_stage: 0,
                assigned_race: None,
                next_stage_cost: 200,
                effect_active: false,
            }],
            ..Default::default()
        },
    };
    out.bevy_bestiary_reply(
        &query,
        wire::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(text(&mut app).contains("321 kills"));
    assert!(text(&mut app).contains("Stage 3: 321/321 complete"));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.bestiary_dispatch_bound(&link.bestiary_binding(), Kind::SelectRace(7), false));
    app.update();
    assert!(text(&mut app).contains("Current stage: 3/3"));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.bestiary_dispatch_bound(&link.bestiary_binding(), Kind::Catalogue(0), false));
    let PlayCommand::Feature(catalogue) = rx.try_recv()? else {
        return Err("catalogue".into());
    };
    let page = wire::catalogue::Page {
        next: None,
        categories: vec!["Reptiles".into()],
        cards: vec![wire::catalogue::Card {
            race: 7,
            name: "Server creature".into(),
            category: "Reptiles".into(),
            detail: "Server creature detail".into(),
            ..Default::default()
        }],
    };
    out.bevy_bestiary_reply(
        &catalogue,
        wire::catalogue::encode_page(&page).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(text(&mut app).contains("Reptiles"));
    assert!(text(&mut app).contains("Server creature"));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.bestiary_dispatch_bound(&link.bestiary_binding(), Kind::Catalogue(2), false));
    let PlayCommand::Feature(detail) = rx.try_recv()? else {
        return Err("detail".into());
    };
    out.bevy_bestiary_reply(
        &detail,
        wire::catalogue::encode_page(&page).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(text(&mut app).contains("Server creature detail"));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.bestiary_dispatch_bound(&link.bestiary_binding(), Kind::SelectCharm(1), false));
    app.update();
    assert!(text(&mut app).contains("Locked"));
    let unlock = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Request(Intent::Unlock) && !c.confirm).then_some(e))
        .ok_or("unlock")?;
    app.world_mut().trigger(Activate { entity: unlock });
    app.update();
    assert!(text(&mut app).contains("200"));
    assert!(rx.try_recv().is_err());
    let confirm = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| c.confirm.then_some(e))
        .ok_or("confirm")?;
    app.world_mut().trigger(Activate { entity: confirm });
    let PlayCommand::Feature(unlock) = rx.try_recv()? else {
        return Err("unlock command".into());
    };
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .charm_progression_panel()
            .reply,
        Some(reply.clone())
    );
    // A server reply can leave the charm locked.
    out.bevy_bestiary_reply(
        &unlock,
        wire::encode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(text(&mut app).contains("Locked"));
    // A higher modal blocks every rendered control, including Close.
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()));
    let controls: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .map(|(e, _)| e)
        .collect();
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .charm_progression_panel();
    for entity in controls {
        app.world_mut().trigger(Activate { entity });
    }
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .charm_progression_panel(),
        before
    );
    assert!(rx.try_recv().is_err());
    out.bevy_bestiary_end();
    app.update();
    assert!(app.world().resource::<State>().binding.is_none());
    Ok(())
}
