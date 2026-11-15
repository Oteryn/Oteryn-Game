use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{cui3::item_forge::Request, forge_preview as wire};
use std::time::Duration;
type Error = Box<dyn std::error::Error>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn item(id: u64, tier: u8) -> Result<wire::Item, Error> {
    Ok(wire::Item {
        handle: std::num::NonZeroU64::new(id).ok_or("handle")?,
        definition_ref: id as u32,
        appearance: 0,
        name: format!("Sword {id}"),
        classification: 1,
        tier,
        maximum_tier: 10,
        container: false,
    })
}
fn panel() -> Result<Panel, Error> {
    let first = item(2, 1)?;
    let second = item(3, 1)?;
    let mut p = Panel {
        selected: true,
        operation: Operation::Fusion,
        first_item: 2,
        second_item: 3,
        ..Default::default()
    };
    p.preview.selected = true;
    p.preview.items = vec![second.clone(), first.clone()];
    p.preview.quote = Some(wire::Quote {
        selection: p.selection().ok_or("selection")?,
        first,
        second,
        resources: wire::Resources {
            carried_gold: 100,
            dust: 50,
            cores: 2,
            ..Default::default()
        },
        cost: wire::Cost {
            gold: 100,
            dust: 50,
            cores: 2,
        },
        target_tier: 2,
        success_percent: 65,
        protection_percent: 0,
    });
    Ok(p)
}
fn fixture(p: Panel) -> Result<Fixture, Error> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_forge_test_panel(p);
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
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> Result<(), Error> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(id, c)| (c.kind == kind).then_some(id))
        .ok_or("control")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity });
    app.update();
    Ok(())
}
fn current(app: &App) -> Result<Panel, Error> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .item_forge_panel())
}
#[test]
fn forge_tabs() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, _) = fixture(p)?;
    for tab in Tab::ALL {
        click(&mut app, Kind::Tab(tab))?;
        assert_eq!(app.world().resource::<State>().tab, tab);
    }
    assert!(
        matches!(commands.try_recv()?, PlayCommand::ServerMechanic(c) if Request::decode(&c.payload).is_some_and(|r| r.operation == Operation::QueryHistory))
    );
    Ok(())
}
#[test]
fn forge_fusion_sort() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let mut lower_definition = item(9, 3)?;
    lower_definition.definition_ref = 1;
    p.preview.items = vec![item(5, 1)?, item(4, 3)?, item(2, 3)?, lower_definition];
    let (mut app, _, _) = fixture(p)?;
    let ids: Vec<_> = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .filter_map(|c| match c.kind {
            Kind::First(id) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, [9, 2, 4, 5]);
    Ok(())
}
#[test]
fn forge_action_guard() -> Result<(), Error> {
    for operation in [Operation::Fusion, Operation::Transfer] {
        let mut p = panel()?;
        p.open = true;
        p.operation = operation;
        let selection = p.selection().ok_or("selection")?;
        p.preview.quote.as_mut().ok_or("quote")?.selection = selection;
        let (mut app, mut commands, outbox) = fixture(p.clone())?;
        for shortage in 0..6 {
            let mut invalid = p.clone();
            match shortage {
                0 => invalid.first_item = 0,
                1 => invalid.second_item = 0,
                2 => {
                    invalid
                        .preview
                        .quote
                        .as_mut()
                        .ok_or("quote")?
                        .resources
                        .carried_gold = 99
                }
                3 => {
                    invalid
                        .preview
                        .quote
                        .as_mut()
                        .ok_or("quote")?
                        .resources
                        .dust = 49
                }
                4 => {
                    invalid
                        .preview
                        .quote
                        .as_mut()
                        .ok_or("quote")?
                        .resources
                        .cores = 1
                }
                _ => invalid.second_item = invalid.first_item,
            }
            outbox.bevy_forge_test_panel(invalid);
            app.update();
            assert!(
                app.world_mut()
                    .query::<(&Control, Option<&bevy::ui::InteractionDisabled>)>()
                    .iter(app.world())
                    .any(|(control, disabled)| control.kind == Kind::Submit && disabled.is_some())
            );
            click(&mut app, Kind::Submit)?;
            assert!(commands.try_recv().is_err(), "shortage {shortage}");
        }
        outbox.bevy_forge_test_panel(p);
        app.update();
        click(&mut app, Kind::Submit)?;
        let PlayCommand::ServerMechanic(c) = commands.try_recv()? else {
            return Err("forge intent".into());
        };
        let r = Request::decode(&c.payload).ok_or("request")?;
        assert_eq!(r.operation, operation);
        assert_eq!(r.first_item.map(|i| i.get()), Some(2));
    }
    Ok(())
}
#[test]
fn forge_result_timeout_refresh() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, _) = fixture(p)?;
    click(&mut app, Kind::Submit)?;
    let PlayCommand::ServerMechanic(c) = commands.try_recv()? else {
        return Err("paid intent".into());
    };
    let paid = Request::decode(&c.payload).ok_or("request")?;
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_millis(4999));
    app.world_mut().run_system_cached(timeout)?;
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_millis(1));
    app.world_mut().run_system_cached(timeout)?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("refresh".into());
    };
    assert!(matches!(
        wire::Request::decode(&c.payload).map(|r| r.selection),
        Some(wire::Selection::Browse { after: 0 })
    ));
    assert_eq!(current(&app)?.retry, Some(paid));
    assert!(current(&app)?.pending);
    app.world_mut().run_system_cached(timeout)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn forge_convergence_clears_selection() -> Result<(), Error> {
    for (normal, convergence) in [
        (Operation::Fusion, Operation::ConvergenceFusion),
        (Operation::Transfer, Operation::ConvergenceTransfer),
    ] {
        let mut p = panel()?;
        p.open = true;
        p.operation = normal;
        let (mut app, _, _) = fixture(p)?;
        click(&mut app, Kind::Convergence)?;
        let p = current(&app)?;
        assert_eq!(p.operation, convergence);
        assert_eq!((p.first_item, p.second_item), (0, 0));
        assert!(p.preview.quote.is_none());
        click(&mut app, Kind::Convergence)?;
        assert_eq!(current(&app)?.operation, normal);
    }
    Ok(())
}

