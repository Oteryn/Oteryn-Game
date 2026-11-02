use oteryn_client_runtime::cui3::item_view::ItemMoveOutcome;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotkeyEquipPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub item_definition: u32,
    pub outcome: Option<ItemMoveOutcome>,
    pub feedback: Option<String>,
}
