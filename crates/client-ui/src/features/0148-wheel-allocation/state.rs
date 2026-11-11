use super::*;

// Presentation bounds from rulesets/progression/wheel-of-destiny/wheel.json topology.
// Server allocation validation remains authoritative for budget, adjacency and eligibility.
pub const SLOT_CAPACITIES: [u16; 36] = [
    200, 150, 100, 100, 150, 200, 150, 100, 75, 75, 100, 150, 100, 75, 50, 50, 75, 100, 100, 75,
    50, 50, 75, 100, 150, 100, 75, 75, 100, 150, 200, 150, 100, 100, 150, 200,
];
/// Wire order is green, red, purple, blue.
pub const SLOT_DOMAINS: [u8; 36] = [
    0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 3, 3, 3, 2, 2, 2, 3, 3, 3, 2, 2, 2, 3, 3,
    3, 2, 2, 2,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelPanel {
    pub gems: super::gems::Presentation,
    pub diagram: super::diagram::Presentation,
    pub presets_open: bool,
    pub preset_fields: [String; 2],
    pub preset_book: super::presets::PresetBook,
    pub preset_feedback: String,
    pub selected: bool,
    /// Capability 483 is negotiated: the Atelier speaks commands 6210-6214 and reports twelve
    /// Gem Vessel slots.
    pub atelier_v2: bool,
    /// The Gem Vessel facts of the last confirmed v2 reply.
    pub vessel_board: Option<crate::gem_atelier::VesselBoard>,
    pub open: bool,
    pub synchronized: bool,
    pub confirmed_wheel: Option<wheel::WheelAllocationReply>,
    pub confirmed_atelier: Option<atelier::AtelierReply>,
    pub confirmed_resources: Option<atelier::resources::ResourceBalances>,
    pub points: [u16; 36],
    pub vessels: wheel::WheelVessels,
    pub slot: usize,
    pub gem: Option<[u8; 16]>,
    pub modifier: Option<(u8, u16)>,
    /// Selected Gem Vessel slot (0-based). Gems have no domain; any gem of a vessel's quality fits.
    pub vessel_slot: u8,
    pub quality: u8,
    /// Mod Reshaping context. `None` until the server reports it, which keeps Reshape disabled.
    pub in_temple: Option<bool>,
    pub premium: Option<bool>,
    pub pending: Option<WheelCommand>,
    /// Exact logical occurrence and expected revision survive an indeterminate response.
    pub retry: Option<WheelCommand>,
    pub feedback: Option<&'static str>,
}

impl Default for WheelPanel {
    fn default() -> Self {
        Self {
            gems: Default::default(),
            diagram: Default::default(),
            presets_open: false,
            preset_fields: Default::default(),
            preset_book: Default::default(),
            preset_feedback: String::new(),
            selected: false,
            atelier_v2: false,
            vessel_board: None,
            open: false,
            synchronized: false,
            confirmed_wheel: None,
            confirmed_atelier: None,
            confirmed_resources: None,
            points: [0; 36],
            vessels: [None; 4],
            slot: 14,
            gem: None,
            modifier: None,
            vessel_slot: 0,
            quality: 0,
            in_temple: None,
            premium: None,
            pending: None,
            retry: None,
            feedback: None,
        }
    }
}

impl WheelPanel {
    pub fn ready(&self) -> bool {
        self.selected
            && self.synchronized
            && self.confirmed_wheel.is_some()
            && self.confirmed_atelier.is_some()
            && self.pending.is_none()
            && self.retry.is_none()
    }

    /// Both native Atelier views use the same admission and current-selection checks.
    pub fn can_reshape(&self) -> bool {
        self.ready()
            && self.atelier_v2
            && !self.dirty()
            && self.gems.pending.is_none()
            && self.diagram.pending.is_none()
            && self.gems.review.is_none()
            && self
                .selected_gem()
                .is_some_and(|gem| super::reshape_block(self, gem).is_none())
    }

    pub fn selected_gem(&self) -> Option<&atelier::GemView> {
        self.confirmed_atelier
            .as_ref()?
            .gems
            .iter()
            .find(|gem| Some(gem.id) == self.gem)
    }

    pub fn modifiers(&self) -> Vec<(u8, u16)> {
        let mut modifiers: Vec<_> =
            self.confirmed_atelier
                .as_ref()
                .map_or_else(Vec::new, |state| {
                    state
                        .grades
                        .iter()
                        .map(|grade| (grade.kind, grade.mod_id))
                        .collect()
                });
        // Current queries include every compatible Workshop modifier. Older servers
        // send sparse grades, so the selected gem still supplies its known choices.
        if let Some(gem) = self.selected_gem() {
            modifiers.extend(
                gem.modifiers
                    .iter()
                    .enumerate()
                    .map(|(index, id)| (u8::from(index == 2), *id)),
            );
        }
        modifiers.sort_unstable();
        modifiers.dedup();
        modifiers
    }

    pub fn grade(&self, kind: u8, mod_id: u16) -> u8 {
        self.confirmed_atelier
            .as_ref()
            .and_then(|state| {
                state
                    .grades
                    .iter()
                    .find(|grade| grade.kind == kind && grade.mod_id == mod_id)
            })
            .map_or(0, |grade| grade.grade)
    }

    /// Selectable Gem Vessel slots: twelve with capability 483, else the four draft vessels.
    pub fn vessel_slot_count(&self) -> u8 {
        if self.atelier_v2 { 12 } else { 4 }
    }

    /// The wire still reports a vessel's slot index in the gem's legacy `domain` field. With
    /// capability 483 vessels are changed by Replace Vessel, never drafted, so there is no draft
    /// to compare.
    pub fn confirmed_vessels(&self) -> Option<wheel::WheelVessels> {
        if self.atelier_v2 {
            return Some(self.vessels);
        }
        let mut vessels = [None; 4];
        for gem in &self.confirmed_atelier.as_ref()?.gems {
            if gem.vessel {
                let slot = vessels.get_mut(usize::from(gem.domain))?;
                if slot.replace(gem.id).is_some() {
                    return None;
                }
            }
        }
        Some(vessels)
    }

    pub fn dirty(&self) -> bool {
        self.confirmed_wheel
            .as_ref()
            .is_some_and(|state| self.points != state.points)
            || self
                .confirmed_vessels()
                .is_some_and(|vessels| vessels != self.vessels)
    }

    pub fn set_points(&mut self, slot: usize, points: u16) -> bool {
        if !self.ready()
            || SLOT_CAPACITIES
                .get(slot)
                .is_none_or(|maximum| points > *maximum)
        {
            self.feedback = Some(tr!("Enter points within the selected slot's limit"));
            return false;
        }
        self.points[slot] = points;
        self.feedback = None;
        true
    }

    pub fn socket_selected(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        let Some(gem) = self.selected_gem() else {
            return false;
        };
        let id = gem.id;
        // Replace Vessel: the chosen slot takes the gem, whatever it held before.
        let slot = usize::from(self.vessel_slot);
        if slot >= self.vessels.len() {
            return false;
        }
        for vessel in &mut self.vessels {
            if *vessel == Some(id) {
                *vessel = None;
            }
        }
        self.vessels[slot] = Some(id);
        self.feedback = Some(tr!("Vessel change is pending; press Save Wheel"));
        true
    }

    pub fn clear_vessel(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        let Some(vessel) = self.vessels.get_mut(usize::from(self.vessel_slot)) else {
            return false;
        };
        *vessel = None;
        self.feedback = Some(tr!("Vessel change is pending; press Save Wheel"));
        true
    }

    pub fn select_gem(&mut self, gem: Option<[u8; 16]>) {
        self.gem = gem;
        let modifiers = self.modifiers();
        if !self.modifier.is_some_and(|id| modifiers.contains(&id)) {
            self.modifier = modifiers.first().copied();
        }
    }
}
