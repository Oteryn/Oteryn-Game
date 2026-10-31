//! Gem Atelier and Fragment Workshop state transitions.
//!
//! Numeric policy uses the pinned catalogue, with explicit executable Crystal corrections.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{wheel::WheelDomain, wheel_gem_data::WheelGemData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GemQuality {
    Lesser,
    Regular,
    Greater,
}

impl GemQuality {
    /// Crystal 201189f player_wheel.cpp:1304-1316. The historical data-only
    /// official catalogue bounds Regular at five; the selected runtime donor permits ten.
    pub const fn revealed_fragment_yield(self) -> (u32, u32) {
        match self {
            Self::Lesser | Self::Greater => (1, 5),
            Self::Regular => (2, 10),
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::Lesser => "lesser",
            Self::Regular => "regular",
            Self::Greater => "greater",
        }
    }
    const fn mod_count(self) -> usize {
        match self {
            Self::Lesser => 1,
            Self::Regular => 2,
            Self::Greater => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FragmentKind {
    Basic,
    Supreme,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gem {
    #[serde(with = "gem_identity")]
    pub id: u128,
    pub quality: GemQuality,
    pub domain: WheelDomain,
    pub mods: Vec<u16>,
    pub locked: bool,
    pub vessel: bool,
    pub ruleset_revision: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtelierState {
    pub revision: u64,
    /// Character vocation supplied by the owner, never by the reveal draw.
    pub vocation: String,
    pub gold: u64,
    pub unrevealed: BTreeMap<GemQuality, u32>,
    pub fragments: BTreeMap<FragmentKind, u32>,
    pub gems: BTreeMap<u128, Gem>,
    /// Catalogue grade 0 is Grade I and 3 is Grade IV.
    #[serde(with = "grade_entries")]
    pub grades: BTreeMap<(FragmentKind, u16), u8>,
    /// Grade identities and values may only be read under the revision that wrote them.
    pub grade_ruleset_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtelierAction {
    Drop {
        quality: GemQuality,
    },
    Reveal {
        id: u128,
        quality: GemQuality,
        domain: WheelDomain,
        mods: Vec<u16>,
    },
    Socket {
        id: u128,
    },
    Remove {
        id: u128,
    },
    SwitchDomain {
        id: u128,
    },
    Lock {
        id: u128,
        locked: bool,
    },
    Dismantle {
        id: u128,
        yield_amount: u32,
    },
    GradeUp {
        kind: FragmentKind,
        mod_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtelierRefusal {
    StaleRevision,
    InvalidDraw,
    NoGem,
    Full,
    Locked,
    InVessel,
    LastDomainGem,
    AlreadySocketed,
    VesselOccupied,
    NotSocketed,
    InsufficientGold,
    InsufficientFragments,
    MaxGrade,
    RulesetMigrationPending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierCommit {
    pub revision: u64,
    pub promotion_points: u32,
}

impl AtelierState {
    /// Applies one atomic owner transition. Persistence callers store the returned state and
    /// revision together; a refusal leaves `self` byte-for-byte unchanged.
    pub fn commit(
        &mut self,
        expected_revision: u64,
        action: AtelierAction,
    ) -> Result<AtelierCommit, AtelierRefusal> {
        if self.revision != expected_revision {
            return Err(AtelierRefusal::StaleRevision);
        }
        let data = WheelGemData::embedded().map_err(|_| AtelierRefusal::InvalidDraw)?;
        let catalogue = &data.gems()["catalogue"];
        let current = data.revision();
        let vocation =
            crate::wheel::base_vocation(&self.vocation).ok_or(AtelierRefusal::InvalidDraw)?;
        let family = &data.gems()["vocations"][vocation];
        let mut next = self.clone();
        match action {
            AtelierAction::Drop { quality } => {
                let count = next.unrevealed.entry(quality).or_default();
                *count = count.checked_add(1).ok_or(AtelierRefusal::InvalidDraw)?;
            }
            AtelierAction::Reveal {
                id,
                quality,
                domain,
                mods,
            } => {
                let limit = catalogue["atelier"]["operation_policy"]["revealed_gem_limit"]
                    .as_u64()
                    .ok_or(AtelierRefusal::InvalidDraw)? as usize;
                if next.gems.len() >= limit {
                    return Err(AtelierRefusal::Full);
                }
                let count = next
                    .unrevealed
                    .get_mut(&quality)
                    .ok_or(AtelierRefusal::NoGem)?;
                if *count == 0 {
                    return Err(AtelierRefusal::NoGem);
                }
                if mods.len() != quality.mod_count() || next.gems.contains_key(&id) {
                    return Err(AtelierRefusal::InvalidDraw);
                }
                // Basic and supreme ids overlap. Membership in their union would allow a
                // supreme-only id in a basic slot, and ignores vocation/position restrictions.
                let lists = [
                    "basic_mods_position_1",
                    "basic_mods_position_2",
                    "supreme_mods",
                ];
                if mods
                    .iter()
                    .zip(lists)
                    .any(|(id, list)| !contains_mod(family, list, *id))
                    || (mods.len() >= 2 && mods[0] == mods[1])
                {
                    return Err(AtelierRefusal::InvalidDraw);
                }
                let fee = catalogue["atelier"]["fees"]["reveal"][quality.key()]
                    .as_u64()
                    .ok_or(AtelierRefusal::InvalidDraw)?;
                next.gold = next
                    .gold
                    .checked_sub(fee)
                    .ok_or(AtelierRefusal::InsufficientGold)?;
                *count -= 1;
                next.gems.insert(
                    id,
                    Gem {
                        id,
                        quality,
                        domain,
                        mods,
                        locked: false,
                        vessel: false,
                        ruleset_revision: current.to_owned(),
                    },
                );
            }
            AtelierAction::Socket { id } => {
                let gem = current_gem(&mut next, id, current)?;
                if gem.vessel {
                    return Err(AtelierRefusal::AlreadySocketed);
                }
                let domain = gem.domain;
                if next
                    .gems
                    .values()
                    .any(|other| other.vessel && other.domain == domain)
                {
                    return Err(AtelierRefusal::VesselOccupied);
                }
                current_gem(&mut next, id, current)?.vessel = true;
            }
            AtelierAction::Remove { id } => {
                let gem = current_gem(&mut next, id, current)?;
                if !gem.vessel {
                    return Err(AtelierRefusal::NotSocketed);
                }
                gem.vessel = false;
            }
            AtelierAction::SwitchDomain { id } => {
                let gem = current_gem(&mut next, id, current)?;
                if gem.locked {
                    return Err(AtelierRefusal::Locked);
                }
                if gem.vessel {
                    return Err(AtelierRefusal::InVessel);
                }
                let quality = gem.quality;
                let domains = [
                    WheelDomain::Green,
                    WheelDomain::Red,
                    WheelDomain::Purple,
                    WheelDomain::Blue,
                ];
                let index = domains
                    .iter()
                    .position(|domain| *domain == gem.domain)
                    .ok_or(AtelierRefusal::InvalidDraw)?;
                gem.domain = domains[(index + 1) % domains.len()];
                let fee = catalogue["atelier"]["fees"]["switch_domain"][quality.key()]
                    .as_u64()
                    .ok_or(AtelierRefusal::InvalidDraw)?;
                next.gold = next
                    .gold
                    .checked_sub(fee)
                    .ok_or(AtelierRefusal::InsufficientGold)?;
            }
            AtelierAction::Lock { id, locked } => {
                current_gem(&mut next, id, current)?.locked = locked
            }
            AtelierAction::Dismantle { id, yield_amount } => {
                let gem = current_gem(&mut next, id, current)?;
                if gem.locked {
                    return Err(AtelierRefusal::Locked);
                }
                if gem.vessel {
                    return Err(AtelierRefusal::InVessel);
                }
                let quality = gem.quality;
                let domain = gem.domain;
                if next
                    .gems
                    .values()
                    .filter(|other| other.domain == domain)
                    .count()
                    <= 1
                {
                    return Err(AtelierRefusal::LastDomainGem);
                }
                let row = &catalogue["atelier"]["fragment_yields"][quality.key()];
                let (low, high) = quality.revealed_fragment_yield();
                if !(low..=high).contains(&yield_amount) {
                    return Err(AtelierRefusal::InvalidDraw);
                }
                let kind = if row["fragment"] == "basic" {
                    FragmentKind::Basic
                } else {
                    FragmentKind::Supreme
                };
                next.gems.remove(&id);
                let held = next.fragments.entry(kind).or_default();
                *held = held
                    .checked_add(yield_amount)
                    .ok_or(AtelierRefusal::InvalidDraw)?;
            }
            AtelierAction::GradeUp { kind, mod_id } => {
                if !next.grades.is_empty()
                    && next.grade_ruleset_revision.as_deref() != Some(current)
                {
                    return Err(AtelierRefusal::RulesetMigrationPending);
                }
                let known = match kind {
                    FragmentKind::Basic => {
                        contains_mod(family, "basic_mods_position_1", mod_id)
                            || contains_mod(family, "basic_mods_position_2", mod_id)
                    }
                    FragmentKind::Supreme => contains_mod(family, "supreme_mods", mod_id),
                };
                if !known {
                    return Err(AtelierRefusal::InvalidDraw);
                }
                let grade = next.grades.get(&(kind, mod_id)).copied().unwrap_or(0);
                if grade >= 3 {
                    return Err(AtelierRefusal::MaxGrade);
                }
                let row = &catalogue["grade_costs"][usize::from(grade)];
                let class = if kind == FragmentKind::Basic {
                    "basic"
                } else {
                    "supreme"
                };
                let gold = row[class]["gold"]
                    .as_u64()
                    .ok_or(AtelierRefusal::InvalidDraw)?;
                let fragments = row[class]["fragments"]
                    .as_u64()
                    .ok_or(AtelierRefusal::InvalidDraw)? as u32;
                next.gold = next
                    .gold
                    .checked_sub(gold)
                    .ok_or(AtelierRefusal::InsufficientGold)?;
                let held = next.fragments.entry(kind).or_default();
                *held = held
                    .checked_sub(fragments)
                    .ok_or(AtelierRefusal::InsufficientFragments)?;
                next.grades.insert((kind, mod_id), grade + 1);
                next.grade_ruleset_revision = Some(current.to_owned());
            }
        }
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(AtelierRefusal::InvalidDraw)?;
        let promotion_points = if next.grade_ruleset_revision.as_deref() == Some(current) {
            next.grades.values().filter(|grade| **grade == 3).count() as u32
        } else {
            0
        };
        *self = next;
        Ok(AtelierCommit {
            revision: self.revision,
            promotion_points,
        })
    }
}

fn contains_mod(family: &serde_json::Value, list: &str, id: u16) -> bool {
    family[list].as_array().is_some_and(|ids| {
        ids.iter()
            .any(|entry| entry.as_u64() == Some(u64::from(id)))
    })
}

// UUIDv7-sized identities exceed JSON's native u64 number range. The gem map already uses
// string keys; encode the identity inside each gem as a string too.
mod gem_identity {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub(super) fn serialize<S: Serializer>(id: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(id)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<u128, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

// JSON objects cannot have tuple keys. Keep the in-memory typed key, with a deterministic
// sequence on disk; reject duplicates instead of silently losing a paid grade on reload.
mod grade_entries {
    use super::FragmentKind;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
    use std::collections::BTreeMap;

    pub(super) fn serialize<S: Serializer>(
        grades: &BTreeMap<(FragmentKind, u16), u8>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        grades.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<(FragmentKind, u16), u8>, D::Error> {
        let entries = Vec::<((FragmentKind, u16), u8)>::deserialize(deserializer)?;
        let mut grades = BTreeMap::new();
        for (key, grade) in entries {
            if grade > 3 || grades.insert(key, grade).is_some() {
                return Err(D::Error::custom("invalid or duplicate atelier grade"));
            }
        }
        Ok(grades)
    }
}

fn current_gem<'state>(
    state: &'state mut AtelierState,
    id: u128,
    revision: &str,
) -> Result<&'state mut Gem, AtelierRefusal> {
    let gem = state.gems.get_mut(&id).ok_or(AtelierRefusal::NoGem)?;
    if gem.ruleset_revision != revision {
        return Err(AtelierRefusal::RulesetMigrationPending);
    }
    Ok(gem)
}

#[cfg(test)]
mod crystal_yield_tests {
    use super::*;

    #[test]
    fn crystal_revealed_fragment_bounds_are_inclusive_and_refusals_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let data = WheelGemData::embedded()?;
        for (quality, low, high, kind) in [
            (GemQuality::Lesser, 1, 5, FragmentKind::Basic),
            (GemQuality::Regular, 2, 10, FragmentKind::Basic),
            (GemQuality::Greater, 1, 5, FragmentKind::Supreme),
        ] {
            assert_eq!(quality.revealed_fragment_yield(), (low, high));
            let state = AtelierState {
                vocation: "knight".into(),
                gems: [1, 2]
                    .into_iter()
                    .map(|id| {
                        (
                            id,
                            Gem {
                                id,
                                quality,
                                domain: WheelDomain::Green,
                                mods: vec![31, 0, 5]
                                    .into_iter()
                                    .take(quality.mod_count())
                                    .collect(),
                                locked: false,
                                vessel: false,
                                ruleset_revision: data.revision().into(),
                            },
                        )
                    })
                    .collect(),
                ..Default::default()
            };
            for yield_amount in [low, high] {
                let mut next = state.clone();
                next.commit(
                    0,
                    AtelierAction::Dismantle {
                        id: 1,
                        yield_amount,
                    },
                )
                .map_err(|error| format!("valid Crystal yield refused: {error:?}"))?;
                assert_eq!(next.fragments.get(&kind), Some(&yield_amount));
                assert!(!next.gems.contains_key(&1));
                assert!(next.gems.contains_key(&2));
                assert_eq!(next.revision, 1);
            }
            for yield_amount in [low - 1, high + 1] {
                let mut next = state.clone();
                assert_eq!(
                    next.commit(
                        0,
                        AtelierAction::Dismantle {
                            id: 1,
                            yield_amount
                        }
                    ),
                    Err(AtelierRefusal::InvalidDraw)
                );
                assert_eq!(next, state);
            }
        }
        Ok(())
    }
}
