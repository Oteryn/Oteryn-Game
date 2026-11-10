//! The member projection is rendered in the existing native party window.
#[must_use]
pub fn panel_text(panel: &crate::party::PartyPanel) -> String {
    crate::party_member_effects::panel_text(panel)
}
