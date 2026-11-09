use crate::party::PartyPanel;
use oteryn_client_runtime::party::PartyResponse;

pub fn apply_result(panel: &mut PartyPanel, response: &PartyResponse) -> bool {
    crate::party_member_effects::apply_result(panel, response)
}
