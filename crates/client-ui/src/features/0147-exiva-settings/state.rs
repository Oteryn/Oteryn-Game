use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExivaPanel {
    pub selected: bool,
    pub open: bool,
    pub list: Option<PlayerList>,
    pub pending: bool,
    pub error: Option<&'static str>,
}
impl ExivaPanel {
    pub fn apply(&mut self, bytes: &[u8]) -> bool {
        if let Some(list) = PlayerList::decode(bytes) {
            self.list = Some(list);
            self.error = None;
            true
        } else {
            self.error = Some("Exiva request refused; refresh the list");
            false
        }
    }
}
