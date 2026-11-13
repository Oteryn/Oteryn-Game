use super::{
    HostOwner,
    metadata::{self, Page, Value},
};

/// One typed edit of the current owner's session-bound transient draft.
pub(super) fn edit(
    _owner: &mut HostOwner,
    _generation: u64,
    _page: Page,
    _id: &str,
    _value: Value,
) -> bool {
    // Intentionally unwired for the first red caller check.
    false
}
