//! DEATH-DUR-1: idempotent one-item TRANSFER and necklace BURN under the Character item fence.
//! The persisted receipt supplies the death cell. DEATH-3 owns selection and workflow ordering;
//! these primitives neither select items nor change CharacterRevision or receipt constraints.
//! The caller holds its Channel spell lane from before begin through commit (key-33 Ground).
//! Contents retain their parent identity: no child is written or expanded into audit lines.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, numeric_u64, uuid_text};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{
    self as mint, SelectedType2Tuple, check_technical_text, check_uuid_v7,
};
use super::item_transfer::{CurrentCharacterItemFence, definition_message};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};

// GAME-ITEM-01 semantic durable slot code; distinct from wire and death draw indices.
const NECKLACE_SLOT: u8 = 7;

pub mod audit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDeathDropCause {
    pub death_occurrence: [u8; 16],
    pub item_instance: [u8; 16],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDeathAmuletCause {
    pub death_occurrence: [u8; 16],
    pub item_instance: [u8; 16],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDeathItemRequest {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
    pub server_build_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDeathItemOutcome {
    Written,
    Replay,
}
#[derive(Debug)]
pub enum PlayerDeathItemError {
    InvalidInput,
    ConflictingCause,
    StaleState,
    AuthorityRejected,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for PlayerDeathItemError {
    fn from(value: DurabilityError) -> Self {
        Self::Unavailable(value)
    }
}
impl From<sqlx::Error> for PlayerDeathItemError {
    fn from(value: sqlx::Error) -> Self {
        Self::Unavailable(value.into())
    }
}
impl std::fmt::Display for PlayerDeathItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PlayerDeathItemError {}
type Result<T> = std::result::Result<T, PlayerDeathItemError>;

/// Fixed one-item plan. Slot zero means a direct backpack entry; nine is the main backpack.
/// No splitting, nested expansion, or stack of necklaces is admitted.
fn plan(burn: bool, slot: u8, quantity: u32, depth: u8) -> Result<(u32, u32)> {
    if quantity == 0
        || quantity > 100
        || slot > 10
        || depth > 1
        || (burn && (slot != NECKLACE_SLOT || quantity != 1 || depth != 0))
    {
        return Err(PlayerDeathItemError::InvalidInput);
    }
    Ok(if burn { (0, 2) } else { (quantity, 1) })
}
impl PlayerDeathItemRequest {
    fn validate(&self, occurrence: &[u8; 16], item: &[u8; 16]) -> Result<()> {
        for id in [occurrence, item, &self.transaction_id, &self.event_id] {
            check_uuid_v7(id).map_err(|_| PlayerDeathItemError::InvalidInput)?;
        }
        if self.transaction_id == self.event_id || self.occurred_at_unix_ms <= 0 {
            return Err(PlayerDeathItemError::InvalidInput);
        }
        for text in [
            &self.content_revision,
            &self.ruleset_revision,
            &self.sim_revision,
        ] {
            mint::check_content_key(text).map_err(|_| PlayerDeathItemError::InvalidInput)?;
        }
        check_technical_text(&self.server_build_id).map_err(|_| PlayerDeathItemError::InvalidInput)
    }
    fn binding(
        &self,
        burn: bool,
        occurrence: &[u8; 16],
        item: &[u8; 16],
        character: &[u8; 16],
    ) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.player_death_item.v1");
        hash.update([u8::from(burn)]);
        for id in [
            occurrence,
            item,
            character,
            &self.transaction_id,
            &self.event_id,
        ] {
            hash.update(id);
        }
        hash.update(self.occurred_at_unix_ms.to_be_bytes());
        for text in [
            &self.content_revision,
            &self.ruleset_revision,
            &self.sim_revision,
            &self.server_build_id,
        ] {
            hash.update((text.len() as u64).to_be_bytes());
            hash.update(text.as_bytes());
        }
        hash.finalize().into()
    }
}
impl DurabilityRoot {
    pub async fn commit_player_death_drop(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        cause: PlayerDeathDropCause,
        request: PlayerDeathItemRequest,
    ) -> Result<PlayerDeathItemOutcome> {
        self.commit_player_death_item(permit, authority, node, fence, (cause, false), request)
            .await
    }
    pub async fn commit_player_death_amulet(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        cause: PlayerDeathAmuletCause,
        request: PlayerDeathItemRequest,
    ) -> Result<PlayerDeathItemOutcome> {
        self.commit_player_death_item(
            permit,
            authority,
            node,
            fence,
            (
                PlayerDeathDropCause {
                    death_occurrence: cause.death_occurrence,
                    item_instance: cause.item_instance,
                },
                true,
            ),
            request,
        )
        .await
    }
    async fn commit_player_death_item(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        operation: (PlayerDeathDropCause, bool),
        request: PlayerDeathItemRequest,
    ) -> Result<PlayerDeathItemOutcome> {
        let (cause, burn) = operation;
        permit.check_scope(fence.runtime_scope)?;
        request.validate(&cause.death_occurrence, &cause.item_instance)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| PlayerDeathItemError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let permit = *context;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // Terminal evidence answers a retry without reacquiring mutation authority.
                    match replay(&mut tx, &fence, cause, &request, burn).await {
                        Ok(Some(outcome)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(outcome));
                        }
                        Ok(None) => {}
                        Err(PlayerDeathItemError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    // Serialize both kinds of cause and item under the current Character root lock.
                    let root = match assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(_) => return Ok(Err(PlayerDeathItemError::AuthorityRejected)),
                    };
                    if root.content_revision != request.content_revision
                        || root.ruleset_revision != request.ruleset_revision
                    {
                        return Ok(Err(PlayerDeathItemError::StaleState));
                    }
                    let tuple = tx.tuple();
                    match write(&mut tx, permit, &fence, cause, &request, burn, tuple).await {
                        Ok(outcome) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(PlayerDeathItemError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn replay(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    cause: PlayerDeathDropCause,
    request: &PlayerDeathItemRequest,
    burn: bool,
) -> Result<Option<PlayerDeathItemOutcome>> {
    let character = fence.character_id.as_bytes();
    let binding = request.binding(
        burn,
        &cause.death_occurrence,
        &cause.item_instance,
        character,
    );
    let records = sqlx::query("SELECT request_binding FROM game_player_death_item_records WHERE (death_occurrence_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid) OR transaction_id=encode($3,'hex')::uuid OR event_id=encode($4,'hex')::uuid LIMIT 3")
        .bind(cause.death_occurrence.as_slice()).bind(cause.item_instance.as_slice()).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice()).fetch_all(&mut *connection).await?;
    if !records.is_empty() {
        return if records.len() == 1
            && records[0].try_get::<Vec<u8>, _>("request_binding")? == binding
        {
            Ok(Some(PlayerDeathItemOutcome::Replay))
        } else {
            Err(PlayerDeathItemError::ConflictingCause)
        };
    }
    let reused: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid OR event_id=encode($2,'hex')::uuid)")
        .bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice()).fetch_one(&mut *connection).await?;
    if reused {
        return Err(PlayerDeathItemError::ConflictingCause);
    }
    Ok(None)
}

async fn write(
    connection: &mut PgConnection,
    permit: &SpellLanePermit,
    fence: &CurrentCharacterItemFence,
    cause: PlayerDeathDropCause,
    request: &PlayerDeathItemRequest,
    burn: bool,
    tuple: SelectedType2Tuple,
) -> Result<PlayerDeathItemOutcome> {
    if let Some(outcome) = replay(connection, fence, cause, request, burn).await? {
        return Ok(outcome);
    }
    let character = fence.character_id.as_bytes();
    let binding = request.binding(
        burn,
        &cause.death_occurrence,
        &cause.item_instance,
        character,
    );
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(PlayerDeathItemError::AuthorityRejected);
    };
    permit.check_channel(world_id, channel_id)?;
    let death = sqlx::query("SELECT death_spatial_position, death_map_revision, content_revision FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND death_world_id=encode($3,'hex')::uuid AND death_channel_id=encode($4,'hex')::uuid")
        .bind(cause.death_occurrence.as_slice()).bind(character.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).fetch_optional(&mut *connection).await?.ok_or(PlayerDeathItemError::StaleState)?;
    if death.try_get::<String, _>("content_revision")? != request.content_revision {
        return Err(PlayerDeathItemError::StaleState);
    }
    let item = sqlx::query("SELECT definition_family, definition_production_key, definition_revision_ref, quantity, COALESCE((SELECT key_number FROM game_item_keys k WHERE k.item_instance_id=game_item_instances.item_instance_id),0) AS key_number, state_revision::text AS revision FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(cause.item_instance.as_slice()).bind(world_id.as_bytes().as_slice()).fetch_optional(&mut *connection).await?.ok_or(PlayerDeathItemError::StaleState)?;
    let source = sqlx::query("SELECT 9::smallint AS slot, NULL::text AS parent, NULL::text AS ordinal, placed_transaction_id::text AS placement FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid UNION ALL SELECT slot,NULL,NULL,placed_transaction_id::text FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid UNION ALL SELECT 0::smallint,parent_item_instance_id::text,placement_ordinal::text,placed_transaction_id::text FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid AND parent_item_instance_id IN(SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid) LIMIT 2")
        .bind(character.as_slice()).bind(cause.item_instance.as_slice()).fetch_all(&mut *connection).await?;
    let [source] = source.as_slice() else {
        return Err(PlayerDeathItemError::StaleState);
    };
    let slot = u8::try_from(source.try_get::<i16, _>("slot")?)
        .map_err(|_| PlayerDeathItemError::StaleState)?;
    // Only direct contents of a main backpack move implicitly. Any nested placement needs a decision.
    let too_deep: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_nested_entries n ON n.parent_item_instance_id=e.item_instance_id WHERE e.parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
        .bind(cause.item_instance.as_slice()).fetch_one(&mut *connection).await?;
    if too_deep {
        return Err(PlayerDeathItemError::InvalidInput);
    }
    let has_contents: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
        .bind(cause.item_instance.as_slice()).fetch_one(&mut *connection).await?;
    if has_contents && (slot != 9 || burn) {
        return Err(PlayerDeathItemError::InvalidInput);
    }
    let quantity = u32::try_from(item.try_get::<i64, _>("quantity")?)
        .map_err(|_| PlayerDeathItemError::StaleState)?;
    let (after, lifecycle) = plan(burn, slot, quantity, u8::from(has_contents))?;
    let revision = numeric_u64(&item, "revision")?;
    let parent = source
        .try_get::<Option<String>, _>("parent")?
        .map(|v| uuid_text(&v))
        .transpose()?;
    let ordinal = source
        .try_get::<Option<String>, _>("ordinal")?
        .map(|v| {
            v.parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()?;
    let equipment_revision = if slot != 0 && slot != 9 {
        Some(sqlx::query_scalar::<_,String>("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(character.as_slice()).fetch_one(&mut *connection).await?)
    } else {
        None
    };
    let definition = TypedDefinitionRef {
        family: item.try_get("definition_family")?,
        production_key: item.try_get("definition_production_key")?,
        revision_ref: item.try_get("definition_revision_ref")?,
    };
    let key_number = u32::try_from(item.try_get::<i32, _>("key_number")?)
        .map_err(|_| PlayerDeathItemError::StaleState)?;
    let state = |quantity, lifecycle| mint::OneItemStateV1 {
        item_instance_id: cause.item_instance.to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        definition: Some(definition_message(&definition)),
        quantity,
        lifecycle,
        key_number,
    };
    let ground = mint::OneItemGroundV1 {
        world_id: world_id.as_bytes().to_vec(),
        channel_id: channel_id.as_bytes().to_vec(),
        spatial_position: death.try_get("death_spatial_position")?,
        corpse_ref: cause.death_occurrence.to_vec(),
        map_revision: death.try_get("death_map_revision")?,
        content_revision: request.content_revision.clone(),
        native_room_placement_context: cause.death_occurrence.to_vec(),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
    };
    let event = audit::OneItemPlayerDeathItemV1 {
        death_occurrence: cause.death_occurrence.to_vec(),
        item_instance: cause.item_instance.to_vec(),
        before: Some(state(quantity, 1)),
        after: Some(state(after, lifecycle)),
        character_id: character.to_vec(),
        source_slot: u32::from(slot),
        source_entry: parent.zip(ordinal).map(|(p, o)| {
            super::item_transfer_audit::OneItemContainerEntryV1 {
                parent_item_instance_id: p.to_vec(),
                placement_ordinal: o,
            }
        }),
        ground: (!burn).then_some(ground.clone()),
        typed_sink: if burn {
            audit::AMULET_SINK.into()
        } else {
            String::new()
        },
        content_revision: request.content_revision.clone(),
        ruleset_revision: request.ruleset_revision.clone(),
        sim_revision: request.sim_revision.clone(),
        session_generation: fence.connection_generation.get(),
        game_session_id: fence.game_session_id.as_bytes().to_vec(),
        lease_generation: fence.character_lease_generation,
    };
    let envelope = audit::encode(request, burn, event, tuple.tuple())
        .map_err(|_| PlayerDeathItemError::InvalidInput)?;
    sqlx::query("INSERT INTO game_player_death_item_records(death_occurrence_id,item_instance_id,operation,transaction_id,event_id,character_id,world_id,channel_id,source_slot,source_parent,source_ordinal,source_placement,quantity_before,state_revision_before,equipment_revision_before,request_binding,occurred_at,envelope_sha256,scope_generation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid,$11::text::numeric,$12::uuid,$13,$14::text::numeric,$15::text::numeric,$16,$17,sha256($18),$19::text::numeric)")
        .bind(cause.death_occurrence.as_slice()).bind(cause.item_instance.as_slice()).bind(if burn {2_i16} else {1_i16}).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice()).bind(character.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(i16::from(slot)).bind(parent.map(|v|v.to_vec())).bind(ordinal.map(|v|v.to_string())).bind(source.try_get::<String,_>("placement")?).bind(i64::from(quantity)).bind(revision.to_string()).bind(equipment_revision.as_ref()).bind(binding.as_slice()).bind(request.occurred_at_unix_ms).bind(envelope.as_slice()).bind(fence.scope_ownership_generation.get().to_string()).execute(&mut *connection).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=$2,lifecycle=$3,last_transaction_id=encode($4,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(cause.item_instance.as_slice()).bind(i64::from(after)).bind(i16::try_from(lifecycle).map_err(|_| PlayerDeathItemError::InvalidInput)?).bind(request.transaction_id.as_slice()).execute(&mut *connection).await?;
    sqlx::query(match slot { 0 => "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid", 9 => "DELETE FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid", _ => "DELETE FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid" })
        .bind(cause.item_instance.as_slice()).execute(&mut *connection).await?;
    if let Some(revision) = equipment_revision {
        sqlx::query("UPDATE game_character_equipment_state SET revision=$1::text::numeric+1,last_transaction_id=encode($2,'hex')::uuid WHERE character_id=encode($3,'hex')::uuid")
            .bind(revision).bind(request.transaction_id.as_slice()).bind(character.as_slice()).execute(&mut *connection).await?;
    }
    if !burn {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
            .bind(cause.item_instance.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(ground.runtime_scope_ownership_generation.to_string()).bind(&ground.spatial_position).bind(&ground.corpse_ref).bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.native_room_placement_context).execute(&mut *connection).await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(request.event_id.as_slice()).bind(request.transaction_id.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(cause.item_instance.as_slice()).bind(request.occurred_at_unix_ms).bind(mint::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut *connection).await?;
    Ok(PlayerDeathItemOutcome::Written)
}

#[cfg(test)]
mod tests;
