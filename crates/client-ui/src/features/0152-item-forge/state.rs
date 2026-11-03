use oteryn_client_runtime::cui3::item_forge::Operation;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemForgePanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub operation: Operation,
    pub occurrence: [u8; 16],
    pub first_item: u64,
    pub second_item: u64,
    pub boost: bool,
    pub protect: bool,
    pub history_limit: u8,
    pub response: Vec<u8>,
    pub feedback: Option<String>,
}
impl Default for ItemForgePanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            pending: false,
            operation: Operation::QueryHistory,
            occurrence: [0; 16],
            first_item: 0,
            second_item: 0,
            boost: false,
            protect: false,
            history_limit: 20,
            response: vec![],
            feedback: None,
        }
    }
}
