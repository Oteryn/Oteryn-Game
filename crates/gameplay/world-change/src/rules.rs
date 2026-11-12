//! Bounded reset rules. Native schedules delegate to the existing daily owner.
use crate::daily::{self, DailyWorldChanges};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_DEFINITIONS: usize = 256;
pub const MAX_CATALOGUE_BYTES: usize = 131_072;
pub const EMBEDDED: &str =
    include_str!("../../../../content/encounters/world-changes/runtime.json");

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub key: String,
    pub schedule: Schedule,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<Counter>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    NativeDaily {
        change: NativeChange,
    },
    AtReset {
        chance_permille: u16,
    },
    Calendar {
        months: Vec<u8>,
        days: Vec<u8>,
    },
    Rotation {
        period_epochs: u16,
        active_epoch: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeChange {
    FuryGates,
    Grimvale,
    TheirMastersVoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    pub creature: String,
    pub kills: u32,
    pub initial_active: bool,
    pub active_on_goal: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    changes: Vec<Definition>,
}

#[derive(Clone, Debug)]
pub struct Catalogue {
    document: Document,
    encoded: Vec<u8>,
    digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleError {
    Bounds,
    Definition,
    Schedule,
    NativeBinding,
    Encoding,
}

pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 100
        && key
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

impl Definition {
    pub fn validate(&self) -> Result<(), RuleError> {
        if !valid_key(&self.key)
            || self.sources.is_empty()
            || self.sources.len() > 16
            || self
                .sources
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || self.counter.as_ref().is_some_and(|c| {
                !c.creature.starts_with("oteryn:creature.")
                    || c.creature.len() > 256
                    || c.creature.chars().any(char::is_control)
                    || c.kills == 0
            })
        {
            return Err(RuleError::Definition);
        }
        match &self.schedule {
            Schedule::AtReset { chance_permille } if *chance_permille > 1000 => {
                Err(RuleError::Schedule)
            }
            Schedule::Calendar { months, days }
                if (months.is_empty() && days.is_empty())
                    || months.len() > 12
                    || days.len() > 31
                    || months.iter().any(|m| !(1..=12).contains(m))
                    || days.iter().any(|d| !(1..=31).contains(d))
                    || months.iter().collect::<BTreeSet<_>>().len() != months.len()
                    || days.iter().collect::<BTreeSet<_>>().len() != days.len() =>
            {
                Err(RuleError::Schedule)
            }
            Schedule::Rotation {
                period_epochs,
                active_epoch,
            } if *period_epochs == 0 || *period_epochs > 366 || active_epoch >= period_epochs => {
                Err(RuleError::Schedule)
            }
            _ => Ok(()),
        }
    }

    pub fn selected(
        &self,
        world: &[u8; 16],
        epoch: u64,
        month: u8,
        day: u8,
        native: &DailyWorldChanges,
    ) -> bool {
        if self.validate().is_err()
            || epoch == 0
            || !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || native.reset_epoch != epoch
        {
            return false;
        }
        match &self.schedule {
            Schedule::NativeDaily { change } => match change {
                NativeChange::FuryGates => native.fury_gates.is_some(),
                NativeChange::Grimvale => native.grimvale_full_moon,
                NativeChange::TheirMastersVoice => native.their_masters_voice,
            },
            Schedule::AtReset { chance_permille } => {
                let mut purpose = String::with_capacity(77);
                purpose.push_str("world-change/");
                for byte in Sha256::digest(self.key.as_bytes()).iter().copied() {
                    purpose.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
                    purpose.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
                }
                (u128::from(daily::draw_for_epoch(world, epoch, &purpose)) * 1000) >> 64
                    < u128::from(*chance_permille)
            }
            Schedule::Calendar { months, days } => {
                (months.is_empty() || months.contains(&month))
                    && (days.is_empty() || days.contains(&day))
            }
            Schedule::Rotation {
                period_epochs,
                active_epoch,
            } => epoch % u64::from(*period_epochs) == u64::from(*active_epoch),
        }
    }
}

impl Catalogue {
    pub fn from_definitions(changes: Vec<Definition>) -> Result<Self, RuleError> {
        let bytes = serde_json::to_vec(&Document { schema: 1, changes })
            .map_err(|_| RuleError::Encoding)?;
        Self::decode(&bytes)
    }
    pub fn embedded() -> Result<Self, RuleError> {
        Self::decode(EMBEDDED.as_bytes())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, RuleError> {
        if bytes.len() > MAX_CATALOGUE_BYTES {
            return Err(RuleError::Bounds);
        }
        let mut document: Document =
            serde_json::from_slice(bytes).map_err(|_| RuleError::Encoding)?;
        if document.schema != 1 || document.changes.len() > MAX_DEFINITIONS {
            return Err(RuleError::Bounds);
        }
        document.changes.sort_by(|a, b| a.key.cmp(&b.key));
        let mut keys = BTreeSet::new();
        for definition in &document.changes {
            definition.validate()?;
            if !keys.insert(&definition.key) {
                return Err(RuleError::Definition);
            }
        }
        for (key, change) in [
            ("fury_gates", NativeChange::FuryGates),
            ("grimvale", NativeChange::Grimvale),
            ("their_masters_voice", NativeChange::TheirMastersVoice),
        ] {
            if !document.changes.iter().any(|d| {
                d.key == key
                    && d.schedule == (Schedule::NativeDaily { change })
                    && d.counter.is_none()
            }) {
                return Err(RuleError::NativeBinding);
            }
        }
        let encoded = serde_json::to_vec(&document).map_err(|_| RuleError::Encoding)?;
        if encoded.len() > MAX_CATALOGUE_BYTES {
            return Err(RuleError::Bounds);
        }
        let digest = Sha256::digest(&encoded).into();
        Ok(Self {
            document,
            encoded,
            digest,
        })
    }
    pub fn definitions(&self) -> &[Definition] {
        &self.document.changes
    }
    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_order_does_not_change_persisted_identity() -> Result<(), String> {
        let original = Catalogue::embedded().map_err(|error| format!("{error:?}"))?;
        let mut reordered = original.definitions().to_vec();
        reordered.reverse();
        let restored =
            Catalogue::from_definitions(reordered).map_err(|error| format!("{error:?}"))?;
        assert_eq!(original.encoded(), restored.encoded());
        assert_eq!(original.digest(), restored.digest());
        let decoded =
            Catalogue::decode(restored.encoded()).map_err(|error| format!("{error:?}"))?;
        assert_eq!(decoded.digest(), original.digest());
        Ok(())
    }

    #[test]
    fn catalogue_rejects_duplicate_and_rebound_native_rules() -> Result<(), String> {
        let original = Catalogue::embedded().map_err(|error| format!("{error:?}"))?;
        let mut duplicate = original.definitions().to_vec();
        duplicate.extend_from_slice(original.definitions());
        assert_eq!(
            Catalogue::from_definitions(duplicate).err(),
            Some(RuleError::Definition)
        );
        for key in ["fury_gates", "grimvale", "their_masters_voice"] {
            let mut modified = original.definitions().to_vec();
            for definition in &mut modified {
                if definition.key == key {
                    definition.schedule = Schedule::AtReset {
                        chance_permille: 1000,
                    };
                }
            }
            assert_eq!(
                Catalogue::from_definitions(modified).err(),
                Some(RuleError::NativeBinding),
                "accepted rebound native rule {key}"
            );
            let missing = original
                .definitions()
                .iter()
                .filter(|definition| definition.key != key)
                .cloned()
                .collect();
            assert_eq!(
                Catalogue::from_definitions(missing).err(),
                Some(RuleError::NativeBinding),
                "accepted missing native rule {key}"
            );
        }
        Ok(())
    }
}
