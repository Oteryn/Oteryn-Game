#[path = "state/items.rs"]
pub(super) mod items;
#[path = "state/measurements.rs"]
mod measurements;
pub use items::ItemStats;
pub use measurements::{Clock, Series, chart_values, eta_seconds, per_hour};
// Measurements continue while the window is closed; charts retain at most 60 minute buckets.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub tool: Tool,
    pub clock: Clock,
    pub xp_chart: Series,
    pub raw_xp_chart: Series,
    pub gold_chart: Series,
    pub loot: std::collections::BTreeMap<u32, ItemStats>,
    pub supply: std::collections::BTreeMap<u32, ItemStats>,
    pub supply_prices: std::collections::BTreeMap<u32, u64>,
    pub omitted_supplies: u64,
    pub item_info: std::collections::BTreeMap<u32, oteryn_client_runtime::analyser::ItemInfo>,
    pub catalogue_pending: Vec<u32>,
    pub loot_prices: std::collections::BTreeMap<u32, u64>,
    pub price_input: [String; 2],
    pub omitted_items: u64,
    pub goal_xp: Option<u64>,
    pub goal_input: String,
    pub feedback: Option<&'static str>,
    pub selected: bool,
    pub open: bool,
    pub kills: u64,
    pub raw_xp: u64,
    pub gained_xp: u64,
    pub gold: u64,
    pub loot_items: u64,
    pub supplies: u64,
    pub damage: u64,
    pub healing: u64,
    pub received: u64,
    pub dropped: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Overview,
    Xp,
    Loot,
    Supply,
    Impact,
    Input,
    Session,
    Party,
    LootTracker,
    Kills,
}
impl Tool {
    pub const ALL: [Self; 10] = [
        Self::Overview,
        Self::Xp,
        Self::Loot,
        Self::Supply,
        Self::Impact,
        Self::Input,
        Self::Session,
        Self::Party,
        Self::LootTracker,
        Self::Kills,
    ];
    pub const fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Xp => "xp",
            Self::Loot => "loot",
            Self::Supply => "supply",
            Self::Impact => "impact",
            Self::Input => "input",
            Self::Session => "session",
            Self::Party => "party",
            Self::LootTracker => "loot-tracker",
            Self::Kills => "kills",
        }
    }
}
