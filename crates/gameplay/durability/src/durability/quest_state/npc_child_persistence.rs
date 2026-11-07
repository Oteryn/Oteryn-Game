//! Same-transaction persistence of a fully evaluated grouped Source-NPC plan.
//! The private arm independently re-proves the sealed plan before INSERT.
use super::DurabilityError;
use super::{npc_endpoints::PreparedQuestChild, pins::PreparedQuestPin};
use crate::durability::npc_cosmetics::NpcProgramContext;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(in crate::durability) struct PreparedOwnerWrite {
    pub child: PreparedQuestChild,
    pub writes_state: bool,
    pub(super) pin: PreparedQuestPin,
    /// Exact source-before values retained separately from native initial values.
    pub source_before: BTreeMap<String, i64>,
    pub native_operations: Vec<String>,
    pub actual_claim_projections: BTreeMap<String, [u8; 16]>,
}
impl PreparedOwnerWrite {
    pub fn observation(&self) -> Result<Value, DurabilityError> {
        let changes = self
            .child
            .changes
            .iter()
            .map(|change| {
                let before = self
                    .source_before
                    .get(&change.track)
                    .ok_or(DurabilityError::InvalidStoredState)?;
                let mut proof = json!({"key":change.track,"quest":self.child.quest,
                "before":change.before,"source_before":before,"after":change.after});
                if let Some(claim) = self.actual_claim_projections.get(&change.track) {
                    proof["claim_transaction_id"] =
                        json!(sqlx::types::Uuid::from_bytes(*claim).to_string());
                }
                Ok(proof)
            })
            .collect::<Result<Vec<_>, DurabilityError>>()?;
        if self.source_before.len() != changes.len()
            || self.child.definition_hash != self.pin.evaluation_definition_hash
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(
            json!({"quest":self.child.quest,"writes_state":self.writes_state,
            "pinned_content_revision":self.pin.pinned_content_revision,
            "base_definition_hash":hex(&self.pin.pinned_definition_hash),
            "evaluation_definition_hash":hex(&self.pin.evaluation_definition_hash),
            "transition_keys":self.child.transition_keys,
            "native_operations":self.native_operations,
            "actual_claim_projections":self.actual_claim_projections.iter().map(|(key,id)|json!({"key":key,"claim_transaction_id":sqlx::types::Uuid::from_bytes(*id).to_string()})).collect::<Vec<_>>(),"changes":changes,
            "absent_initial_projections":self.child.absent_initial_projections.iter()
                .map(|(key, initial)|json!({"key":key,"initial":initial})).collect::<Vec<_>>(),
            "completes":self.child.completes}),
        )
    }
}

/// Insert the reciprocal immutable child before invoking the shared mature
/// state writer. Any failure rolls back the caller's parent and all children.
pub(in crate::durability) async fn persist(
    tx: &mut sqlx::PgConnection,
    context: &NpcProgramContext<'_>,
    prepared: &PreparedOwnerWrite,
) -> Result<(), DurabilityError> {
    let observation = prepared.observation()?;
    let changes = observation
        .get("changes")
        .ok_or(DurabilityError::InvalidStoredState)?;
    let inserted = sqlx::query("INSERT INTO game_npc_quest_children(parent_transaction_id,quest_key,character_id,pinned_content_revision,base_definition_hash,evaluation_definition_hash,program_sha256,plan_sha256,committed_revision,transition_keys,native_operations,track_changes,completes) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9::text::numeric(20,0),$10::text[],$11::text[],$12,$13)")
        .bind(context.npc_transaction.as_slice()).bind(&prepared.child.quest)
        .bind(context.fence.character_id.as_bytes().as_slice())
        .bind(&prepared.pin.pinned_content_revision)
        .bind(prepared.pin.pinned_definition_hash.as_slice())
        .bind(prepared.pin.evaluation_definition_hash.as_slice())
        .bind(context.program_sha256.as_slice()).bind(context.plan_sha256.as_slice())
        .bind(context.next_revision.to_string()).bind(&prepared.child.transition_keys)
        .bind(&prepared.native_operations)
        .bind(changes).bind(prepared.child.completes).execute(&mut *tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !prepared.writes_state {
        if prepared.child.quest != "oteryn:quest.npc_cosmetic_final_acquisition"
            || prepared.pin.state_exists
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        return super::persistence::write_tracks(
            tx,
            context.fence.character_id,
            &prepared.child.quest,
            &prepared.child.changes,
            context.next_revision,
        )
        .await;
    }
    super::persistence::write_state(
        tx,
        &super::persistence::EvaluatedStateWrite {
            character: context.fence.character_id,
            quest: &prepared.child.quest,
            pin: &prepared.pin.pinned_content_revision,
            hash: &prepared.pin.pinned_definition_hash,
            completes: prepared.child.completes,
            changes: &prepared.child.changes,
            committed_revision: context.next_revision,
            state_exists: prepared.pin.state_exists,
        },
    )
    .await
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}
