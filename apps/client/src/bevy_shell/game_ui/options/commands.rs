use super::{
    HostOwner,
    metadata::{self, Page, Value},
};

/// One typed edit of the current owner's session-bound transient draft.
pub(super) fn edit(
    owner: &mut HostOwner,
    generation: u64,
    page: Page,
    id: &str,
    value: Value,
) -> bool {
    if generation != owner.generation() || owner.preferences_draft().is_none() {
        return false;
    }
    let Some(row) = metadata::find(page, id) else {
        return false;
    };
    let mut edited = false;
    owner.edit_preferences(|draft| edited = row.set(draft, value));
    edited
}
