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
    /// chsw (capability 512): the Change Channel window, its confirmation and its errors.
    ChannelSwitch,
    Context {
        menu: Option<Box<Menu>>,
        party: Option<Box<party::Target>>,
        inspection: Option<(TileCoord, oteryn_session::EntityRef)>,
        report: Option<Box<rule_violation::Target>>,
        trade: Option<(TileCoord, oteryn_session::EntityRef)>,
        browse: Option<(TileCoord, i16)>,
        npc: Option<(TileCoord, oteryn_session::EntityRef)>,
        /// The name of the clicked player other than the own character.
        player: Option<String>,
        /// The clicked creature or player Alt+click would attack.
        /// The clicked player or monster Alt+click would attack (never the own character or
        /// an NPC), and whether it is the current target.
        attack: Option<(TileCoord, bool)>,
    },
}
#[derive(Clone, Copy, Component)]
enum Choice {
    Cancel,
    Logout,
    SwitchRow([u8; 16]),
    SwitchConfirm,
    SwitchBack,
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
    Attack,
    Message,
    Vip,
    Ignore,
    CopyName,
}
impl Dialogs {
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }
    pub fn clear(&mut self) {
        self.pending = None;
    }
    /// The player name of the pending context menu `id`.
    fn context_player_name(&self, id: u64) -> Option<String> {
        match self.pending.as_ref().filter(|request| request.id == id)? {
            Request {
                content:
                    Content::Context {
                        player: Some(name), ..
                    },
                ..
            } => Some(name.clone()),
            _ => None,
        }
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
    pub(super) fn request_channel_switch(&mut self) {
        if let Some(play) = &self.play {
            play.link.channel_switch_open();
            self.dialogs.open(self.generation(), Content::ChannelSwitch);
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
        let player = self
            .play
            .as_ref()
            .and_then(|play| play.view.party_context_target(tile)?.1);
        let attack = self
            .play
            .as_ref()
            .filter(|_| {
                menu.as_ref().is_some_and(|menu| {
                    matches!(
                        menu.target,
                        oteryn_client_ui::world_interaction::menu::Target::Creature(_)
                    )
                })
            })
            .and_then(|play| play.view.attack_context(tile, &play.link))
            .map(|attacking| (tile, attacking));
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
                    player,
                    attack,
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
                    player: None,
                    attack: None,
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
        if matches!(request.content, Content::ChannelSwitch) {
            return self.choose_channel_switch(request, choice);
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
            (_, Choice::Cancel | Choice::CopyName) => true,
            (
                Content::Context {
                    attack: Some((tile, _)),
                    ..
                },
                Choice::Attack,
            ) => self
                .play
                .as_ref()
                .is_some_and(|play| play.view.attack_toggle_click(tile, &play.link)),
            (
                Content::Context {
                    player: Some(name), ..
                },
                Choice::Message | Choice::Vip | Choice::Ignore,
            ) => self.play.as_ref().is_some_and(|play| {
                if play.view.gameplay_input_blocked(&play.link) {
                    return false;
                }
                match choice {
                    Choice::Message => play.link.chat_open_conversation(&name),
                    Choice::Vip => {
                        play.link
                            .vip_request(oteryn_client_ui::vip::Intent::Add(name));
                        true
                    }
                    _ => {
                        play.link.chat_communication(
                            Some(oteryn_client_ui::chat::CommunicationList::Ignore),
                            &name,
                            true,
                        );
                        true
                    }
                }
            }),
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
    checked: Handle<Image>,
    unchecked: Handle<Image>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, (escape.after(super::input::route), project).chain())
        .init_resource::<scale::ModuleScale>()
        .add_systems(Update, (art::project, popup::layout, scale::zoom))
        .add_systems(
            PostUpdate,
            scale::apply.before(bevy::ui::UiSystems::Prepare),
        );
}
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Skin {
        buttons: source::ButtonImages {
            up: assets.load("images/skin/classic/button-grey-up.png"),
            down: assets.load("images/skin/classic/button-grey-down.png"),
        },
        frame: assets.load("images/skin/classic/dialog-frame-borderimage.png"),
        font: assets.load("fonts/verdanab.ttf"),
        checked: assets.load("images/skin/classic/checkbox-checked.png"),
        unchecked: assets.load("images/skin/classic/checkbox-unchecked.png"),
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
        if let Some(id) = owner
            .dialogs
            .pending
            .as_ref()
            .filter(|request| matches!(request.content, Content::ChannelSwitch))
            .map(|request| request.id)
        {
            owner.choose_dialog(id, Choice::Cancel);
        } else {
            owner.dialogs.clear();
            owner.pointer_context_closed();
        }
    }
}
fn activate(
    event: On<Activate>,
    replies: Query<&Reply>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::game_ui::modal::ModalGuard,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    if !modal.allows(event.entity) {
        return;
    }
    if let Ok(reply) = replies.get(event.entity) {
        if matches!(reply.choice, Choice::CopyName)
            && let Some(mut clipboard) = clipboard
            && let Some(name) = owner.dialogs.context_player_name(reply.id)
        {
            let _ = clipboard.set_text(name);
        }
        owner.choose_dialog(reply.id, reply.choice);
    }
}

pub(super) mod art;
mod channel_switch;
mod npc;
pub(super) mod popup;
mod projection;
pub(super) mod scale;
use projection::project;

#[cfg(test)]
mod furniture_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod party_tests;
