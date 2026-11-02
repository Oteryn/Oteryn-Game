use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 230,
        command_type: 440,
        payload,
    }
}
pub fn request(
    panel: &mut super::BrowseFieldPanel,
    command: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    crate::server_mechanic::request(panel, command)
}
pub fn apply_result(panel: &mut super::BrowseFieldPanel, result: &ServerMechanicResponse) {
    crate::server_mechanic::apply_result(panel, result);
}
