//! Deferred free text stays bound to the actor and the server's original conversation context.
use super::*;
#[derive(Clone)]
pub(super) struct Pending {
    anchor: Entity,
    field: Entity,
    generation: u64,
    actor: EntityRef,
    lines: Vec<String>,
    keywords: Vec<String>,
}
pub(super) fn request(
    owner: &HostOwner,
    state: &mut State,
    anchor: Entity,
    field: Entity,
    actor: EntityRef,
) {
    if state.conversation.pending.is_some() {
        return;
    }
    let Some(panel) = current(owner, owner.generation(), actor) else {
        return;
    };
    if state.generation != Some(owner.generation()) || state.conversation.lines != panel.lines {
        return;
    }
    state.pending = None;
    state.conversation.pending = Some(Pending {
        anchor,
        field,
        generation: owner.generation(),
        actor,
        lines: panel.lines.clone(),
        keywords: panel.keywords().into_iter().map(str::to_owned).collect(),
    });
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut fields: Query<(&Message, &mut EditableText)>,
    guard: ModalGuard,
) {
    let Some(pending) = state.conversation.pending.take() else {
        return;
    };
    let Some(panel) = current(&owner, pending.generation, pending.actor) else {
        return;
    };
    if state.generation != Some(pending.generation)
        || !guard.allows(pending.anchor)
        || panel.lines != pending.lines
        || panel
            .keywords()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
            != pending.keywords
    {
        return;
    }
    let Ok((field, mut editor)) = fields.get_mut(pending.field) else {
        return;
    };
    if field.0 != pending.generation {
        return;
    }
    if super::super::submission::editing(&editor) {
        state.conversation.pending = Some(pending);
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play
        .link
        .npc_dialogue_reply(pending.actor, &pending.lines, &editor.value().to_string())
    {
        set_text(&mut editor, "");
        state.conversation.feedback.clear();
    } else {
        state.conversation.feedback =
            "This message cannot be sent in the current conversation.".into();
    }
}
