use oteryn_client_runtime::cl25a_protocol::VipPresenceBatch;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VipPresencePanel {
    pub selected: bool,
    pub open: bool,
    pub presence: VipPresenceBatch,
}
