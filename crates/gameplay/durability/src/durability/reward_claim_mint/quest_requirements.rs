//! Source-authored chest prerequisites checked by both MINT admission passes.
use super::*;
use sqlx::{Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical owner and initial value come from the activated Quest catalogue.
/// These are server facts, never client-supplied progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimQuestRequirement {
    pub track: String,
    pub quest: String,
    pub definition_hash: [u8; 32],
    pub initial: i64,
    pub minimum: i64,
    pub maximum: i64,
}

pub(super) fn valid(requirements: &[RewardClaimQuestRequirement]) -> bool {
    let keys: BTreeSet<_> = requirements.iter().map(|r| &r.track).collect();
    requirements.len() <= 8
        && keys.len() == requirements.len()
        && requirements.iter().all(|r| {
            valid_quest_key(&r.track) && valid_quest_key(&r.quest) && r.minimum <= r.maximum
        })
}

/// The current item fence and Character root have already been locked.
/// A failed prerequisite does not reserve a claim, mint an item or issue progress.
pub(super) async fn holds(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    requirements: &[RewardClaimQuestRequirement],
) -> std::result::Result<bool, DurabilityError> {
    if requirements.is_empty() {
        return Ok(true);
    }
    let mut owners: Vec<_> = requirements.iter().map(|r| r.quest.as_str()).collect();
    owners.sort_unstable();
    owners.dedup();
    let states = sqlx::query("SELECT quest_key,definition_hash,completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=ANY($2::text[]) ORDER BY quest_key FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&owners).fetch_all(&mut **tx).await?;
    let mut hashes = BTreeMap::new();
    for row in states {
        hashes.insert(
            row.try_get::<String, _>("quest_key")?,
            (
                row.try_get::<Vec<u8>, _>("definition_hash")?,
                row.try_get::<bool, _>("completed")?,
            ),
        );
    }
    let mut keys: Vec<_> = requirements.iter().map(|r| r.track.as_str()).collect();
    keys.sort_unstable();
    let rows = sqlx::query("SELECT track_key,quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    let mut values = BTreeMap::new();
    for row in rows {
        values.insert(
            row.try_get::<String, _>("track_key")?,
            (
                row.try_get::<String, _>("quest_key")?,
                row.try_get::<i64, _>("value")?,
            ),
        );
    }
    for requirement in requirements {
        if let Some((hash, completed)) = hashes.get(&requirement.quest)
            && !*completed
            && hash.as_slice() != requirement.definition_hash.as_slice()
        {
            return Ok(false);
        }
        let value = match values.get(&requirement.track) {
            Some((owner, value)) if owner == &requirement.quest => {
                if !hashes.contains_key(&requirement.quest) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                *value
            }
            Some(_) => return Err(DurabilityError::InvalidStoredState),
            None => requirement.initial,
        };
        if !(requirement.minimum..=requirement.maximum).contains(&value) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Appended only for guarded claims; old unguarded intent bindings are unchanged.
pub(super) fn append_intent(
    canonical: &mut Vec<u8>,
    requirements: &[RewardClaimQuestRequirement],
) -> Result<()> {
    if requirements.is_empty() {
        return Ok(());
    }
    canonical.push(0x47);
    canonical
        .push(u8::try_from(requirements.len()).map_err(|_| RewardClaimMintError::InvalidInput)?);
    for requirement in requirements {
        push_text(canonical, requirement.track.as_bytes()).map_err(from_transfer_input)?;
        push_text(canonical, requirement.quest.as_bytes()).map_err(from_transfer_input)?;
        canonical.extend_from_slice(&requirement.definition_hash);
        for value in [
            requirement.initial,
            requirement.minimum,
            requirement.maximum,
        ] {
            canonical.extend_from_slice(&value.to_be_bytes());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn requirement() -> RewardClaimQuestRequirement {
        RewardClaimQuestRequirement {
            track: "oteryn:quest-progress/test".into(),
            quest: "oteryn:quest.test".into(),
            definition_hash: [7; 32],
            initial: 0,
            minimum: 2,
            maximum: 2,
        }
    }
    #[test]
    fn guard_intents_retain_empty_legacy_bytes_and_bind_every_source_fact() -> Result<()> {
        let mut legacy = vec![1, 2, 3];
        append_intent(&mut legacy, &[])?;
        assert_eq!(legacy, [1, 2, 3]);
        let original = requirement();
        let mut expected = legacy.clone();
        append_intent(&mut expected, std::slice::from_ref(&original))?;
        for field in 0..6 {
            let mut changed = original.clone();
            match field {
                0 => changed.track.push('x'),
                1 => changed.quest.push('x'),
                2 => changed.definition_hash[0] ^= 1,
                3 => changed.initial = 1,
                4 => changed.minimum = 1,
                _ => changed.maximum = 3,
            }
            let mut actual = legacy.clone();
            append_intent(&mut actual, &[changed])?;
            assert_ne!(actual, expected);
        }
        Ok(())
    }
    #[test]
    fn duplicated_excessive_and_reversed_guards_fail_closed() {
        let good = requirement();
        assert!(valid(std::slice::from_ref(&good)));
        assert!(!valid(&[good.clone(), good.clone()]));
        assert!(!valid(&vec![good.clone(); 9]));
        assert!(!valid(&[RewardClaimQuestRequirement {
            minimum: 3,
            ..good
        }]));
    }
}
