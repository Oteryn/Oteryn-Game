use super::*;
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, StepDirection, WorldSpatialEntity};
fn npc() -> EntityRef {
    EntityRef {
        identity: [9; 16],
        generation: 2,
    }
}
fn reply(outbox: &crate::play::Outbox, lines: &[&str]) {
    outbox.push(vec![SessionEvent::NpcConversation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: npc_service::NpcConversation {
            open: Some((npc(), npc_service::NpcConversationState::Open)),
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        },
    })]);
}
#[test]
fn normalized_npc_shortcut_reaches_current_floor_actor_and_respects_blocked_modal() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut host, mut commands, outbox) = fixture()?;
    outbox.bevy_npc_dialogue_supported();
    host.play.as_ref().ok_or("play")?.link.npc_shop_close();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![
        WorldSpatialEntity {
            display_name: None,
            kind: EntityKind::Npc,
            entity: npc(),
            position: ActorPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
    ])?;
    view.click(TileCoord::new(101, 100))?;
    assert_eq!(view.selected_npc(), Some(npc()));
    host.play.as_mut().ok_or("play")?.view = view;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(27)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    host.route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    host.open_graphics();
    host.route_input(std::slice::from_ref(&key), false);
    assert!(commands.try_recv().is_err());
    host.cancel();
    host.route_input(&[key], false);
    let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
        return Err("NPC greeting missing".into());
    };
    assert_eq!(intent.npc_actor, npc());
    assert_eq!(intent.text, "hi");
    assert!(commands.try_recv().is_err());
    let link = &host.play.as_ref().ok_or("play")?.link;
    link.npc_dialogue_close();
    assert!(
        link.npc_dialogue_panel().pending,
        "dismissing must preserve the in-flight gate"
    );
    assert!(!link.npc_dialogue_is_visible());
    assert!(!link.npc_dialogue_submit(npc(), "trade"));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn actual_server_conversation_opens_the_single_floating_npc_window_without_shop() -> Result {
    let (host, mut commands, outbox) = fixture()?;
    outbox.bevy_npc_dialogue_supported();
    host.play.as_ref().ok_or("play")?.link.npc_shop_close();
    reply(&outbox, &["Welcome. Ask for {trade}."]);
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_dialogue_panel()
            .visible()
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Panel>>()
            .iter(app.world())
            .count(),
        1,
        "canonical NPC conversation must open the existing shared window"
    );
    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<Panel>>()
        .single(app.world())?;
    let widgets = app.world().resource::<State>().widgets.ok_or("widgets")?;
    assert_eq!(
        app.world()
            .get::<Node>(widgets.trade)
            .ok_or("trade region")?
            .display,
        Display::None
    );
    assert_eq!(
        app.world()
            .get::<Text>(widgets.conversation.transcript)
            .ok_or("transcript")?
            .0,
        "Welcome. Ask for {trade}."
    );
    let keyword = app.world_mut().query::<(Entity, &conversation::Control)>().iter(app.world())
        .find(|(_,control)|matches!(&control.kind, conversation::Kind::Keyword(word) if word == "trade"))
        .map(|(entity,_)|entity).ok_or("server keyword")?;
    app.world_mut().trigger(Activate { entity: keyword });
    let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
        return Err("trade talk".into());
    };
    assert_eq!(intent.npc_actor, npc());
    assert_eq!(intent.text, "trade");
    publish(&outbox, 8, 12);
    reply(&outbox, &["Here are my goods."]);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Panel>>()
            .single(app.world())?,
        panel
    );
    assert_eq!(
        app.world()
            .get::<Node>(widgets.trade)
            .ok_or("trade region")?
            .display,
        Display::Flex
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, control)| matches!(control.kind, Kind::Close))
        .map(|(entity, _)| entity)
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_dialogue_is_visible()
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_shop_is_open()
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Panel>>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[path = "conversation_text_tests.rs"]
mod text_tests;

#[test]
fn retained_reply_boundary_rejects_changed_actor_or_server_context_before_mutating() -> Result {
    let (host, mut commands, outbox) = fixture()?;
    outbox.bevy_npc_dialogue_supported();
    reply(&outbox, &["First {trade} question."]);
    let link = &host.play.as_ref().ok_or("play")?.link;
    let first = link.npc_dialogue_panel();
    reply(&outbox, &["Different {trade} question."]);
    assert!(!link.npc_dialogue_reply(npc(), &first.lines, "trade"));
    let current = link.npc_dialogue_panel();
    assert!(!link.npc_dialogue_reply(
        EntityRef {
            identity: [8; 16],
            generation: 3
        },
        &current.lines,
        "trade"
    ));
    assert!(!link.npc_dialogue_panel().pending);
    assert!(commands.try_recv().is_err());
    assert!(link.npc_dialogue_reply(npc(), &current.lines, "trade"));
    let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
        return Err("talk".into());
    };
    assert_eq!(intent.npc_actor, npc());
    assert_eq!(intent.text, "trade");
    Ok(())
}
