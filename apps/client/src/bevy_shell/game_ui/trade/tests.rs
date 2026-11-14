use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::player_trade::{PlayerTradeResponse, player_trade};
use oteryn_client_ui::player_trade::PlayerTradePanel;
use oteryn_session::{CommandStatus, EntityRef};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn panel() -> PlayerTradePanel {
    PlayerTradePanel {
        selected: true,
        show_requested: true,
        state: Some(player_trade::State {
            trade_id: Some([3; 16]),
            partner: Some(EntityRef {
                identity: [4; 16],
                generation: 1,
            }),
            own: Some(player_trade::Offer {
                definition: 981234,
                count: 2,
                sub_type: 0,
            }),
            counter: Some(player_trade::Offer {
                definition: 997654,
                count: 4,
                sub_type: 0,
            }),
            phase: 2,
            reason: 0,
            ..default()
        }),
        ..default()
    }
}
fn app() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    assert!(player_trade::encode_state(&panel().state.ok_or("state")?).is_ok());
    outbox.bevy_trade_initialize(panel().state);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(host)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world_mut().spawn((TradeDock, Node::default()));
    app.update();
    Ok((app, commands, outbox))
}
fn update(outbox: &Outbox, panel: PlayerTradePanel) {
    if let Some(value) = panel.state {
        assert!(
            player_trade::encode_state(&value).is_ok(),
            "canonical wire fixture"
        );
        outbox.push(vec![oteryn_session::SessionEvent::PlayerTrade(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value,
            },
        )]);
    }
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| action.kind == kind)
        .map(|(entity, _)| entity)
        .ok_or_else(|| "trade control absent".into())
}
#[test]
fn trade_real_accept_click_reaches_canonical_queue_once_without_optimistic_accept() -> TestResult {
    let (mut app, mut commands, _) = app()?;
    let accept = control(&mut app, Kind::Accept)?;
    app.world_mut().trigger(Activate { entity: accept });
    app.world_mut().trigger(Activate { entity: accept });
    let PlayCommand::PlayerTrade(command) = commands.try_recv()? else {
        return Err("trade command absent".into());
    };
    assert_eq!(
        command.intent,
        player_trade::Intent::Accept { trade_id: [3; 16] }
    );
    assert!(commands.try_recv().is_err());
    let current = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .player_trade_panel();
    assert!(current.pending);
    assert_eq!(current.state.ok_or("state")?.accepted, [false, false]);
    Ok(())
}
#[test]
fn stale_offer_and_previous_generation_controls_cannot_accept_a_replacement() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let old = control(&mut app, Kind::Accept)?;
    let mut replacement = panel();
    replacement
        .state
        .as_mut()
        .ok_or("state")?
        .counter
        .as_mut()
        .ok_or("offer")?
        .count = 9;
    update(&outbox, replacement);
    // The session owner changes between rendering and the actual native Activate event.
    app.world_mut().trigger(Activate { entity: old });
    assert!(commands.try_recv().is_err());
    app.update();
    let old_generation = control(&mut app, Kind::Accept)?;
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.world_mut().trigger(Activate {
        entity: old_generation,
    });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn accepted_busy_terminal_and_real_modal_gate_trade_buttons() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    for state in [
        {
            let mut p = panel();
            p.state.as_mut().ok_or("state")?.accepted[0] = true;
            p
        },
        {
            let mut p = panel();
            p.state.as_mut().ok_or("state")?.phase = 3;
            p.state.as_mut().ok_or("state")?.accepted = [true; 2];
            p
        },
        {
            let mut p = panel();
            p.state.as_mut().ok_or("state")?.phase = 4;
            p.state.as_mut().ok_or("state")?.reason = 2;
            p
        },
    ] {
        update(&outbox, state);
        app.update();
        if let Ok(accept) = control(&mut app, Kind::Accept) {
            app.world_mut().trigger(Activate { entity: accept });
        }
        assert!(commands.try_recv().is_err());
    }
    assert!(player_trade::encode_state(&panel().state.ok_or("state")?).is_ok());
    outbox.bevy_trade_initialize(panel().state);
    app.update();
    let accept = control(&mut app, Kind::Accept)?;
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(220), Node::default()));
    app.world_mut().trigger(Activate { entity: accept });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn refusal_reenables_accept_and_source_grid_never_displays_internal_identifiers() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let accept = control(&mut app, Kind::Accept)?;
    app.world_mut().trigger(Activate { entity: accept });
    let _ = commands.try_recv()?;
    outbox.bevy_trade_response(&PlayerTradeResponse {
        status: CommandStatus::Rejected,
        result: player_trade::ResultCode::PartnerBusy,
        state: panel().state.ok_or("state")?,
    });
    app.update();
    let accept = control(&mut app, Kind::Accept)?;
    app.world_mut().trigger(Activate { entity: accept });
    assert!(matches!(commands.try_recv()?, PlayCommand::PlayerTrade(_)));
    let labels = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|text| text.contains("already trading")));
    assert!(
        !labels
            .iter()
            .any(|text| text.contains("981234") || text.contains("997654"))
    );
    assert_eq!(
        app.world_mut()
            .query::<&OfferSlot>()
            .iter(app.world())
            .count(),
        4
    );
    Ok(())
}

