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
    fixture_selected(true)
}
fn fixture_selected(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_prey_initialize(selected);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(19)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    assert!(link.prey_input(&[event]));
    if selected {
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Prey(prey::Request::Query)
        ));
        response(&outbox, prey::Request::Query, &reply())?;
    } else {
        assert!(commands.try_recv().is_err());
    }
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
        "Manual",
        "Select any (5 cards)",
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
        let cancel = caption_button(&mut app, "Cancel")?;
        app.world_mut().trigger(Activate { entity: cancel });
        assert!(app.world().resource::<State>().confirmation.is_none());
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?, before);
        app.update();
        let propose = control(&mut app, Some(intent.clone()), true, None)?;
        app.world_mut().trigger(Activate { entity: propose });
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
    let editor = app
        .world_mut()
        .query::<(Entity, &CreatureField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 2).then_some(entity))
        .ok_or("creature editor")?;
    app.world_mut()
        .get_mut::<bevy::text::EditableText>(editor)
        .ok_or("editable")?
        .editor_mut()
        .set_text("oteryn:creature.bear");
    let select_any = control(
        &mut app,
        Some(Intent::SelectAny {
            slot: 2,
            creature: String::new(),
        }),
        true,
        None,
    )?;
    app.world_mut().trigger(Activate { entity: select_any });
    assert!(commands.try_recv().is_err());
    app.world_mut().flush();
    let intent = Intent::SelectAny {
        slot: 2,
        creature: "oteryn:creature.bear".into(),
    };
    let confirm = control(&mut app, Some(intent), false, None)?;
    app.world_mut().trigger(Activate { entity: confirm });
    let PlayCommand::Prey(request) = commands.try_recv()? else {
        return Err("select any request".into());
    };
    assert_eq!(
        request,
        prey::Request::SelectAny {
            slot: 2,
            creature: "oteryn:creature.bear".into()
        }
    );
    assert_eq!(panel(&app)?.confirmed, Some(reply()));
    response(&outbox, request, &reply())?;
    app.update();
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
    unavailable_close(false)?;
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

fn unavailable_close(escape: bool) -> ResultTest {
    let (mut app, mut commands, _outbox) = fixture_selected(false)?;
    app.update();
    assert!(!panel(&app)?.selected);
    if escape {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::Escape,
                logical_key: bevy::input::keyboard::Key::Escape,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    } else {
        let close = control(&mut app, None, false, None)?;
        app.world_mut().trigger(Activate { entity: close });
    }
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn prey_close_and_escape_allow_unavailable_and_pending() -> ResultTest {
    unavailable_close(false)?;
    unavailable_close(true)?;
    {
        let (mut app, mut commands, _outbox) = fixture()?;
        let refresh = control(&mut app, Some(Intent::Refresh), false, None)?;
        app.world_mut().trigger(Activate { entity: refresh });
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Prey(prey::Request::Query)
        ));
        escape_key(&mut app);
        app.update();
        assert!(!panel(&app)?.open);
        assert_eq!(panel(&app)?.pending, Some(prey::Request::Query));
        assert!(app.world().resource::<State>().root.is_none());
        assert!(commands.try_recv().is_err());
    }
    for escape in [false, true] {
        let (mut app, mut commands, _outbox) = fixture()?;
        let refresh = control(&mut app, Some(Intent::Refresh), false, None)?;
        app.world_mut().trigger(Activate { entity: refresh });
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Prey(prey::Request::Query)
        ));
        let pending = panel(&app)?.pending;
        if escape {
            let higher = app
                .world_mut()
                .spawn((GameModal, GlobalZIndex(220), Node::default()))
                .id();
            escape_key(&mut app);
            app.update();
            assert!(panel(&app)?.open);
            app.world_mut().entity_mut(higher).despawn();
            app.update();
            escape_key(&mut app);
            app.update();
        } else {
            let close = control(&mut app, None, false, None)?;
            app.world_mut().trigger(Activate { entity: close });
        }
        assert!(!panel(&app)?.open);
        assert_eq!(panel(&app)?.pending, pending);
        app.update();
        assert!(app.world().resource::<State>().root.is_none());
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
fn escape_key(app: &mut App) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
}

fn caption_button(app: &mut App, label: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    let root = app.world().resource::<State>().root.ok_or("root")?;
    app.world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| {
            if text.0 != label {
                return None;
            }
            let button = parent.parent();
            let mut entity = button;
            while let Some(parent) = app.world().get::<ChildOf>(entity) {
                if parent.parent() == root {
                    return Some(button);
                }
                entity = parent.parent();
            }
            None
        })
        .ok_or("caption button absent".into())
}

