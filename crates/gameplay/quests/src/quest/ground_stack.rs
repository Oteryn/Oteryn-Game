//! Exact ordered predicates over a complete mutable Ground Item snapshot.
//!
//! These observations are read data, not custody authority. The participant must
//! obtain the complete tile stack from the Ground owner under its current fence
//! and keep that readset protected until the dependent operation commits. Map
//! appearances and carried Items cannot stand in for Ground Item instances.
use super::world_callbacks::Position;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_STACK_ITEMS: usize = 64;

pub mod alternatives;
mod dense_pattern;

/// One required mutable stack entry, in bottom-to-top append order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackItemRequirement {
    pub item: String,
    pub revision: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundStackRequirement {
    pub position: Position,
    /// The entire mutable stack; an empty list explicitly requires absence.
    pub items: Vec<StackItemRequirement>,
}

/// A real instance in Ground custody. Equal definitions remain distinct entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroundStackEntry {
    pub instance: [u8; 16],
    pub item: String,
    pub revision: String,
    pub quantity: u32,
    pub ordinal: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundStackSnapshot {
    pub position: Position,
    pub entries: Vec<GroundStackEntry>,
}

/// The exact readset to bind into the dependent participant; never a write permit.
#[derive(Debug, PartialEq, Eq)]
pub struct GroundStackWitness<'a> {
    pub position: Position,
    pub entries: &'a [GroundStackEntry],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundStackError {
    InvalidRequirement,
    InvalidObservation,
    LimitExceeded,
}

fn valid_item(item: &str, revision: &str, quantity: u32) -> bool {
    item.strip_prefix("oteryn:item.")
        .is_some_and(|suffix| !suffix.is_empty())
        && super::valid_quest_key(item)
        && !revision.is_empty()
        && revision.len() <= 128
        && revision.bytes().all(|byte| byte.is_ascii_graphic())
        && quantity > 0
}

impl GroundStackRequirement {
    pub fn validate(&self) -> Result<(), GroundStackError> {
        if self.items.len() > MAX_STACK_ITEMS {
            return Err(GroundStackError::LimitExceeded);
        }
        if self
            .items
            .iter()
            .any(|item| !valid_item(&item.item, &item.revision, item.quantity))
        {
            return Err(GroundStackError::InvalidRequirement);
        }
        Ok(())
    }
}

/// Compare the entire mutable stack, without sorting, aggregating equal
/// definitions, or substituting an appearance for an Item instance. Positive
/// append ordinals may have gaps after pickup/removal, but may not tie or reverse.
pub fn match_stack<'a>(
    requirement: &GroundStackRequirement,
    snapshot: &'a GroundStackSnapshot,
) -> Result<Option<GroundStackWitness<'a>>, GroundStackError> {
    requirement.validate()?;
    if snapshot.entries.len() > MAX_STACK_ITEMS {
        return Err(GroundStackError::LimitExceeded);
    }
    let mut identities = BTreeSet::new();
    let mut previous = 0;
    for entry in &snapshot.entries {
        if entry.ordinal <= previous
            || !identities.insert(entry.instance)
            || !valid_item(&entry.item, &entry.revision, entry.quantity)
        {
            return Err(GroundStackError::InvalidObservation);
        }
        previous = entry.ordinal;
    }
    if requirement.position != snapshot.position
        || requirement.items.len() != snapshot.entries.len()
        || !requirement
            .items
            .iter()
            .zip(&snapshot.entries)
            .all(|(expected, actual)| {
                expected.item == actual.item
                    && expected.revision == actual.revision
                    && expected.quantity == actual.quantity
            })
    {
        return Ok(None);
    }
    Ok(Some(GroundStackWitness {
        position: snapshot.position,
        entries: &snapshot.entries,
    }))
}

