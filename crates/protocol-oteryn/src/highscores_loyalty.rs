//! N014: highscores category 13 (Loyalty) under `HIGHSCORES_V1` (`highscores-loyalty-v1.md`).
//! No new command or byte layout: a session that selected this capability may name category 13
//! in `HIGHSCORES_QUERY_V1`; without it the query stays `NOT_SUPPORTED`.
pub const CAPABILITY_HIGHSCORES_LOYALTY_V1: u32 = 498;
pub const LOYALTY_CATEGORY: u32 = 13;
