//! Free tutorial departures. Anna's landing is owner-selected Oteryn design;
//! the seagull's destination and double confirmation follow pinned Crystal e5519f2e.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewhavenRoute {
    Anna,
    AnnaTarguna,
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
    /// Exact normalized requests; Anna's owner-selected default remains Thais.
    pub fn offer(self, text: &str) -> Option<Self> {
        match (self, text) {
            (Self::Anna, "passage") => Some(Self::Anna),
            (Self::Anna, "targuna" | "passage to targuna") => Some(Self::AnnaTarguna),
            (Self::Seagull, "rookgaard") => Some(Self::Seagull),
            _ => None,
        }
    }

    pub const fn home_town(self) -> Option<u32> {
        match self {
            Self::AnnaTarguna => Some(34),
            Self::Anna | Self::Seagull => None,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Anna => "thais_peninsula",
            Self::AnnaTarguna => "targuna",
            Self::Seagull => "rookgaard",
        }
    }
    pub const fn destination(self) -> (i32, i32, i16) {
        match self {
            Self::Anna => (32472, 32288, 7),
            Self::AnnaTarguna => (31938, 31928, 7),
            Self::Seagull => (32097, 32219, 7),
        }
    }
    pub const fn min_level(self) -> u32 {
        match self {
            Self::Anna | Self::AnnaTarguna => 8,
            Self::Seagull => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NewhavenRoute;

    #[test]
    fn m38_anna_targuna_is_explicit_and_preserves_the_thais_default() {
        assert_eq!(
            NewhavenRoute::for_npc("oteryn:npc.anna"),
            Some(NewhavenRoute::Anna)
        );
        assert_eq!(
            NewhavenRoute::Anna.offer("passage"),
            Some(NewhavenRoute::Anna)
        );
        for request in ["targuna", "passage to targuna"] {
            assert_eq!(
                NewhavenRoute::Anna.offer(request),
                Some(NewhavenRoute::AnnaTarguna)
            );
            assert_eq!(NewhavenRoute::Seagull.offer(request), None);
        }
        assert_eq!(NewhavenRoute::AnnaTarguna.key(), "targuna");
        assert_eq!(NewhavenRoute::AnnaTarguna.destination(), (31938, 31928, 7));
        assert_eq!(NewhavenRoute::AnnaTarguna.home_town(), Some(34));
        assert_eq!(NewhavenRoute::AnnaTarguna.min_level(), 8);
        assert_eq!(NewhavenRoute::Anna.home_town(), None);
        assert_eq!(
            NewhavenRoute::Seagull.offer("rookgaard"),
            Some(NewhavenRoute::Seagull)
        );
        assert_eq!(NewhavenRoute::for_npc("oteryn:npc.other"), None);
    }
}
