//! RANK-1 (HS-WIRE-1): the gameplay transport's command type 14 handler is
//! [`crate::durability::highscore_snapshot::reply`]; this module binds the category 10 point
//! owner to the activated Achievement catalogue.

pub(crate) use crate::durability::highscore_snapshot::reply::{
    HighscoresReply, answer_session_highscores,
};

#[cfg(test)]
#[path = "highscores_tests.rs"]
mod tests;