#[test]
fn forge_shortcut_opens_and_requests_data() -> Result<(), Error> {
    let (mut app, mut commands, _) = fixture(panel()?)?;
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(9)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), true);
    assert!(!current(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(current(&app)?.open);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if wire::Request::decode(&c.payload).is_some_and(|r| r.selection == wire::Selection::Browse { after: 0 }))
    );
    Ok(())
}
#[test]
fn forge_guard_uses_only_spendable_bank_gold() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let q = p.preview.quote.as_mut().ok_or("quote")?;
    q.resources.carried_gold = 0;
    q.resources.bank_gold = 100;
    let (mut app, mut commands, outbox) = fixture(p.clone())?;
    click(&mut app, Kind::Submit)?;
    assert!(commands.try_recv().is_err());
    p.preview
        .quote
        .as_mut()
        .ok_or("quote")?
        .resources
        .bank_spendable = true;
    outbox.bevy_forge_test_panel(p);
    app.update();
    click(&mut app, Kind::Submit)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(_)
    ));
    Ok(())
}
#[test]
fn forge_stale_activation_cannot_spend() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, outbox) = fixture(p.clone())?;
    let control = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .find(|c| c.kind == Kind::Submit)
        .cloned()
        .ok_or("submit")?;
    p.first_item = 0;
    outbox.bevy_forge_test_panel(p);
    let mut state = app.world_mut().remove_resource::<State>().ok_or("state")?;
    perform(app.world().non_send::<HostOwner>(), &mut state, &control);
    app.world_mut().insert_resource(state);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn forge_conversion_reviews_then_sends_existing_intent() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, outbox) = fixture(p)?;
    click(&mut app, Kind::Tab(Tab::Conversion))?;
    click(&mut app, Kind::Convert(Operation::DustToSlivers))?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("conversion preview".into());
    };
    let r = wire::Request::decode(&c.payload).ok_or("preview")?;
    assert_eq!(
        r.selection,
        wire::Selection::Resource(Operation::DustToSlivers)
    );
    let mut p = current(&app)?;
    p.preview.pending = None;
    p.resource_dialog.offer(
        Operation::DustToSlivers,
        Some(wire::ResourceQuote {
            operation: Operation::DustToSlivers,
            full_resources: true,
            before: wire::Resources {
                dust: 50,
                dust_limit: 100,
                ..Default::default()
            },
            after_dust: 0,
            after_limit: 100,
            after_slivers: 3,
            after_cores: 0,
        }),
    );
    let serial = p.resource_dialog.serial;
    outbox.bevy_forge_test_panel(p);
    app.update();
    click(&mut app, Kind::Confirm(serial))?;
    let PlayCommand::ServerMechanic(c) = commands.try_recv()? else {
        return Err("conversion intent".into());
    };
    assert_eq!(
        Request::decode(&c.payload).ok_or("request")?.operation,
        Operation::DustToSlivers
    );
    Ok(())
}
#[test]
fn forge_result_cancels_timeout() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, outbox) = fixture(p)?;
    click(&mut app, Kind::Submit)?;
    commands.try_recv()?;
    outbox.bevy_forge_test_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload:
                br#"{"operation":"fusion","succeeded":true,"applied":true,"bonus":0,"result":null}"#
                    .to_vec(),
        },
    );
    app.update();
    assert!(!current(&app)?.pending);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(5));
    app.world_mut().run_system_cached(timeout)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn forge_modal_fence_blocks_paid_activation() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    let (mut app, mut commands, _) = fixture(p)?;
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(250), Node::default()));
    click(&mut app, Kind::Submit)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn forge_selection_requests_existing_pair_quote() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    p.first_item = 0;
    p.second_item = 0;
    p.preview.quote = None;
    let (mut app, mut commands, _) = fixture(p)?;
    click(&mut app, Kind::First(2))?;
    click(&mut app, Kind::Second(3))?;
    click(&mut app, Kind::Quote)?;
    let PlayCommand::Feature(c) = commands.try_recv()? else {
        return Err("pair quote".into());
    };
    assert_eq!(
        wire::Request::decode(&c.payload)
            .ok_or("request")?
            .selection,
        current(&app)?.selection().ok_or("pair")?
    );
    Ok(())
}
#[test]
fn forge_history_renders_all_received_rows() -> Result<(), Error> {
    use oteryn_client_ui::item_forge::reply::{Receipt, Reply};
    let mut p = panel()?;
    p.open = true;
    p.decoded = Some(Reply::History(vec![
        (
            8,
            Receipt {
                operation: Operation::Fusion,
                succeeded: true,
                applied: true,
                reason: None,
                bonus: 0,
                resources: None,
            },
        ),
        (
            7,
            Receipt {
                operation: Operation::Transfer,
                succeeded: false,
                applied: false,
                reason: Some("INSUFFICIENT_GOLD".into()),
                bonus: 0,
                resources: None,
            },
        ),
    ]));
    let (mut app, _, _) = fixture(p)?;
    click(&mut app, Kind::Tab(Tab::History))?;
    let lines: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect();
    assert!(lines.iter().any(|text| text.contains("History #8")));
    assert!(lines.iter().any(|text| text.contains("History #7")));
    Ok(())
}
#[test]
fn forge_reopens_on_retained_transfer_tab() -> Result<(), Error> {
    let mut p = panel()?;
    p.open = true;
    p.operation = Operation::Transfer;
    let (app, _, _) = fixture(p)?;
    assert_eq!(app.world().resource::<State>().tab, Tab::Transfer);
    Ok(())
}
