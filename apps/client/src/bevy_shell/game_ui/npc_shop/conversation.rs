//! The existing NPC conversation owner shares the shop's one floating window.
use super::*;
use bevy::{input::keyboard::KeyboardInput, input_focus::FocusCause};
use oteryn_client_ui::npc_dialogue::DialoguePanel;
use oteryn_session::EntityRef;
mod submission;
mod widgets;
pub(super) use widgets::{Widgets, build};
#[derive(Component)]
pub(super) struct Message(pub u64);
#[derive(Component, Clone)]
pub(super) struct Control {
    pub generation: u64,
    pub target: Option<EntityRef>,
    pub kind: Kind,
    pub lines: Vec<String>,
}
#[derive(Clone)]
pub(super) enum Kind {
    Send,
    Trade,
    Keyword(String),
}
#[derive(Default)]
pub(super) struct Ui {
    generation: Option<u64>,
    target: Option<EntityRef>,
    field: Option<Entity>,
    keywords: Vec<String>,
    lines: Vec<String>,
    feedback: String,
    pending: Option<submission::Pending>,
    focus: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.add_message::<KeyboardInput>()
        .add_systems(Update, keyboard.before(super::project))
        .add_systems(
            PostUpdate,
            (
                submission::finish.after(bevy::text::EditableTextSystems),
                autofocus,
            ),
        );
}
fn set_text(editor: &mut EditableText, value: &str) {
    editor.pending_edits.clear();
    editor.pending_paste = None;
    editor.editor_mut().set_text(value);
    editor.queue_edit(bevy::text::TextEdit::TextEnd(false));
}
fn current(owner: &HostOwner, generation: u64, actor: EntityRef) -> Option<DialoguePanel> {
    if owner.generation() != generation || owner.modal_active() {
        return None;
    }
    let panel = owner.play.as_ref()?.link.npc_dialogue_panel();
    (panel.selected && panel.open && !panel.pending && panel.target == Some(actor)).then_some(panel)
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut fields: Query<&mut EditableText, With<Message>>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(actor) = control.target else {
        return;
    };
    let Some(panel) = current(&owner, control.generation, actor) else {
        return;
    };
    if panel.lines != control.lines {
        return;
    }
    let Some(widgets) = state.widgets.map(|widgets| widgets.conversation) else {
        return;
    };
    match &control.kind {
        Kind::Send => submission::request(&owner, &mut state, event.entity, widgets.message, actor),
        Kind::Trade | Kind::Keyword(_) => {
            let text = match &control.kind {
                Kind::Trade => "trade",
                Kind::Keyword(word) if panel.keywords().contains(&word.as_str()) => word,
                _ => return,
            };
            state.pending = None;
            state.conversation.pending = None;
            let Some(play) = &owner.play else {
                return;
            };
            if play.link.npc_dialogue_reply(actor, &control.lines, text) {
                if let Ok(mut editor) = fields.get_mut(widgets.message) {
                    set_text(&mut editor, "");
                }
                state.conversation.feedback.clear();
            }
        }
    }
}
fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    focus: Res<InputFocus>,
    fields: Query<&Message>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if keys.is_empty() {
        return;
    }
    let Some(entity) = focus.get().filter(|entity| guard.allows(*entity)) else {
        keys.clear();
        return;
    };
    let Ok(field) = fields.get(entity) else {
        keys.clear();
        return;
    };
    let Some(actor) = state.conversation.target else {
        keys.clear();
        return;
    };
    if current(&owner, field.0, actor).is_none() {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
            && matches!(key.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
        {
            submission::request(&owner, &mut state, entity, entity, actor);
        }
    }
}
// A single retained dialogue projects into the existing NPC frame and two hidden trade regions.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut fields: Query<&mut EditableText, With<Message>>,
    mut nodes: Query<&mut Node>,
    mut text: Query<&mut Text>,
) {
    let Some(root) = state.widgets else {
        state.conversation = Ui::default();
        return;
    };
    let Some(play) = &owner.play else {
        state.conversation = Ui::default();
        return;
    };
    let dialogue = play.link.npc_dialogue_panel();
    let shop_visible = play.link.npc_shop_is_open();
    for region in [root.trade, root.shop_controls] {
        if let Ok(mut node) = nodes.get_mut(region) {
            node.display = if shop_visible {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
    let widgets = root.conversation;
    let visible = dialogue.selected && dialogue.visible();
    if let Ok(mut node) = nodes.get_mut(widgets.controls) {
        node.display = if visible && dialogue.target.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    let generation = owner.generation();
    let changed = state.conversation.generation != Some(generation)
        || state.conversation.target != dialogue.target
        || state.conversation.field != Some(widgets.message);
    if changed {
        state.conversation = Ui {
            generation: Some(generation),
            target: dialogue.target,
            field: Some(widgets.message),
            focus: visible.then_some(widgets.message),
            ..default()
        };
        if let Ok(mut editor) = fields.get_mut(widgets.message) {
            set_text(&mut editor, "");
        }
    }
    let context_changed = changed || state.conversation.lines != dialogue.lines;
    if context_changed {
        state.conversation.lines.clone_from(&dialogue.lines);
        for entity in [widgets.send, widgets.trade] {
            let kind = if entity == widgets.send {
                Kind::Send
            } else {
                Kind::Trade
            };
            commands.entity(entity).insert(Control {
                generation,
                target: dialogue.target,
                kind,
                lines: dialogue.lines.clone(),
            });
        }
    }
    let words: Vec<String> = if visible {
        dialogue.keywords().into_iter().map(str::to_owned).collect()
    } else {
        Vec::new()
    };
    if context_changed || words != state.conversation.keywords {
        commands.entity(widgets.keywords).despawn_children();
        if let Some(actor) = dialogue.target {
            widgets::keywords(
                &mut commands,
                &skins,
                widgets.keywords,
                generation,
                actor,
                &words,
                &dialogue.lines,
            );
        }
        state.conversation.keywords = words;
    }
    if let Ok(mut caption) = text.get_mut(widgets.transcript) {
        let value = if visible {
            dialogue.text()
        } else {
            String::new()
        };
        if caption.0 != value {
            caption.0 = value;
        }
    }
    let progress = if !state.conversation.feedback.is_empty() {
        state.conversation.feedback.as_str()
    } else if state.conversation.pending.is_some() {
        "Waiting for text input…"
    } else if dialogue.pending {
        "Waiting for NPC…"
    } else {
        ""
    };
    if let Ok(mut caption) = text.get_mut(widgets.progress) {
        if caption.0 != progress {
            caption.0 = progress.into();
        }
    }
}
fn autofocus(mut state: ResMut<State>, mut focus: ResMut<InputFocus>, guard: ModalGuard) {
    if let Some(entity) = state.conversation.focus.take()
        && guard.allows(entity)
    {
        focus.set(entity, FocusCause::Navigated);
    }
}
