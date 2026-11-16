//! Tibia Global's Manual Sort Mode (docs/reference/tibia-manual/controls.md:79) through the
//! production host input, container projection and drop route.
use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, play_channel},
};
use oteryn_input_actions::{
    ButtonState, KeyCode as Key, Modifier, Modifiers, NormalizedInputEvent,
};
use oteryn_session::container_tree::{ContainerView, ContainerViews};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn app() -> Result<(App, tokio::sync::mpsc::UnboundedReceiver<PlayCommand>)> {
    use bevy::asset::AssetPlugin;
    let (link, commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(ContainerViews {
            views: vec![ContainerView {
                view_id: 0,
                container_handle: 10.try_into()?,
                parent_handle: None,
                capacity: 8,
                entries: vec![tests::entry(11, 7)?, tests::entry(12, 3)?],
                browse: None,
            }],
        }),
        // Uncounted moves: a dropped stack moves whole, without the amount prompt.
        false,
        true,
    );
    PlayLink::bevy_manual_sort_fixture(&outbox, true);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<bevy::text::RemSize>()
    .init_resource::<UiScale>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .insert_non_send(tests::host(link)?)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            crate::bevy_shell::game_ui::setup,
        )
            .chain(),
    )
    .add_systems(Update, crate::bevy_shell::game_ui::project);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands))
}

fn shift_s(app: &mut App) -> Result {
    let mut event = NormalizedInputEvent::Key {
        code: Key::new(22)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Shift),
        repeat: false,
    };
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        host.route_input(std::slice::from_ref(&event), false);
        if let NormalizedInputEvent::Key { state, .. } = &mut event {
            *state = ButtonState::Released;
        }
        host.route_input(&[event], false);
    }
    app.update();
    Ok(())
}

fn title_color(app: &mut App) -> Result<Color> {
    app.world_mut()
        .query::<(&Text, &TextColor)>()
        .iter(app.world())
        .find(|(text, _)| text.0.ends_with("(2/8)"))
        .map(|(_, color)| color.0)
        .ok_or_else(|| "container title".into())
}

/// The projected container slot `index` and the destination its drop sends.
fn drop_on(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    index: usize,
) -> Result<ItemMoveDestination> {
    let slots: Vec<Slot> = app
        .world_mut()
        .query::<&Slot>()
        .iter(app.world())
        .filter(|slot| matches!(slot.location, Location::Container { view: 0, .. }))
        .copied()
        .collect();
    let target = slots
        .iter()
        .filter(|slot| slot.item.is_none())
        .nth(index - 2)
        .ok_or("empty container slot")?;
    let source = slots
        .iter()
        .find(|slot| slot.item.is_some())
        .ok_or("source")?;
    let owner = app.world().non_send::<HostOwner>();
    let play = owner.play.as_ref().ok_or("play")?;
    assert!(play.link.item_drag_begin(source.drag(owner).ok_or("drag")?));
    play.link.item_drag_drop(target.destination);
    let PlayCommand::MoveItem(request) = commands.try_recv()? else {
        return Err("move".into());
    };
    Ok(request.destination)
}

#[test]
fn shift_s_turns_titles_orange_and_a_slot_drop_sends_its_index() -> Result {
    let [r, g, b] = oteryn_client_ui::container_manual_sort::ACTIVE_TITLE_RGB;
    let orange = Color::srgb_u8(r, g, b);
    let container = 10.try_into()?;
    for manual in [false, true] {
        let (mut app, mut commands) = app()?;
        assert_eq!(title_color(&mut app)?, oteryn_client_ui_bevy::TEXT_COLOR);
        if manual {
            shift_s(&mut app)?;
            assert_eq!(title_color(&mut app)?, orange);
        }
        assert_eq!(
            drop_on(&mut app, &mut commands, 5)?,
            ItemMoveDestination::Container(container, manual.then_some(5)),
            "manual sort: {manual}"
        );
        if manual {
            shift_s(&mut app)?;
            assert_eq!(title_color(&mut app)?, oteryn_client_ui_bevy::TEXT_COLOR);
        }
    }
    Ok(())
}
