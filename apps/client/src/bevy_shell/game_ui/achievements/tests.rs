use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult, Response,
};
type Error = Box<dyn std::error::Error>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Error> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_achievements_initialize();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, wanted: impl Fn(&Kind) -> bool) -> Result<(), Error> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| wanted(&a.kind).then_some(e))
        .ok_or("missing achievements action")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|s| s.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn achievements_window_shows_server_data_only() -> Result<(), Error> {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    assert!(!text(&mut app).contains("Server scout"));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(0)
    ));
    assert!(!text(&mut app).contains("Server scout"));
    let response = Response::Page(AccountAchievementsResult {
        page: 0,
        fact_count: 1,
        total_points: 19,
        has_more: false,
        rows: vec![AccountAchievementRow {
            key: "oteryn:achievement/scout".into(),
            name: "Server scout".into(),
            description: "Server description".into(),
            grade: 2,
            points: 19,
            earned_at: 1234,
            secret: true,
        }],
    });
    outbox.bevy_achievements_result(&response);
    app.update();
    assert!(text(&mut app).contains("Server scout — Grade 2 — 19 points — Secret"));
    click(&mut app, |k| matches!(k, Kind::Detail(_)))?;
    assert!(text(&mut app).contains("Server description\nEarned: 1234 (server Unix ms)"));
    click(&mut app, |k| matches!(k, Kind::Refresh))?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(0)
    ));
    outbox.bevy_achievements_result(&response);
    app.update();
    click(&mut app, |k| matches!(k, Kind::Filter))?;
    assert!(!text(&mut app).contains("Server scout"));
    click(&mut app, |k| matches!(k, Kind::Filter))?;
    assert!(text(&mut app).contains("Server scout"));
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Search>>()
        .iter(app.world())
        .next()
        .ok_or("search")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("no matching server row"));
    click(&mut app, |k| matches!(k, Kind::Search))?;
    assert!(!text(&mut app).contains("Server scout"));
    click(&mut app, |k| matches!(k, Kind::Sort))?;
    assert!(text(&mut app).contains("Sort: Points"));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn achievements_window_registered() -> Result<(), Error> {
    let (mut app, mut commands, _outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::Achievements).then_some(e))
        .ok_or("missing toolbar entry")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().overlay.is_some());
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(0)
    ));
    click(&mut app, |k| matches!(k, Kind::Close))?;
    assert!(app.world().resource::<State>().overlay.is_none());
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(4)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(app.world().resource::<State>().overlay.is_none());
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::None
    );
    Ok(())
}

#[test]
fn achievements_rejected_when_session_replaced_or_target_changed_ui() -> Result<(), Error> {
    let (mut app, mut commands, old_outbox) = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let _ = commands.try_recv()?;
    let empty = Response::Page(AccountAchievementsResult {
        page: 0,
        fact_count: 0,
        total_points: 0,
        has_more: false,
        rows: vec![],
    });
    old_outbox.bevy_achievements_result(&empty);
    app.update();
    let old = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| matches!(a.kind, Kind::Refresh).then_some(e))
        .ok_or("refresh")?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: old });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let p = link.achievements_panel();
        assert!(link.achievements_browse_bound(&p, "changed", Filter::Secret, Sort::Name, 0));
    }
    app.world_mut().trigger(Activate { entity: old });
    assert!(commands.try_recv().is_err());
    old_outbox.bevy_achievements_end();
    let (link, mut replacement_commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_achievements_initialize();
    assert!(link.achievements_open_bound(&link.achievements_panel()));
    let _ = replacement_commands.try_recv()?;
    outbox.bevy_achievements_result(&empty);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let generation = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > generation);
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .achievements_panel();
    app.world_mut().trigger(Activate { entity: old });
    assert!(replacement_commands.try_recv().is_err());
    assert_eq!(
        before,
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .achievements_panel()
    );
    Ok(())
}

#[test]
fn achievements_catalog_queries_each_server_page_once() -> Result<(), Error> {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(0)
    ));
    let response = |page, name: &str, more| {
        Response::Page(AccountAchievementsResult {
            page,
            fact_count: 2,
            total_points: 2,
            has_more: more,
            rows: vec![AccountAchievementRow {
                key: format!("oteryn:achievement/{name}"),
                name: name.into(),
                description: "From server".into(),
                grade: 1,
                points: 1,
                earned_at: 1,
                secret: false,
            }],
        })
    };
    outbox.bevy_achievements_result(&response(0, "First", true));
    app.update();
    click(&mut app, |k| matches!(k, Kind::Next))?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(1)
    ));
    outbox.bevy_achievements_result(&response(1, "Second", false));
    app.update();
    click(&mut app, |k| matches!(k, Kind::Refresh))?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(0)
    ));
    outbox.bevy_achievements_result(&response(0, "First", true));
    app.update();
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::AccountAchievements(1)
    ));
    app.update();
    assert!(commands.try_recv().is_err());
    outbox.bevy_achievements_result(&response(1, "Second", false));
    app.update();
    assert!(text(&mut app).contains("First — Grade 1 — 1 points"));
    assert!(text(&mut app).contains("Second — Grade 1 — 1 points"));
    Ok(())
}
