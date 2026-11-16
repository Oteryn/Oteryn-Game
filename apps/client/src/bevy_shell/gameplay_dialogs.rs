//! Generation-bound Bevy presentation for existing logout and world-context owners.
use super::{
    game_ui::{item_details, party, rule_violation},
    owner::{HostOwner, Phase},
};
use bevy::{
    input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::Activate,
};
use oteryn_client_ui::world_interaction::menu::Menu;
use oteryn_client_ui_bevy as source;
use oteryn_renderer::TileCoord;

#[derive(Default)]
pub(super) struct Dialogs {
    next: u64,
    pending: Option<Request>,
}
struct Request {
    id: u64,
    generation: u64,
    content: Content,
}
enum Content {
    Logout,
    Context {
        menu: Option<Box<Menu>>,
        party: Option<Box<party::Target>>,
        inspection: Option<(TileCoord, oteryn_session::EntityRef)>,
        report: Option<Box<rule_violation::Target>>,
        trade: Option<(TileCoord, oteryn_session::EntityRef)>,
        browse: Option<(TileCoord, i16)>,
        npc: Option<(TileCoord, oteryn_session::EntityRef)>,
    },
}
#[derive(Clone, Copy, Component)]
enum Choice {
    Cancel,
    Logout,
    Context(usize),
    Inspect,
    Invite,
    Party,
    Report,
    Trade,
    Browse,
    Talk,
    NpcTrade,
    Rotate,
    Wrap,
}
impl Dialogs {
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }
    pub fn clear(&mut self) {
        self.pending = None;
    }
    fn open(&mut self, generation: u64, content: Content) {
        let Some(id) = self.next.checked_add(1) else {
            self.clear();
            return;
        };
        self.next = id;
        self.pending = Some(Request {
            id,
            generation,
            content,
        });
    }
}
impl HostOwner {
    pub(super) fn request_logout_confirmation(&mut self) {
        if self.phase == Phase::Playing && self.play.is_some() {
            self.dialogs.open(self.generation(), Content::Logout);
        }
    }
    pub(super) fn request_world_context(&mut self, tile: TileCoord) {
        let menu = self
            .play
            .as_ref()
            .and_then(|play| play.view.world_context(tile, &play.link))
            .map(Box::new);
        // Capture the player before opening our own modal, which correctly blocks new world input.
        let party = party::context(self, tile).map(Box::new);
        let report = rule_violation::context(self, tile).map(Box::new);
        let inspection = item_details::player_context(self, tile).map(|entity| (tile, entity));
        let trade = self
            .play
            .as_ref()
            .filter(|play| {
                let panel = play.link.player_trade_panel();
                panel.selected
                    && !panel.pending
                    && !panel.items.pending
                    && crate::play::feature_0060_player_trade::view::trade_phase(&panel)
                        == crate::play::feature_0060_player_trade::view::TradePhase::Inactive
            })
            .and_then(|play| play.view.player_trade_context_target(tile))
            .map(|entity| (tile, entity));
        let browse = self
            .play
            .as_ref()
            .filter(|play| {
                let panel = play.link.browse_field_panel();
                panel.selected && !panel.pending
            })
            .map(|play| (tile, play.view.satellite_player().2));
        let npc = npc::target(self, tile).map(|entity| (tile, entity));
        if self.phase == Phase::Playing
            && (menu.is_some()
                || npc.is_some()
                || party.is_some()
                || report.is_some()
                || inspection.is_some()
                || trade.is_some()
                || browse.is_some())
        {
            self.dialogs.open(
                self.generation(),
                Content::Context {
                    menu,
                    party,
                    inspection,
                    report,
                    trade,
                    browse,
                    npc,
                },
            );
        }
    }
    pub(super) fn request_creature_context(&mut self, target: oteryn_session::EntityRef) {
        let unambiguous = self.play.as_ref().and_then(|play| {
            let row = play
                .link
                .filtered_battle_rows(play.link.active_battle_list())
                .into_iter()
                .find(|row| row.entity == target)?;
            let tile = TileCoord::new(row.position.x, row.position.y);
            let menu = play.view.world_context(tile, &play.link)?;
            (menu.target == oteryn_client_ui::world_interaction::menu::Target::Creature(target))
                .then_some(tile)
        });
        if let Some(tile) = unambiguous {
            self.request_world_context(tile);
            return;
        }
        let menu = self
            .play
            .as_ref()
            .and_then(|play| play.view.creature_context(target, &play.link));
        if self.phase == Phase::Playing
            && let Some(menu) = menu
        {
            self.dialogs.open(
                self.generation(),
                Content::Context {
                    menu: Some(Box::new(menu)),
                    party: None,
                    inspection: None,
                    report: None,
                    trade: None,
                    browse: None,
                    npc: None,
                },
            );
        }
    }
    /// Tibia Global offers Rotate and Wrap/Unwrap on a clicked item; the session's house
    /// furniture lists (capabilities 445 and 207) then name the eligible furniture.
    fn furniture(&self, menu: Option<&Menu>) -> (bool, bool) {
        use oteryn_client_ui::world_interaction::menu::Target;
        let Some(play) = self.play.as_ref().filter(|_| {
            menu.is_some_and(|menu| matches!(menu.target, Target::Item(_) | Target::Object(_)))
        }) else {
            return (false, false);
        };
        let rotation = play.link.house_rotation_panel();
        let wrap = play.link.house_furniture_panel();
        (
            rotation.selected && !rotation.pending,
            wrap.selected && !wrap.pending,
        )
    }
    fn choose_dialog(&mut self, id: u64, choice: Choice) -> bool {
        if self
            .dialogs
            .pending
            .as_ref()
            .is_none_or(|request| request.id != id)
        {
            return false;
        }
        let Some(request) = self.dialogs.pending.take() else {
            return false;
        };
        self.pointer_context_closed();
        if request.generation != self.generation()
            || self.phase != Phase::Playing
            || self.play.is_none()
        {
            return false;
        }
        match (request.content, choice) {
            (
                Content::Context {
                    inspection: Some((tile, entity)),
                    ..
                },
                Choice::Inspect,
            ) => item_details::inspect_player(self, tile, entity),
            (
                Content::Context {
                    trade: Some((tile, target)),
                    ..
                },
                Choice::Trade,
            ) => {
                if self.modal_active() {
                    return false;
                }
                self.play.as_ref().is_some_and(|play| {
                    !play.view.gameplay_input_blocked(&play.link)
                        && play.view.player_trade_context_target(tile) == Some(target)
                        && play.link.player_trade_choose_partner(target)
                })
            }
            (
                Content::Context {
                    browse: Some((tile, floor)),
                    ..
                },
                Choice::Browse,
            ) => self.request_container_tool(
                super::controls::ContainerTool::BrowseField,
                Some((tile.x, tile.y, floor)),
                None,
            ),
            (
                Content::Context {
                    npc: Some((tile, entity)),
                    ..
                },
                Choice::Talk | Choice::NpcTrade,
            ) => npc::talk(self, tile, entity, matches!(choice, Choice::NpcTrade)),
            (_, Choice::Cancel) => true,
            (Content::Logout, Choice::Logout) => self.play.as_mut().is_some_and(|play| {
                play.view.logout();
                true
            }),
            (
                Content::Context {
                    menu: Some(menu), ..
                },
                Choice::Context(index),
            ) => self.play.as_ref().is_some_and(|play| {
                menu.entries.get(index).is_some_and(|(_, action)| {
                    play.view.world_context_choose(&play.link, &menu, *action)
                })
            }),
            (
                Content::Context {
                    party: Some(target),
                    ..
                },
                Choice::Invite,
            ) => party::invite(self, &target),
            (Content::Context { party: Some(_), .. }, Choice::Party) => {
                if self.modal_active() {
                    return false;
                }
                self.play.as_ref().is_some_and(|play| {
                    if play.view.gameplay_input_blocked(&play.link) {
                        return false;
                    }
                    play.link.open_party();
                    true
                })
            }
            (
                Content::Context {
                    report: Some(target),
                    ..
                },
                Choice::Report,
            ) => rule_violation::open(self, &target),
            (Content::Context { menu, .. }, Choice::Rotate | Choice::Wrap) => {
                let (rotate, wrap) = self.furniture(menu.as_deref());
                if self.modal_active() {
                    return false;
                }
                self.play.as_ref().is_some_and(|play| {
                    if play.view.gameplay_input_blocked(&play.link) {
                        return false;
                    }
                    match choice {
                        Choice::Rotate if rotate => play.link.house_rotation_open(),
                        Choice::Wrap if wrap => play.link.house_furniture_open(),
                        _ => return false,
                    }
                    true
                })
            }
            _ => false,
        }
    }
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Reply {
    id: u64,
    choice: Choice,
}
#[derive(Resource)]
struct Skin {
    buttons: source::ButtonImages,
    frame: Handle<Image>,
    font: Handle<Font>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, (escape.after(super::input::route), project).chain());
}
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Skin {
        buttons: source::ButtonImages {
            up: assets.load("images/skin/classic/button-grey-up.png"),
            down: assets.load("images/skin/classic/button-grey-down.png"),
        },
        frame: assets.load("images/skin/classic/dialog-frame-borderimage.png"),
        font: assets.load("fonts/verdanab.ttf"),
    });
    commands.spawn((
        Overlay,
        super::game_ui::GameModal,
        GlobalZIndex(160),
        Node {
            width: percent(100.),
            height: percent(100.),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
    ));
}
fn escape(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::game_ui::modal::ModalGuard,
    overlays: Query<Entity, With<Overlay>>,
) {
    if !overlays.iter().any(|entity| modal.allows(entity)) {
        keys.clear();
        return;
    }
    if owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
    {
        keys.clear();
        return;
    }
    if keys.read().any(|key| {
        key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
    }) {
        owner.dialogs.clear();
        owner.pointer_context_closed();
    }
}
fn activate(
    event: On<Activate>,
    replies: Query<&Reply>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.entity) {
        return;
    }
    if let Ok(reply) = replies.get(event.entity) {
        owner.choose_dialog(reply.id, reply.choice);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skin>,
    mut overlays: Query<(Entity, &mut Node), With<Overlay>>,
    mut previous: Local<Option<u64>>,
    mut panel: Local<Option<Entity>>,
    mut focus: ResMut<InputFocus>,
) {
    let pending = owner.dialogs.pending.as_ref();
    let id = pending.map(|request| request.id);
    let Ok((overlay, mut node)) = overlays.single_mut() else {
        return;
    };
    node.display = if pending.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if id == *previous {
        return;
    }
    *previous = id;
    if let Some(entity) = panel.take() {
        commands.entity(entity).despawn();
    }
    let Some(request) = pending else {
        return;
    };
    let window = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                width: px(300.),
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
    let title = match request.content {
        Content::Logout => "Logout",
        Content::Context { .. } => "Actions",
    };
    source::dialog_caption(&mut commands, window, title, skin.font.clone());
    match &request.content {
        Content::Logout => {
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
                "Ok",
                Choice::Logout,
            );
        }
        Content::Context {
            menu,
            party,
            inspection,
            report,
            trade,
            browse,
            npc,
        } => {
            if npc.is_some() {
                for (caption, choice) in [("Talk", Choice::Talk), ("Trade", Choice::NpcTrade)] {
                    button(&mut commands, &skin, window, request.id, caption, choice);
                }
            }
            if browse.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Browse Field",
                    Choice::Browse,
                );
            }
            if trade.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Trade with",
                    Choice::Trade,
                );
            }
            if report.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Report",
                    Choice::Report,
                );
            }
            for (index, (label, _)) in menu.iter().flat_map(|menu| menu.entries.iter()).enumerate()
            {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    label,
                    Choice::Context(index),
                );
            }
            let (rotate, wrap) = owner.furniture(menu.as_deref());
            for (shown, caption, choice) in [
                (rotate, "Rotate", Choice::Rotate),
                (wrap, "Wrap / Unwrap", Choice::Wrap),
            ] {
                if shown {
                    button(&mut commands, &skin, window, request.id, caption, choice);
                }
            }
            if inspection.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Inspect",
                    Choice::Inspect,
                );
            }
            if let Some(target) = party {
                if let Some(name) = target.name() {
                    let label = commands
                        .spawn(source::text(name, source::font(skin.font.clone(), false)))
                        .insert((
                            TextLayout::default(),
                            Node {
                                width: percent(100.),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(window).add_child(label);
                }
                let invite = button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Invite to Party",
                    Choice::Invite,
                );
                if !target.invite_enabled() {
                    commands
                        .entity(invite)
                        .insert(bevy::ui::InteractionDisabled);
                }
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Party",
                    Choice::Party,
                );
            }
        }
    }
    let cancel = button(
        &mut commands,
        &skin,
        window,
        request.id,
        "Cancel",
        Choice::Cancel,
    );
    focus.set(cancel, FocusCause::Navigated);
}
fn button(
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

mod npc;

#[cfg(test)]
mod furniture_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod party_tests;
