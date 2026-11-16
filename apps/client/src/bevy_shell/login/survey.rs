//! The optional client survey as an in-client dialog over the login form, as Tibia Global
//! shows its survey: invitation, answer, review of the exact response, then consent.
use super::*;
use crate::about;

enum Step {
    Invite,
    Question,
    /// The exact response that Yes queues for delivery.
    Consent(String),
}

pub(in crate::bevy_shell) struct State {
    step: Option<Step>,
    /// Where consented responses wait for the next signed-in play session.
    directory: Option<PathBuf>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            step: None,
            directory: crate::support_delivery::directory(),
        }
    }
}
impl State {
    pub fn visible(&self) -> bool {
        self.step.is_some()
    }
    pub(super) fn open(&mut self) {
        self.step = Some(Step::Invite);
    }
    fn text(&self) -> String {
        match &self.step {
            Some(Step::Invite) => about::survey_invitation().to_owned(),
            Some(Step::Question) => about::survey_question().to_owned(),
            Some(Step::Consent(response)) => about::survey_consent(response),
            None => String::new(),
        }
    }
    /// Applies Yes or No to the shown step; returns the login message once a response is queued
    /// or cannot be saved.
    fn answer(&mut self, yes: bool) -> Option<&'static str> {
        match self.step.take()? {
            Step::Invite if yes => self.step = Some(Step::Question),
            Step::Question => self.step = Some(Step::Consent(about::survey_response(yes))),
            Step::Consent(response) if yes => {
                let saved = self.directory.as_ref().map(|directory| {
                    about::queue_survey_response(&directory.join("approved-survey.txt"), &response)
                });
                return Some(match saved {
                    Some(Ok(_)) => {
                        tr!("Survey response queued for delivery on your next play session.")
                    }
                    _ => tr!("Cannot save the response. No response was sent."),
                });
            }
            Step::Invite | Step::Consent(_) => {}
        }
        None
    }
}
#[derive(Component)]
struct Dialog;
#[derive(Component)]
struct Body;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Yes,
    No,
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
                width: px(460.),
                max_width: percent(100.),
                max_height: percent(100.),
                padding: UiRect {
                    top: px(29.),
                    bottom: px(12.),
                    left: px(16.),
                    right: px(16.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(10.),
                ..default()
            },
        ))
        .id();
    commands.entity(modal).add_child(panel);
    source::dialog_caption(commands, panel, "Client Survey", skins.font.clone());
    let body = commands
        .spawn((
            Body,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    let footer = commands
        .spawn(Node {
            column_gap: px(5.),
            justify_content: JustifyContent::End,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    for (action, caption) in [(Action::Yes, "Yes"), (Action::No, "No")] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 86.), action))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    commands.entity(panel).add_children(&[body, footer]);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.survey.visible() || !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if let Some(message) = owner.login.survey.answer(matches!(action, Action::Yes)) {
        owner.login.form.feedback(message.to_owned());
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    dialogs: Query<Entity, With<Dialog>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if owner.login.survey.visible() && dialogs.iter().any(|entity| guard.allows(entity)) {
        for key in keys.read() {
            if key.key_code == KeyCode::Escape
                && key.state == bevy::input::ButtonState::Pressed
                && !key.repeat
            {
                owner.login.survey.step = None;
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
    let visible = owner.login.survey.visible();
    for mut node in &mut dialogs {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if visible {
        let value = owner.login.survey.text();
        for mut text in &mut bodies {
            if text.0 != value {
                text.0.clone_from(&value);
            }
        }
    }
    if visible && !*shown {
        // Declining is the safe default for an optional survey.
        if let Some((entity, _)) = actions.iter().find(|(_, action)| **action == Action::No) {
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
#[path = "survey_tests.rs"]
mod tests;
