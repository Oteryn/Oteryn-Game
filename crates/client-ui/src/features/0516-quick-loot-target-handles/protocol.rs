use crate::quick_loot::{
    CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1, COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES,
    QuickLootRequest, QuickLootWireError, encode_quick_loot_request,
};
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;

pub(crate) fn request(
    request: &QuickLootRequest,
) -> Result<ServerMechanicCommand, QuickLootWireError> {
    Ok(ServerMechanicCommand {
        capability: CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1,
        command_type: COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES,
        payload: encode_quick_loot_request(request)?,
    })
}
