use super::ContainerPanel;
use oteryn_client_runtime::container_tree::{ContainerViewIntent, ContainerViewOutcome, feedback};
pub fn request(panel: &mut ContainerPanel) -> Option<ContainerViewIntent> {
    panel.open_selected()
}
pub fn apply_result(panel: &mut ContainerPanel, outcome: Option<ContainerViewOutcome>) {
    if outcome == Some(ContainerViewOutcome::Opened) {
        panel.accept_focus();
    } else {
        panel.clear_focus();
    }
    panel.finish(Some(crate::localization::tr(feedback(outcome))));
}
