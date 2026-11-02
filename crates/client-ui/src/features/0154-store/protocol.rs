use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};

pub fn command(command_type: u32, payload: Vec<u8>) -> Option<ServerMechanicCommand> {
    (183..=186)
        .contains(&command_type)
        .then_some(ServerMechanicCommand {
            capability: 154,
            command_type,
            payload,
        })
}
pub fn request(
    panel: &mut super::StorePanel,
    command: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    crate::server_mechanic::request(panel, command)
}
pub fn apply_result(panel: &mut super::StorePanel, result: &ServerMechanicResponse) {
    crate::server_mechanic::apply_result(panel, result);
}
