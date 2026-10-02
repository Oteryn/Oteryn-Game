//! Candidate Character equipment owner on the actual durable ItemInstance custody graph.
//! Helpers participate in the compositor's SAME physical transaction; none commits independently.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::equipment_policy_abi::{EquipmentPolicyLookup, claims};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug)]
pub(crate) enum EquipmentError {
    Durability(DurabilityError),
    Database(sqlx::Error),
    Rejected(&'static str),
}
impl From<DurabilityError> for EquipmentError {
    fn from(e: DurabilityError) -> Self {
        Self::Durability(e)
    }
}
impl From<sqlx::Error> for EquipmentError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
type Result<T> = std::result::Result<T, EquipmentError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum CombatMode {
    Attack,
    Balanced,
    Defense,
}
impl CombatMode {
    pub(crate) fn attack_factor(self) -> f64 {
        match self {
            Self::Attack => 1.0,
            Self::Balanced => 0.75,
            Self::Defense => 0.5,
        }
    }
    fn code(self) -> i16 {
        match self {
            Self::Attack => 1,
            Self::Balanced => 2,
            Self::Defense => 3,
        }
    }
    fn from_code(code: i16) -> Result<Self> {
        match code {
            1 => Ok(Self::Attack),
            2 => Ok(Self::Balanced),
            3 => Ok(Self::Defense),
            _ => Err(EquipmentError::Rejected("stored combat mode")),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquippedItem {
    pub(crate) slot: u8,
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: super::item_mint::TypedDefinitionRef,
    pub(crate) quantity: u32,
    pub(crate) state_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentSnapshot {
    pub(crate) character: [u8; 16],
    pub(crate) content_digest: [u8; 32],
    pub(crate) character_revision: u64,
    pub(crate) revision: u64,
    pub(crate) combat_mode: Option<CombatMode>,
    pub(crate) items: Vec<EquippedItem>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) enum EquipmentIntent {
    Move {
        item: [u8; 16],
        from_slot: Option<u8>,
        to_slot: Option<u8>,
    },
    SetCombatMode(CombatMode),
}
#[derive(Debug)]
pub(crate) struct EquipmentReceipt {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) revision_after: u64,
    pub(crate) replayed: bool,
}
/// Private current proof, issued only after recovery, node/session/lease/scope and independently
/// issued active Content have been checked against actual durable owners in this transaction.
#[derive(Debug)]
pub(crate) struct EquipmentAuthority {
    physical_transaction: String,
    fence: CurrentCharacterItemFence,
    command: Option<CommandRef>,
    world: [u8; 16],
    channel: [u8; 16],
    content_digest: [u8; 32],
    character_revision: u64,
    account: [u8; 16],
}
fn rejected(message: &'static str) -> EquipmentError {
    EquipmentError::Rejected(message)
}
fn admission_equipment_unavailable(
    _stage: &'static str,
    _error: EquipmentError,
) -> DurabilityError {
    #[cfg(test)]
    if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
        match &_error {
            EquipmentError::Rejected(reason) => {
                eprintln!("SEAM_EVIDENCE equipment_owner stage={_stage} rejected={reason}")
            }
            EquipmentError::Database(sqlx::Error::Database(error)) => eprintln!(
                "SEAM_EVIDENCE equipment_owner stage={_stage} sqlstate={}",
                error.code().as_deref().unwrap_or("unknown")
            ),
            EquipmentError::Durability(error) => {
                eprintln!("SEAM_EVIDENCE equipment_owner stage={_stage} durability={error:?}")
            }
            EquipmentError::Database(_) => {
                eprintln!("SEAM_EVIDENCE equipment_owner stage={_stage} database=non_server_error")
            }
        }
    }
    DurabilityError::Unavailable
}
fn uuid(text: String) -> Result<[u8; 16]> {
    let compact: String = text.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 {
        return Err(rejected("stored equipment UUID"));
    }
    let mut output = [0; 16];
    for (index, slot) in output.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
            .map_err(|_| rejected("stored equipment UUID"))?;
    }
    Ok(output)
}
fn number(text: String) -> Result<u64> {
    text.parse()
        .map_err(|_| rejected("stored equipment unsigned value"))
}
async fn same_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
) -> Result<()> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    if current != authority.physical_transaction {
        return Err(rejected("equipment authority transaction"));
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn assert_equipment_authority_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content_digest: [u8; 32],
) -> Result<EquipmentAuthority> {
    let recovery = recovery
        .record_for(root)
        .map_err(|_| rejected("equipment recovery"))?;
    assert_equipment_fence_in_transaction(tx, &recovery, node, fence, command, content_digest).await
}
#[allow(clippy::too_many_arguments)]
async fn assert_equipment_fence_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content_digest: [u8; 32],
) -> Result<EquipmentAuthority> {
    assert_current_equipment_in_transaction(
        tx,
        recovery,
        node,
        fence,
        Some(command),
        content_digest,
    )
    .await
}
pub(super) async fn assert_current_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: Option<CommandRef>,
    content_digest: [u8; 32],
) -> Result<EquipmentAuthority> {
    if content_digest == [0; 32] {
        return Err(rejected("equipment Content pin"));
    }
    assert_recovery_fence(tx, recovery).await?;
    super::db::lock_admission_relations(tx).await?;
    let (world, channel) = scope_of(fence).map_err(|_| rejected("equipment Channel scope"))?;
    // Match game_content_record_activation's existing per-scope transaction lock.
    // The runtime has SELECT, not UPDATE: table/row SHARE locks require write
    // privileges. This lock blocks the authorized append producer without
    // granting the runtime any Content mutation authority.
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea, 'hex')::uuid::text || ':' || encode($2::bytea, 'hex')::uuid::text, 0))")
        .bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    // Existing fence takes the actual Character root lock, so all custody writers serialize.
    if let Some(command) = command {
        if !character_item_fence_is_current(
            tx,
            node,
            fence,
            command,
            fence.character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
        {
            return Err(rejected("equipment current fence"));
        }
    } else {
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or_else(|| rejected("equipment Character root"))?;
        let gameplay = super::character_progression::CurrentCharacterGameplayFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            expected_character_revision: crate::domain::CharacterRevision::new(number(revision)?)
                .map_err(|_| rejected("equipment Character revision"))?,
        };
        super::character_progression::assert_gameplay_fence(tx, &gameplay, node)
            .await?
            .map_err(|_| rejected("equipment current lifecycle fence"))?;
    }
    let digest:Option<Vec<u8>>=sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if digest.as_deref() != Some(content_digest.as_slice()) {
        return Err(rejected("equipment independent active Content"));
    }
    let character=sqlx::query("SELECT character_revision::text,account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let physical_transaction = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(EquipmentAuthority {
        physical_transaction,
        fence: *fence,
        command,
        world: *world.as_bytes(),
        channel: *channel.as_bytes(),
        content_digest,
        character_revision: number(character.try_get("character_revision")?)?,
        account: uuid(character.try_get("account_id")?)?,
    })
}
/// Creates only the equipment owner's declared empty seed under the actual current fence.
/// Its combat mode stays explicitly unknown until a real SetCombatMode command commits.
pub(crate) async fn initialize_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
) -> Result<()> {
    same_transaction(tx, authority).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
        .bind(authority.fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}
pub(crate) async fn read_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
) -> Result<EquipmentSnapshot> {
    same_transaction(tx, authority).await?;
    let row=sqlx::query("SELECT revision::text,combat_mode FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment owner not initialized"))?;
    // The validated equipment authority already holds the Character root lock.
    // Slot locations are immutable (INSERT/DELETE only); lock their actual item
    // rows, as the custody exclusivity trigger does, without requiring slot UPDATE.
    let rows=sqlx::query("SELECT s.slot,i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.state_revision::text,i.lifecycle,i.world_id::text AS item_world,s.world_id::text AS slot_world FROM game_character_equipment_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid ORDER BY s.slot FOR UPDATE OF i")
        .bind(authority.fence.character_id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 9 {
        return Err(rejected("equipment slot bound"));
    }
    let mut items = Vec::new();
    for row in rows {
        if row.try_get::<i16, _>("lifecycle")? != 1
            || row.try_get::<String, _>("definition_family")? != "Item"
            || uuid(row.try_get("item_world")?)? != authority.world
            || uuid(row.try_get("slot_world")?)? != authority.world
        {
            return Err(rejected("equipment corrupt actual custody"));
        }
        items.push(EquippedItem {
            slot: u8::try_from(row.try_get::<i16, _>("slot")?)
                .map_err(|_| rejected("equipment slot"))?,
            item_instance_id: uuid(row.try_get("item_instance_id")?)?,
            definition: super::item_mint::TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| rejected("equipment quantity"))?,
            state_revision: number(row.try_get("state_revision")?)?,
        });
    }
    Ok(EquipmentSnapshot {
        character: *authority.fence.character_id.as_bytes(),
        content_digest: authority.content_digest,
        character_revision: authority.character_revision,
        revision: number(row.try_get("revision")?)?,
        combat_mode: row
            .try_get::<Option<i16>, _>("combat_mode")?
            .map(CombatMode::from_code)
            .transpose()?,
        items,
    })
}
/// Atomic custody move/explicit combat mode: compares the complete current owner snapshot before
/// allocating trusted IDs, changing any item or writing the receipt. Caller owns final SQL commit.
pub(crate) async fn apply_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    expected: &EquipmentSnapshot,
    content: &impl EquipmentPolicyLookup,
    intent: &EquipmentIntent,
) -> Result<EquipmentReceipt> {
    same_transaction(tx, authority).await?;
    let command = authority
        .command
        .ok_or_else(|| rejected("equipment writer needs actual command"))?;
    if content.source_digest() != authority.content_digest {
        return Err(rejected("equipment active profile pin"));
    }
    let binding_bytes=serde_json::to_vec(&json!({"schema":"OTERYN_EQUIPMENT_COMMAND/v1","character":expected.character,"owner_revision":expected.revision,"content_digest":expected.content_digest,"intent":intent})).map_err(|_|rejected("equipment binding"))?;
    let binding: Vec<u8> = Sha256::digest(&binding_bytes).to_vec();
    let existing=sqlx::query("SELECT transaction_id::text,event_id::text,revision_after::text,binding,intent FROM game_character_equipment_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    if let Some(row) = existing {
        if row.try_get::<Vec<u8>, _>("binding")? != binding
            || row.try_get::<Vec<u8>, _>("intent")? != binding_bytes
        {
            return Err(rejected("substituted equipment replay"));
        }
        return Ok(EquipmentReceipt {
            transaction_id: uuid(row.try_get("transaction_id")?)?,
            event_id: uuid(row.try_get("event_id")?)?,
            revision_after: number(row.try_get("revision_after")?)?,
            replayed: true,
        });
    }
    let actual = read_equipment_in_transaction(tx, authority).await?;
    if &actual != expected {
        return Err(rejected("stale equipment snapshot"));
    }
    let next = actual
        .revision
        .checked_add(1)
        .ok_or(rejected("equipment revision exhausted"))?;
    let mut mode = actual.combat_mode;
    let mut item = None;
    let mut from = None;
    let mut to = None;
    let mut parent = None;
    let mut ordinal = None;
    let mut item_revision = None;
    let operation = match intent {
        EquipmentIntent::SetCombatMode(value) => {
            mode = Some(*value);
            2_i16
        }
        EquipmentIntent::Move {
            item: identity,
            from_slot,
            to_slot,
        } => {
            if from_slot.is_some() == to_slot.is_some() {
                return Err(rejected("equipment exact source/destination"));
            }
            let row=sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
                .bind(identity.as_slice()).bind(authority.world.as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment actual live item"))?;
            let policy = content
                .equipment_policy(
                    &row.try_get::<String, _>("definition_production_key")?,
                    &row.try_get::<String, _>("definition_revision_ref")?,
                )
                .ok_or(rejected("equipment active item definition"))?;
            if policy.source_digest() != authority.content_digest {
                return Err(rejected("equipment member profile pin"));
            }
            if row.try_get::<String, _>("definition_family")? != "Item" {
                return Err(rejected("equipment item family"));
            }
            // The Character root and backpack item serialize this immutable slot.
            let backpack=sqlx::query("SELECT s.item_instance_id::text,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid FOR UPDATE OF i")
                .bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment requires actual backpack"))?;
            let backpack_id = uuid(backpack.try_get("item_instance_id")?)?;
            if let Some(slot) = to_slot {
                let proposed = claims(&policy, *slot)?;
                if proposed.level > 0 || !proposed.vocations.is_empty() {
                    let progression = sqlx::query("SELECT p.level,b.vocation FROM game_character_progression_state p LEFT JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid FOR SHARE OF p")
                        .bind(actual.character.as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment current progression"))?;
                    let level: u64 = u64::try_from(progression.try_get::<i64, _>("level")?)
                        .map_err(|_| rejected("equipment current level"))?;
                    if level < u64::from(proposed.level) {
                        return Err(rejected("equipment minimum level"));
                    }
                    if !proposed.vocations.is_empty() {
                        use super::equipment_policy_abi::EquipmentBaseVocation as Base;
                        let vocation: Option<String> = progression.try_get("vocation")?;
                        let base = match vocation.as_deref() {
                            Some("druid" | "elder_druid") => Base::Druid,
                            Some("sorcerer" | "master_sorcerer") => Base::Sorcerer,
                            Some("knight" | "elite_knight") => Base::Knight,
                            Some("paladin" | "royal_paladin") => Base::Paladin,
                            Some("monk" | "exalted_monk") => Base::Monk,
                            _ => return Err(rejected("equipment actual vocation")),
                        };
                        if !proposed.vocations.contains(&base) {
                            return Err(rejected("equipment vocation requirement"));
                        }
                    }
                }

                for old in &actual.items {
                    let oldpolicy = content
                        .equipment_policy(
                            &old.definition.production_key,
                            &old.definition.revision_ref,
                        )
                        .ok_or(rejected("equipment existing active policy"))?;
                    if oldpolicy.source_digest() != authority.content_digest {
                        return Err(rejected("equipment existing profile pin"));
                    }
                    let current = claims(&oldpolicy, old.slot)?;
                    if current
                        .slots
                        .iter()
                        .any(|value| proposed.slots.contains(value))
                        || current
                            .groups
                            .iter()
                            .any(|value| proposed.groups.contains(value))
                    {
                        return Err(rejected("equipment occupancy conflict"));
                    }
                }
                // This transaction already holds the Character, source item and
                // backpack locks. Immutable placement rows need no UPDATE grant.
                let source=sqlx::query("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid")
                    .bind(identity.as_slice()).bind(authority.fence.character_id.as_bytes().as_slice()).bind(backpack_id.as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment source is not direct backpack entry"))?;
                ordinal = Some(number(source.try_get("placement_ordinal")?)?);
            } else {
                if !actual
                    .items
                    .iter()
                    .any(|old| old.item_instance_id == *identity && Some(old.slot) == *from_slot)
                {
                    return Err(rejected("equipment source exact slot"));
                }
                // Other legitimate custody writers for this backpack serialize
                // on the same held Character root before changing its entries.
                let rows=sqlx::query("SELECT placement_ordinal::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal").bind(backpack_id.as_slice()).fetch_all(&mut **tx).await?;
                let backpack_policy = content
                    .equipment_policy(
                        &backpack.try_get::<String, _>("definition_production_key")?,
                        &backpack.try_get::<String, _>("definition_revision_ref")?,
                    )
                    .ok_or(rejected("equipment actual backpack active policy"))?;
                let capacity = backpack_policy
                    .container_capacity()
                    .ok_or(rejected("equipment unknown backpack capacity"))?;
                if backpack_policy.source_digest() != authority.content_digest {
                    return Err(rejected("equipment backpack profile pin"));
                }
                if rows.len() >= usize::from(capacity).min(20) {
                    return Err(rejected("equipment backpack capacity"));
                }
                ordinal = Some(
                    rows.last()
                        .map(|r| number(r.try_get("placement_ordinal")?))
                        .transpose()?
                        .unwrap_or(0)
                        .checked_add(1)
                        .ok_or(rejected("equipment backpack ordinal exhausted"))?,
                );
            }
            item = Some(*identity);
            from = *from_slot;
            to = *to_slot;
            parent = Some(backpack_id);
            item_revision = Some(number(row.try_get("state_revision")?)?);
            1_i16
        }
    };
    let ids=sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id").fetch_one(&mut **tx).await?;
    let transaction_id = uuid(ids.try_get("transaction_id")?)?;
    let event_id = uuid(ids.try_get("event_id")?)?;
    sqlx::query("INSERT INTO game_character_equipment_receipts(transaction_id,event_id,character_id,game_session_id,command_id,connection_generation,lease_generation,world_id,channel_id,scope_generation,content_digest,revision_before,revision_after,mode_before,mode_after,operation,item_instance_id,state_revision_before,from_slot,to_slot,backpack_item_instance_id,backpack_ordinal,binding,intent) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,$12::text::numeric,$13::text::numeric,$14,$15,$16,CASE WHEN $17::bytea IS NULL THEN NULL ELSE encode($17,'hex')::uuid END,$18::text::numeric,$19,$20,CASE WHEN $21::bytea IS NULL THEN NULL ELSE encode($21,'hex')::uuid END,$22::text::numeric,$23,$24)")
        .bind(transaction_id.as_slice()).bind(event_id.as_slice()).bind(actual.character.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(authority.fence.connection_generation.get().to_string()).bind(authority.fence.character_lease_generation.to_string()).bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(authority.fence.scope_ownership_generation.get().to_string()).bind(authority.content_digest.as_slice()).bind(actual.revision.to_string()).bind(next.to_string()).bind(actual.combat_mode.map(CombatMode::code)).bind(mode.map(CombatMode::code)).bind(operation).bind(item.map(|v|v.to_vec())).bind(item_revision.map(|v|v.to_string())).bind(from.map(i16::from)).bind(to.map(i16::from)).bind(parent.map(|v|v.to_vec())).bind(ordinal.map(|v|v.to_string())).bind(&binding).bind(&binding_bytes).execute(&mut **tx).await?;
    if let Some(item) = item {
        if to.is_some() {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("DELETE FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid").bind(actual.character.as_slice()).bind(item.as_slice()).execute(&mut **tx).await?;
        }
        sqlx::query("UPDATE game_item_instances SET last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(transaction_id.as_slice()).bind(item.as_slice()).execute(&mut **tx).await?;
        if let Some(slot) = to {
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid)").bind(actual.character.as_slice()).bind(i16::from(slot)).bind(item.as_slice()).bind(authority.world.as_slice()).bind(transaction_id.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)").bind(actual.character.as_slice()).bind(parent.ok_or(rejected("equipment parent"))?.as_slice()).bind(item.as_slice()).bind(authority.world.as_slice()).bind(ordinal.ok_or(rejected("equipment ordinal"))?.to_string()).bind(transaction_id.as_slice()).execute(&mut **tx).await?;
        }
    }
    sqlx::query("UPDATE game_character_equipment_state SET revision=$1::text::numeric,combat_mode=$2,last_transaction_id=encode($3,'hex')::uuid WHERE character_id=encode($4,'hex')::uuid").bind(next.to_string()).bind(mode.map(CombatMode::code)).bind(transaction_id.as_slice()).bind(actual.character.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_audit_outbox(event_id,transaction_id,envelope) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)").bind(event_id.as_slice()).bind(transaction_id.as_slice()).bind(binding_bytes).execute(&mut **tx).await?;
    Ok(EquipmentReceipt {
        transaction_id,
        event_id,
        revision_after: next,
        replayed: false,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn source_combat_modes_require_explicit_supported_owner_code() {
        assert_eq!(
            CombatMode::from_code(1).expect("attack").attack_factor(),
            1.0
        );
        assert_eq!(
            CombatMode::from_code(2).expect("balanced").attack_factor(),
            0.75
        );
        assert_eq!(
            CombatMode::from_code(3).expect("defense").attack_factor(),
            0.5
        );
        for unknown in [0, 4, -1] {
            assert!(CombatMode::from_code(unknown).is_err());
        }
    }
    #[test]
    fn equipment_snapshot_equality_tracks_owner_item_and_content_revisions() {
        let original = EquipmentSnapshot {
            character: [1; 16],
            content_digest: [2; 32],
            character_revision: 4,
            revision: 7,
            combat_mode: None,
            items: vec![],
        };
        for changed_field in 0..4 {
            let mut changed = original.clone();
            match changed_field {
                0 => changed.character_revision += 1,
                1 => changed.revision += 1,
                2 => changed.content_digest = [3; 32],
                _ => changed.combat_mode = Some(CombatMode::Attack),
            }
            assert_ne!(original, changed);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RawCastDurableFacts {
    pub(crate) account: [u8; 16],
    pub(crate) equipment: EquipmentSnapshot,
    pub(crate) build: super::character_build::DurableBuildState,
    pub(crate) level: u32,
    pub(crate) command: CommandRef,
    pub(crate) wheel: Option<super::character_wheel::WheelSnapshot>,
    pub(crate) fence: CurrentCharacterItemFence,
}
fn stored_u16(row: &sqlx::postgres::PgRow, key: &str) -> Result<u16> {
    u16::try_from(row.try_get::<i32, _>(key)?).map_err(|_| rejected("corrupt Character skill"))
}
fn stored_u64(row: &sqlx::postgres::PgRow, key: &str) -> Result<u64> {
    u64::try_from(row.try_get::<i64, _>(key)?).map_err(|_| rejected("corrupt Character progress"))
}
pub(crate) async fn read_raw_cast_facts_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
) -> Result<RawCastDurableFacts> {
    let command = authority
        .command
        .ok_or_else(|| rejected("cast read needs actual command"))?;
    let raw = read_lifecycle_facts_in_transaction(tx, authority).await?;
    let wheel = super::character_wheel::read_optional_in_transaction(tx, &raw.equipment)
        .await
        .map_err(|_| rejected("current Wheel owner snapshot"))?;
    Ok(RawCastDurableFacts {
        wheel,
        account: raw.account,
        equipment: raw.equipment,
        build: raw.build,
        level: raw.level,
        fence: raw.fence,
        command,
    })
}
async fn read_lifecycle_facts_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
) -> Result<RawLifecycleDurableFacts> {
    let equipment = read_equipment_in_transaction(tx, authority).await?;
    let row=sqlx::query("SELECT vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(EquipmentError::Rejected("current classed Character build"))?;
    let skills = [
        ("fist_level", "fist_tries"),
        ("club_level", "club_tries"),
        ("sword_level", "sword_tries"),
        ("axe_level", "axe_tries"),
        ("distance_level", "distance_tries"),
        ("shielding_level", "shielding_tries"),
        ("fishing_level", "fishing_tries"),
    ]
    .map(|(level, tries)| Ok((stored_u16(&row, level)?, stored_u64(&row, tries)?)))
    .into_iter()
    .collect::<Result<Vec<_>>>()?;
    let build = super::character_build::DurableBuildState::new(
        row.try_get::<String, _>("vocation")?,
        (
            stored_u16(&row, "magic_level")?,
            stored_u64(&row, "mana_spent")?,
        ),
        skills
            .try_into()
            .map_err(|_| EquipmentError::Rejected("Character seven skills"))?,
    )
    .map_err(|_| EquipmentError::Rejected("invalid Character build"))?;
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(EquipmentError::Rejected("current Character level"))?;
    let level =
        u32::try_from(level).map_err(|_| EquipmentError::Rejected("Character level bound"))?;
    Ok(RawLifecycleDurableFacts {
        account: authority.account,
        equipment,
        build,
        level,
        fence: authority.fence,
    })
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_raw_cast_facts_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content_digest: [u8; 32],
) -> Result<RawCastDurableFacts> {
    let authority =
        assert_equipment_fence_in_transaction(tx, recovery, node, fence, command, content_digest)
            .await?;
    read_raw_cast_facts_in_transaction(tx, &authority).await
}
impl DurabilityRoot {
    pub(crate) async fn read_cast_durable_facts(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        command: CommandRef,
        content_digest: [u8; 32],
    ) -> Result<RawCastDurableFacts> {
        let record = recovery
            .record_for(self)
            .map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let facts = load_raw_cast_facts_in_transaction(
                        &mut tx,
                        &record,
                        &node,
                        &fence,
                        command,
                        content_digest,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(facts)
                })
            })
            .await
            .map_err(EquipmentError::from)
    }
}

/// Immutable SQL observations; no command or authority is fabricated for STEP/login/ticks.
#[derive(Debug, Clone)]
pub(crate) struct RawLifecycleDurableFacts {
    pub(crate) account: [u8; 16],
    pub(crate) equipment: EquipmentSnapshot,
    pub(crate) build: super::character_build::DurableBuildState,
    pub(crate) level: u32,
    pub(crate) fence: CurrentCharacterItemFence,
}
#[derive(Debug, Clone)]
pub(crate) struct MovementEquipmentSnapshot {
    fence: CurrentCharacterItemFence,
    equipment: EquipmentSnapshot,
}
impl MovementEquipmentSnapshot {
    pub(crate) fn fence(&self) -> &CurrentCharacterItemFence {
        &self.fence
    }
    pub(crate) fn equipment(&self) -> &EquipmentSnapshot {
        &self.equipment
    }
}
impl DurabilityRoot {
    pub(crate) async fn read_movement_equipment(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content_digest: [u8; 32],
    ) -> Result<MovementEquipmentSnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let authority = assert_current_equipment_in_transaction(
                        &mut tx,
                        &record,
                        &node,
                        &fence,
                        None,
                        content_digest,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let equipment = read_equipment_in_transaction(&mut tx, &authority)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(MovementEquipmentSnapshot { fence, equipment })
                })
            })
            .await
            .map_err(EquipmentError::from)
    }
    pub(crate) async fn read_lifecycle_durable_facts(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &super::character_progression::CurrentCharacterGameplayFence,
        content_digest: [u8; 32],
    ) -> Result<RawLifecycleDurableFacts> {
        let record = recovery
            .record_for(self)
            .map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let gameplay = *fence;
        let fence = CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let authority = assert_current_equipment_in_transaction(
                        &mut tx,
                        &record,
                        &node,
                        &fence,
                        None,
                        content_digest,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    if authority.character_revision != gameplay.expected_character_revision.get() {
                        return Err(DurabilityError::Unavailable);
                    }
                    let facts = read_lifecycle_facts_in_transaction(&mut tx, &authority)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(facts)
                })
            })
            .await
            .map_err(EquipmentError::from)
    }
}

