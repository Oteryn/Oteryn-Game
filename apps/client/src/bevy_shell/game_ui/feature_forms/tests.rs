use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_feature_forms_selected();
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

/// One key press through the shell's real router, then one frame.
fn press(app: &mut App, code: u16, modifiers: Modifiers) -> TestResult {
    let event = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(code)?,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    Ok(())
}

fn has_text(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == value)
}

fn click(app: &mut App, action: Option<usize>) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.action == action).then_some(entity))
        .ok_or("form button missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

fn form_open(app: &App, title: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let form = FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(link) == title)
        .ok_or("form missing")?;
    Ok(link.feature_form_view(form).open)
}

#[test]
fn every_form_has_a_field_per_label_and_a_unique_title() -> TestResult {
    let (link, _commands, _outbox) = play_channel();
    let mut titles = std::collections::BTreeSet::new();
    for form in FeatureForm::all() {
        let view = link.feature_form_view(*form);
        assert!(titles.insert(view.title), "duplicate form {}", view.title);
        assert!(!view.open, "{} opens without its key", view.title);
        assert!(
            view.fields.len() >= view.labels.len(),
            "{} has fewer fields than labels",
            view.title
        );
    }
    Ok(())
}

#[test]
fn party_chat_key_opens_form_and_send_reaches_session() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        21,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Party Chat")?);
    assert!(has_text(&mut app, "Party Chat"));
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        assert_eq!(field.0, 0);
        *text = EditableText::new("hello party");
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    let PlayCommand::ChatPublic(_) = commands.try_recv()? else {
        return Err("party chat message missing".into());
    };
    click(&mut app, None)?;
    assert!(!form_open(&app, "Party Chat")?);
    app.update();
    assert!(!has_text(&mut app, "Party Chat"));
    Ok(())
}

#[test]
fn party_effects_key_requests_status_and_disables_buttons_while_pending() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        19,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("party effects request missing".into());
    };
    assert_eq!((command.capability, command.kind), (280, 496));
    assert!(form_open(&app, "Party effects — Ctrl+Alt+Shift+P")?);
    assert!(has_text(&mut app, "Party effects — Ctrl+Alt+Shift+P"));
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Control, With<InteractionDisabled>>()
            .iter(app.world())
            .filter(|control| control.action.is_some())
            .count(),
        1
    );
    // The pending refresh refuses a second press; nothing more reaches the session.
    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn staff_and_cast_keys_open_their_forms_through_the_router() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    let chord = Modifiers::one(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    for (code, modifiers, title) in [
        (
            39,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "Staff authorized actions",
        ),
        (22, chord, "Cast settings"),
        (11, chord, "Live cast chat"),
    ] {
        press(&mut app, code, modifiers)?;
        assert!(form_open(&app, title)?, "{title} did not open");
        assert!(has_text(&mut app, title), "{title} is not shown");
        click(&mut app, None)?;
        assert!(!form_open(&app, title)?, "{title} did not close");
        app.update();
    }
    Ok(())
}

#[test]
fn cast_settings_reject_invalid_values_without_reaching_the_session() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        22,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Cast settings")?);
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            *text = EditableText::new("maybe");
        }
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert_eq!(
        link.cast_control_panel().feedback,
        tr!("Enter valid saved cast settings")
    );
    Ok(())
}

#[test]
fn communication_lists_form_edits_the_chat_lists_it_was_opened_for() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    let lists = |app: &App| -> Result<_, Box<dyn std::error::Error>> {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        Ok(link.chat_panel().console.communication)
    };
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.chat_lists_open(Some("Alice"));
    }
    app.update();
    assert!(form_open(&app, "Communication lists")?);
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 0 && text.value() == "Alice")
    );
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(1))?;
    assert!(lists(&app)?.whitelist.contains("alice"));
    click(&mut app, Some(3))?;
    let after = lists(&app)?;
    assert!(!after.whitelist.contains("alice") && !after.ignored.contains("alice"));
    assert!(commands.try_recv().is_err(), "the lists stay local");
    click(&mut app, None)?;
    assert!(!form_open(&app, "Communication lists")?);
    Ok(())
}

