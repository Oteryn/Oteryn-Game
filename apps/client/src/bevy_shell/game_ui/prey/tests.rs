use super::*;
use bevy::ui::InteractionDisabled;
#[test]
fn prey_slot_state_labels() {
    assert_eq!(
        [slot_title(2), slot_title(1), slot_title(0)],
        ["Active", "Inactive", "Locked"]
    );
}
#[test]
fn prey_locked_slot_actions() {
    assert!(slot_actions(0).is_empty());
}
#[test]
fn prey_active_slot_action_list() {
    let actions = slot_actions(2);
    assert!(actions.contains(&Kind::Send(Intent::BonusReroll { slot: 1 })));
    assert!(actions.contains(&Kind::Reroll(1)));
    assert!(actions.contains(&Kind::Send(Intent::SetOption { slot: 1, option: 1 })));
    assert!(actions.contains(&Kind::Send(Intent::SetOption { slot: 1, option: 2 })));
}
#[test]
fn prey_confirmation_is_one_shot() {
    let mut confirmation = None;
    assert_eq!(resolve(Kind::Reroll(1), &mut confirmation), None);
    assert_eq!(confirmation, Some(1));
    assert_eq!(
        resolve(Kind::Confirm, &mut confirmation),
        Some(Intent::ListReroll { slot: 1 })
    );
    assert_eq!(resolve(Kind::Confirm, &mut confirmation), None);
}
#[test]
fn prey_shortcut_requests_data() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut commands, _) = play_channel();
    assert!(link.prey_input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(19)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false
    }]));
    assert!(link.prey_panel().open);
    assert!(commands.try_recv().is_err()); // no capability selected: no wire command
    Ok(())
}
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    fixture_balance(5)
}
fn fixture_balance(
    balance: u8,
) -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    fixture_details(balance, 0)
}
fn fixture_details(
    balance: u8,
    free_reroll_at: u64,
) -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    use crate::play::{PlayView, play_channel};
    use oteryn_client_ui::prey::{CommandStatus, PreyResponse, prey};
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_prey_initialize(true);
    outbox.bevy_store_initialize();
    link.prey_set_open(true);
    link.prey_request(Intent::Refresh);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Prey(prey::Request::Query)
    ));
    let reply = prey::Reply {
        status: 0,
        wildcard_balance: balance,
        list_size: 9,
        expansion_owned: false,
        offered_today: vec![],
        offered_next: String::new(),
        slots: (1..=3)
            .map(|slot| prey::Slot {
                slot,
                state: if slot == 1 {
                    2
                } else if slot == 2 {
                    1
                } else {
                    0
                },
                creatures: if slot == 2 {
                    vec!["oteryn:creature.rat".into()]
                } else {
                    vec![]
                },
                selected: if slot == 1 {
                    "oteryn:creature.dragon".into()
                } else {
                    String::new()
                },
                bonus: 1,
                grade: 7,
                percent: 19,
                hunting_seconds: 3660,
                option: 0,
                free_reroll_at,
                chosen_tier: 1,
                allowed_tier: 1,
                selection_epoch: 1,
            })
            .collect(),
    };
    let bytes = prey::encode_reply(&reply).map_err(|e| format!("{e:?}"))?;
    outbox.bevy_prey_result(&PreyResponse {
        request: prey::Request::Query,
        status: CommandStatus::Accepted,
        payload: bytes,
        current: None,
    });
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.insert_non_send(host).insert_resource(Skins {
        buttons: source::ButtonImages {
            up: Handle::default(),
            down: Handle::default(),
        },
        checked: Handle::default(),
        unchecked: Handle::default(),
        frame: Handle::default(),
        inset: Handle::default(),
        font: Handle::default(),
    });
    install(&mut app);
    app.update();
    Ok((app, commands))
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| (action.kind == kind).then_some(entity))
        .ok_or("missing button")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn prey_three_slots_states() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::prey::prey;
    let (mut app, mut commands) = fixture()?;
    let texts: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    for title in ["Slot 1 · Active", "Slot 2 · Inactive", "Slot 3 · Locked"] {
        assert!(texts.iter().any(|s| s == title));
    }
    assert!(
        texts
            .iter()
            .any(|s| s.contains("dragon · Damage +19% · 7/10 · 01:01"))
    );
    click(&mut app, Kind::Reroll(2))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Confirm)?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::ListReroll { slot: 2 })
    ));
    Ok(())
}
#[test]
fn prey_active_slot_controls_send_intents() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::prey::prey;
    for intent in [
        Intent::BonusReroll { slot: 1 },
        Intent::SetOption { slot: 1, option: 1 },
        Intent::SetOption { slot: 1, option: 2 },
    ] {
        let (mut app, mut commands) = fixture()?;
        click(&mut app, Kind::Send(intent.clone()))?;
        assert!(commands.try_recv().is_err());
        app.update();
        click(&mut app, Kind::ConfirmAny(intent.clone()))?;
        let PlayCommand::Prey(request) = commands.try_recv()? else {
            return Err("wrong command".into());
        };
        assert!(matches!(
            (intent, request),
            (
                Intent::BonusReroll { slot: 1 },
                prey::Request::BonusReroll { slot: 1 }
            ) | (
                Intent::SetOption { slot: 1, option: 1 },
                prey::Request::SetOption { slot: 1, option: 1 }
            ) | (
                Intent::SetOption { slot: 1, option: 2 },
                prey::Request::SetOption { slot: 1, option: 2 }
            )
        ));
    }
    let (mut app, mut commands) = fixture()?;
    click(&mut app, Kind::Select(2, 1))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Choose(2, 1))?;
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Prey(prey::Request::Select { slot: 2, index: 1 })
    ));
    Ok(())
}
#[test]
fn prey_shortcut_dispatch_respects_input_fence() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands) = fixture()?;
    let mut host = app.world_mut().non_send_mut::<HostOwner>();
    host.play.as_ref().ok_or("play")?.link.prey_set_open(false);
    // Global's ShowPrey chord is Ctrl+Y.
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(28)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    host.route_input(std::slice::from_ref(&event), true);
    assert!(!host.play.as_ref().ok_or("play")?.link.prey_panel().open);
    assert!(commands.try_recv().is_err());
    host.route_input(&[event], false);
    assert!(host.play.as_ref().ok_or("play")?.link.prey_panel().open);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Prey(oteryn_client_ui::prey::prey::Request::Query)
    ));
    Ok(())
}

