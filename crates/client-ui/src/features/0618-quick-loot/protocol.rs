use crate::quick_loot::{CAPABILITY_QUICK_LOOT_V2, COMMAND_TYPE_QUICK_LOOT_V2};
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;

pub(crate) fn request(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: CAPABILITY_QUICK_LOOT_V2,
        command_type: COMMAND_TYPE_QUICK_LOOT_V2,
        payload,
    }
}
