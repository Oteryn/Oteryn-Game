//! STARTER-1 (CP D1438, STARTER-BACKPACK0 Q3 b): the durable starter grant of a Character's
//! first entry. One grant row per Character and template, forever: MINTED puts a fresh main
//! backpack into the empty `container` slot in the same transaction; SKIPPED records that the
//! slot was already occupied. A replay of the same Character and template writes nothing.
//! The template's item must be a container of the active Content generation; otherwise nothing
//! is written and the entry proceeds without a starter item (`NotAdmitted`).

use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::item_loop_move::ItemLoopContent;
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use serde_json::json;

/// One starter template: its registered label and revision, and the one item it grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StarterTemplate {
    pub key: String,
    pub revision: String,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

impl StarterTemplate {
    /// `oteryn:starter-template.main` (record `oteryn:starter.main_backpack`): one backpack
    /// `oteryn:item.tibia.i2854` into the `container` slot.
    pub(crate) fn main() -> Self {
        Self {
            key: "oteryn:starter-template.main".to_owned(),
            revision: "starter-kit-r1".to_owned(),
            definition: TypedDefinitionRef {
                family: "Item".to_owned(),
                production_key: "oteryn:item.tibia.i2854".to_owned(),
                revision_ref: "definition-r1".to_owned(),
            },
            quantity: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StarterGrantOutcome {
    /// The fresh main backpack.
    Minted([u8; 16]),
    Skipped,
    AlreadyGranted,
    NotAdmitted,
}

fn grant_error(error: EquipmentError) -> DurabilityError {
    match error {
        EquipmentError::Durability(error) => error,
        EquipmentError::Database(_) | EquipmentError::Rejected(_) => DurabilityError::Unavailable,
    }
}

async fn new_uuid(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<[u8; 16], DurabilityError> {
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&mut **tx)
        .await?;
    bytes
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

impl DurabilityRoot {
    /// STARTER-1: the starter grant of `template` for the fenced Character, under its current
    /// gameplay fence and the active Content digest of `content`.
    pub(crate) async fn grant_starter_items<C: ItemLoopContent>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content: &C,
        template: &StarterTemplate,
    ) -> std::result::Result<StarterGrantOutcome, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let template = template.clone();
        let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::Unavailable)?;
        let admitted = content
            .loop_facts(&template.definition)
            .is_some_and(|facts| {
                facts.definition == template.definition && facts.container_capacity.is_some()
            })
            && (1..=100).contains(&template.quantity);
        let digest = content.loop_content_digest();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_current_equipment_in_transaction(
                        &mut tx, &record, &node, &fence, None, digest,
                    )
                    .await
                    .map_err(grant_error)?;
                    let character = fence.character_id;
                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:starter-grant:' || encode($1, 'hex'), 0))",
                    )
                    .bind(character.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    let granted: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_starter_grants \
                          WHERE character_id = encode($1,'hex')::uuid AND template_key = $2)",
                    )
                    .bind(character.as_bytes().as_slice())
                    .bind(&template.key)
                    .fetch_one(&mut *tx)
                    .await?;
                    if granted {
                        super::db::commit_semantic_transaction(tx, deadline).await?;
                        return Ok(StarterGrantOutcome::AlreadyGranted);
                    }
                    if !admitted {
                        super::db::commit_semantic_transaction(tx, deadline).await?;
                        return Ok(StarterGrantOutcome::NotAdmitted);
                    }
                    let occupied: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_item_container_slots \
                          WHERE character_id = encode($1,'hex')::uuid)",
                    )
                    .bind(character.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    let transaction_id = new_uuid(&mut tx).await?;
                    let event_id = new_uuid(&mut tx).await?;
                    let item = if occupied {
                        None
                    } else {
                        Some(new_uuid(&mut tx).await?)
                    };
                    let envelope = serde_json::to_vec(&json!({
                        "character": character.as_bytes(),
                        "template": template.key,
                        "template_revision": template.revision,
                        "definition": [
                            template.definition.family,
                            template.definition.production_key,
                            template.definition.revision_ref,
                        ],
                        "quantity": template.quantity,
                        "outcome": if occupied { "SKIPPED" } else { "MINTED" },
                        "item": item,
                    }))
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                    sqlx::query(
                        "INSERT INTO game_character_starter_grants (character_id, template_key, \
                                template_revision, world_id, definition_family, \
                                definition_production_key, definition_revision_ref, quantity, \
                                outcome, item_instance_id, transaction_id, event_id, \
                                grant_envelope, grant_envelope_sha256) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, $5, $6, \
                                 $7, $8, $9, encode($10,'hex')::uuid, encode($11,'hex')::uuid, \
                                 encode($12,'hex')::uuid, $13, sha256($13))",
                    )
                    .bind(character.as_bytes().as_slice())
                    .bind(&template.key)
                    .bind(&template.revision)
                    .bind(world.as_bytes().as_slice())
                    .bind(&template.definition.family)
                    .bind(&template.definition.production_key)
                    .bind(&template.definition.revision_ref)
                    .bind(i64::from(template.quantity))
                    .bind(if occupied { 2_i16 } else { 1_i16 })
                    .bind(item.map(|item| item.to_vec()))
                    .bind(transaction_id.as_slice())
                    .bind(event_id.as_slice())
                    .bind(envelope.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if let Some(item) = item {
                        sqlx::query(
                            "INSERT INTO game_item_instances (item_instance_id, world_id, \
                                    definition_family, definition_production_key, \
                                    definition_revision_ref, quantity, lifecycle, \
                                    minted_transaction_id) \
                             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, \
                                     $6, 1, encode($7,'hex')::uuid)",
                        )
                        .bind(item.as_slice())
                        .bind(world.as_bytes().as_slice())
                        .bind(&template.definition.family)
                        .bind(&template.definition.production_key)
                        .bind(&template.definition.revision_ref)
                        .bind(i64::from(template.quantity))
                        .bind(transaction_id.as_slice())
                        .execute(&mut *tx)
                        .await?;
                        sqlx::query(
                            "INSERT INTO game_item_container_slots (character_id, \
                                    item_instance_id, world_id, placed_transaction_id) \
                             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                                     encode($3,'hex')::uuid, encode($4,'hex')::uuid)",
                        )
                        .bind(character.as_bytes().as_slice())
                        .bind(item.as_slice())
                        .bind(world.as_bytes().as_slice())
                        .bind(transaction_id.as_slice())
                        .execute(&mut *tx)
                        .await?;
                    }
                    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                        .execute(&mut *tx)
                        .await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(match item {
                        Some(item) => StarterGrantOutcome::Minted(item),
                        None => StarterGrantOutcome::Skipped,
                    })
                })
            })
            .await
    }
}
