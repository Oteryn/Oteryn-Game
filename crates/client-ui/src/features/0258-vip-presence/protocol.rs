use super::VipPresencePanel;
pub fn replace(
    panel: &mut VipPresencePanel,
    value: oteryn_client_runtime::cl25a_protocol::VipPresenceBatch,
) {
    panel.presence = value;
}
