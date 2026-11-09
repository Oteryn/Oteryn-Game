use oteryn_client_runtime::spell::ActionBar;
#[derive(Debug, Clone, Default)]
pub struct ActionBarPanel {
    pub observed_spells: std::collections::BTreeSet<u32>,
    pub open: bool,
    pub bar: ActionBar,
    pub feedback: String,
    pub cooldown_received_at: Option<std::time::Instant>,
}
