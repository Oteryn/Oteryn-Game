//! Numeric draft presentation over the retained movement prompt; only Confirm submits a move.
use super::*;
use bevy::text::EditableText;
mod input;
mod presentation;
pub(super) use input::prepare_confirm;

#[derive(Component)]
pub(super) struct Field(pub Prompt);
#[derive(Component)]
struct Root;
#[derive(Component)]
struct Feedback;
#[derive(Component)]
struct Range;

pub(super) fn install(app: &mut App) {
    app.init_resource::<bevy::input_focus::InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(
            Update,
            (
                presentation::project,
                presentation::focus_new,
                input::keyboard,
            )
                .chain()
                .after(super::view::project),
        )
        .add_systems(
            PostUpdate,
            (input::edited, input::finish_confirm)
                .chain()
                .after(bevy::text::EditableTextSystems),
        );
}
pub(super) fn source_entry(owner: &HostOwner, prompt: Prompt) -> Option<ItemEntry> {
    if owner.generation() != prompt.generation {
        return None;
    }
    let play = owner.play.as_ref()?;
    let inventory = play.link.inventory_panel();
    let containers = play.link.container_panel();
    inventory
        .inventory
        .iter()
        .flat_map(|inventory| {
            inventory
                .main_backpack
                .iter()
                .chain(&inventory.entries)
                .chain(inventory.equipment.iter().map(|equipped| &equipped.item))
        })
        .chain(
            containers
                .views
                .iter()
                .flat_map(|views| &views.views)
                .flat_map(|view| &view.entries),
        )
        .find(|item| item.handle == prompt.item.handle)
        .copied()
}
fn maximum(owner: &HostOwner, ui: &InventoryUi, prompt: Prompt) -> Option<u32> {
    if !super::actions::prompt_current(prompt, owner, ui) {
        return None;
    }
    let expected = ui.split_entry.or_else(|| source_entry(owner, prompt))?;
    if expected.handle != prompt.item.handle {
        return None;
    }
    let item = super::current_drag_item(
        owner,
        super::super::ItemDragSource {
            generation: prompt.generation,
            item: expected,
        },
    )?;
    Some(item.count.get().min(prompt.item.count).min(100))
}
fn value(ui: &InventoryUi) -> String {
    ui.split_text
        .clone()
        .unwrap_or_else(|| ui.split_count.to_string())
}
fn validate(owner: &HostOwner, ui: &InventoryUi, prompt: Prompt, raw: &str) -> Result<u32, String> {
    let Some(maximum) = maximum(owner, ui, prompt) else {
        return Err("The selected item is no longer available.".into());
    };
    let parsed = (!raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse::<u32>().ok())
        .flatten();
    parsed
        .filter(|count| *count >= 1 && *count <= maximum)
        .ok_or_else(|| format!("Enter a whole amount from 1 to {maximum}."))
}
fn set_draft(owner: &HostOwner, ui: &mut InventoryUi, prompt: Prompt, raw: String) {
    let result = validate(owner, ui, prompt, &raw);
    ui.split_text = Some(raw);
    match result {
        Ok(count) => {
            ui.split_count = count;
            ui.split_feedback.clear();
        }
        Err(message) => ui.split_feedback = message,
    }
}
pub(super) fn refresh(owner: &HostOwner, ui: &mut InventoryUi, prompt: Prompt) {
    if owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
        || owner.modal_active()
    {
        return;
    }
    let raw = value(ui);
    set_draft(owner, ui, prompt, raw);
}
pub(super) fn change_count(
    owner: &HostOwner,
    ui: &mut InventoryUi,
    prompt: Prompt,
    delta: Option<i32>,
) {
    let Some(maximum) = maximum(owner, ui, prompt) else {
        ui.split_feedback = "The selected item is no longer available.".into();
        return;
    };
    let count = match delta {
        Some(delta) => ui
            .split_count
            .saturating_add_signed(delta)
            .clamp(1, maximum),
        None => maximum,
    };
    set_draft(owner, ui, prompt, count.to_string());
}
pub(super) fn confirm(owner: &HostOwner, ui: &mut InventoryUi, prompt: Prompt) {
    if !super::actions::prompt_current(prompt, owner, ui) {
        if ui.prompt == Some(prompt) {
            ui.split_feedback = "The move selection has changed.".into();
        }
        return;
    }
    match validate(owner, ui, prompt, &value(ui)) {
        Ok(count) => {
            if let Some(play) = &owner.play {
                play.link.item_move_confirm(Some(count));
                ui.split_count = count;
                ui.split_feedback.clear();
            }
        }
        Err(message) => ui.split_feedback = message,
    }
}
fn set_editor(editor: &mut EditableText, value: &str) {
    if editor.value() != value {
        editor.pending_edits.clear();
        editor.pending_paste = None;
        editor.editor_mut().set_text(value);
        editor.queue_edit(bevy::text::TextEdit::TextEnd(false));
    }
}
