//! Crystal201189f creatures_definitions.hpp:1470-1483 and monster.cpp:2341-2348.
use super::WorldSpatialError;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulpitCreatureIcon {
    Normal,
    Strong,
    Powerful,
    Boss,
}
impl SoulpitCreatureIcon {
    pub fn from_stack(stack: u8) -> Result<Self, WorldSpatialError> {
        match stack {
            1 => Ok(Self::Normal),
            5 => Ok(Self::Strong),
            15 => Ok(Self::Powerful),
            40 => Ok(Self::Boss),
            _ => Err(WorldSpatialError::Malformed),
        }
    }
    pub const fn count(self) -> u8 {
        match self {
            Self::Normal => 1,
            Self::Strong => 5,
            Self::Powerful => 15,
            Self::Boss => 0,
        }
    }
    pub const fn icon(self) -> u8 {
        if matches!(self, Self::Boss) { 7 } else { 6 }
    }
}
