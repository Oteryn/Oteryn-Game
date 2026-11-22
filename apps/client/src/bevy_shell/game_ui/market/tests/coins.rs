use super::*;
use crate::bevy_shell::{game_ui::coin_market, owner::Phase};
use oteryn_client_runtime::{coin_market as coin_wire, server_mechanic::ServerMechanicCommand};

fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, candidate)| (candidate.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}").into())
}

fn request(
    commands: &mut CommandsReceiver,
) -> Result<ServerMechanicCommand, Box<dyn std::error::Error>> {
    match commands.try_recv()? {
        PlayCommand::ServerMechanic(command) => {
            assert_eq!(
                (command.capability, command.command_type),
                (coin_wire::CAPABILITY, coin_wire::COMMAND)
            );
            Ok(command)
        }
        _ => Err("expected a coin market request".into()),
    }
}

fn enter_offer(app: &mut App) -> TestResult {
    for (index, value) in ["2", "100", "1"].into_iter().enumerate() {
        let field = named(app, &format!("coin_market.field.{index}"))?;
        app.world_mut()
            .entity_mut(field)
            .insert(EditableText::new(value));
    }
    Ok(())
}

#[test]
fn coin_market_accept_requires_confirm_and_bound_session() -> TestResult {
    let (mut app, mut commands, outbox, _) = app()?;
    app.world_mut().non_send_mut::<HostOwner>().phase = Phase::Playing;
    app.init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        })
        .add_systems(Update, project);
    coin_market::install(&mut app);
    app.world_mut().resource_mut::<State>().seen = None;
    app.update();
    let open = named(&mut app, "market.operations.coins")?;
    app.world_mut().trigger(Activate { entity: open });
    let book = request(&mut commands)?;
    assert_eq!(
        coin_wire::decode(&book.payload)
            .map_err(|error| format!("{error:?}"))?
            .action,
        1
    );
    app.update();
    let waiting = named(&mut app, "coin_market.action.6")?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(waiting)
            .is_some()
    );
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: coin_wire::encode_reply(r#"{"result":"ok","offers":[{"offer":"07070707-0707-7007-8007-070707070707","remaining":5,"price":18446744073709551615,"side":"sell","state":1}]}"#)
            .map_err(|error| format!("{error:?}"))?,
    });
    app.update();
    let confirm = named(&mut app, "coin_market.action.6")?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    enter_offer(&mut app)?;
    let review = named(&mut app, "coin_market.action.4")?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    let label = named(&mut app, "coin_market.server")?;
    assert!(
        app.world()
            .get::<Text>(label)
            .ok_or("invalid coin review")?
            .0
            .contains("Enter valid amount, price and an offer row")
    );
    let confirm = named(&mut app, "coin_market.action.6")?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    let refresh = named(&mut app, "coin_market.action.0")?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert_eq!(request(&mut commands)?, book);
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: coin_wire::encode_reply(r#"{"result":"ok","offers":[{"offer":"07070707-0707-7007-8007-070707070707","remaining":5,"price":100,"side":"sell","state":1}]}"#)
            .map_err(|error| format!("{error:?}"))?,
    });
    app.update();
    enter_offer(&mut app)?;
    let review = named(&mut app, "coin_market.action.4")?;
    app.world_mut().trigger(Activate { entity: review });
    assert!(commands.try_recv().is_err());
    app.update();
    let label = named(&mut app, "coin_market.server")?;
    assert!(
        app.world()
            .get::<Text>(label)
            .ok_or("coin review")?
            .0
            .contains("REVIEW Buy from sell offer")
    );
    let confirm = named(&mut app, "coin_market.action.6")?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    app.world_mut().trigger(Activate { entity: confirm });
    let accepted = request(&mut commands)?;
    let decoded = coin_wire::decode(&accepted.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!((decoded.action, decoded.amount), (5, 2));
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 3,
        server_sequence: 3,
        status: CommandStatus::Accepted,
        payload: coin_wire::encode_reply(r#"{"result":"unavailable"}"#)
            .map_err(|error| format!("{error:?}"))?,
    });
    app.update();
    let retry = named(&mut app, "coin_market.action.7")?;
    let retry_panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .market_panel()
        .coins;
    app.world_mut().trigger(Activate { entity: retry });
    assert_eq!(request(&mut commands)?, accepted);
    app.world_mut().trigger(Activate { entity: retry });
    assert!(commands.try_recv().is_err());
    app.update();
    let close = named(&mut app, "coin_market.close")?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(commands.try_recv().is_err());
    let original_play = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?;
    assert!(!original_play.link.market_panel().coins.open);

    outbox.bevy_coin_market_panel(retry_panel.clone());
    app.update();
    let retry = named(&mut app, "coin_market.action.7")?;
    let (replacement, mut replacement_commands, replacement_outbox) = play_channel();
    replacement_outbox.bevy_market_initialize(true);
    replacement_outbox.bevy_coin_market_panel(retry_panel);
    let active = &mut app.world_mut().non_send_mut::<HostOwner>().play;
    let original = std::mem::replace(&mut active.as_mut().ok_or("play")?.link, replacement);
    app.world_mut().trigger(Activate { entity: retry });
    assert!(replacement_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    drop(original);
    Ok(())
}
