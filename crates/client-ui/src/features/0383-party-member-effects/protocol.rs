use crate::party::PartyPanel;
use oteryn_client_runtime::party::PartyResponse;

pub fn apply_result(panel: &mut PartyPanel, response: &PartyResponse) -> bool {
    crate::party::apply_result(panel, response)
}
