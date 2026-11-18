//! Per-frame projection of the active owner dialog into the Bevy overlay.
use super::*;
use channel_switch::{SwitchView, TableRow};

#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skin>,
    windows: Query<&Window>,
    mut overlays: Query<(Entity, &mut Node, &mut BackgroundColor), With<Overlay>>,
    mut previous: Local<Option<(u64, Option<SwitchView>)>>,
    mut rows: Query<(&TableRow, &mut Text, &mut TextColor)>,
    mut panel: Local<Option<Entity>>,
    mut focus: ResMut<InputFocus>,
) {
    let pending = owner.dialogs.pending.as_ref();
    let switch = pending
        .filter(|request| matches!(request.content, Content::ChannelSwitch))
        .and_then(|_| owner.play.as_ref())
        .map(|play| {
            let now_ms = crate::play::feature_0512_channel_switch::view::now_ms();
            let panel = play.link.channel_switch_panel();
            let left = panel
                .status
                .map_or(0, |status| i64::from(status.cooldown_left_ms))
                .saturating_sub(now_ms.saturating_sub(panel.status_at_ms).max(0))
                .max(0);
            ((panel, (left + 999) / 1000), now_ms)
        });
    let Ok((overlay, mut node, mut background)) = overlays.single_mut() else {
        return;
    };
    node.display = if pending.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let id = pending.map(|request| request.id);
    let switch_view = switch.as_ref().map(|(view, _)| view);
    if previous.as_ref().map(|(id, view)| (*id, view.as_ref())) == id.map(|id| (id, switch_view)) {
        return;
    }
    if let (Some((old_id, Some((old_panel, second)))), Some(id), Some(((new_panel, next), now_ms))) =
        (previous.as_mut(), id, switch.as_ref())
        && *old_id == id
        && old_panel == new_panel
    {
        channel_switch::refresh_rows(&mut rows, new_panel, *now_ms);
        *second = *next;
        return;
    }
    *previous = id.map(|id| (id, switch_view.cloned()));
    if let Some(entity) = panel.take() {
        commands.entity(entity).despawn();
    }
    let Some(request) = pending else {
        return;
    };
    if let Content::Context { menu, .. } = &request.content {
        // The popup takes the whole overlay so its scrim catches the click outside the menu.
        background.0 = Color::NONE;
        let (rotate, wrap) = owner.furniture(menu.as_deref());
        let entries = context_items(&request.content, rotate, wrap);
        let items: Vec<_> = entries.iter().map(|(item, _)| item.clone()).collect();
        let look = super::popup::Look {
            font: skin.font.clone(),
            frame: skin.frame.clone(),
            checked: Some(skin.checked.clone()),
            unchecked: Some(skin.unchecked.clone()),
        };
        let popup =
            super::popup::spawn(&mut commands, &look, super::popup::cursor(&windows), &items);
        let layer = commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            })
            .add_children(&[popup.scrim, popup.root])
            .id();
        commands.entity(overlay).add_child(layer);
        commands
            .entity(popup.scrim)
            .insert(Reply {
                id: request.id,
                choice: Choice::Cancel,
            })
            .observe(dismiss);
        let choices = entries.iter().filter_map(|(item, choice)| {
            matches!(item, super::popup::Item::Entry { .. }).then_some(*choice)
        });
        for (entity, choice) in popup.entries.iter().zip(choices) {
            commands
                .entity(*entity)
                .insert(Reply {
                    id: request.id,
                    choice,
                })
                .observe(activate);
            if let (
                Content::Context {
                    party: Some(target),
                    ..
                },
                Choice::Invite,
            ) = (&request.content, choice)
                && !target.invite_enabled()
            {
                commands
                    .entity(*entity)
                    .insert(bevy::ui::InteractionDisabled);
            }
        }
        *panel = Some(layer);
        focus.clear();
        return;
    }
    background.0 = Color::srgba(0., 0., 0., 0.45);
    let window = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                width: px(if switch.is_some() { 460. } else { 300. }),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::axes(px(16.), px(29.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(window);
    *panel = Some(window);
    if let Some(((panel, _), now_ms)) = switch {
        source::dialog_caption(
            &mut commands,
            window,
            tr!("Change Channel"),
            skin.font.clone(),
        );
        channel_switch::channel_switch_body(
            &mut commands,
            &skin,
            window,
            request.id,
            &panel,
            now_ms,
        );
    } else {
        source::dialog_caption(&mut commands, window, tr!("Logout"), skin.font.clone());
        let text = commands
            .spawn(source::text(
                "Log out of the game?",
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(window).add_child(text);
        button(
            &mut commands,
            &skin,
            window,
            request.id,
            tr!("Ok"),
            Choice::Logout,
        );
    }
    let cancel = button(
        &mut commands,
        &skin,
        window,
        request.id,
        tr!("Cancel"),
        Choice::Cancel,
    );
    focus.set(cancel, FocusCause::Navigated);
}

/// A click outside the popup closes it without choosing.
fn dismiss(
    event: On<bevy::picking::events::PointerClick>,
    replies: Query<&Reply>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::super::game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.entity) {
        return;
    }
    if let Ok(reply) = replies.get(event.entity) {
        owner.choose_dialog(reply.id, Choice::Cancel);
    }
}

// Default (Regular) control scheme of Tibia Global: Shift+click looks, Ctrl+click uses,
// Alt+click attacks.
const LOOK: &str = "Shift";
const USE: &str = "Ctrl";
const ATTACK: &str = "Alt";

/// The popup rows of a context menu in Tibia Global order, each with the choice it makes.
/// Items and ground: Look, Use, Use with, Open, then Rotate, Wrap/Unwrap, Browse Field and
/// Trade with. Creatures and players: Attack, Look, Message to, Add to VIP list, Ignore,
/// Invite to Party, then Inspect, Report and Copy Name.
pub(super) fn context_items(
    content: &Content,
    rotate: bool,
    wrap: bool,
) -> Vec<(super::popup::Item, Choice)> {
    use super::popup::Item;
    use oteryn_client_runtime::world_interaction::Action;
    let Content::Context {
        menu,
        party,
        inspection,
        report,
        trade,
        browse,
        npc,
        player,
        attack,
    } = content
    else {
        return Vec::new();
    };
    let entries: Vec<(usize, Action, &str)> = menu
        .iter()
        .flat_map(|menu| menu.entries.iter())
        .enumerate()
        .map(|(index, (label, action))| (index, *action, *label))
        .collect();
    let find = |wanted: Action| {
        entries
            .iter()
            .find(|(_, action, _)| *action == wanted)
            .map(|(index, _, _)| Choice::Context(*index))
    };
    let mut groups: Vec<Vec<(Item, Choice)>> = Vec::new();
    let mut first = Vec::new();
    if let Some((_, (_, attacking))) = attack {
        let label = if *attacking { "Stop Attack" } else { "Attack" };
        first.push((Item::shortcut(label, ATTACK), Choice::Attack));
    }
    if let Some(choice) = find(Action::Look) {
        first.push((Item::shortcut("Look", LOOK), choice));
    }
    // Plain Use stays reachable on a bed; a bed without one is used through its bed action.
    let bed_as_use = find(Action::Use).is_none();
    if let Some(choice) = find(Action::Use).or_else(|| find(Action::BedInteract)) {
        first.push((Item::shortcut("Use", USE), choice));
    }
    if let Some(choice) = find(Action::UseWith) {
        first.push((Item::shortcut("Use with ...", USE), choice));
    }
    if let Some(choice) = find(Action::UseRune) {
        first.push((Item::entry("Use rune ..."), choice));
    }
    if let Some(choice) = find(Action::Open) {
        first.push((Item::entry("Open"), choice));
    }
    // Remaining owner entries (a bed's sleep choices) keep their own captions, after Open.
    for (index, action, label) in &entries {
        if !matches!(
            action,
            Action::Look | Action::Use | Action::UseWith | Action::UseRune | Action::Open
        ) && !(bed_as_use && *action == Action::BedInteract)
        {
            first.push((Item::entry(*label), Choice::Context(*index)));
        }
    }
    groups.push(first);
    let mut world = Vec::new();
    if rotate {
        world.push((Item::entry("Rotate"), Choice::Rotate));
    }
    if wrap {
        world.push((Item::entry("Wrap/Unwrap"), Choice::Wrap));
    }
    if browse.is_some() {
        world.push((Item::entry("Browse Field"), Choice::Browse));
    }
    if npc.is_some() {
        world.push((Item::entry("Talk"), Choice::Talk));
        world.push((Item::entry("Trade"), Choice::NpcTrade));
    }
    groups.push(world);
    let name = player.as_deref();
    let mut social = Vec::new();
    if let Some(name) = name {
        social.push((Item::entry(format!("Message to {name}")), Choice::Message));
        social.push((Item::entry("Add to VIP list"), Choice::Vip));
        social.push((Item::entry(format!("Ignore {name}")), Choice::Ignore));
    }
    if party.is_some() {
        social.push((Item::entry("Invite to Party"), Choice::Invite));
        social.push((Item::entry("Party"), Choice::Party));
    }
    if trade.is_some() {
        let name = name.or_else(|| party.as_ref().and_then(|party| party.name()));
        social.push((
            Item::entry(name.map_or_else(
                || "Trade with ...".to_owned(),
                |name| format!("Trade with {name}"),
            )),
            Choice::Trade,
        ));
    }
    groups.push(social);
    let mut tools = Vec::new();
    if inspection.is_some() {
        tools.push((Item::entry("Inspect"), Choice::Inspect));
    }
    if report.is_some() {
        tools.push((Item::entry("Report"), Choice::Report));
    }
    if name.is_some() {
        tools.push((Item::entry("Copy Name"), Choice::CopyName));
    }
    groups.push(tools);
    let mut items = Vec::new();
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !items.is_empty() {
            items.push((Item::Separator, Choice::Cancel));
        }
        items.extend(group);
    }
    items
}
pub(super) fn button(
    commands: &mut Commands,
    skin: &Skin,
    parent: Entity,
    id: u64,
    caption: &str,
    choice: Choice,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), 260.),
            Reply { id, choice },
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
    button
}
