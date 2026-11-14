//! Caller-supplied confirmation over the same numeric stack dialog used by inventory.
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, InputFocus},
    ui_widgets::ValueChange,
};

pub(in crate::bevy_shell::game_ui) trait AmountTarget:
    Send + Sync
{
    fn maximum(&self, owner: &HostOwner) -> Option<u32>;
    fn confirm(&self, owner: &HostOwner, amount: u32) -> bool;
}
struct Request {
    id: u64,
    generation: u64,
    target: Box<dyn AmountTarget>,
    title: &'static str,
    confirm: &'static str,
    maximum: u32,
    count: u32,
    raw: String,
    feedback: String,
    pending: bool,
}
#[derive(Resource, Default)]
pub(in crate::bevy_shell::game_ui) struct CallerAmount {
    next: u64,
    request: Option<Request>,
}
impl CallerAmount {
    pub(in crate::bevy_shell::game_ui) fn open(
        &mut self,
        owner: &HostOwner,
        target: Box<dyn AmountTarget>,
        title: &'static str,
        confirm: &'static str,
    ) -> bool {
        if self.request.is_some() || owner.modal_active() {
            return false;
        }
        let Some(maximum) = target.maximum(owner).filter(|maximum| *maximum > 0) else {
            return false;
        };
        let Some(id) = self.next.checked_add(1) else {
            return false;
        };
        let count = maximum.min(100);
        self.next = id;
        self.request = Some(Request {
            id,
            generation: owner.generation(),
            target,
            title,
            confirm,
            maximum,
            count,
            raw: count.to_string(),
            feedback: String::new(),
            pending: false,
        });
        true
    }
}
#[derive(Component)]
pub(super) struct Field(pub u64);
#[derive(Component)]
pub(super) struct Range;
#[derive(Component)]
pub(super) struct Feedback;
#[derive(Component)]
pub(super) struct Control {
    pub id: u64,
    pub action: AmountAction,
}
type Labels = Or<(With<Feedback>, With<Range>)>;