#[test]
fn battle_list_editor_saves_adds_and_removes_lists_through_the_form() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    let lists = |app: &App| -> Result<_, Box<dyn std::error::Error>> {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        Ok((link.active_battle_list(), link.battle_list_preferences()))
    };
    {
        let owner = app.world().non_send::<HostOwner>();
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .battle_list_editor_open();
    }
    app.update();
    assert!(form_open(&app, "Battle lists")?);
    assert!(has_text(&mut app, "Battle lists"));
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(1))?;
    let (active, added) = lists(&app)?;
    assert_eq!((active, added.len()), (1, 2));
    assert_eq!(added[1].name, "List 1");
    app.update();
    let values = ["Hunt", "dragon", "60", "no", "yes", "low", "any", "any"];
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        *text = EditableText::new(values.get(field.0).copied().unwrap_or(""));
    }
    click(&mut app, Some(0))?;
    let (_, saved) = lists(&app)?;
    let hunt = &saved[1];
    assert_eq!(
        (
            hunt.name.as_str(),
            hunt.search.as_str(),
            hunt.max_health,
            hunt.players,
            hunt.creatures,
            hunt.sort,
        ),
        (
            "Hunt",
            "dragon",
            60,
            false,
            true,
            crate::play::feature_0017_attack::view::BattleSort::HealthAscending
        )
    );
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            *text = EditableText::new("150");
        }
    }
    click(&mut app, Some(0))?;
    assert_eq!(lists(&app)?.1[1].max_health, 60, "an invalid HP is refused");
    click(&mut app, Some(2))?;
    let (active, left) = lists(&app)?;
    assert_eq!((active, left.len()), (0, 1));
    assert!(commands.try_recv().is_err(), "the lists stay local");
    click(&mut app, None)?;
    assert!(!form_open(&app, "Battle lists")?);
    Ok(())
}

#[test]
fn boss_rewards_key_browses_the_typed_death_and_shows_the_server_rows() -> TestResult {
    use oteryn_client_runtime::boss_rewards::{
        BossRewardsCommand, BossRewardsReply, BossRewardsResponse, CommandStatus, boss_rewards,
    };
    let (mut app, mut commands, outbox) = fixture()?;
    press(
        &mut app,
        30,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Boss rewards")?);
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            *text = EditableText::new("09".repeat(16));
        }
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    let PlayCommand::BossRewards(BossRewardsCommand::Browse(request)) = commands.try_recv()? else {
        return Err("boss reward browse missing".into());
    };
    assert_eq!(
        request,
        boss_rewards::BrowseRequest {
            death: [9; 16],
            after_ordinal: 0,
        }
    );
    outbox.bevy_boss_rewards_result(&BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Browse(boss_rewards::BrowseReply {
            status: boss_rewards::BossRewardStatus::Ok,
            death: [9; 16],
            expires_at_unix_ms: 99,
            rows: vec![boss_rewards::RewardRow {
                ordinal: 4,
                definition_reference: 700,
                quantity: 3,
            }],
            more: false,
        })),
    });
    app.update();
    let shown = app
        .world_mut()
        .query_filtered::<&Text, With<Summary>>()
        .iter(app.world())
        .any(|text| text.0.contains("#4: item 700 x3") && text.0.contains(&"09".repeat(16)));
    assert!(shown, "the browsed rewards are not shown");
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 1 {
            *text = EditableText::new("4");
        }
    }
    click(&mut app, Some(1))?;
    let PlayCommand::BossRewards(BossRewardsCommand::Take(take)) = commands.try_recv()? else {
        return Err("boss reward take missing".into());
    };
    assert_eq!((take.death, take.ordinal), ([9; 16], 4));
    outbox.bevy_boss_rewards_result(&BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Collect(boss_rewards::CollectReply {
            status: boss_rewards::BossRewardStatus::Ok,
            moved: 1,
            remaining: 0,
        })),
    });
    app.update();
    click(&mut app, Some(3))?;
    let PlayCommand::BossRewards(BossRewardsCommand::CollectAll(all)) = commands.try_recv()? else {
        return Err("boss reward collect all missing".into());
    };
    assert_eq!(all.death, None);
    Ok(())
}

