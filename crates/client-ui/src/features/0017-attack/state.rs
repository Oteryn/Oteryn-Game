use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattlePanel {
    pub metadata: super::PlayerMetadataState,
    pub open: bool,
    pub pending: bool,
    pub pending_command: Option<CombatCommand>,
    pub native_running: bool,
    pub shutdown: bool,
    /// Visible actors in arrival order, including non-attackable NPCs.
    pub visible_actors: Vec<WorldSpatialEntity>,
    pub rows: Vec<WorldSpatialEntity>,
    pub combat: Option<ActorCombatState>,
    pub feedback: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Attack(EntityRef),
    Stop,
    Fight(FightMode),
    Chase(ChaseMode),
}
