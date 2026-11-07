//! Locked preparation for exact final branches already admitted by the Quest owner.
use super::npc_child_persistence::PreparedOwnerWrite;
use super::{CharacterId, DurabilityError, QUESTSTATE0_RL_05, QUESTSTATE0_RL_08_STATES};
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue};
use sqlx::Row;
use std::collections::BTreeMap;

pub(in crate::durability) struct VerifiedNoop {
    pub key: String,
    pub quest: String,
    pub value: i64,
}
#[derive(Default)]
pub(in crate::durability) struct PreparedQuestBatch {
    pub writes: Vec<PreparedOwnerWrite>,
    pub noops: Vec<VerifiedNoop>,
}
impl PreparedQuestBatch {
    pub fn noops_observation(&self) -> serde_json::Value {
        serde_json::Value::Array(self.noops.iter().map(|noop|
            serde_json::json!({"key":noop.key,"quest":noop.quest,"value":noop.value})).collect())
    }
}

/// The caller owns the current Character Root throughout preparation and commit.
/// Native supplemental operations are prepared separately and merged before arm;
/// an unmatched source write is always an error, never a dropped effect.
pub(in crate::durability) struct SourceQuestPlanInput<'a> {
    pub character: CharacterId,
    pub source_key: &'a str,
    pub source_changes: &'a [oteryn_gameplay_quests::quest::npc_cosmetics::TrackChange],
    pub catalogue: &'a QuestStateCatalogue,
    pub now: i64,
    pub claims: &'a BTreeMap<String, [u8; 16]>,
    pub bindings: &'a super::npc_endpoints::QualifiedBindings,
}
pub(in crate::durability) async fn prepare(
    tx: &mut sqlx::PgConnection,
    input: SourceQuestPlanInput<'_>,
) -> Result<Result<PreparedQuestBatch, QuestRefusal>, DurabilityError> {
    let SourceQuestPlanInput {
        character,
        source_key,
        source_changes,
        catalogue,
        now,
        claims,
        bindings,
    } = input;
    let mut keys: Vec<&str> = source_changes
        .iter()
        .map(|change| change.key.as_str())
        .collect();
    keys.sort_unstable();
    keys.dedup();
    if keys.len() != source_changes.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let rows = sqlx::query("SELECT track_key,quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR UPDATE")
        .bind(character.as_bytes().as_slice()).bind(&keys).fetch_all(&mut *tx).await?;
    let mut stored = BTreeMap::new();
    for row in rows {
        let key: String = row.try_get("track_key")?;
        let quest: String = row.try_get("quest_key")?;
        if !source_changes
            .iter()
            .any(|change| change.key == key && change.quest == quest)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        stored.insert(key, row.try_get("value")?);
    }
    let mut native_keys = std::collections::BTreeSet::new();
    let mut native_endpoints = Vec::new();
    for change in source_changes {
        if change.before == change.after && stored.get(&change.key) == Some(&change.before) {
            continue;
        }
        if super::npc_endpoints::requires_native(
            source_key,
            &change.key,
            catalogue,
            &stored,
            now,
            bindings,
        )? {
            let projection = claims.get(&change.key).map(|claim| {
                super::native_final_endpoints::FinalClaimProjection {
                    track: change.key.clone(),
                    claim_transaction_id: *claim,
                    source_value: change.before,
                }
            });
            let endpoint = super::native_final_endpoints::prepare_native_final_endpoint(
                source_key,
                change,
                &stored,
                catalogue,
                projection.as_ref(),
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            native_keys.insert(change.key.clone());
            native_endpoints.push(endpoint);
        }
    }
    let grouped = super::npc_endpoints::group_final_endpoints(
        super::npc_endpoints::evaluate_final_endpoints(
            source_key,
            source_changes,
            &stored,
            now,
            catalogue,
            &native_keys,
            bindings,
        )?,
    )?;
    let mut grouped: BTreeMap<_, _> = grouped
        .into_iter()
        .map(|child| (child.quest.clone(), child))
        .collect();
    let mut versions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut actual_claims: BTreeMap<String, BTreeMap<String, [u8; 16]>> = BTreeMap::new();
    for endpoint in native_endpoints {
        let version =
            super::native_final_endpoints::operation_version(source_key, &endpoint.change.track)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        if endpoint.source_program_hash
            != oteryn_gameplay_quests::quest::npc_cosmetics::source_program::digest()
            || (endpoint.compatible_extension.is_some()
                && !matches!(
                    version,
                    "foolish_first_addon_ack_v1" | "foolish_second_addon_ack_v1"
                ))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let child = grouped.entry(endpoint.quest.clone()).or_insert_with(|| {
            super::npc_endpoints::PreparedQuestChild {
                quest: endpoint.quest.clone(),
                definition_hash: endpoint.base_definition_hash,
                transition_keys: vec![],
                completes: false,
                changes: vec![],
                absent_initial_projections: vec![],
            }
        });
        if child.definition_hash != endpoint.base_definition_hash
            || child
                .changes
                .iter()
                .any(|change| change.track == endpoint.change.track)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if endpoint.absent_initial_projection {
            child
                .absent_initial_projections
                .push((endpoint.change.track.clone(), endpoint.change.before));
        }
        if let Some(claim) = endpoint.actual_claim_projection {
            actual_claims
                .entry(endpoint.quest.clone())
                .or_default()
                .insert(endpoint.change.track.clone(), claim);
        }
        child.changes.push(endpoint.change);
        let operations = versions.entry(endpoint.quest).or_default();
        if !operations.iter().any(|candidate| candidate == version) {
            operations.push(version.to_owned());
        }
        if child.changes.len() > 8 || child.transition_keys.len() + operations.len() > 8 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    // Reserve the entire group at once. Repeating a per-owner one-slot check
    // would incorrectly admit two new states when only one slot remains.
    let mut writes = Vec::new();
    let mut new_states = 0_usize;
    for (_, child) in grouped {
        let old = super::pins::read_locked(tx, character, &child.quest).await?;
        let pin =
            match super::pins::resolve(catalogue.content_revision(), child.definition_hash, old) {
                Ok(pin) => pin,
                Err(refusal) => return Ok(Err(refusal)),
            };
        let writes_state = child.quest != "oteryn:quest.npc_cosmetic_final_acquisition";
        if !writes_state && pin.state_exists {
            return Err(DurabilityError::InvalidStoredState);
        }
        if writes_state && !pin.state_exists {
            new_states += 1;
        }
        let source_before = child
            .changes
            .iter()
            .map(|change| {
                source_changes
                    .iter()
                    .find(|source| source.key == change.track && source.quest == child.quest)
                    .map(|source| (change.track.clone(), source.before))
                    .ok_or(DurabilityError::InvalidStoredState)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let native_operations = versions.remove(&child.quest).unwrap_or_default();
        let actual_claim_projections = actual_claims.remove(&child.quest).unwrap_or_default();
        writes.push(PreparedOwnerWrite {
            child,
            writes_state,
            pin,
            source_before,
            native_operations,
            actual_claim_projections,
        });
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    let after = usize::try_from(count)
        .unwrap_or(usize::MAX)
        .saturating_add(new_states);
    if after > QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES)
        || !super::capacity::tracks_fit(tx, character, &keys, stored.len()).await?
    {
        return Ok(Err(QuestRefusal::CapacityExceeded));
    }
    let noops = source_changes
        .iter()
        .filter(|change| {
            change.before == change.after && stored.get(&change.key) == Some(&change.before)
        })
        .map(|change| VerifiedNoop {
            key: change.key.clone(),
            quest: change.quest.clone(),
            value: change.before,
        })
        .collect();
    Ok(Ok(PreparedQuestBatch { writes, noops }))
}