#[test]
fn prey_locked_slot() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let mut actions = app.world_mut().query::<&Action>();
    for action in actions.iter(app.world()) {
        assert!(!matches!(
            action.kind,
            Kind::Select(3, _)
                | Kind::Reroll(3)
                | Kind::Send(Intent::BonusReroll { slot: 3 })
                | Kind::Send(Intent::SetOption { slot: 3, .. })
        ));
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn prey_reroll_needs_confirmation() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    click(&mut app, Kind::Reroll(2))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Cancel)?;
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Reroll(2))?;
    app.update();
    click(&mut app, Kind::Confirm)?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Prey(oteryn_client_ui::prey::prey::Request::ListReroll {
            slot: 2
        })
    ));
    Ok(())
}

#[test]
fn prey_blocking_dialog_escape_respects_top_modal() -> TestResult {
    let (mut app, _commands) = fixture()?;
    app.init_resource::<ButtonInput<KeyCode>>();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(300), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .prey_panel()
            .open
    );
    app.world_mut().despawn(higher);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .prey_panel()
            .open
    );
    Ok(())
}

fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(id, n)| (n.as_str() == name).then_some(id))
        .ok_or_else(|| name.to_owned().into())
}
#[test]
fn prey_wildcard_chooser_requires_confirmation_and_sends_exact_key() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let field = named(&mut app, "prey.wildcard.creature.2")?;
    app.world_mut()
        .entity_mut(field)
        .insert(bevy::text::EditableText::new("oteryn:creature.rat"));
    let choose = named(&mut app, "prey.wildcard.choose.2")?;
    app.world_mut().trigger(Activate { entity: choose });
    assert!(commands.try_recv().is_err());
    app.update();
    let confirm = named(&mut app, "prey.wildcard.confirm")?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Prey(
        oteryn_client_ui::prey::prey::Request::SelectAny { slot: 2, creature }
    ) if creature == "oteryn:creature.rat")
    );
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn prey_displays_balance_and_only_unlocked_free_rerolls() -> TestResult {
    let (mut app, _) = fixture()?;
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(labels.iter().any(|t| t == "Wildcards: 5"));
    assert_eq!(
        labels
            .iter()
            .filter(|t| t.as_str() == "Free list reroll available")
            .count(),
        2
    );
    Ok(())
}

#[test]
fn prey_wildcard_chooser_resolves_displayed_creature_names() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let field = named(&mut app, "prey.wildcard.creature.2")?;
    app.world_mut()
        .entity_mut(field)
        .insert(bevy::text::EditableText::new("Rat"));
    let choose = named(&mut app, "prey.wildcard.choose.2")?;
    app.world_mut().trigger(Activate { entity: choose });
    app.update();
    let confirm = named(&mut app, "prey.wildcard.confirm")?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Prey(
        oteryn_client_ui::prey::prey::Request::SelectAny { slot: 2, creature }
    ) if creature == "oteryn:creature.rat")
    );
    Ok(())
}

fn click_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or("missing caption")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}

