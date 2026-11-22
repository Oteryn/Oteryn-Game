//! Retained admission news and calendar, independent of the connection form lifecycle.
use super::*;
use bevy::ui_widgets::ScrollArea;

#[derive(Default)]
pub(in crate::bevy_shell) struct State {
    browser: Option<crate::bulletin::Browser>,
    pub visible: bool,
    calendar: bool,
    body: String,
}
impl State {
    pub fn receive(&mut self, document: &str) -> Result<(), String> {
        let mut browser = crate::bulletin::Browser::default();
        browser.replace(document)?;
        self.browser = Some(browser);
        self.calendar = false;
        self.visible = true;
        self.refresh();
        Ok(())
    }
    fn text(&self) -> &str {
        if self.body.is_empty() {
            "No bulletin has been received. Sign in to receive operator news and events."
        } else {
            &self.body
        }
    }
    fn refresh(&mut self) {
        if let Some(browser) = &self.browser {
            self.body = if self.calendar {
                browser.month()
            } else {
                browser.news()
            };
        }
    }
}
#[derive(Component)]
struct Dialog;
#[derive(Component)]
struct Body;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    News,
    Calendar,
    Previous,
    Next,
    Close,
}

pub(super) fn install(app: &mut App) {
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    app.add_systems(Update, (keyboard, project).chain().after(super::project));
}
pub(super) fn setup(commands: &mut Commands, skins: &Skins) {
    let modal = commands
        .spawn((
            Dialog,
            GameModal,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    let panel = commands
        .spawn((
            super::super::window_drag::DraggableDialog,
            TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(620.),
                max_width: percent(100.),
                height: px(430.),
                max_height: percent(100.),
                padding: UiRect {
                    top: px(29.),
                    bottom: px(12.),
                    left: px(16.),
                    right: px(16.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(modal).add_child(panel);
    source::dialog_caption(commands, panel, "Login Bulletin", skins.font.clone());
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let body = commands
        .spawn((
            Body,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(scroll).add_child(body);
    let footer = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    for (action, caption) in [
        (Action::News, "News"),
        (Action::Calendar, "Calendar"),
        (Action::Previous, "Previous month"),
        (Action::Next, "Next month"),
        (Action::Close, "Close"),
    ] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 105.), action))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    commands.entity(panel).add_children(&[scroll, footer]);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.bulletin.visible || !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let state = &mut owner.login.bulletin;
    match action {
        Action::Close => state.visible = false,
        Action::News => state.calendar = false,
        Action::Calendar => state.calendar = true,
        Action::Previous | Action::Next => {
            state.calendar = true;
            if let Some(browser) = &mut state.browser {
                if matches!(action, Action::Previous) {
                    browser.previous_month();
                } else {
                    browser.next_month();
                }
            }
        }
    }
    state.refresh();
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    dialogs: Query<Entity, With<Dialog>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if owner.login.bulletin.visible && dialogs.iter().any(|entity| guard.allows(entity)) {
        for key in keys.read() {
            if key.key_code == KeyCode::Escape
                && key.state == bevy::input::ButtonState::Pressed
                && !key.repeat
            {
                owner.login.bulletin.visible = false;
            }
        }
    } else {
        keys.clear();
    }
}
fn project(
    owner: NonSend<HostOwner>,
    mut dialogs: Query<&mut Node, With<Dialog>>,
    mut bodies: Query<&mut Text, With<Body>>,
    actions: Query<(Entity, &Action)>,
    mut focus: ResMut<InputFocus>,
    mut shown: Local<bool>,
) {
    let visible = owner.login.bulletin.visible;
    for mut node in &mut dialogs {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if visible {
        let value = owner.login.bulletin.text();
        for mut text in &mut bodies {
            if text.0 != value {
                text.0 = value.to_owned();
            }
        }
    }
    if visible && !*shown {
        if let Some((entity, _)) = actions
            .iter()
            .find(|(_, action)| matches!(action, Action::Close))
        {
            focus.set(entity, FocusCause::Navigated);
        }
    } else if !visible
        && *shown
        && focus
            .get()
            .is_some_and(|entity| actions.get(entity).is_ok())
    {
        focus.clear();
    }
    *shown = visible;
}

#[cfg(test)]
#[path = "bulletin_tests.rs"]
mod tests;