/// Match a bounded multi-tile readset while preserving each Ground owner's
/// ordered witness. The caller must protect all observations through commit.
/// Instance identities cannot be reused across tiles. Larger readsets are limited
/// to a complete6x6 pattern with one required/observed Item per cell.
pub fn match_readset<'a>(
    requirements: &[GroundStackRequirement],
    snapshots: &'a [GroundStackSnapshot],
) -> Result<Option<Vec<GroundStackWitness<'a>>>, GroundStackError> {
    const MAX_TILES: usize = 8;
    if requirements.is_empty()
        || ((requirements.len() > MAX_TILES || snapshots.len() > MAX_TILES)
            && !dense_pattern::admitted(requirements, snapshots))
    {
        return Err(GroundStackError::LimitExceeded);
    }
    let mut positions = BTreeSet::new();
    for requirement in requirements {
        requirement.validate()?;
        if !positions.insert(requirement.position) {
            return Err(GroundStackError::InvalidRequirement);
        }
    }
    let mut observed_positions = BTreeSet::new();
    let mut instances = BTreeSet::new();
    for snapshot in snapshots {
        if snapshot.entries.len() > MAX_STACK_ITEMS {
            return Err(GroundStackError::LimitExceeded);
        }
        if !observed_positions.insert(snapshot.position)
            || snapshot
                .entries
                .iter()
                .any(|entry| !instances.insert(entry.instance))
        {
            return Err(GroundStackError::InvalidObservation);
        }
    }
    if positions != observed_positions {
        return Ok(None);
    }
    let mut witnesses = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        let Some(snapshot) = snapshots
            .iter()
            .find(|s| s.position == requirement.position)
        else {
            return Ok(None);
        };
        let Some(witness) = match_stack(requirement, snapshot)? else {
            return Ok(None);
        };
        witnesses.push(witness);
    }
    Ok(Some(witnesses))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (GroundStackRequirement, GroundStackSnapshot) {
        let position = Position {
            x: 1,
            y: 2,
            floor: -1,
        };
        let items = ["fixture_crate", "fixture_crate", "fixture_amphora"]
            .map(|key| StackItemRequirement {
                item: format!("oteryn:item.{key}"),
                revision: "definition-r1".into(),
                quantity: 1,
            })
            .to_vec();
        let entries = items
            .iter()
            .enumerate()
            .map(|(index, item)| GroundStackEntry {
                instance: [u8::try_from(index + 1).unwrap_or(255); 16],
                item: item.item.clone(),
                revision: item.revision.clone(),
                quantity: item.quantity,
                // Append ordinals are positive and ordered, not necessarily contiguous.
                ordinal: u64::try_from(index + 1).unwrap_or(255) * 11,
            })
            .collect();
        (
            GroundStackRequirement { position, items },
            GroundStackSnapshot { position, entries },
        )
    }

    #[test]
    fn m49_multi_altar_readset_requires_every_tile_and_unique_custody()
    -> Result<(), GroundStackError> {
        let (first, snapshot) = fixture();
        let mut second = first.clone();
        second.position.x += 1;
        let mut other = snapshot.clone();
        other.position = second.position;
        for entry in &mut other.entries {
            entry.instance[0] += 10;
        }
        let required = [first, second];
        assert!(match_readset(&required, std::slice::from_ref(&snapshot))?.is_none());
        assert!(match_readset(&required, &[other.clone(), snapshot.clone()])?.is_some());
        other.entries[0].instance = snapshot.entries[0].instance;
        assert_eq!(
            match_readset(&required, &[snapshot.clone(), other]),
            Err(GroundStackError::InvalidObservation)
        );
        assert_eq!(
            match_readset(&[required[0].clone(), required[0].clone()], &[snapshot]),
            Err(GroundStackError::InvalidRequirement)
        );
        Ok(())
    }

    #[test]
    fn ordered_ground_stack_keeps_custody_identity_quantity_and_order()
    -> Result<(), GroundStackError> {
        let (requirement, snapshot) = fixture();
        let witness =
            match_stack(&requirement, &snapshot)?.ok_or(GroundStackError::InvalidObservation)?;
        assert_eq!(witness.position, snapshot.position);
        assert_eq!(witness.entries, snapshot.entries);
        let mut wrong = snapshot.clone();
        wrong.entries.swap(0, 2);
        // Reordering instances and assigning fresh append ordinals is a valid
        // different stack, not evidence for the old required arrangement.
        for (index, entry) in wrong.entries.iter_mut().enumerate() {
            entry.ordinal = u64::try_from(index + 1).unwrap_or(255);
        }
        assert!(match_stack(&requirement, &wrong)?.is_none());
        let mut wrong = snapshot.clone();
        wrong.entries[0].quantity = 2;
        assert!(match_stack(&requirement, &wrong)?.is_none());
        let mut wrong = snapshot.clone();
        wrong.entries[0].revision = "definition-r2".into();
        assert!(match_stack(&requirement, &wrong)?.is_none());
        let mut wrong = snapshot.clone();
        wrong.position.x += 1;
        assert!(match_stack(&requirement, &wrong)?.is_none());
        let mut wrong = snapshot.clone();
        wrong.entries.pop();
        assert!(match_stack(&requirement, &wrong)?.is_none());
        Ok(())
    }

    #[test]
    fn ordered_ground_stack_rejects_corrupt_or_oversized_readsets() {
        let (requirement, snapshot) = fixture();
        for corruption in 0..5 {
            let mut bad = snapshot.clone();
            match corruption {
                0 => bad.entries[0].ordinal = 0,
                1 => bad.entries[1].ordinal = bad.entries[0].ordinal,
                2 => bad.entries[1].instance = bad.entries[0].instance,
                3 => bad.entries[0].quantity = 0,
                _ => bad.entries.swap(0, 1),
            }
            assert_eq!(
                match_stack(&requirement, &bad),
                Err(GroundStackError::InvalidObservation)
            );
        }
        let mut large = snapshot.clone();
        large.entries = vec![snapshot.entries[0].clone(); MAX_STACK_ITEMS + 1];
        assert_eq!(
            match_stack(&requirement, &large),
            Err(GroundStackError::LimitExceeded)
        );
        let mut large = requirement.clone();
        large.items = vec![requirement.items[0].clone(); MAX_STACK_ITEMS + 1];
        assert_eq!(
            match_stack(&large, &snapshot),
            Err(GroundStackError::LimitExceeded)
        );
        let mut invalid = requirement;
        invalid.items[0].quantity = 0;
        assert_eq!(
            match_stack(&invalid, &snapshot),
            Err(GroundStackError::InvalidRequirement)
        );
    }

    #[test]
    fn ordered_ground_stack_does_not_aggregate_or_ignore_extra_instances()
    -> Result<(), GroundStackError> {
        let (requirement, snapshot) = fixture();
        let mut merged = snapshot.clone();
        merged.entries.remove(1);
        merged.entries[0].quantity = 2;
        assert!(match_stack(&requirement, &merged)?.is_none());
        let mut extra = snapshot.clone();
        let mut entry = snapshot.entries[0].clone();
        entry.instance = [4; 16];
        entry.ordinal = 44;
        extra.entries.push(entry);
        assert!(match_stack(&requirement, &extra)?.is_none());
        let empty = GroundStackRequirement {
            position: snapshot.position,
            items: Vec::new(),
        };
        assert!(match_stack(&empty, &snapshot)?.is_none());
        let empty_snapshot = GroundStackSnapshot {
            position: snapshot.position,
            entries: Vec::new(),
        };
        assert!(match_stack(&empty, &empty_snapshot)?.is_some());
        let invalid =
            serde_json::json!({"position":snapshot.position,"items":[],"appearance":1739});
        assert!(serde_json::from_value::<GroundStackRequirement>(invalid).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod dense_pattern_tests;
