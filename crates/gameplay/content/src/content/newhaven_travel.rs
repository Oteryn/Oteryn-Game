//! The two free tutorial departures. Anna's landing is owner-selected Oteryn design;
//! the seagull's destination and double confirmation follow pinned Crystal e5519f2e.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewhavenRoute {
    Anna,
    Seagull,
}

impl NewhavenRoute {
    pub fn for_npc(npc: &str) -> Option<Self> {
        match npc {
            "oteryn:npc.anna" => Some(Self::Anna),
            "oteryn:npc.a_seagull" => Some(Self::Seagull),
            _ => None,
        }
    }
    pub const fn key(self) -> &'static str {
        match self {
            Self::Anna => "thais_peninsula",
            Self::Seagull => "rookgaard",
        }
    }
    pub const fn destination(self) -> (i32, i32, i16) {
        match self {
            Self::Anna => (32472, 32288, 7),
            Self::Seagull => (32097, 32219, 7),
        }
    }
    pub const fn min_level(self) -> u32 {
        match self {
            Self::Anna => 8,
            Self::Seagull => 0,
        }
    }
}
