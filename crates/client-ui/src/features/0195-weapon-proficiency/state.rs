#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponProficiencyPanel {
    pub selected: bool,
    pub state_selected: bool,
    pub shaping: Option<oteryn_client_runtime::weapon_proficiency_state::State>,
    pub state_refresh_needed: bool,
    pub mutation_uncertain: bool,
    pub catalogue_offset: u32,
    pub catalogue: Option<oteryn_client_runtime::weapon_proficiency::ProficiencyCataloguePage>,
    pub open: bool,
    pub pending: bool,
    pub command_kind: u32,
    pub weapon: u32,
    pub level: u8,
    pub perk: u8,
    pub clear_perk: bool,
    pub slot: u8,
    pub revision: u64,
    pub shaping_choice: u8,
    pub result: Option<String>,
    pub page: Option<oteryn_client_runtime::weapon_proficiency::ProficiencyPage>,
    pub requested_offset: u32,
    pub feedback: Option<&'static str>,
}

impl WeaponProficiencyPanel {
    pub fn invalidate_observation(&mut self) {
        if self.pending && matches!(self.command_kind, 379 | 380 | 392..=395) {
            self.mutation_uncertain = true;
        }
        self.pending = false;
        self.shaping = None;
        self.revision = 0;
        self.state_refresh_needed = self.state_selected && self.weapon != 0;
    }
    pub fn select_weapon(&mut self, row: usize) -> bool {
        if self.pending {
            return false;
        }
        let Some(entry) = self
            .catalogue
            .as_ref()
            .and_then(|page| page.entries.get(row))
        else {
            return false;
        };
        if entry.definition == 0 {
            self.feedback = Some(tr!("This weapon is not admitted by the server"));
            return false;
        }
        self.weapon = entry.definition;
        self.level = 0;
        self.perk = entry
            .levels
            .first()
            .and_then(|level| level.selected)
            .unwrap_or(0);
        self.slot = 1;
        self.shaping_choice = 0;
        self.shaping = None;
        self.revision = 0;
        self.state_refresh_needed = self.state_selected;
        self.feedback = None;
        true
    }
    pub fn weapon_entry(
        &self,
    ) -> Option<&oteryn_client_runtime::weapon_proficiency::ProficiencyCatalogueEntry> {
        self.catalogue
            .as_ref()?
            .entries
            .iter()
            .find(|entry| entry.definition == self.weapon)
    }
    pub fn choose_level(&mut self, level: usize) -> bool {
        if self.pending {
            return false;
        }
        let Some(entry) = self
            .weapon_entry()
            .and_then(|entry| entry.levels.get(level))
        else {
            return false;
        };
        let perk = entry.selected.unwrap_or(0);
        self.level = level as u8;
        self.perk = perk;
        true
    }
}