/// With hireling management (250) negotiated the hireling key opens the management window;
/// its Banker button keeps the banker service (165, command 220) reachable.
#[test]
fn hireling_management_banker_button_opens_the_banker_service() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_hireling_management_selected();
    press(
        &mut app,
        11,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Hireling management")?);
    // The window opens with a List query; its buttons wait for the server's answer.
    let PlayCommand::Feature(list) = commands.try_recv()? else {
        return Err("hireling management list query missing".into());
    };
    assert_eq!(list.capability, 250);
    let mut hireling = [1; 16];
    hireling[6] = 0x70;
    hireling[8] = 0x80;
    outbox.bevy_hireling_management_result(
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: oteryn_client_ui::hireling_management::wire::Reply::State(vec![
                oteryn_client_ui::hireling_management::wire::Entry {
                    hireling,
                    name: "Alice".into(),
                    sex: 2,
                    look_type: 1107,
                    colors: [1, 2, 3, 4],
                    skills: 15,
                    outfits: 1,
                    position: Some((7, 9, -1)),
                },
            ])
            .encode()
            .ok_or("hireling management state")?,
        },
    );
    app.update();
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(12))?;
    let PlayCommand::Hireling(command) = commands.try_recv()? else {
        return Err("banker hireling query missing".into());
    };
    assert_eq!(
        command.0,
        oteryn_client_runtime::hireling::hireling::Request::Query
    );
    assert!(form_open(&app, "Banker Hireling")?);
    outbox.bevy_hireling_result(&oteryn_client_runtime::hireling::HirelingResponse {
        status: oteryn_client_runtime::hireling::CommandStatus::Accepted,
        active: Some(true),
    });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Banker hireling: summoned"))
    );
    Ok(())
}

/// Adds the shell's real world input route (`bevy_shell::input::route`) over a world region
/// the mouse pointer hovers, as the production schedule does under the form window.
fn with_world_route(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    use crate::bevy_shell::game_ui::{GameRegions, WorldRegion, viewport::WorldViewport};
    use bevy::picking::{
        backend::HitData,
        hover::HoverMap,
        pointer::{Location, PointerId, PointerLocation, PointerMap},
    };
    let (w, h) = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .scene()
        .view()
        .viewport_px();
    app.init_resource::<HoverMap>()
        .init_resource::<PointerMap>()
        .add_message::<bevy::winit::RawWinitWindowEvent>()
        .insert_non_send(crate::bevy_shell::input::NativeInput::default())
        .add_systems(Update, crate::bevy_shell::input::route);
    crate::bevy_shell::game_ui::initialize_item_pointer_test_state(app);
    let world = app.world_mut().spawn((WorldRegion, Node::default())).id();
    let mut window = Window::default();
    window.resolution.set_physical_resolution(w, h);
    window.resolution.set_scale_factor_override(Some(1.));
    let window = app
        .world_mut()
        .spawn((window, bevy::window::PrimaryWindow))
        .id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let pointer = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target,
                position: Vec2::ZERO,
            }),
        ))
        .id();
    app.world_mut()
        .resource_mut::<HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(world, HitData::new(window, 0., None, None));
    app.insert_resource(WorldViewport {
        origin: UVec2::ZERO,
        size: UVec2::new(w, h),
    })
    .insert_resource(GameRegions {
        vitals: world,
        minimap: world,
        equipment: world,
        containers: world,
        battle: world,
        skills: world,
        vip: world,
        analyzers: world,
        chat: world,
        bars: world,
        world,
    });
    // The first routed frame admits the session generation.
    app.update();
    Ok(pointer)
}

