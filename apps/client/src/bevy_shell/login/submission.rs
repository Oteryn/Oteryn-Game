//! Only the original form's completed editor values may start admission.
use super::*;
#[derive(Clone)]
pub(super) struct Pending {
    generation: u64,
    revision: u64,
    fields: Vec<Entity>,
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn request(owner: &mut HostOwner, fields: &Query<(Entity, &LoginField, &EditableText)>) {
    owner.login.pending_submit = None;
    if fields.iter().any(|(_, _, text)| editing(text)) {
        owner.login.pending_submit = Some(Pending {
            generation: owner.generation(),
            revision: owner.login.revision,
            fields: fields.iter().map(|(id, _, _)| id).collect(),
        });
    } else {
        read_fields(owner, fields);
        owner.submit_login();
    }
}
pub(super) fn finish(
    mut owner: NonSendMut<HostOwner>,
    fields: Query<(Entity, &LoginField, &EditableText)>,
    guard: ModalGuard,
) {
    let Some(pending) = owner.login.pending_submit.take() else {
        return;
    };
    if owner.generation() != pending.generation
        || owner.login.revision != pending.revision
        || !owner.login.visible
        || !owner.login.form.can_submit()
        || owner.selection.pending().is_some()
        || fields.iter().count() != pending.fields.len()
        || pending
            .fields
            .iter()
            .any(|id| !guard.allows(*id) || fields.get(*id).is_err())
    {
        return;
    }
    if fields.iter().any(|(_, _, text)| editing(text)) {
        owner.login.pending_submit = Some(pending);
        return;
    }
    read_fields(&mut owner, &fields);
    owner.submit_login();
}
