// oteryn-choice: saturating local counters retain constant space, including when the window is closed.
#[derive(Debug, Clone, Default)]
pub struct Panel {
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
