use super::*;
use atelier::v2;

/// Gem Vessel facts of one v2 reply, kept next to the domain-free gem list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VesselBoard {
    /// All twelve slots in slot order; empty when the server reports none.
    pub slots: Vec<v2::VesselSlotView>,
    /// Wheel slots (1..=36) of the vocation's Gem Vessel nodes.
    pub nodes: Vec<u8>,
    /// Flat damage and healing each fully allocated Gem Vessel node grants.
    pub node_bonus: u32,
    /// The socketed gems' resistance against an element exceeds the 30% cap.
    pub resistance_capped: bool,
    /// The Wheel revision Replace Vessel must expect.
    pub wheel_revision: u64,
    pub in_temple: bool,
    pub premium: bool,
}

/// A v2 reply in the shape the Wheel panel already renders (a gem's vessel slot is carried in
/// its legacy `domain` field) plus the Gem Vessel facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub atelier: atelier::AtelierReply,
    pub board: VesselBoard,
}

#[must_use]
pub fn project(reply: &v2::AtelierReply) -> Projection {
    Projection {
        atelier: atelier::AtelierReply {
            refused: reply.refused,
            revision: reply.revision,
            gems: reply
                .gems
                .iter()
                .map(|gem| atelier::GemView {
                    id: gem.id,
                    quality: gem.quality,
                    domain: gem.slot.unwrap_or(0),
                    locked: gem.locked,
                    vessel: gem.slot.is_some(),
                    modifiers: gem.modifiers.clone(),
                })
                .collect(),
            grades: reply.grades.clone(),
            quality_quotes: reply
                .quality_quotes
                .iter()
                .map(|quote| atelier::QualityQuote {
                    quality: quote.quality,
                    reveal_gold: quote.reveal_gold,
                    switch_gold: 0,
                    fragment_kind: quote.fragment_kind,
                    fragment_min: quote.fragment_min,
                    fragment_max: quote.fragment_max,
                })
                .collect(),
            grade_quotes: reply.grade_quotes.clone(),
        },
        board: VesselBoard {
            slots: reply.vessels.clone(),
            nodes: reply.vessel_nodes.clone(),
            node_bonus: reply.vessel_node_bonus,
            resistance_capped: reply.resistance_capped,
            wheel_revision: reply.wheel_revision,
            in_temple: reply.in_temple,
            premium: reply.premium,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceBlock {
    NoVessels,
    SlotLocked,
    QualityMismatch,
    AlreadyInSlot,
}

impl ReplaceBlock {
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoVessels => "The server reports no Gem Vessels for this character",
            Self::SlotLocked => "This Gem Vessel is not unlocked yet",
            Self::QualityMismatch => "The gem's quality must match the vessel's quality",
            Self::AlreadyInSlot => "The gem already sits in this vessel",
        }
    }
}

/// Why `gem` cannot be placed into vessel `slot`, or `None` when Replace Vessel may be sent:
/// the vessel must be open and hold a gem of exactly the vessel's quality.
#[must_use]
pub fn replace_block(
    board: &VesselBoard,
    gem: &atelier::GemView,
    slot: u8,
) -> Option<ReplaceBlock> {
    let Some(vessel) = board.slots.get(usize::from(slot)) else {
        return Some(ReplaceBlock::NoVessels);
    };
    if !vessel.open {
        return Some(ReplaceBlock::SlotLocked);
    }
    if vessel.quality != gem.quality {
        return Some(ReplaceBlock::QualityMismatch);
    }
    (vessel.gem == Some(gem.id)).then_some(ReplaceBlock::AlreadyInSlot)
}