/// One mouse press and release on a map tile through the real window-event route.
fn world_click(
    app: &mut App,
    pointer: Entity,
    tile: (i32, i32),
    button: winit::event::MouseButton,
) -> TestResult {
    use winit::{
        dpi::PhysicalPosition,
        event::{DeviceId, ElementState, WindowEvent},
        window::WindowId,
    };
    let point = {
        let owner = app.world().non_send::<HostOwner>();
        let view = &owner.play.as_ref().ok_or("play")?.view;
        let (w, h) = view.scene().view().viewport_px();
        let target = oteryn_renderer::TileCoord::new(tile.0, tile.1);
        (0..h)
            .step_by(4)
            .flat_map(|y| (0..w).step_by(4).map(move |x| (x, y)))
            .find(|(x, y)| view.click_tile(f64::from(*x), f64::from(*y)) == Some(target))
            .map(|(x, y)| Vec2::new(x as f32 + 1., y as f32 + 1.))
            .ok_or("tile not on screen")?
    };
    app.world_mut()
        .get_mut::<bevy::picking::pointer::PointerLocation>(pointer)
        .ok_or("pointer")?
        .location
        .as_mut()
        .ok_or("location")?
        .position = point;
    let event = |event| bevy::winit::RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event,
    };
    app.world_mut()
        .write_message(event(WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(point.x), f64::from(point.y)),
        }));
    for state in [ElementState::Pressed, ElementState::Released] {
        app.world_mut()
            .write_message(event(WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state,
                button,
            }));
    }
    app.update();
    app.update();
    Ok(())
}

/// Summon needs a picked location: the banker form's Map location button arms a crosshair
/// pick and hides the window, so the next map click through the real window-event route
/// reaches the map; a click next to the player sets the target, the form returns and
/// Summon sends it. Escape and a right-click leave the pick and bring the form back.
#[test]
fn banker_summon_uses_the_adjacent_tile_the_player_picked() -> TestResult {
    use oteryn_client_runtime::hireling::{
        CommandStatus, HirelingCommand, HirelingResponse, hireling::Request,
    };
    use winit::event::MouseButton;
    let (mut app, mut commands, outbox) = fixture()?;
    let pointer = with_world_route(&mut app)?;
    outbox.bevy_hireling_selected();
    press(
        &mut app,
        11,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Banker Hireling")?);
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::Hireling(HirelingCommand(Request::Query))
    ));
    outbox.bevy_hireling_result(&HirelingResponse {
        status: CommandStatus::Accepted,
        active: Some(false),
    });
    app.update();

    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    // The press is submitted after the editors settle; its feedback shows on the next frame.
    app.update();
    // The press is submitted in PostUpdate, after this frame's projection; the refusal
    // text appears on the next frame.
    let refused = |app: &mut App| {
        app.update();
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Select an adjacent summon location"))
    };
    assert!(refused(&mut app));

    // Escape and a right-click leave the pick; the form comes back each time.
    click(&mut app, Some(3))?;
    app.update();
    assert!(!has_text(&mut app, "Banker Hireling"));
    press(
        &mut app,
        oteryn_input_actions::KeyCode::ESCAPE.get(),
        Modifiers::NONE,
    )?;
    app.update();
    assert!(has_text(&mut app, "Banker Hireling"));
    click(&mut app, Some(3))?;
    app.update();
    assert!(!has_text(&mut app, "Banker Hireling"));
    world_click(&mut app, pointer, (101, 100), MouseButton::Right)?;
    assert!(has_text(&mut app, "Banker Hireling"));
    assert!(commands.try_recv().is_err());

    // A tile that is not next to the player keeps the pick armed and the form hidden.
    click(&mut app, Some(3))?;
    app.update();
    assert!(!has_text(&mut app, "Banker Hireling"));
    world_click(&mut app, pointer, (103, 100), MouseButton::Left)?;
    assert!(!has_text(&mut app, "Banker Hireling"));
    assert!(commands.try_recv().is_err());

    world_click(&mut app, pointer, (101, 100), MouseButton::Left)?;
    assert!(has_text(&mut app, "Banker Hireling"));
    assert!(!refused(&mut app));
    click(&mut app, Some(0))?;
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Hireling(HirelingCommand(Request::Summon {
            x: 101,
            y: 100,
            floor: 7
        }))
    );
    Ok(())
}