#[test]
fn prey_toolbar_is_playing_only_and_opens_the_owner() -> ResultTest {
    let (mut app, mut commands, _outbox) = fixture_selected(false)?;
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    let close = control(&mut app, None, false, None)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::Prey).then_some(e))
        .ok_or("Prey toolbar")?;
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::Flex
    );
    app.world_mut().trigger(Activate { entity: toolbar });
    assert!(panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    Ok(())
}

#[test]
fn prey_select_any_accepts_active_unlocked_slot_with_prefilled_editor() -> ResultTest {
    let (mut app, mut commands, _) = fixture()?;
    let editor = app
        .world_mut()
        .query::<(&CreatureField, &bevy::text::EditableText)>()
        .iter(app.world())
        .find_map(|(field, value)| (field.0 == 3).then_some(value.value().to_string()))
        .ok_or("active slot editor")?;
    assert_eq!(editor, "oteryn:");
    let intent = Intent::SelectAny {
        slot: 3,
        creature: "oteryn:creature.bear".into(),
    };
    let propose = control(
        &mut app,
        Some(Intent::SelectAny {
            slot: 3,
            creature: String::new(),
        }),
        true,
        None,
    )?;
    let editor_entity = app
        .world_mut()
        .query::<(Entity, &CreatureField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 3).then_some(entity))
        .ok_or("editor")?;
    app.world_mut()
        .entity_mut(editor_entity)
        .insert(bevy::text::EditableText::new("oteryn:creature.bear"));
    app.world_mut().trigger(Activate { entity: propose });
    assert!(commands.try_recv().is_err());
    app.world_mut().flush();
    let confirm = control(&mut app, Some(intent), false, None)?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::SelectAny { slot: 3, .. })
    ));
    Ok(())
}

#[test]
fn review_regression_prey_edit_invalidates_paid_creature_confirmation() -> ResultTest {
    for update_before_confirm in [false, true] {
        let (mut app, mut commands, _) = fixture()?;
        let editor = app
            .world_mut()
            .query::<(Entity, &CreatureField)>()
            .iter(app.world())
            .find_map(|(entity, field)| (field.0 == 2).then_some(entity))
            .ok_or("editor")?;
        app.world_mut()
            .get_mut::<bevy::text::EditableText>(editor)
            .ok_or("editable")?
            .editor_mut()
            .set_text("oteryn:creature.bear");
        let propose = control(
            &mut app,
            Some(Intent::SelectAny {
                slot: 2,
                creature: String::new(),
            }),
            true,
            None,
        )
        .map_err(|e| format!("proposal: {e}"))?;
        app.world_mut().trigger(Activate { entity: propose });
        app.world_mut().flush();
        let confirm = control(
            &mut app,
            Some(Intent::SelectAny {
                slot: 2,
                creature: "oteryn:creature.bear".into(),
            }),
            false,
            None,
        )
        .map_err(|e| format!("bear review: {e}"))?;
        app.world_mut()
            .get_mut::<bevy::text::EditableText>(editor)
            .ok_or("editable")?
            .editor_mut()
            .set_text("oteryn:creature.dragon");
        if update_before_confirm {
            app.update();
        }
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(
            commands.try_recv().is_err(),
            "a changed target must not spend cards on bear"
        );
        assert!(app.world().resource::<State>().confirmation.is_none());
        app.update();
        assert_eq!(
            app.world()
                .get::<bevy::text::EditableText>(editor)
                .ok_or("retained editor")?
                .value(),
            "oteryn:creature.dragon"
        );
        app.world_mut().trigger(Activate { entity: propose });
        app.world_mut().flush();
        assert!(app.world_mut().query::<&Text>().iter(app.world())
            .any(|text| text.0.contains("oteryn:creature.dragon") && text.0.contains("5 cards")));
        let confirm = control(
            &mut app,
            Some(Intent::SelectAny {
                slot: 2,
                creature: "oteryn:creature.dragon".into(),
            }),
            false,
            None,
        )
        .map_err(|e| format!("dragon review: {e}"))?;
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(
            matches!(commands.try_recv()?, PlayCommand::Prey(prey::Request::SelectAny { slot: 2, creature }) if creature == "oteryn:creature.dragon")
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
