use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_runtime::{
    boss_encounter::{
        bosstiary_slots::{BossSlotReply, BossSlotRequest, SlotDetails},
        catalogue::{Entry, Page},
    },
    raw_feature::{CommandStatus, GatedCommandResult},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bosstiary_test_initialize(true);
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
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    toggle(&mut app.world_mut().non_send_mut::<HostOwner>(), false);
    app.update();
    Ok((app, commands, outbox))
}
fn snapshot(app: &App) -> std::result::Result<Snapshot, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bosstiary_snapshot())
}
fn bound(app: &App, expected: &Snapshot, kind: Kind, confirmed: bool) -> bool {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .is_some_and(|p| p.link.bosstiary_dispatch_bound(expected, kind, confirmed))
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn reply(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bosstiary_test_reply(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
}
fn slots() -> BossSlotReply {
    BossSlotReply {
        details: Some(SlotDetails {
            base_bonus: 25,
            bonuses: [30, 0],
            boosted: "oteryn:creature.boosted".into(),
            now: 5000,
        }),
        status: 0,
        unlocked: 3,
        bonus: 30,
        points: 77,
        remove_fee: 12345,
        ready_at_ms: 4500,
        bosses: ["oteryn:creature.alpha".into(), String::new()],
    }
}
fn load(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> Result {
    click(app, Kind::Request(Intent::Catalogue))?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!((command.capability, command.kind), (117, 133));
    let page = Page {
        next: Some(64),
        points: 77,
        entries: vec![
            Entry {
                key: "oteryn:creature.alpha".into(),
                name: "Server Alpha".into(),
                category: 1,
                kills: 25,
                thresholds: [25, 100, 300],
                points: [5, 20, 50],
                look: 21,
                object: false,
            },
            Entry {
                key: "oteryn:creature.beta".into(),
                name: "Server Beta".into(),
                category: 1,
                kills: 100,
                thresholds: [25, 100, 300],
                points: [5, 20, 50],
                look: 22,
                object: false,
            },
        ],
    };
    reply(outbox, page.encode().ok_or("page encode")?);
    app.update();
    click(app, Kind::Request(Intent::SlotsDetailed))?;
    commands.try_recv()?;
    reply(outbox, slots().encode().ok_or("slot encode")?);
    app.update();
    assert!(bound(
        app,
        &snapshot(app)?,
        Kind::Edit(1, "1".into()),
        false
    ));
    app.update();
    Ok(())
}
#[test]
fn bosstiary_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    assert_eq!(
        main_text(&snapshot(&app)?),
        boss::panel_text(&snapshot(&app)?.panel)
    );
    assert!(!main_text(&snapshot(&app)?).contains("Server Alpha"));
    load(&mut app, &mut commands, &outbox)?;
    let expected = snapshot(&app)?;
    assert_eq!(expected.panel.slots, Some(slots()));
    let text = main_text(&expected);
    assert!(text.contains("Server Alpha"));
    assert!(text.contains("12345"));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|value| value.0 == text)
    );
    assert!(bound(&app, &expected, Kind::Toggle(true), false));
    app.update();
    click(&mut app, Kind::Request(Intent::Refresh))?;
    commands.try_recv()?;
    // Existing snapshot wire fixture: now=100, points=7, cooldown oteryn:boss.a=200.
    let mut payload = vec![8, 100, 40, 7, 18, 18, 10, 13];
    payload.extend_from_slice(b"oteryn:boss.a");
    // key is 13 bytes; row includes tag/value deadline (three bytes).
    payload.extend_from_slice(&[16, 200, 1]);
    reply(&outbox, payload);
    app.update();
    let current = snapshot(&app)?;
    assert_eq!(current.panel.snapshot.as_ref().ok_or("snapshot")?.points, 7);
    assert!(boss::cooldown_text(&current.panel, 0).contains("oteryn:boss.a"));
    assert_eq!(slots(), current.panel.slots.ok_or("slots preserved")?);
    Ok(())
}
#[test]
fn bosstiary_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    let before = snapshot(&app)?;
    assert!(!bound(
        &app,
        &before,
        Kind::Request(Intent::SlotRemoveSelected),
        false
    ));
    click(&mut app, Kind::Request(Intent::SlotRemoveSelected))?;
    assert!(commands.try_recv().is_err());
    assert_eq!(snapshot(&app)?, before);
    let entity = app
        .world_mut()
        .query::<(Entity, &Confirm)>()
        .iter(app.world())
        .find_map(|(e, c)| c.0.then_some(e))
        .ok_or("confirmation")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!((command.capability, command.kind), (117, 491));
    assert_eq!(
        BossSlotRequest::decode(&command.payload),
        Some(BossSlotRequest::Detailed(Box::new(
            BossSlotRequest::Remove { slot: 1 }
        )))
    );
    assert_eq!(snapshot(&app)?.panel.slots, before.panel.slots);
    assert!(!bound(
        &app,
        &before,
        Kind::Request(Intent::SlotRemoveSelected),
        true
    ));
    assert!(commands.try_recv().is_err());
    let mut removed = slots();
    removed.bosses[0].clear();
    removed.ready_at_ms = 9000;
    reply(&outbox, removed.encode().ok_or("slot encode")?);
    app.update();
    assert_eq!(snapshot(&app)?.panel.slots, Some(removed));
    Ok(())
}
#[test]
fn bosstiary_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    let before = snapshot(&app)?;
    assert!(bound(
        &app,
        &before,
        Kind::Select("oteryn:creature.beta".into()),
        false
    ));
    let changed = snapshot(&app)?;
    assert!(!bound(
        &app,
        &before,
        Kind::Request(Intent::SlotRemoveSelected),
        true
    ));
    assert_eq!(snapshot(&app)?, changed);
    let mut control = Control {
        key: app.world().resource::<State>().key.ok_or("key")?,
        kind: Kind::Request(Intent::Refresh),
        cooldowns: false,
    };
    control.key.generation += 1;
    app.world_mut()
        .resource_scope(|world, mut state: Mut<State>| {
            assert!(!perform(
                world.non_send::<HostOwner>(),
                &mut state,
                control,
                false
            ));
        });
    let blocker = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    click(&mut app, Kind::Request(Intent::Refresh))?;
    assert_eq!(snapshot(&app)?, changed);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(blocker).despawn();
    outbox.bosstiary_test_end();
    let ended = snapshot(&app)?;
    assert!(!bound(
        &app,
        &changed,
        Kind::Request(Intent::Refresh),
        false
    ));
    assert_eq!(snapshot(&app)?, ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn bosstiary_window_registered() -> Result {
    assert!(include_str!("../mod.rs").contains("bosstiary::install(app)"));
    assert!(include_str!("../../ui.rs").contains("shell.bosstiary"));
    let (mut app, _commands, _outbox) = fixture()?;
    assert!(snapshot(&app)?.panel.open);
    let entity = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            matches!(c, crate::bevy_shell::ui::Control::Bosstiary(false)).then_some(e)
        })
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(!snapshot(&app)?.panel.open);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::KeyJ,
            logical_key: bevy::input::keyboard::Key::Character("j".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(snapshot(&app)?.panel.open);
    click(&mut app, Kind::Close(false))?;
    assert!(!snapshot(&app)?.panel.open);
    Ok(())
}
