//! Native world dragging reuses the authoritative creature push intent.
use oteryn_client_runtime::{
    creature_push::{CreaturePushIntent, encode_creature_push_intent},
    server_mechanic::ServerMechanicCommand,
};

pub fn request(intent: CreaturePushIntent) -> Option<ServerMechanicCommand> {
    Some(ServerMechanicCommand {
        capability: 82,
        command_type: 98,
        payload: encode_creature_push_intent(&intent).ok()?,
    })
}
