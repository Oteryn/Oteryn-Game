//! Party join and leave notices (domain 57, capability 576) refresh the Party window through
//! the existing Party query; the notice itself carries only an opaque member handle.
use crate::party::PartyPanel;

/// Whether a refresh is still owed after `notices` new join or leave notices arrived.
pub fn observed(waiting: bool, notices: usize) -> bool {
    waiting || notices > 0
}

/// Consumes an owed refresh when the Party query may be sent now. A pending Party command keeps
/// the refresh owed; a session without the Party capability drops it.
pub fn take_refresh(panel: &PartyPanel, waiting: &mut bool) -> bool {
    if !*waiting {
        return false;
    }
    if !panel.selected {
        *waiting = false;
        return false;
    }
    if panel.pending.is_some() {
        return false;
    }
    *waiting = false;
    true
}

#[cfg(test)]
#[test]
fn notices_owe_one_refresh_until_the_party_query_is_free() {
    assert!(!observed(false, 0));
    assert!(observed(false, 2));
    assert!(observed(true, 0));
    let mut panel = PartyPanel {
        selected: true,
        ..PartyPanel::default()
    };
    let mut waiting = false;
    assert!(!take_refresh(&panel, &mut waiting));
    waiting = true;
    panel.pending = Some(crate::party::PartyCommand::query());
    assert!(!take_refresh(&panel, &mut waiting));
    assert!(waiting);
    panel.pending = None;
    assert!(take_refresh(&panel, &mut waiting));
    assert!(!waiting);
    panel.selected = false;
    waiting = true;
    assert!(!take_refresh(&panel, &mut waiting));
    assert!(!waiting);
}
