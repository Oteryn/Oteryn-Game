use oteryn_client_runtime::cui3::item_view::ItemMoveOutcome;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotkeyEquipPanel {
    pub items: Vec<u32>,
    pub bindings: [Option<std::num::NonZeroU32>; super::ITEM_HOTKEY_SLOTS],
    pub selected: bool,
    pub smart: bool,
    pub smart_active: bool,
    pub dirty: bool,
    pub edit: u64,
    pub open: bool,
    pub pending: bool,
    pub item_definition: u32,
    pub outcome: Option<ItemMoveOutcome>,
    pub feedback: Option<String>,
}