#[test]
fn prey_resources_and_costs_are_visible() -> TestResult {
    let (mut app, _) = fixture()?;
    let texts: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    for expected in [
        "Prey Wildcards: 5 / 52",
        "Creature list: 9 · Expansion not owned",
        "Bonus reroll · 1 Wildcard",
        "[ ] Auto-reroll · 1 Wildcard per renewal",
        "[ ] Lock bonus · 5 Wildcards per renewal",
        "Free list reroll available",
    ] {
        assert!(
            texts.iter().any(|text| text == expected),
            "missing {expected}"
        );
    }
    Ok(())
}

#[test]
fn prey_unlock_opens_store_without_granting_slot() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    click_caption(&mut app, "Unlock slot 3 in Store")?;
    let host = app.world().non_send::<HostOwner>();
    let link = &host.play.as_ref().ok_or("play")?.link;
    assert!(!link.prey_panel().open);
    assert!(link.store_panel().open);
    assert_eq!(link.prey_panel().slot(3).ok_or("slot")?.state, 0);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("expected Store catalogue request".into());
    };
    assert_eq!(
        command.command_type,
        oteryn_client_runtime::cui3::store::browse::COMMAND_TYPE
    );
    assert!(commands.try_recv().is_err()); // opens catalogue; never purchases or grants locally
    Ok(())
}

#[test]
fn prey_insufficient_wildcards_disable_spending_controls() -> TestResult {
    for (balance, intent) in [
        (0, Intent::BonusReroll { slot: 1 }),
        (0, Intent::SetOption { slot: 1, option: 1 }),
        (4, Intent::SetOption { slot: 1, option: 2 }),
    ] {
        let (mut app, mut commands) = fixture_balance(balance)?;
        let disabled = app
            .world_mut()
            .query::<(&Action, &InteractionDisabled)>()
            .iter(app.world())
            .any(|(action, _)| action.kind == Kind::Send(intent.clone()));
        assert!(disabled, "unaffordable action must be disabled");
        click(&mut app, Kind::Send(intent))?; // direct activation must also be fenced
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn prey_paid_reroll_warns_about_missing_gold_quote() -> TestResult {
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 7200;
    let (mut app, mut commands) = fixture_details(5, future)?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.starts_with("Next free list reroll in "))
    );
    click(&mut app, Kind::Reroll(2))?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("gold price is not provided"))
    );
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Cancel)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn prey_unoffered_name_resolves_to_canonical_key() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let field = named(&mut app, "prey.wildcard.creature.2")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("Dragon Lord"));
    click(&mut app, Kind::ChooseAny(2))?;
    app.update();
    let confirm = named(&mut app, "prey.wildcard.confirm")?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Prey(
        oteryn_client_ui::prey::prey::Request::SelectAny { slot: 2, creature }
    ) if creature == "oteryn:creature.dragon_lord")
    );
    Ok(())
}
#[test]
fn prey_wildcard_selection_costs_five_and_rejects_insufficient_balance() -> TestResult {
    let (mut app, _) = fixture()?;
    let field = named(&mut app, "prey.wildcard.creature.2")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("Rat"));
    click(&mut app, Kind::ChooseAny(2))?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("5 Prey wildcard cards"))
    );
    for balance in [0, 1, 4] {
        let (mut app, mut commands) = fixture_balance(balance)?;
        let field = named(&mut app, "prey.wildcard.creature.2")?;
        app.world_mut()
            .entity_mut(field)
            .insert(EditableText::new("Rat"));
        let choose = named(&mut app, "prey.wildcard.choose.2")?;
        assert!(app.world().get::<InteractionDisabled>(choose).is_some());
        app.world_mut().trigger(Activate { entity: choose });
        assert!(app.world().resource::<State>().wildcard.is_none());
        let panel = app
            .world()
            .get::<Action>(choose)
            .ok_or("action")?
            .panel
            .clone();
        assert!(!affordable(
            &panel,
            &Kind::ConfirmAny(Intent::SelectAny {
                slot: 2,
                creature: "oteryn:creature.rat".into()
            })
        ));
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn prey_wildcard_chooser_only_accepts_selection_slots() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    assert!(named(&mut app, "prey.wildcard.choose.1").is_err());
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .prey_panel();
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .spawn((Wildcard(1), EditableText::new("Dragon")));
    let entity = app
        .world_mut()
        .spawn(Action {
            generation,
            panel,
            kind: Kind::ChooseAny(1),
        })
        .observe(activate)
        .id();
    let root = app.world().resource::<State>().root.ok_or("prey root")?;
    app.world_mut().entity_mut(root).add_child(entity);
    app.world_mut().trigger(Activate { entity });
    assert!(app.world().resource::<State>().wildcard.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}
