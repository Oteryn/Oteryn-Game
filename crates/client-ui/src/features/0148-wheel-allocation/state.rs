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
    pub selected: bool,
    pub open: bool,
    pub synchronized: bool,
    pub confirmed_wheel: Option<wheel::WheelAllocationReply>,
    pub confirmed_atelier: Option<atelier::AtelierReply>,
    pub points: [u16; 36],
    pub vessels: wheel::WheelVessels,
    pub slot: usize,
    pub gem: Option<[u8; 16]>,
    pub modifier: Option<(u8, u16)>,
    pub domain: u8,
    pub quality: u8,
    pub pending: Option<WheelCommand>,
    /// Exact logical occurrence and expected revision survive an indeterminate response.
    pub retry: Option<WheelCommand>,
    pub feedback: Option<&'static str>,
}

impl Default for WheelPanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            synchronized: false,
            confirmed_wheel: None,
            confirmed_atelier: None,
            points: [0; 36],
            vessels: [None; 4],
            slot: 14,
            gem: None,
            modifier: None,
            domain: 0,
            quality: 0,
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

    pub fn selected_gem(&self) -> Option<&atelier::GemView> {
        self.confirmed_atelier
            .as_ref()?
            .gems
            .iter()
            .find(|gem| Some(gem.id) == self.gem)
    }

    pub fn modifiers(&self) -> Vec<(u8, u16)> {
        self.selected_gem().map_or_else(Vec::new, |gem| {
            gem.modifiers
                .iter()
                .enumerate()
                .map(|(index, id)| (u8::from(index == 2), *id))
                .collect()
        })
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

    pub fn confirmed_vessels(&self) -> Option<wheel::WheelVessels> {
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
            self.feedback = Some("Enter points within the selected slot's limit");
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
        let (domain, id) = (usize::from(gem.domain), gem.id);
        let Some(vessel) = self.vessels.get_mut(domain) else {
            return false;
        };
        *vessel = Some(id);
        self.feedback = Some("Vessel change is pending; press Save Wheel");
        true
    }

    pub fn clear_vessel(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        let Some(vessel) = self.vessels.get_mut(usize::from(self.domain)) else {
            return false;
        };
        *vessel = None;
        self.feedback = Some("Vessel change is pending; press Save Wheel");
        true
    }

    pub fn select_gem(&mut self, gem: Option<[u8; 16]>) {
        self.gem = gem;
        self.modifier = self.modifiers().first().copied();
    }
}
