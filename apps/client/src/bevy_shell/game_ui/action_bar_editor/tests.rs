use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<App, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    // The receiver is retained for the lifetime of the fixture's session.
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { link, view });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(CommandsReceiver(commands))
        .insert_resource(Server(outbox))
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    install(&mut app);
    app.update();
    Ok(app)
}
#[derive(Resource)]
struct CommandsReceiver(tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>);
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn action_bar_editor_window_shows_owner_data_only() -> TestResult {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .action_bar_editor_snapshot();
    assert!(snapshot.choices.is_empty());
    let state = app.world().resource::<State>();
    assert_eq!(
        state.snapshot.as_ref().map(|(_, p)| &p.layout),
        Some(&snapshot.layout)
    );
    assert!(
        app.world_mut()
            .query::<&Choice>()
            .iter(app.world())
            .next()
            .is_none()
    );
    assert_eq!(
        app.world_mut()
            .query::<&RowCaption>()
            .iter(app.world())
            .count(),
        3
    );
    assert!(
        app.world_mut()
            .resource_mut::<CommandsReceiver>()
            .0
            .try_recv()
            .is_err()
    );
    seed(&app)?;
    app.update();
    let snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .action_bar_editor_snapshot();
    assert_eq!(
        snapshot
            .choices
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        ["Server spell 1", "Server spell 2", "Object 1"]
    );
    assert_eq!(
        app.world_mut().query::<&Choice>().iter(app.world()).count(),
        3
    );
    click(&mut app, Kind::Known)?;
    assert_eq!(
        app.world_mut().query::<&Choice>().iter(app.world()).count(),
        2
    );
    Ok(())
}
#[test]
fn action_bar_editor_window_registered() -> TestResult {
    let mut app = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            matches!(c, crate::bevy_shell::ui::Control::ActionBarEditor).then_some(e)
        })
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    use oteryn_input_actions::{
        ButtonState, KeyCode as OwnerKey, Modifier, Modifiers, NormalizedInputEvent,
    };
    let key = NormalizedInputEvent::Key {
        code: OwnerKey::new(12)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    // Close is admitted even while the shell owns another modal.
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .open_chat();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), false);
    app.update();
    assert!(
        app.world().resource::<State>().root.is_none(),
        "chat focus must suppress the owner opening chord"
    );
    Ok(())
}

#[derive(Resource)]
struct Server(crate::play::Outbox);
fn seed(app: &App) -> TestResult {
    use oteryn_session::spell_catalog::{SpellCatalogEntry, encode_catalog};
    let entries = (1..=2)
        .map(|id| SpellCatalogEntry {
            cast_index: id,
            name: format!("Server spell {id}"),
            words: format!("words{id}"),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned: id == 1,
            available: id == 1,
            cooldown_groups: vec![id],
            rune_item: None,
        })
        .collect::<Vec<_>>();
    let outbox = &app.world().resource::<Server>().0;
    crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
        outbox,
        encode_catalog(&entries).map_err(|e| format!("{e:?}"))?,
    )?;
    crate::play::feature_local_spell_action_bar::view::action_bar_editor_test_inventory(
        outbox,
        oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                handle: std::num::NonZeroU64::MIN,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        },
    );
    Ok(())
}
#[test]
fn action_bar_editor_actions_are_bound_and_confirmed() -> TestResult {
    let mut app = fixture()?;
    seed(&app)?;
    let original = BarAction::Text {
        text: "Original owner assignment".into(),
        send_automatically: true,
    };
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .assign_bar_action(0, original.clone())
    );
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    click(&mut app, Kind::Pick(0))?;
    click(&mut app, Kind::Aim(SpellAim::Cursor))?;
    click(&mut app, Kind::Subslot(1))?;
    click(&mut app, Kind::Pick(2))?;
    click(&mut app, Kind::Use(ItemUseMode::SelfActor))?;
    click(&mut app, Kind::Save)?;
    assert!(app.world().resource::<State>().confirm.is_some());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .action_bar_editor_snapshot()
            .actions
            .0
            .get(&0),
        Some(&original)
    );
    // Any subsequent edit invalidates the frozen replacement.
    click(&mut app, Kind::Use(ItemUseMode::Target))?;
    assert!(app.world().resource::<State>().confirm.is_none());
    click(&mut app, Kind::Save)?;
    let frozen = app
        .world()
        .resource::<State>()
        .confirm
        .clone()
        .ok_or("confirmation")?;
    click(&mut app, Kind::Confirm)?;
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .action_bar_editor_snapshot()
            .actions
            .0
            .get(&0),
        Some(&frozen)
    );
    for row in 0..3 {
        for kind in [
            Kind::Visible(row),
            Kind::Dock(row),
            Kind::Lock(row),
            Kind::Count(row, -1),
            Kind::Scroll(row, 1),
        ] {
            click(&mut app, kind)?;
        }
    }
    let owner = app.world().non_send::<HostOwner>();
    let layout = owner.play.as_ref().ok_or("play")?.link.bar_layout();
    assert!(
        layout
            .rows
            .iter()
            .all(|row| row.count == 49 && row.offset == 1)
    );
    assert_eq!(
        layout.rows.map(|row| row.dock),
        [BarDock::Left, BarDock::Right, BarDock::Bottom]
    );
    assert!(
        app.world_mut()
            .resource_mut::<CommandsReceiver>()
            .0
            .try_recv()
            .is_err()
    );
    Ok(())
}
#[test]
fn action_bar_editor_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let mut app = fixture()?;
    seed(&app)?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    click(&mut app, Kind::Pick(0))?;
    let find = |app: &mut App, kind| {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(e, c)| (c.kind == kind).then_some(e))
            .ok_or("control")
    };
    let save = find(&mut app, Kind::Save)?;
    let next = find(&mut app, Kind::Slot(1))?;
    app.world_mut().trigger(Activate { entity: next });
    app.world_mut().trigger(Activate { entity: save });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .action_bar_editor_snapshot()
            .actions
            .0
            .is_empty()
    );
    app.update();
    click(&mut app, Kind::Pick(0))?;
    let save = find(&mut app, Kind::Save)?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: save });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .action_bar_editor_snapshot()
            .actions
            .0
            .is_empty()
    );
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    let (replacement, commands, _) = play_channel();
    replacement.use_shared_frontend();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    app.world_mut().insert_resource(CommandsReceiver(commands));
    app.world_mut().trigger(Activate { entity: save });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .action_bar_editor_snapshot()
            .actions
            .0
            .is_empty()
    );
    Ok(())
}
