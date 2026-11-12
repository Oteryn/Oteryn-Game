//! Capability 491 `IMBUEMENT_SUCCESS_CHANCE_V1`: each imbuement offer carries the success
//! percentage the server rolls with. Without it the shrine panel keeps its fixed 100% line.
use crate::imbue::ImbuePanel;

/// The server's success percentage of one offer, when the session selected the capability and
/// the window it belongs to is the one shown.
#[must_use]
pub fn success_percent(panel: &ImbuePanel, offer: usize) -> Option<u8> {
    if !panel.success_selected {
        return None;
    }
    let window = panel.window.as_ref()?;
    if panel.success.len() != window.offers.len() {
        return None;
    }
    panel.success.get(offer).copied().filter(|p| *p <= 100)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
