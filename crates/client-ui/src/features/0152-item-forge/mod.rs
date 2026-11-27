//! Native controls for Item Forge.
#[path = "state/exaltation.rs"]
pub mod exaltation;
#[path = "protocol/preview.rs"]
pub mod preview;
mod protocol;
#[path = "state/resource_dialog.rs"]
pub mod resource_dialog;
#[path = "state/selection.rs"]
mod selection;
mod state;
pub use protocol::reply;
mod view;
pub use protocol::{apply_result, command, control_request, preview_request, request};
pub use selection::{MAX_CANDIDATES, PreviewState};
pub use state::ItemForgePanel;
pub use view::{panel_text, selected_pair_text};
#[cfg(test)]
#[path = "state/selection_tests.rs"]
mod selection_tests;

/// Open the Forge after a production world-context USE reply and request its first page.
pub fn begin_open(
    panel: &mut ItemForgePanel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    use oteryn_client_runtime::{cui3::item_forge::Operation, forge_preview::Selection};
    if !panel.selected {
        return None;
    }
    if !panel.operation.requires_pair() && panel.retry.is_none() {
        panel.operation = Operation::Fusion;
    }
    panel.open = true;
    if panel.pending {
        return None;
    }
    panel.preview.invalidate();
    panel.preview.items.clear();
    panel.preview.page = None;
    preview_request(panel, Selection::Browse { after: 0 })
}
