use oteryn_client_runtime::cui3::item_forge::Operation;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemForgePanel {
    pub preview: super::PreviewState,
    pub retry: Option<oteryn_client_runtime::cui3::item_forge::Request>,
    pub submitted: Option<oteryn_client_runtime::cui3::item_forge::Request>,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub operation: Operation,
    pub query_current_dust: bool,
    pub occurrence: [u8; 16],
    pub first_item: u64,
    pub second_item: u64,
    pub boost: bool,
    pub protect: bool,
    pub history_limit: u8,
    pub response: Vec<u8>,
    pub decoded: Option<super::reply::Reply>,
    pub feedback: Option<String>,
}
impl Default for ItemForgePanel {
    fn default() -> Self {
        Self {
            preview: Default::default(),
            retry: None,
            submitted: None,
            selected: false,
            open: false,
            pending: false,
            operation: Operation::QueryHistory,
            query_current_dust: false,
            occurrence: [0; 16],
            first_item: 0,
            second_item: 0,
            boost: false,
            protect: false,
            history_limit: 20,
            response: vec![],
            decoded: None,
            feedback: None,
        }
    }
}
