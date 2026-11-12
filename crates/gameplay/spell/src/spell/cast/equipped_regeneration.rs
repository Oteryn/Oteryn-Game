use super::PlayerSpellState;
use oteryn_gameplay_progression::regeneration::{
    EquipmentRegeneration, EquipmentRegenerationClock,
};

impl PlayerSpellState {
    /// Replace the worn regeneration conditions with a fresh committed equipment read.
    /// A slot whose params are unchanged keeps its counters (the item stayed worn); every
    /// other entry starts at 0 (Crystal creates a new condition on equip).
    pub fn install_equipped_regeneration(&mut self, worn: Vec<(u8, EquipmentRegeneration)>) {
        self.equipped_regeneration = worn
            .into_iter()
            .map(|(slot, regeneration)| {
                match self
                    .equipped_regeneration
                    .iter()
                    .find(|clock| clock.slot == slot && clock.regeneration == regeneration)
                    .copied()
                {
                    Some(clock) => clock,
                    None => EquipmentRegenerationClock {
                        slot,
                        regeneration,
                        health_ms: 0,
                        mana_ms: 0,
                    },
                }
            })
            .collect();
    }

    #[must_use]
    pub fn equipped_regeneration(&self) -> &[EquipmentRegenerationClock] {
        &self.equipped_regeneration
    }
}
