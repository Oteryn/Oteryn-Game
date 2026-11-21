//! Reviews the optional survey response and collects consent in the login window.
use super::*;
use crate::about;
use bevy::diagnostic::FrameCount;
use bevy_tasks::{IoTaskPool, Task, futures::check_ready};
use std::io;

#[cfg(test)]
type Writer = std::sync::Arc<dyn Fn(&std::path::Path, &str) -> io::Result<bool> + Send + Sync>;

struct Save {
    generation: u64,
    task: Task<io::Result<bool>>,
}

enum Step {
    Invite,
    Question,
    /// The exact response that Yes queues for delivery.
    Consent(String),
    Saving,
}

pub(in crate::bevy_shell) struct State {
    step: Option<Step>,
    shown: Option<(std::mem::Discriminant<Step>, u32)>,
    generation: u64,
    save: Option<Save>,
    #[cfg(test)]
    writer: Option<Writer>,
    /// Where consented responses wait for the next signed-in play session.
    directory: Option<PathBuf>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            step: None,
            shown: None,
            generation: 0,
            save: None,
            #[cfg(test)]
            writer: None,
            directory: crate::support_delivery::directory(),
        }
    }
}
impl State {
    pub fn visible(&self) -> bool {
        self.step.is_some()
    }
    pub(super) fn open(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.step = Some(Step::Invite);
        self.shown = None;
    }
    fn close(&mut self) {
        if self.step.take().is_some() {
            self.generation = self.generation.wrapping_add(1);
        }
    }
    fn action_enabled(&self, action: Action) -> bool {
        match &self.step {
            Some(Step::Saving) => false,
            Some(Step::Consent(_)) if action == Action::Yes => self.save.is_none(),
            _ => true,
        }
    }
    fn can_answer(&self, frame: u32) -> bool {
        self.shown.is_some_and(|(shown, shown_frame)| {
            frame != shown_frame
                && self
                    .step
                    .as_ref()
                    .is_some_and(|step| std::mem::discriminant(step) == shown)
        })
    }
    fn text(&self) -> String {
        match &self.step {
            Some(Step::Invite) => about::survey_invitation().to_owned(),
            Some(Step::Question) => about::survey_question().to_owned(),
            Some(Step::Consent(response)) => about::survey_consent(response),
            Some(Step::Saving) => "Saving survey response…".into(),
            None => String::new(),
        }
    }
    fn answer(&mut self, action: Action) {
        let yes = action == Action::Yes;
        self.step = match self.step.take() {
            Some(Step::Invite) if yes => Some(Step::Question),
            Some(Step::Question) => Some(Step::Consent(about::survey_response(yes))),
            Some(Step::Consent(response)) if yes => {
                self.start_save(response);
                Some(Step::Saving)
            }
            _ => None,
        };
    }
    fn start_save(&mut self, response: String) {
        let directory = self.directory.clone();
        #[cfg(test)]
        let writer = self.writer.clone();
        let task = IoTaskPool::get().spawn(async move {
            let path = directory
                .ok_or_else(|| io::Error::other("No survey directory"))?
                .join("approved-survey.txt");
            #[cfg(test)]
            if let Some(writer) = writer {
                return writer(&path, &response);
            }
            about::queue_survey_response(&path, &response)
        });
        self.save = Some(Save {
            generation: self.generation,
            task,
        });
    }
    fn poll_save(&mut self) -> Option<&'static str> {
        let save = self.save.as_mut()?;
        let result = check_ready(&mut save.task)?;
        let current = save.generation == self.generation;
        self.save = None;
        if !current {
            return None;
        }
        self.close();
        Some(match result {
            Ok(true) => tr!("Survey response queued for delivery on your next play session."),
            _ => tr!("Cannot save the response. No response was sent."),
        })
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
    app.add_systems(
        Update,
        (keyboard, poll_save, project).chain().after(super::project),
    );
}
pub(super) fn setup(commands: &mut Commands, skins: &Skins) {
    let modal = commands
        .spawn((
            Dialog,
            GameModal,
            GlobalZIndex(161),
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
                order: 4,
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
    source::dialog_caption(commands, panel, tr!("Survey"), skins.font.clone());
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
    for (action, caption) in [(Action::Yes, tr!("Yes")), (Action::No, tr!("No"))] {
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
    frame: Res<FrameCount>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.visible
        || !owner.login.survey.can_answer(frame.0)
        || !guard.allows(event.entity)
    {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if owner.login.survey.action_enabled(*action) {
        owner.login.survey.answer(*action);
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
                owner.login.survey.close();
            }
        }
    } else {
        keys.clear();
    }
}
fn poll_save(mut owner: NonSendMut<HostOwner>) {
    if !owner.login.visible {
        owner.login.survey.close();
    }
    if let Some(message) = owner.login.survey.poll_save() {
        owner.login.form.inform(tr!("Survey"), message.to_owned());
    }
}
fn project(
    mut owner: NonSendMut<HostOwner>,
    mut dialogs: Query<&mut Node, With<Dialog>>,
    mut bodies: Query<&mut Text, With<Body>>,
    actions: Query<(Entity, &Action, Has<InteractionDisabled>)>,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
    frame: Res<FrameCount>,
) {
    let step = owner.login.survey.step.as_ref().map(std::mem::discriminant);
    let visible = step.is_some();
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
    let shown = owner.login.survey.shown.map(|(shown, _)| shown);
    for (entity, action, disabled) in &actions {
        let enabled = owner.login.survey.action_enabled(*action);
        if enabled == disabled {
            if enabled {
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
        if !enabled && focus.get() == Some(entity) {
            focus.clear();
        } else if enabled && visible && shown != step && *action == Action::No {
            // Declining is the safe default for an optional survey.
            focus.set(entity, FocusCause::Navigated);
        } else if !visible && focus.get() == Some(entity) {
            focus.clear();
        }
    }
    if shown != step {
        owner.login.survey.shown = step.map(|step| (step, frame.0));
    }
}

#[cfg(test)]
#[path = "survey_tests.rs"]
mod tests;