impl DurabilityRoot {
    /// Legitimate first-entry owner bootstrap: proves the actual committed FreshAdmission
    /// receipt and independently current recovery/session/root/scope/Content in the same TX.
    /// Pre-equipment storage has only inventory/ground/house custody; no hand slot is inferred.
    pub(crate) async fn initialize_admission_equipment(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content_digest: [u8; 32],
        operation: &crate::foundation::fresh_admission_durability::FreshAdmissionOperationV1,
    ) -> Result<EquipmentSnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        let replay_key = operation.authorization.facts.replay_key().to_bytes();
        if operation.authorization.candidate_session != fence.game_session_id {
            return Err(rejected("equipment admission session binding"));
        }
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
            let authority=assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,None,content_digest)
                .await.map_err(|error| admission_equipment_unavailable("assert_current", error))?;
            let committed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_fresh_admission_receipts WHERE replay_key=$1 AND game_session_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND account_id=encode($4,'hex')::uuid AND world_id=encode($5,'hex')::uuid AND channel_id=encode($6,'hex')::uuid AND character_lease_generation=$7::text::numeric AND scope_ownership_generation=$8::text::numeric)")
                .bind(replay_key.as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fence.character_id.as_bytes().as_slice()).bind(authority.account.as_slice())
                .bind(authority.world.as_slice()).bind(authority.channel.as_slice())
                .bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
                .fetch_one(&mut *tx).await?;
            if !committed {return Err(admission_equipment_unavailable("admission_receipt", rejected("equipment committed admission receipt absent")));}
            initialize_equipment_in_transaction(&mut tx,&authority).await.map_err(|error| admission_equipment_unavailable("initialize", error))?;
            let equipment=read_equipment_in_transaction(&mut tx,&authority).await.map_err(|error| admission_equipment_unavailable("read", error))?;
            super::db::commit_semantic_transaction(tx,deadline).await?; Ok(equipment)
        })).await.map_err(EquipmentError::from)
    }
}
