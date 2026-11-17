//! Cosmetic Token commands share the real Store panel and input queue.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
pub fn request(
    panel: &mut crate::store::StorePanel,
    kind: u32,
    payload: Vec<u8>,
) -> Option<ServerMechanicCommand> {
    crate::store::request(panel, crate::store::command(kind, payload)?)
}
