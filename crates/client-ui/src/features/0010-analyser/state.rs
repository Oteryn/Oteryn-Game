// oteryn-choice: saturating local counters retain constant space, including when the window is closed.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub tool: Tool,
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

/// Separate views over the same confirmed session facts.
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
}
impl Tool {
    pub const ALL: [Self; 7] = [
        Self::Overview,
        Self::Hunting,
        Self::Experience,
        Self::Loot,
        Self::Supplies,
        Self::Damage,
        Self::Healing,
    ];
    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Hunting => "Hunting",
            Self::Experience => "Experience",
            Self::Loot => "Loot",
            Self::Supplies => "Supplies",
            Self::Damage => "Damage",
            Self::Healing => "Healing",
        }
    }
}
