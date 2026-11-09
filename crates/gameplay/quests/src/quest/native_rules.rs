//! Closed adapters for source-specific rules inside the fenced quest writer.
//!
//! A rule consumes exactly its declared locked tracks and returns their complete
//! next snapshot. It never obtains values from a session cache or writes state.

use super::{
    QuestEffect, QuestEffectKind, QuestRefusal, barbarian_mead, cults_of_tibia, ice_islands_seals,
    unnatural_selection,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRule {
    BarbarianMead,
    CultsSymbol { symbol: u8 },
    UnnaturalDance { tile: u8 },
    IceSealPaint { seal: u16 },
    SoulWarRemorse,
    SoulWarEnergy,
}

impl NativeRule {
    pub const fn quest(self) -> &'static str {
        match self {
            Self::BarbarianMead => barbarian_mead::QUEST,
            Self::CultsSymbol { .. } => cults_of_tibia::QUEST,
            Self::UnnaturalDance { .. } => unnatural_selection::QUEST,
            Self::IceSealPaint { .. } => ice_islands_seals::QUEST,
            Self::SoulWarRemorse | Self::SoulWarEnergy => super::soul_war::source_items::QUEST,
        }
    }

    pub fn tracks(self) -> Vec<&'static str> {
        match self {
            Self::BarbarianMead => barbarian_mead::NATIVE_TRACKS.to_vec(),
            Self::CultsSymbol { .. } => vec![
                cults_of_tibia::MISSION,
                cults_of_tibia::TEMP,
                cults_of_tibia::OBJECTS,
            ],
            Self::UnnaturalDance { .. } => unnatural_selection::TRACKS.to_vec(),
            Self::SoulWarRemorse => vec![super::soul_war::source_items::REMORSE],
            Self::SoulWarEnergy => super::soul_war::source_items::ENERGY_TRACKS.to_vec(),
            Self::IceSealPaint { .. } => ice_islands_seals::TRACKS.to_vec(),
        }
    }

    /// Stable definition-hash encoding. Tags are append-only and never reused.
    pub fn encode(self, out: &mut Vec<u8>) {
        match self {
            Self::BarbarianMead => out.push(3),
            Self::CultsSymbol { symbol } => out.extend_from_slice(&[1, symbol]),
            Self::UnnaturalDance { tile } => out.extend_from_slice(&[2, tile]),
            Self::IceSealPaint { seal } => {
                out.push(5);
                out.extend_from_slice(&seal.to_be_bytes());
            }
            Self::SoulWarRemorse => out.push(6),
            Self::SoulWarEnergy => out.push(4),
        }
    }

    const fn valid_id(self) -> bool {
        match self {
            Self::BarbarianMead => true,
            Self::CultsSymbol { symbol } => symbol < cults_of_tibia::SYMBOL_COUNT,
            Self::UnnaturalDance { tile } => tile >= 1 && tile <= 9,
            Self::IceSealPaint { seal } => seal < ice_islands_seals::targets::TARGET_COUNT,
            Self::SoulWarRemorse | Self::SoulWarEnergy => true,
        }
    }

    /// Only one closed rule may own all declared effects of this transition.
    pub fn valid_effects(self, quest: &str, effects: &[QuestEffect]) -> bool {
        self.valid_id()
            && quest == self.quest()
            && effects.len() == self.tracks().len()
            && effects
                .iter()
                .all(|effect| effect.effect == QuestEffectKind::Native(self))
            && self.tracks().iter().all(|track| {
                effects
                    .iter()
                    .filter(|effect| effect.track == **track)
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
        self.evaluate_with_roll(values, None)
    }

    pub fn evaluate_with_roll(
        self,
        values: &BTreeMap<String, i64>,
        roll: Option<u8>,
    ) -> Result<BTreeMap<String, i64>, QuestRefusal> {
        let tracks = self.tracks();
        if !self.valid_id() || values.len() != tracks.len() {
            return Err(QuestRefusal::NotSupported);
        }
        let read = |key: &str| values.get(key).copied().ok_or(QuestRefusal::NotSupported);
        let next = match self {
            Self::BarbarianMead => {
                let before = [
                    read(barbarian_mead::TRACKS[0])?,
                    read(barbarian_mead::TRACKS[1])?,
                    read(barbarian_mead::TRACKS[2])?,
                    read(barbarian_mead::TRACKS[3])?,
                ];
                let random = barbarian_mead::needs_draw(before[0], before[3]);
                let roll = if random {
                    roll.ok_or(QuestRefusal::NotSupported)?
                } else {
                    1
                };
                let mut after = barbarian_mead::apply(before, roll)?.to_vec();
                after.push(if random { i64::from(roll) } else { 0 });
                after
            }
            Self::CultsSymbol { symbol } => {
                let snapshot = cults_of_tibia::apply_symbol(
                    symbol,
                    cults_of_tibia::Snapshot {
                        mission: read(cults_of_tibia::MISSION)?,
                        temp: read(cults_of_tibia::TEMP)?,
                        objects: read(cults_of_tibia::OBJECTS)?,
                    },
                )?;
                vec![snapshot.mission, snapshot.temp, snapshot.objects]
            }
            Self::IceSealPaint { seal } => {
                let snapshot = ice_islands_seals::paint(
                    seal,
                    ice_islands_seals::Snapshot {
                        questline: read(ice_islands_seals::QUESTLINE)?,
                        targets: read(ice_islands_seals::TARGETS)?,
                        mission04: read(ice_islands_seals::MISSION04)?,
                    },
                )?;
                vec![snapshot.questline, snapshot.targets, snapshot.mission04]
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
                vec![
                    snapshot.dance_status,
                    snapshot.mission03,
                    snapshot.questline,
                ]
            }
            Self::SoulWarRemorse => {
                use super::soul_war::source_items::{Kind, REMORSE, SourceItemState};
                let before = u8::try_from(read(REMORSE)?).map_err(|_| QuestRefusal::OutOfRange)?;
                let plan = SourceItemState {
                    remorse: before,
                    ..Default::default()
                }
                .apply(Kind::Remorse, 0, true)
                .ok_or(QuestRefusal::OutOfRange)?;
                vec![i64::from(plan.after.remorse)]
            }
            Self::SoulWarEnergy => {
                use super::soul_war::source_items::{ENERGY_TRACKS, Kind, SourceItemState};
                let mut floors = 0;
                for (index, track) in ENERGY_TRACKS.iter().skip(1).enumerate() {
                    match read(track)? {
                        0 => {}
                        1 => floors |= 1 << index,
                        _ => return Err(QuestRefusal::OutOfRange),
                    }
                }
                let plan = SourceItemState {
                    energy: read(ENERGY_TRACKS[0])?,
                    floors,
                    ..Default::default()
                }
                .apply(Kind::Energy, 0, true)
                .ok_or(QuestRefusal::OutOfRange)?;
                vec![
                    plan.after.energy,
                    i64::from(plan.after.floors & 1 != 0),
                    i64::from(plan.after.floors & 2 != 0),
                    i64::from(plan.after.floors & 4 != 0),
                ]
            }
        };
        Ok(tracks
            .iter()
            .copied()
            .zip(next)
            .map(|(track, value)| (track.to_owned(), value))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::QuestComparison;

    fn values(rule: NativeRule, values: [i64; 3]) -> BTreeMap<String, i64> {
        rule.tracks()
            .iter()
            .copied()
            .zip(values)
            .map(|(track, value)| (track.to_owned(), value))
            .collect()
    }

    fn effects(rule: NativeRule) -> Vec<QuestEffect> {
        rule.tracks()
            .iter()
            .copied()
            .map(|track| QuestEffect {
                track: track.to_owned(),
                from: QuestComparison::Between(0, i64::MAX),
                effect: QuestEffectKind::Native(rule),
            })
            .collect()
    }

    #[test]
    fn native_adapter_applies_complete_source_snapshots() {
        let cults = NativeRule::CultsSymbol { symbol: 12 };
        let before = values(cults, [3, 0, 0]);
        assert_eq!(cults.evaluate(&before), Ok(values(cults, [3, 4096, 1])));
        assert_eq!(before, values(cults, [3, 0, 0]));

        let dance = NativeRule::UnnaturalDance { tile: 2 };
        assert_eq!(
            dance.evaluate(&values(dance, [16, 0, 15])),
            Ok(values(dance, [17, 3, 7]))
        );
    }

    #[test]
    fn native_adapter_refuses_partial_extra_and_foreign_snapshots() {
        let rule = NativeRule::CultsSymbol { symbol: 0 };
        let mut partial = values(rule, [3, 0, 0]);
        partial.remove(cults_of_tibia::TEMP);
        assert_eq!(rule.evaluate(&partial), Err(QuestRefusal::NotSupported));

        let mut extra = values(rule, [3, 0, 0]);
        extra.insert("oteryn:quest-progress/unrelated".into(), 0);
        assert_eq!(rule.evaluate(&extra), Err(QuestRefusal::NotSupported));

        let foreign = values(NativeRule::UnnaturalDance { tile: 5 }, [1, 2, 6]);
        assert_eq!(rule.evaluate(&foreign), Err(QuestRefusal::NotSupported));
    }

    #[test]
    fn native_effects_require_exact_quest_keys_and_one_rule() {
        let rule = NativeRule::CultsSymbol { symbol: 0 };
        let expected = effects(rule);
        assert!(rule.valid_effects(cults_of_tibia::QUEST, &expected));
        assert!(!rule.valid_effects(unnatural_selection::QUEST, &expected));
        assert!(!rule.valid_effects(cults_of_tibia::QUEST, &expected[..2]));

        let mut duplicate = expected.clone();
        duplicate[2].track = duplicate[0].track.clone();
        assert!(!rule.valid_effects(cults_of_tibia::QUEST, &duplicate));

        let mut mixed = expected.clone();
        mixed[1].effect = QuestEffectKind::Native(NativeRule::CultsSymbol { symbol: 1 });
        assert!(!rule.valid_effects(cults_of_tibia::QUEST, &mixed));

        let mut scalar = expected;
        scalar[2].effect = QuestEffectKind::Add(0);
        assert!(!rule.valid_effects(cults_of_tibia::QUEST, &scalar));
    }

    #[test]
    fn native_invalid_identifiers_cannot_pass_the_adapter() {
        for rule in [
            NativeRule::CultsSymbol { symbol: 13 },
            NativeRule::CultsSymbol { symbol: u8::MAX },
            NativeRule::UnnaturalDance { tile: 0 },
            NativeRule::UnnaturalDance { tile: 10 },
            NativeRule::IceSealPaint {
                seal: ice_islands_seals::targets::TARGET_COUNT,
            },
            NativeRule::IceSealPaint { seal: u16::MAX },
        ] {
            assert!(!rule.valid_effects(rule.quest(), &effects(rule)));
            assert_eq!(
                rule.evaluate(&values(rule, [0, 0, 0])),
                Err(QuestRefusal::NotSupported)
            );
        }
    }

    #[test]
    fn native_definition_encoding_keeps_rule_and_identifier_separate() {
        let mut encoded = vec![99];
        NativeRule::CultsSymbol { symbol: 0 }.encode(&mut encoded);
        NativeRule::CultsSymbol { symbol: 12 }.encode(&mut encoded);
        NativeRule::UnnaturalDance { tile: 1 }.encode(&mut encoded);
        NativeRule::UnnaturalDance { tile: 9 }.encode(&mut encoded);
        assert_eq!(encoded, [99, 1, 0, 1, 12, 2, 1, 2, 9]);
    }

    #[test]
    fn native_json_rejects_unknown_kinds_and_fields() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::from_str::<NativeRule>(r#"{"kind":"cults_symbol","symbol":12}"#)?,
            NativeRule::CultsSymbol { symbol: 12 }
        );
        assert_eq!(
            serde_json::from_str::<NativeRule>(r#"{"kind":"unnatural_dance","tile":9}"#)?,
            NativeRule::UnnaturalDance { tile: 9 }
        );
        for text in [
            r#"{"kind":"arbitrary_script","symbol":0}"#,
            r#"{"kind":"cults_symbol","symbol":0,"tile":1}"#,
            r#"{"kind":"unnatural_dance","tile":1,"extra":true}"#,
            r#"{"kind":"cults_symbol","symbol":256}"#,
        ] {
            assert!(serde_json::from_str::<NativeRule>(text).is_err());
        }
        Ok(())
    }
    #[test]
    fn srx3_source_items_native_remorse_rule_uses_exact_locked_track_set() -> Result<(), String> {
        let rule = NativeRule::SoulWarRemorse;
        let values =
            BTreeMap::from([(super::super::soul_war::source_items::REMORSE.to_owned(), 1)]);
        let after = rule.evaluate(&values).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            after.get(super::super::soul_war::source_items::REMORSE),
            Some(&0)
        );
        let mut extra = values;
        extra.insert("oteryn:unrelated".into(), 0);
        assert_eq!(rule.evaluate(&extra), Err(QuestRefusal::NotSupported));
        Ok(())
    }
    #[test]
    fn srx3_source_items_native_energy_rule_preserves_completed_item_counter() -> Result<(), String>
    {
        let rule = NativeRule::SoulWarEnergy;
        let tracks = super::super::soul_war::source_items::ENERGY_TRACKS;
        let values = tracks
            .into_iter()
            .zip([69, 0, 0, 0])
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let after = rule.evaluate(&values).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            tracks.map(|k| after.get(k).copied()),
            [Some(0), Some(1), Some(1), Some(1)]
        );
        let values = tracks
            .into_iter()
            .zip([1000, 0, 0, 1])
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let after = rule.evaluate(&values).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            tracks.map(|k| after.get(k).copied()),
            [Some(1001), Some(0), Some(0), Some(1)]
        );
        Ok(())
    }
}
