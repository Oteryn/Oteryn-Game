//! Closed adapters for source-specific rules inside the fenced quest writer.
//!
//! A rule consumes exactly its three locked tracks and returns their complete
//! next snapshot. It never obtains values from a session cache or writes state.

use super::{QuestEffect, QuestEffectKind, QuestRefusal, cults_of_tibia, unnatural_selection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRule {
    CultsSymbol { symbol: u8 },
    UnnaturalDance { tile: u8 },
}

impl NativeRule {
    pub const fn quest(self) -> &'static str {
        match self {
            Self::CultsSymbol { .. } => cults_of_tibia::QUEST,
            Self::UnnaturalDance { .. } => unnatural_selection::QUEST,
        }
    }

    pub const fn tracks(self) -> [&'static str; 3] {
        match self {
            Self::CultsSymbol { .. } => [
                cults_of_tibia::MISSION,
                cults_of_tibia::TEMP,
                cults_of_tibia::OBJECTS,
            ],
            Self::UnnaturalDance { .. } => unnatural_selection::TRACKS,
        }
    }

    /// Stable definition-hash encoding. Tags are append-only and never reused.
    pub fn encode(self, out: &mut Vec<u8>) {
        match self {
            Self::CultsSymbol { symbol } => out.extend_from_slice(&[1, symbol]),
            Self::UnnaturalDance { tile } => out.extend_from_slice(&[2, tile]),
        }
    }

    const fn valid_id(self) -> bool {
        match self {
            Self::CultsSymbol { symbol } => symbol < cults_of_tibia::SYMBOL_COUNT,
            Self::UnnaturalDance { tile } => tile >= 1 && tile <= 9,
        }
    }

    /// Only one closed rule may own all three effects of this transition.
    pub fn valid_effects(self, quest: &str, effects: &[QuestEffect]) -> bool {
        self.valid_id()
            && quest == self.quest()
            && effects.len() == 3
            && effects
                .iter()
                .all(|effect| effect.effect == QuestEffectKind::Native(self))
            && self.tracks().iter().all(|track| {
                effects
                    .iter()
                    .filter(|effect| effect.track == *track)
                    .count()
                    == 1
            })
    }

    /// Evaluate once from a complete locked snapshot; missing and extra tracks
    /// are refused, including tracks of the same quest outside this rule.
    pub fn evaluate(
        self,
        values: &BTreeMap<String, i64>,
    ) -> Result<BTreeMap<String, i64>, QuestRefusal> {
        let tracks = self.tracks();
        if !self.valid_id() || values.len() != tracks.len() {
            return Err(QuestRefusal::NotSupported);
        }
        let read = |key: &str| values.get(key).copied().ok_or(QuestRefusal::NotSupported);
        let next = match self {
            Self::CultsSymbol { symbol } => {
                let snapshot = cults_of_tibia::apply_symbol(
                    symbol,
                    cults_of_tibia::Snapshot {
                        mission: read(cults_of_tibia::MISSION)?,
                        temp: read(cults_of_tibia::TEMP)?,
                        objects: read(cults_of_tibia::OBJECTS)?,
                    },
                )?;
                [snapshot.mission, snapshot.temp, snapshot.objects]
            }
            Self::UnnaturalDance { tile } => {
                let snapshot = unnatural_selection::apply_dance(
                    tile,
                    unnatural_selection::Snapshot {
                        dance_status: read(unnatural_selection::DANCE_STATUS_TRACK)?,
                        mission03: read(unnatural_selection::MISSION03_TRACK)?,
                        questline: read(unnatural_selection::QUESTLINE_TRACK)?,
                    },
                )?;
                [
                    snapshot.dance_status,
                    snapshot.mission03,
                    snapshot.questline,
                ]
            }
        };
        Ok(tracks
            .into_iter()
            .zip(next)
            .map(|(track, value)| (track.to_owned(), value))
            .collect())
    }
}
