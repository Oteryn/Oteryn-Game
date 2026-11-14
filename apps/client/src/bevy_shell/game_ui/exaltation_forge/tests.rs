use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{cui3::item_forge::Request, forge_preview as wire};
type Error = Box<dyn std::error::Error>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn item(handle: u64) -> Result<wire::Item, Error> {
    Ok(wire::Item {
        handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
        definition_ref: 7,
        appearance: 3264,
        name: format!("Server sword {handle}"),
        classification: 1,
        tier: 0,
        maximum_tier: 1,
        container: false,
    })
}
fn fixture() -> Result<Fixture, Error> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_forge_setup();
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
fn click(app: &mut App, kind: Kind) -> Result<Entity, Error> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("missing Forge button")?;
    app.world_mut().trigger(Activate { entity });
    Ok(entity)
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn panel(app: &App) -> Result<Panel, Error> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .item_forge_panel())
}
fn preview(
    outbox: &Outbox,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    payload: wire::Payload,
) -> Result<(), Error> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("preview command".into());
    };
    let request = wire::Request::decode(&command.payload).ok_or("preview")?;
    outbox.bevy_forge_preview(
        &command,
        wire::Reply {
            nonce: request.nonce,
            payload,
        },
    )?;
    Ok(())
}
fn browse(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> Result<(), Error> {
    click(app, Kind::Owner(ForgeControl::Refresh))?;
    preview(
        outbox,
        commands,
        wire::Payload::Page(wire::Page {
            after: 0,
            next: None,
            items: vec![item(2)?, item(3)?],
            resources: resources(),
        }),
    )?;
    app.update();
    Ok(())
}
fn resources() -> wire::Resources {
    wire::Resources {
        carried_gold: 25_000,
        bank_gold: 800,
        bank_spendable: true,
        dust: 100,
        dust_limit: 100,
        cores: 2,
        slivers: 77,
    }
}
fn quote(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
    operation: Operation,
) -> Result<(), Error> {
    click(app, Kind::Owner(ForgeControl::Operation(operation)))?;
    app.update();
    click(app, Kind::Owner(ForgeControl::First(2)))?;
    app.update();
    click(app, Kind::Owner(ForgeControl::Second(3)))?;
    app.update();
    click(app, Kind::Owner(ForgeControl::Quote))?;
    preview(
        outbox,
        commands,
        wire::Payload::Quote(wire::Quote {
            selection: wire::Selection::Pair {
                operation,
                first: item(2)?.handle,
                second: item(3)?.handle,
                boost: false,
                protect: false,
            },
            first: item(2)?,
            second: item(3)?,
            resources: resources(),
            cost: wire::Cost {
                gold: 25_000,
                dust: 100,
                cores: 2,
            },
            target_tier: 1,
            success_percent: 65,
            protection_percent: 50,
        }),
    )?;
    app.update();
    Ok(())
}
fn receipt(succeeded: bool) -> serde_json::Value {
    serde_json::json!({"operation":"fusion","succeeded":succeeded,"applied":true,"bonus":0,
        "result":{"dust":0,"dust_limit":100,"first_tier":if succeeded {1} else {0},"second_tier":null,
        "gold_spent":25000,"cores_spent":2,"slivers_spent":0,"slivers_created":0,"cores_created":0}})
}
#[test]
fn forge_window_shows_server_options_only() -> Result<(), Error> {
    let (mut app, mut commands, outbox) = fixture()?;
    let initial = text(&mut app);
    for label in [
        "Fusion",
        "Transfer",
        "Convert dust to slivers",
        "Convert slivers to a core",
    ] {
        assert!(initial.contains(label), "{label}");
    }
    assert!(!initial.contains("Success: 65%"));
    assert!(!initial.contains("Server sword"));
    browse(&mut app, &mut commands, &outbox)?;
    assert!(text(&mut app).contains("Server sword 2"));
    quote(&mut app, &mut commands, &outbox, Operation::Fusion)?;
    let shown = text(&mut app);
    for value in [
        "25000 gold",
        "100 dust",
        "2 cores",
        "77 slivers",
        "Success: 65%",
    ] {
        assert!(shown.contains(value), "{value}: {shown}");
    }
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Owner(ForgeControl::Close))?;
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(9)?,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::one(
                oteryn_input_actions::Modifier::Control,
            )
            .with(oteryn_input_actions::Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    app.update();
    assert!(
        panel(&app)?.open,
        "CtrlShiftF reaches the retained Forge owner"
    );
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    Ok(())
}
#[test]
fn forge_operation_is_item_bound_and_confirmed() -> Result<(), Error> {
    for operation in [
        Operation::Fusion,
        Operation::Transfer,
        Operation::ConvergenceFusion,
        Operation::ConvergenceTransfer,
    ] {
        for succeeded in [true, false] {
            let (mut app, mut commands, outbox) = fixture()?;
            browse(&mut app, &mut commands, &outbox)?;
            quote(&mut app, &mut commands, &outbox, operation)?;
            click(&mut app, Kind::Review)?;
            app.update();
            assert!(
                commands.try_recv().is_err(),
                "review is not a paid operation"
            );
            let confirm = click(&mut app, Kind::Confirm)?;
            app.world_mut().trigger(Activate { entity: confirm });
            let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
                return Err("paid command".into());
            };
            let request = Request::decode(&command.payload).ok_or("request")?;
            assert_eq!(request.operation, operation);
            assert_eq!(request.first_item, Some(item(2)?.handle));
            assert_eq!(request.second_item, Some(item(3)?.handle));
            assert!(commands.try_recv().is_err());
            assert!(panel(&app)?.decoded.is_none());
            let mut result = receipt(succeeded);
            result["operation"] = serde_json::Value::String(
                match operation {
                    Operation::Fusion => "fusion",
                    Operation::Transfer => "transfer",
                    Operation::ConvergenceFusion => "convergence_fusion",
                    _ => "convergence_transfer",
                }
                .into(),
            );
            outbox.bevy_forge_result(serde_json::to_vec(&result)?);
            app.update();
            let shown = text(&mut app);
            assert!(shown.contains(if succeeded { "succeeded" } else { "failed" }));
            assert!(shown.contains(if succeeded {
                "Tiers: Some(1)"
            } else {
                "Tiers: Some(0)"
            }));
        }
    }
    for operation in [
        Operation::DustToSlivers,
        Operation::SliversToCores,
        Operation::DustLimit,
    ] {
        let (mut app, mut commands, outbox) = fixture()?;
        click(&mut app, Kind::Owner(ForgeControl::Resource(operation)))?;
        let before = resources();
        preview(
            &outbox,
            &mut commands,
            wire::Payload::Resource(wire::ResourceQuote {
                operation,
                full_resources: true,
                before,
                after_dust: if operation == Operation::SliversToCores {
                    100
                } else {
                    0
                },
                after_limit: if operation == Operation::DustLimit {
                    125
                } else {
                    100
                },
                after_slivers: match operation {
                    Operation::DustToSlivers => 80,
                    Operation::SliversToCores => 27,
                    _ => 77,
                },
                after_cores: if operation == Operation::SliversToCores {
                    3
                } else {
                    2
                },
            }),
        )?;
        app.update();
        assert!(commands.try_recv().is_err());
        assert!(!panel(&app)?.pending);
        let serial = panel(&app)?.resource_dialog.serial;
        let confirm = click(&mut app, Kind::Owner(ForgeControl::ConfirmResource(serial)))?;
        app.world_mut().trigger(Activate { entity: confirm });
        let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("conversion command".into());
        };
        let request = Request::decode(&command.payload).ok_or("conversion")?;
        assert_eq!(request.operation, operation);
        assert_eq!((request.first_item, request.second_item), (None, None));
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn forge_history_shows_server_entries() -> Result<(), Error> {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(!text(&mut app).contains("History #"));
    click(&mut app, Kind::Owner(ForgeControl::History))?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("history command".into());
    };
    assert_eq!(
        Request::decode(&command.payload)
            .ok_or("history")?
            .operation,
        Operation::QueryHistory
    );
    outbox.bevy_forge_result(serde_json::to_vec(&serde_json::json!([
        {"ordinal":91,"receipt":receipt(true)}, {"ordinal":88,"receipt":receipt(false)}]))?);
    app.update();
    let shown = text(&mut app);
    assert!(shown.contains("History #91"));
    assert!(shown.contains("History #88"));
    assert!(shown.find("History #91") < shown.find("History #88"));
    assert!(!shown.contains("History #90"));
    Ok(())
}
#[test]
fn forge_rejected_when_session_replaced_or_item_changed() -> Result<(), Error> {
    for failure in 0..5 {
        let (mut app, mut commands, outbox) = fixture()?;
        browse(&mut app, &mut commands, &outbox)?;
        quote(&mut app, &mut commands, &outbox, Operation::Fusion)?;
        click(&mut app, Kind::Review)?;
        app.update();
        let controls = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .map(|(e, _)| e)
            .collect::<Vec<_>>();
        match failure {
            0 => outbox.bevy_forge_item_changed(),
            1 => outbox.bevy_forge_end(),
            2 => {
                app.world_mut()
                    .non_send_mut::<HostOwner>()
                    .play
                    .as_mut()
                    .ok_or("play")?
                    .link = play_channel().0;
            }
            3 => {
                app.world_mut()
                    .spawn((GameModal, GlobalZIndex(999), Node::default()));
            }
            _ => {
                click(&mut app, Kind::Owner(ForgeControl::Close))?;
            }
        }
        let before = panel(&app)?;
        for entity in controls {
            app.world_mut().trigger(Activate { entity });
        }
        assert_eq!(
            panel(&app)?,
            before,
            "every rejected operation changes no owner state"
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
