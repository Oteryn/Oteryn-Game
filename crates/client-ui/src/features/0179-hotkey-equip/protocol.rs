use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 179,
        command_type: 348,
        payload,
    }
}
pub fn request(
    panel: &mut super::HotkeyEquipPanel,
    command: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    crate::server_mechanic::request(panel, command)
}
pub fn apply_result(panel: &mut super::HotkeyEquipPanel, result: &ServerMechanicResponse) {
    crate::server_mechanic::apply_result(panel, result);
}
