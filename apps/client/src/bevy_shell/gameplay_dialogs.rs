//! Generation-bound Bevy presentation for existing logout and world-context owners.
use super::owner::{HostOwner, Phase};
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
    Context(Menu),
}
#[derive(Clone, Copy, Component)]
enum Choice {
    Cancel,
    Logout,
    Context(usize),
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
            .and_then(|play| play.view.world_context(tile, &play.link));
        if self.phase == Phase::Playing
            && let Some(menu) = menu
        {
            self.dialogs.open(self.generation(), Content::Context(menu));
        }
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
        if request.generation != self.generation() || self.phase != Phase::Playing {
            return false;
        }
        let Some(play) = &mut self.play else {
            return false;
        };
        match (request.content, choice) {
            (_, Choice::Cancel) => true,
            (Content::Logout, Choice::Logout) => {
                play.view.logout();
                true
            }
            (Content::Context(menu), Choice::Context(index)) => {
                menu.entries.get(index).is_some_and(|(_, action)| {
                    play.view.world_context_choose(&play.link, &menu, *action)
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
) {
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
fn activate(event: On<Activate>, replies: Query<&Reply>, mut owner: NonSendMut<HostOwner>) {
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
        Content::Context(_) => "Actions",
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
        Content::Context(menu) => {
            for (index, (label, _)) in menu.entries.iter().enumerate() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    label,
                    Choice::Context(index),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{PlayView, play_channel};
    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn shared_logout_cancel_duplicate_and_stale_confirm_reach_existing_queue() -> Result {
        // Test-only play fixture; shipping admission still comes exclusively from Reentry.
        let mut host = HostOwner::new(None)?;
        let (link, mut commands, _events) = play_channel();
        host.play = Some(super::super::owner::ActivePlay {
            view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        host.phase = Phase::Playing;
        host.request_logout_confirmation();
        let first = host.dialogs.pending.as_ref().ok_or("confirmation")?.id;
        assert!(host.choose_dialog(first, Choice::Cancel));
        assert!(!host.dialogs.active());
        assert!(commands.try_recv().is_err());
        host.request_logout_confirmation();
        let second = host
            .dialogs
            .pending
            .as_ref()
            .ok_or("second confirmation")?
            .id;
        assert!(!host.choose_dialog(first, Choice::Logout));
        assert!(
            host.dialogs.active(),
            "late click must not dismiss a newer dialog"
        );
        assert!(host.choose_dialog(second, Choice::Logout));
        assert!(!host.choose_dialog(second, Choice::Logout));
        let play = host.play.as_mut().ok_or("play")?;
        play.view
            .tick(&play.link)
            .map_err(|end| format!("unexpected end {end:?}"))?;
        assert!(matches!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::Logout
        ));
        assert!(commands.try_recv().is_err());
        Ok(())
    }

    #[test]
    fn previous_generation_and_wrong_context_target_cannot_dispatch() -> Result {
        let mut host = HostOwner::new(None)?;
        let (link, mut commands, _events) = play_channel();
        host.play = Some(super::super::owner::ActivePlay {
            view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        host.phase = Phase::Playing;
        host.request_logout_confirmation();
        let request = host.dialogs.pending.as_mut().ok_or("confirmation")?;
        request.generation = request.generation.saturating_add(1);
        let id = request.id;
        assert!(!host.choose_dialog(id, Choice::Logout));
        let menu = Menu {
            tile: Some(TileCoord::new(i32::MAX, i32::MAX)),
            binding: None,
            target: oteryn_client_runtime::world_interaction::Target::Position(
                oteryn_session::ActorPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            ),
            entries: vec![(
                "Look",
                oteryn_client_runtime::world_interaction::Action::Look,
            )],
        };
        host.dialogs.open(host.generation(), Content::Context(menu));
        let id = host.dialogs.pending.as_ref().ok_or("context")?.id;
        assert!(!host.choose_dialog(id, Choice::Context(0)));
        assert!(commands.try_recv().is_err());
        Ok(())
    }
}