pub(super) fn install(app: &mut App) {
    app.init_resource::<CallerAmount>()
        .add_systems(
            Update,
            (project, focus_new, keyboard)
                .chain()
                .after(super::presentation::project),
        )
        .add_systems(
            PostUpdate,
            (edited, finish)
                .chain()
                .after(bevy::text::EditableTextSystems),
        );
}
fn current(request: &Request, owner: &HostOwner) -> Option<u32> {
    (request.generation == owner.generation() && !owner.modal_active())
        .then(|| request.target.maximum(owner))
        .flatten()
        .filter(|max| *max > 0)
}
fn draft(request: &mut Request, owner: &HostOwner, raw: String) {
    let result = current(request, owner)
        .ok_or_else(|| "The selected item is no longer available.".to_string())
        .and_then(|maximum| validate_amount(&raw, maximum));
    request.raw = raw;
    match result {
        Ok(count) => {
            request.count = count;
            request.feedback.clear();
        }
        Err(message) => request.feedback = message,
    }
}
#[allow(clippy::too_many_arguments)] // Keep the live numeric editor while independently refreshing its labels and session fence.
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    inventory: Res<InventoryUi>,
    mut ui: ResMut<CallerAmount>,
    mut previous: Local<Option<(u64, Entity)>>,
    fields: Query<(Entity, &Field)>,
    mut labels: Query<(&mut Text, Has<Feedback>), Labels>,
    mut focus: ResMut<InputFocus>,
) {
    if ui.request.as_ref().is_some_and(|request| {
        request.generation != owner.generation()
            || owner.play.is_none()
            || inventory.prompt.is_some()
            || (!owner.modal_active() && request.target.maximum(&owner).is_none())
    }) {
        ui.request = None;
    }
    if previous.as_ref().map(|(id, _)| *id) != ui.request.as_ref().map(|request| request.id) {
        if let Some((_, root)) = previous.take() {
            if fields.iter().any(|(entity, _)| focus.get() == Some(entity)) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        if let Some(request) = &ui.request {
            let root = super::presentation::build_dialog(
                &mut commands,
                &skins,
                DialogSpec {
                    source: DialogSource::Caller(request.id),
                    title: request.title,
                    confirm: request.confirm,
                    value: &request.raw,
                    maximum: request.maximum,
                    item: None,
                },
            );
            *previous = Some((request.id, root));
        }
    }
    if let Some(request) = &ui.request {
        for (mut text, feedback) in &mut labels {
            let value = if feedback {
                request.feedback.clone()
            } else {
                format!("of {}", request.maximum)
            };
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}
fn focus_new(
    fields: Query<(Entity, &Field), Added<Field>>,
    guard: ModalGuard,
    mut focus: ResMut<InputFocus>,
) {
    for (entity, _) in &fields {
        if guard.allows(entity) {
            focus.set(entity, FocusCause::Navigated);
        }
    }
}
pub(super) fn changed(
    event: On<ValueChange<String>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut fields: Query<(&Field, &mut EditableText)>,
    mut ui: ResMut<CallerAmount>,
) {
    if !guard.allows(event.source) {
        return;
    }
    let Ok((field, mut editor)) = fields.get_mut(event.source) else {
        return;
    };
    let Some(request) = ui
        .request
        .as_mut()
        .filter(|request| request.id == field.0 && current(request, &owner).is_some())
    else {
        return;
    };
    draft(request, &owner, event.value.clone());
    set_editor(&mut editor, &event.value);
}
fn editing(editor: &EditableText) -> bool {
    editor.pending_paste.is_some()
        || editor
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut fields: Query<(&Field, &mut EditableText)>,
    mut ui: ResMut<CallerAmount>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(request) = ui
        .request
        .as_mut()
        .filter(|request| request.id == control.id)
    else {
        return;
    };
    let Some(maximum) = current(request, &owner) else {
        return;
    };
    match control.action {
        AmountAction::Cancel => ui.request = None,
        AmountAction::Confirm => request.pending = true,
        action => {
            let count = match action {
                AmountAction::All => maximum,
                AmountAction::Change(delta) => {
                    request.count.saturating_add_signed(delta).clamp(1, maximum)
                }
                _ => return,
            };
            // A delayed paste must finish before changing the amount with a button.
            if fields
                .iter()
                .any(|(field, editor)| field.0 == request.id && editing(editor))
            {
                return;
            }
            draft(request, &owner, count.to_string());
            for (field, mut editor) in &mut fields {
                if field.0 == request.id {
                    set_editor(&mut editor, &request.raw);
                }
            }
        }
    }
}
fn edited(
    fields: Query<(Entity, &Field, &EditableText), Changed<EditableText>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<CallerAmount>,
) {
    let Some(request) = ui.request.as_mut() else {
        return;
    };
    for (entity, field, editor) in &fields {
        if field.0 == request.id
            && guard.allows(entity)
            && current(request, &owner).is_some()
            && !editing(editor)
        {
            let raw = editor.value().to_string();
            if raw != request.raw {
                draft(request, &owner, raw);
            }
        }
    }
}
fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    guard: ModalGuard,
    focus: Res<InputFocus>,
    fields: Query<&Field>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<CallerAmount>,
) {
    let field = focus
        .get()
        .filter(|entity| guard.allows(*entity))
        .and_then(|entity| fields.get(entity).ok());
    let Some(field) = field else {
        keys.clear();
        return;
    };
    if !ui
        .request
        .as_ref()
        .is_some_and(|request| request.id == field.0 && current(request, &owner).is_some())
    {
        keys.clear();
        return;
    }
    for key in keys
        .read()
        .filter(|key| key.state == bevy::input::ButtonState::Pressed && !key.repeat)
    {
        if key.key_code == KeyCode::Escape {
            ui.request = None;
            return;
        }
        if matches!(key.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
            && let Some(request) = &mut ui.request
        {
            request.pending = true;
        }
    }
}
fn finish(
    fields: Query<(Entity, &Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<CallerAmount>,
) {
    let Some(request) = ui.request.as_mut().filter(|request| request.pending) else {
        return;
    };
    let Some((entity, _, editor)) = fields.iter().find(|(_, field, _)| field.0 == request.id)
    else {
        request.pending = false;
        return;
    };
    if !guard.allows(entity) || current(request, &owner).is_none() {
        request.pending = false;
        return;
    }
    if editing(editor) {
        return;
    }
    request.pending = false;
    draft(request, &owner, editor.value().to_string());
    if !request.feedback.is_empty() {
        return;
    }
    if request.target.confirm(&owner, request.count) {
        ui.request = None;
    } else {
        request.feedback = "The selection or session has changed.".into();
    }
}
