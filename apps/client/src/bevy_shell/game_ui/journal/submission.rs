//! Retain the original journal action while Bevy completes its editor queue.
use super::*;
#[derive(Clone)]
pub(super) struct Pending {
    action: Action,
    source: Entity,
    fields: Vec<Entity>,
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn request(
    owner: &mut HostOwner,
    state: &mut State,
    action: Action,
    fields: &Query<(Entity, &Field, &EditableText)>,
    source: Entity,
) {
    state.pending = None;
    if matches!(
        action.kind,
        Kind::Search | Kind::Sort | Kind::Refresh | Kind::Vocation | Kind::Premium | Kind::Group
    ) && fields.iter().any(|(_, _, text)| editing(text))
    {
        state.pending = Some(Pending {
            action,
            source,
            fields: fields.iter().map(|(id, _, _)| id).collect(),
        });
    } else {
        actions::perform(owner, state, action, &actions::values(fields));
    }
}
pub(super) fn finish(
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(Entity, &Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let visible = match pending.action.key.tab {
        Tab::Quests => {
            let quest = play.link.quest_panel();
            quest.open || quest.tracker_open
        }
        Tab::Spells => {
            let book = play.link.spellbook_state();
            let catalogue = play.link.spell_catalog_panel();
            book.open || catalogue.open || catalogue.archive_open
        }
    };
    if !visible
        || state.key != Some(pending.action.key)
        || owner.generation() != pending.action.key.generation
        || owner.modal_active()
        || !guard.allows(pending.source)
        || fields.iter().count() != pending.fields.len()
        || pending
            .fields
            .iter()
            .any(|id| !guard.allows(*id) || fields.get(*id).is_err())
    {
        return;
    }
    if fields.iter().any(|(_, _, text)| editing(text)) {
        state.pending = Some(pending);
        return;
    }
    actions::perform(
        &mut owner,
        &mut state,
        pending.action,
        &actions::values(&fields),
    );
}