/// Opens Character titles through its key and applies a self query reply that earns
/// `earned`; `None` is a refused query that leaves the titles unloaded.
fn titles_window(
    earned: Option<&[oteryn_client_runtime::character_info_panel::CharacterTitle]>,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_character_info_selected();
    press(
        &mut app,
        28,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Character titles — Ctrl+Alt+Shift+Y")?);
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("title query missing".into());
    };
    let (status, payload) = match earned {
        Some(earned) => {
            // CHARACTER-INFO-1 self reply: tag, eight counters, padding, earned ids, selected.
            let mut payload = vec![0];
            for value in [37i64, 120_500, 4, 6, 12, 3, 1, 0] {
                payload.extend(value.to_le_bytes());
            }
            payload.extend([0, 0, 0, 0, u8::try_from(earned.len())?]);
            payload.extend(earned.iter().map(|title| title.id()));
            payload.push(0);
            (CommandStatus::Accepted, payload)
        }
        None => (CommandStatus::Rejected, Vec::new()),
    };
    outbox.bevy_properties_result(
        &query,
        &GatedCommandResult {
            command_id: 1,
            status,
            server_sequence: 1,
            payload,
        },
    );
    app.update();
    Ok((app, commands, outbox))
}

/// A row number typed next to an unapplied Search draft names the displayed row: Use
/// selected and Unlock selected act on that title, not on row 1 of the draft-filtered list.
#[test]
fn title_row_actions_resolve_the_displayed_row_not_the_unapplied_search() -> TestResult {
    use oteryn_client_runtime::character_info_panel::{
        CharacterInfoOperation, CharacterTitle, encode_operation,
    };
    let mut titles = CharacterTitle::ALL.to_vec();
    titles.sort_by_key(|title| title.name());
    let (first, last) = (
        *titles.first().ok_or("no titles")?,
        *titles.last().ok_or("no titles")?,
    );
    assert!(
        !first
            .name()
            .to_lowercase()
            .contains(&last.name().to_lowercase())
    );
    // Use selected (3) on an earned row, Unlock selected (2) on a locked row.
    for (action, earned, expected) in [
        (
            3,
            vec![first, last],
            CharacterInfoOperation::Select(Some(first)),
        ),
        (2, vec![], CharacterInfoOperation::Unlock(first)),
    ] {
        let (mut app, mut commands, _outbox) = titles_window(Some(&earned))?;
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains(&format!("\n1. {} — ", first.name())))
        );
        for (field, mut text) in app
            .world_mut()
            .query::<(&Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            match field.0 {
                0 => *text = EditableText::new(last.name()),
                2 => *text = EditableText::new("1"),
                _ => {}
            }
        }
        click(&mut app, Some(action))?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("title action missing".into());
        };
        assert_eq!(command.payload, encode_operation(expected));
    }
    Ok(())
}

/// Row actions the state refuses say why: Unlock on an earned title, Use on a locked one and
/// Clear selection before the titles loaded each show feedback and send nothing.
#[test]
fn refused_title_actions_show_why_instead_of_doing_nothing() -> TestResult {
    use oteryn_client_runtime::character_info_panel::CharacterTitle;
    let first = *CharacterTitle::ALL
        .iter()
        .min_by_key(|title| title.name())
        .ok_or("no titles")?;
    for (earned, action, row, expected) in [
        (Some(vec![first]), 2, "1", "Already owned"),
        (Some(vec![]), 3, "1", "Locked or ineligible"),
        (None, 4, "", "Refresh first and settle any pending request"),
    ] {
        let (mut app, mut commands, _outbox) = titles_window(earned.as_deref())?;
        for (field, mut text) in app
            .world_mut()
            .query::<(&Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 2 {
                *text = EditableText::new(row);
            }
        }
        assert!(!has_summary(&mut app, expected));
        click(&mut app, Some(action))?;
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "{expected}: a command was sent"
        );
        assert!(
            has_summary(&mut app, expected),
            "{expected}: no feedback shown"
        );
    }
    Ok(())
}

fn has_summary(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query_filtered::<&Text, With<Summary>>()
        .iter(app.world())
        .any(|text| text.0.contains(value))
}
