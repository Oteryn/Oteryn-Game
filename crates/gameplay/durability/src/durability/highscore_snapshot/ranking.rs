//! HIGHSCORES0 ranking compatibility for native Character identities.

pub use oteryn_gameplay_progression::ranking::{RANKING_ROWS_MAX, RankingError, ranked_lists};

pub type RankingRow<'a, Value> =
    oteryn_gameplay_progression::ranking::RankingRow<'a, Value, crate::foundation::CharacterId>;
pub type RankedRow<'a, Value> =
    oteryn_gameplay_progression::ranking::RankedRow<'a, Value, crate::foundation::CharacterId>;