#[test]
fn picker_binds_exact_live_item_and_never_uses_a_reused_row() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let mut incoming = panel();
    let trade = incoming.state.as_mut().ok_or("state")?;
    trade.phase = 1;
    trade.trade_id = None;
    trade.own = None;
    trade.counter = None;
    update(&outbox, incoming);
    let handle = std::num::NonZeroU64::new(41).ok_or("handle")?;
    let item = oteryn_session::ItemEntry {
        handle,
        item_definition_ref: std::num::NonZeroU32::new(99123).ok_or("definition")?,
        count: std::num::NonZeroU32::new(3).ok_or("count")?,
        sub_type: 0,
    };
    let inventory = |item| {
        oteryn_session::SessionEvent::Inventory(oteryn_session::AppliedDelta {
            server_sequence: 3,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::CharacterInventory {
                entries: vec![item],
                ..default()
            },
        })
    };
    outbox.push(vec![inventory(item)]);
    app.update();
    let stale = control(&mut app, Kind::Select(handle))?;
    let replacement = oteryn_session::ItemEntry {
        handle: std::num::NonZeroU64::new(42).ok_or("replacement")?,
        ..item
    };
    outbox.push(vec![inventory(replacement)]);
    app.world_mut().trigger(Activate { entity: stale });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_trade_panel()
            .offer
            .is_none()
    );
    app.update();
    let select = control(&mut app, Kind::Select(replacement.handle))?;
    app.world_mut().trigger(Activate { entity: select });
    app.update();
    let offer = control(&mut app, Kind::Offer)?;
    app.world_mut().trigger(Activate { entity: offer });
    let PlayCommand::PlayerTrade(command) = commands.try_recv()? else {
        return Err("offer missing".into());
    };
    assert_eq!(
        command.intent,
        player_trade::Intent::Offer {
            partner: panel()
                .state
                .and_then(|state| state.partner)
                .ok_or("partner")?,
            item: 42,
        }
    );
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_trade_panel()
            .state
            .as_ref()
            .ok_or("state")?
            .own
            .is_none()
    );
    Ok(())
}

#[test]
fn closing_inactive_sidebar_clears_only_presentation_without_sending_cancel() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    let events = [oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(23)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::NONE
            .with(oteryn_input_actions::Modifier::Control),
        repeat: false,
    }];
    assert!(link.player_trade_input(&events));
    let mut terminal = panel();
    terminal.state.as_mut().ok_or("state")?.phase = 4;
    terminal.state.as_mut().ok_or("state")?.reason = 2;
    update(&outbox, terminal.clone());
    app.update();
    let close = control(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(commands.try_recv().is_err());
    let host = app.world().non_send::<HostOwner>();
    let current = host.play.as_ref().ok_or("play")?.link.player_trade_panel();
    assert!(!current.open && !current.show_requested);
    assert_eq!(current.state, terminal.state);
    // A later actual core shortcut can reopen the same terminal presentation.
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .player_trade_input(&events)
    );
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .player_trade_panel()
            .open
    );
    Ok(())
}

#[test]
fn fresh_canonical_context_partner_opens_picker_but_terminal_partner_does_not() -> TestResult {
    let (mut app, _commands, outbox) = app()?;
    let mut terminal = panel();
    terminal.state.as_mut().ok_or("state")?.phase = 4;
    terminal.state.as_mut().ok_or("state")?.reason = 2;
    update(&outbox, terminal);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Action>()
            .iter(app.world())
            .next()
            .is_none()
    );
    let host = app.world().non_send::<HostOwner>();
    let link = &host.play.as_ref().ok_or("play")?.link;
    assert!(link.player_trade_choose_partner(EntityRef {
        identity: [9; 16],
        generation: 2
    }));
    assert!(
        link.player_trade_panel().open,
        "explicit live context selection must open the canonical picker"
    );
    app.update();
    assert!(control(&mut app, Kind::Close).is_ok());
    Ok(())
}

#[test]
fn source_offer_clicks_and_reject_use_existing_look_and_cancel_commands() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let look = control(&mut app, Kind::Look(true))?;
    app.world_mut().trigger(Activate { entity: look });
    let PlayCommand::PlayerTrade(command) = commands.try_recv()? else {
        return Err("look absent".into());
    };
    assert_eq!(
        command.intent,
        player_trade::Intent::Look {
            trade_id: [3; 16],
            counter_offer: true
        }
    );
    let mut response = panel().state.ok_or("state")?;
    response.look = "A server-described object.".into();
    outbox.bevy_trade_response(&PlayerTradeResponse {
        status: CommandStatus::Accepted,
        result: player_trade::ResultCode::Offered,
        state: response,
    });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("A server-described object."))
    );
    let reject = control(&mut app, Kind::Cancel)?;
    app.world_mut().trigger(Activate { entity: reject });
    let PlayCommand::PlayerTrade(command) = commands.try_recv()? else {
        return Err("cancel absent".into());
    };
    assert_eq!(command.intent, player_trade::Intent::Cancel);
    Ok(())
}

#[test]
fn an_existing_outgoing_offer_cannot_be_replaced_through_the_picker() -> TestResult {
    let (mut app, mut commands, outbox) = app()?;
    let mut offered = panel();
    let state = offered.state.as_mut().ok_or("state")?;
    state.phase = 1;
    state.trade_id = None;
    state.counter = None;
    update(&outbox, offered);
    app.update();
    assert!(control(&mut app, Kind::Offer).is_err());
    let cancel = control(&mut app, Kind::Cancel)?;
    app.world_mut().trigger(Activate { entity: cancel });
    let PlayCommand::PlayerTrade(command) = commands.try_recv()? else {
        return Err("cancel absent".into());
    };
    assert_eq!(command.intent, player_trade::Intent::Cancel);
    Ok(())
}
