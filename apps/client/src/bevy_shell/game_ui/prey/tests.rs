use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::prey::{CommandStatus, PreyResponse, prey};
type ResultTest = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn reply() -> prey::Reply {
    prey::Reply {
        status: 0,
        wildcard_balance: 7,
        list_size: 9,
        expansion_owned: false,
        offered_today: vec![],
        offered_next: String::new(),
        slots: (1..=3)
            .map(|slot| prey::Slot {
                slot,
                state: slot - 1,
                creatures: if slot == 2 {
                    vec!["oteryn:creature.rat".into()]
                } else {
                    vec![]
                },
                selected: if slot == 3 {
                    "oteryn:creature.dragon".into()
                } else {
                    String::new()
                },
                bonus: 3,
                grade: 8,
                percent: 24,
                hunting_seconds: 1234,
                option: 0,
                free_reroll_at: 0,
                chosen_tier: 1,
                allowed_tier: 3,
                selection_epoch: 17,
            })
            .collect(),
    }
}
fn response(outbox: &Outbox, request: prey::Request, reply: &prey::Reply) -> ResultTest {
    let payload = prey::encode_reply(reply).map_err(|e| format!("{e:?}"))?;
    outbox.bevy_prey_response(&PreyResponse {
        request,
        status: CommandStatus::Accepted,
        current: Some(payload.clone()),
        payload,
    });
    Ok(())
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_prey_initialize(true);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(19)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    assert!(link.prey_input(&[event]));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Query)
    ));
    response(&outbox, prey::Request::Query, &reply())?;
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
fn control(
    app: &mut App,
    intent: Option<Intent>,
    confirm: bool,
    candidate: Option<(u8, u8)>,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            (c.intent == intent && c.confirm == confirm && c.candidate == candidate).then_some(e)
        })
        .ok_or("control absent".into())
}
fn panel(app: &App) -> Result<PreyPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .prey_panel())
}
#[test]
fn prey_window_shows_server_slots_only() -> ResultTest {
    let (mut app, mut commands, _) = fixture()?;
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "Slot 1",
        "Locked",
        "Inactive",
        "Active",
        "dragon",
        "Experience +24%",
        "grade 8",
        "1234 seconds",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert_eq!(panel(&app)?.confirmed, Some(reply()));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn prey_actions_are_slot_bound_and_confirm_paid() -> ResultTest {
    let (mut app, mut commands, outbox) = fixture()?;
    let choose = control(&mut app, None, false, Some((2, 1)))?;
    app.world_mut().trigger(Activate { entity: choose });
    app.update();
    let select = control(&mut app, Some(Intent::Select { slot: 2 }), false, None)?;
    app.world_mut().trigger(Activate { entity: select });
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Select { slot: 2, index: 1 })
    ));
    assert_eq!(panel(&app)?.confirmed, Some(reply()));
    response(
        &outbox,
        prey::Request::Select { slot: 2, index: 1 },
        &reply(),
    )?;
    app.update();
    for intent in [
        Intent::ListReroll { slot: 2 },
        Intent::BonusReroll { slot: 3 },
        Intent::SetOption { slot: 3, option: 2 },
    ] {
        let before = panel(&app)?;
        let propose = control(&mut app, Some(intent.clone()), true, None)?;
        app.world_mut().trigger(Activate { entity: propose });
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?, before);
        app.world_mut().flush();
        let confirm = control(&mut app, Some(intent), false, None)?;
        app.world_mut().trigger(Activate { entity: confirm });
        app.world_mut().trigger(Activate { entity: confirm });
        let PlayCommand::Prey(request) = commands.try_recv()? else {
            return Err("wrong command".into());
        };
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?.confirmed, before.confirmed);
        response(&outbox, request, &reply())?;
        app.update();
    }
    let offers = Intent::OfferedToday {
        slot: 2,
        after: String::new(),
    };
    let button = control(&mut app, Some(offers), false, None)?;
    app.world_mut().trigger(Activate { entity: button });
    let PlayCommand::Prey(request) = commands.try_recv()? else {
        return Err("offers request".into());
    };
    let mut quoted = reply();
    quoted.offered_today = vec![prey::OfferedCreature {
        creature: "oteryn:creature.bear".into(),
        price_gold: 45000,
        occupied_slot: 0,
    }];
    response(&outbox, request, &quoted)?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("45000 gold"))
    );
    let intent = Intent::ConfirmPickAgain(oteryn_client_ui::prey::PickConfirmation {
        slot: 2,
        creature: "oteryn:creature.bear".into(),
        price_gold: 45000,
    });
    let propose = control(&mut app, Some(intent.clone()), true, None)?;
    app.world_mut().trigger(Activate { entity: propose });
    assert!(commands.try_recv().is_err());
    app.world_mut().flush();
    let confirm = control(&mut app, Some(intent), false, None)?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::PickAgain {
            slot: 2,
            expected_price_gold: 45000,
            ..
        })
    ));
    Ok(())
}
#[test]
fn prey_actions_rejected_when_session_replaced_or_slot_changed() -> ResultTest {
    let (mut app, mut commands, outbox) = fixture()?;
    let controls = app
        .world_mut()
        .query_filtered::<Entity, With<Control>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    let before = panel(&app)?;
    for entity in &controls {
        app.world_mut().trigger(Activate { entity: *entity });
    }
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    let old = control(&mut app, None, false, Some((2, 1)))?;
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.prey_request(Intent::Refresh);
    }
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Query)
    ));
    let mut changed = reply();
    changed.slots[1].state = 0;
    response(&outbox, prey::Request::Query, &changed)?;
    let before = panel(&app)?;
    app.world_mut().trigger(Activate { entity: old });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    outbox.bevy_prey_end();
    let (mut replacement, mut replacement_commands, _replacement_outbox) = fixture()?;
    let play = replacement
        .world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .take();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let generation = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > generation);
        owner.play = play;
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    for entity in &controls {
        app.world_mut().trigger(Activate { entity: *entity });
    }
    assert!(replacement_commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().close();
    for entity in controls {
        app.world_mut().trigger(Activate { entity });
    }
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
