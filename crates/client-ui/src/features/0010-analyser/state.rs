#[path = "state/items.rs"]
pub(super) mod items;
#[path = "state/measurements.rs"]
mod measurements;
pub use items::ItemStats;
pub use measurements::{Clock, Series, chart_values, eta_seconds, per_hour};
// Measurements continue while the window is closed; charts retain at most 60 minute buckets.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub party_import: Option<super::party_import::Summary>,
    pub boss_drops: std::collections::VecDeque<super::bosses::Drop>,
    pub boss_drops_truncated: bool,
    pub tool: Tool,
    pub party: Vec<oteryn_client_runtime::analyser::party::Member>,
    pub party_pending: bool,
    pub party_feedback: Option<&'static str>,
    pub party_revision: u64,
    pub party_pending_revision: u64,
    pub clock: Clock,
    pub xp_chart: Series,
    pub raw_xp_chart: Series,
    pub gold_chart: Series,
    pub damage_chart: Series,
    pub healing_chart: Series,
    pub received_chart: Series,
    pub input_types: std::collections::BTreeMap<u32, u64>,
    pub input_sources: std::collections::BTreeMap<(u8, u32), u64>,
    pub impact_types: std::collections::BTreeMap<u32, u64>,
    pub loot: std::collections::BTreeMap<u32, ItemStats>,
    pub loot_tracked: std::collections::BTreeSet<u32>,
    pub loot_only_tracked: bool,
    pub loot_drops: std::collections::VecDeque<super::tracking::Drop>,
    pub omitted_drops: u64,
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
    pub kills_by_race: std::collections::BTreeMap<u32, u64>,
    pub kill_goals: std::collections::BTreeMap<u32, u64>,
    pub task_progress: Option<Vec<super::kills::TaskProgress>>,
    pub task_feedback: Option<&'static str>,
    pub export_path: String,
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

/// Dedicated cyclopedia and analytics views over the same confirmed session facts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Overview,
    Hunting,
    Experience,
    Loot,
    Supplies,
    Damage,
    Healing,
    Impact,
    Input,
    Session,
    Party,
    LootTracker,
    Kills,
    Bosses,
}
impl Tool {
    pub const CYCLOPEDIA: [Self; 7] = [
        Self::Overview,
        Self::Hunting,
        Self::Experience,
        Self::Loot,
        Self::Supplies,
        Self::Damage,
        Self::Healing,
    ];
    pub const ALL: [Self; 14] = [
        Self::Overview,
        Self::Hunting,
        Self::Experience,
        Self::Loot,
        Self::Supplies,
        Self::Damage,
        Self::Healing,
        Self::Impact,
        Self::Input,
        Self::Session,
        Self::Party,
        Self::LootTracker,
        Self::Kills,
        Self::Bosses,
    ];
    #[allow(
        non_upper_case_globals,
        reason = "Preserve the analytics selector API while sharing the cyclopedia Experience tool"
    )]
    pub const Xp: Self = Self::Experience;
    #[allow(
        non_upper_case_globals,
        reason = "Preserve the analytics selector API while sharing the cyclopedia Supplies tool"
    )]
    pub const Supply: Self = Self::Supplies;
    pub const fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Hunting => "hunting",
            Self::Experience => "xp",
            Self::Loot => "loot",
            Self::Supplies => "supply",
            Self::Damage => "damage",
            Self::Healing => "healing",
            Self::Impact => "impact",
            Self::Input => "input",
            Self::Session => "session",
            Self::Party => "party",
            Self::LootTracker => "loot-tracker",
            Self::Kills => "kills",
            Self::Bosses => "bosses",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => tr!("Overview"),
            Self::Hunting => tr!("Hunting"),
            Self::Experience => tr!("Experience"),
            Self::Loot => tr!("Loot"),
            Self::Supplies => tr!("Supplies"),
            Self::Damage => tr!("Damage"),
            Self::Healing => tr!("Healing"),
            Self::Impact => tr!("Impact"),
            Self::Input => tr!("Input"),
            Self::Session => tr!("Hunting Session"),
            Self::Party => tr!("Party Hunt"),
            Self::LootTracker => tr!("Loot Tracker"),
            Self::Kills => tr!("Kill Tracker"),
            Self::Bosses => tr!("Boss Tracker"),
        }
    }
}
