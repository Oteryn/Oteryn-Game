//! Real Character Wheel allocation owner. Unknown external quest/gem modifiers are refused;
//! allocations, current revision, source Content pin and writer receipts live in PostgreSQL.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::{EquipmentError, EquipmentSnapshot};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_premium_abi::{AuthenticatedSnapshot, Classification, TrustedTime};
use super::spell_wheel_abi::CompiledWheelProfile;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug)]
pub(crate) enum WheelError {
    Durability(DurabilityError),
    Equipment(EquipmentError),
    Database(sqlx::Error),
    Rejected(&'static str),
}
impl From<DurabilityError> for WheelError {
    fn from(e: DurabilityError) -> Self {
        Self::Durability(e)
    }
}
impl From<EquipmentError> for WheelError {
    fn from(e: EquipmentError) -> Self {
        Self::Equipment(e)
    }
}
impl From<sqlx::Error> for WheelError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
type Result<T> = std::result::Result<T, WheelError>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WheelSnapshot {
    pub(crate) character: [u8; 16],
    pub(crate) character_revision: u64,
    pub(crate) content_digest: [u8; 32],
    pub(crate) revision: u64,
    pub(crate) allocation: [u16; 36],
    pub(crate) extra_points: u16,
    pub(crate) maximum_grade_modifier: u8,
    pub(crate) revelation_bonus: [u16; 4],
}
#[derive(Debug)]
pub(crate) struct WheelReceipt {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) revision_after: u64,
    pub(crate) replayed: bool,
}
fn reject(s: &'static str) -> WheelError {
    WheelError::Rejected(s)
}
fn number(s: String) -> Result<u64> {
    s.parse().map_err(|_| reject("stored Wheel revision"))
}
fn uuid(s: String) -> Result<[u8; 16]> {
    let text = s.replace('-', "");
    if text.len() != 32 || !text.is_ascii() {
        return Err(reject("stored Wheel UUID"));
    }
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
            .map_err(|_| reject("stored Wheel UUID"))?;
    }
    Ok(out)
}
pub(super) async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    equipment: &EquipmentSnapshot,
) -> Result<WheelSnapshot> {
    let row=sqlx::query("SELECT revision::text,content_digest,allocation,extra_points,maximum_grade_modifier,revelation_bonus,selected_gems FROM game_character_wheel_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
  .bind(equipment.character.as_slice()).fetch_optional(&mut **tx).await?.ok_or_else(||reject("Wheel owner not initialized"))?;
    let digest: Vec<u8> = row.try_get("content_digest")?;
    if digest != equipment.content_digest
        || row.try_get::<serde_json::Value, _>("selected_gems")? != serde_json::json!([])
    {
        return Err(reject("Wheel active Content/unsupported gem state"));
    }
    let allocation: Vec<i32> = row.try_get("allocation")?;
    let bonus: Vec<i32> = row.try_get("revelation_bonus")?;
    let convert = |values: Vec<i32>| -> Result<Vec<u16>> {
        values
            .into_iter()
            .map(|v| u16::try_from(v).map_err(|_| reject("Wheel points bound")))
            .collect()
    };
    Ok(WheelSnapshot {
        character: equipment.character,
        character_revision: equipment.character_revision,
        content_digest: equipment.content_digest,
        revision: number(row.try_get("revision")?)?,
        allocation: convert(allocation)?
            .try_into()
            .map_err(|_| reject("Wheel36slots"))?,
        extra_points: u16::try_from(row.try_get::<i32, _>("extra_points")?)
            .map_err(|_| reject("Wheel extra point bound"))?,
        maximum_grade_modifier: u8::try_from(row.try_get::<i32, _>("maximum_grade_modifier")?)
            .map_err(|_| reject("Wheel modifier bound"))?,
        revelation_bonus: convert(bonus)?
            .try_into()
            .map_err(|_| reject("Wheel4colours"))?,
    })
}
pub(super) async fn read_optional_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    equipment: &EquipmentSnapshot,
) -> Result<Option<WheelSnapshot>> {
    let pin:Option<Vec<u8>>=sqlx::query_scalar("SELECT content_digest FROM game_character_wheel_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(equipment.character.as_slice()).fetch_optional(&mut **tx).await?;
    // A previous Content pin does not qualify this Wheel, while unrelated spells
    // can consume their independently complete build/equipment observations.
    if pin.as_deref() != Some(equipment.content_digest.as_slice()) {
        return Ok(None);
    }
    read_in_transaction(tx, equipment).await.map(Some)
}
/// This writer does not grant Premium. The genuinely authenticated fresh proof must still
/// match the committed account source high-water/fingerprint before source allocation rules run.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_wheel_assignment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    profile: &CompiledWheelProfile,
    expected: &WheelSnapshot,
    next: [u16; 36],
    premium: &AuthenticatedSnapshot,
    time: TrustedTime,
) -> Result<WheelReceipt> {
    let auth = super::character_equipment::assert_equipment_authority_in_transaction(
        tx,
        root,
        recovery,
        node,
        fence,
        command,
        profile.source_digest(),
    )
    .await?;
    let raw = super::character_equipment::read_raw_cast_facts_in_transaction(tx, &auth).await?;
    let current = read_in_transaction(tx, &raw.equipment).await?;
    let intent=serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_WHEEL_ASSIGNMENT/v1","character":expected.character,"character_revision":expected.character_revision,"content_digest":expected.content_digest,"revision":expected.revision,"allocation":next.to_vec()})).map_err(|_|reject("Wheel intent"))?;
    let binding: Vec<u8> = Sha256::digest(&intent).to_vec();
    let retained=sqlx::query("SELECT transaction_id::text,event_id::text,revision_after::text,binding,intent FROM game_character_wheel_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
  .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    if let Some(row) = retained {
        if row.try_get::<Vec<u8>, _>("binding")? != binding
            || row.try_get::<Vec<u8>, _>("intent")? != intent
        {
            return Err(reject("substituted Wheel retry"));
        }
        return Ok(WheelReceipt {
            transaction_id: uuid(row.try_get("transaction_id")?)?,
            event_id: uuid(row.try_get("event_id")?)?,
            revision_after: number(row.try_get("revision_after")?)?,
            replayed: true,
        });
    }
    if &current != expected {
        return Err(reject("Wheel full current owner changed"));
    }
    if premium.account() != raw.account
        || premium.classification(time, false, true) != Classification::Current
    {
        return Err(reject("Wheel needs current authenticated Premium"));
    }
    let account=sqlx::query("SELECT source_authority,authority_revision::text,fingerprint,conflicting FROM game_spell_premium_accounts WHERE account_id=encode($1,'hex')::uuid FOR SHARE")
  .bind(raw.account.as_slice()).fetch_optional(&mut **tx).await?.ok_or_else(||reject("Wheel Premium fence missing"))?;
    if account.try_get::<String, _>("source_authority")? != premium.source()
        || number(account.try_get("authority_revision")?)? != premium.authority_revision()
        || account.try_get::<Vec<u8>, _>("fingerprint")? != premium.fingerprint()
        || account.try_get::<bool, _>("conflicting")?
    {
        return Err(reject("Wheel Premium fence changed"));
    }
    let vocation = raw.build.vocation();
    if !matches!(
        vocation,
        "elite_knight" | "royal_paladin" | "elder_druid" | "master_sorcerer" | "exalted_monk"
    ) {
        return Err(reject("Wheel requires promoted vocation"));
    }
    // Monk shrine quest points have an independent owner which this allocation owner cannot grant.
    if vocation == "exalted_monk" {
        return Err(reject("Monk shrine bonus owner unavailable"));
    }
    let budget = profile
        .available_points(
            raw.level,
            Some(current.extra_points),
            Some(current.maximum_grade_modifier),
        )
        .map_err(reject)?;
    let planned = profile
        .plan_assignment(current.allocation, next, budget)
        .map_err(reject)?;
    let revision = current
        .revision
        .checked_add(1)
        .ok_or_else(|| reject("Wheel revision exhausted"))?;
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    let event: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    let (world, channel) =
        super::item_transfer::scope_of(fence).map_err(|_| reject("Wheel Channel scope"))?;
    sqlx::query("INSERT INTO game_character_wheel_receipts(transaction_id,event_id,character_id,game_session_id,command_id,character_revision,connection_generation,lease_generation,world_id,channel_id,ownership_generation,content_digest,revision_before,revision_after,allocation_before,allocation_after,premium_account_id,premium_authority_revision,binding,intent) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8::text::numeric,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13::text::numeric,$14::text::numeric,$15,$16,encode($17,'hex')::uuid,$18::text::numeric,$19,$20)")
  .bind(&transaction).bind(&event).bind(current.character.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
  .bind(current.character_revision.to_string()).bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).bind(current.content_digest.as_slice())
  .bind(current.revision.to_string()).bind(revision.to_string()).bind(current.allocation.iter().map(|v|i32::from(*v)).collect::<Vec<_>>()).bind(planned.iter().map(|v|i32::from(*v)).collect::<Vec<_>>())
  .bind(raw.account.as_slice()).bind(premium.authority_revision().to_string()).bind(&binding).bind(&intent).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_character_wheel_state SET revision=$1::text::numeric,allocation=$2,last_transaction_id=$3::uuid WHERE character_id=encode($4,'hex')::uuid")
  .bind(revision.to_string()).bind(planned.iter().map(|v|i32::from(*v)).collect::<Vec<_>>()).bind(&transaction).bind(current.character.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_wheel_audit_outbox(event_id,transaction_id,envelope) VALUES($1::uuid,$2::uuid,$3)").bind(&event).bind(&transaction).bind(&intent).execute(&mut **tx).await?;
    Ok(WheelReceipt {
        transaction_id: uuid(transaction)?,
        event_id: uuid(event)?,
        revision_after: revision,
        replayed: false,
    })
}

impl DurabilityRoot {
    pub(crate) async fn read_wheel_snapshot(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content_digest: [u8; 32],
    ) -> Result<WheelSnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("Wheel recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let auth = super::character_equipment::assert_current_equipment_in_transaction(
                        &mut tx,
                        &record,
                        &node,
                        &fence,
                        None,
                        content_digest,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let equipment =
                        super::character_equipment::read_equipment_in_transaction(&mut tx, &auth)
                            .await
                            .map_err(|_| DurabilityError::Unavailable)?;
                    let snapshot = read_in_transaction(&mut tx, &equipment)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(snapshot)
                })
            })
            .await
            .map_err(WheelError::from)
    }
    pub(crate) async fn initialize_admission_wheel(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        profile: &CompiledWheelProfile,
        operation: &crate::foundation::fresh_admission_durability::FreshAdmissionOperationV1,
    ) -> Result<WheelSnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("Wheel recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        let digest = profile.source_digest();
        let replay_key = operation.authorization.facts.replay_key().to_bytes();
        if operation.authorization.candidate_session != fence.game_session_id {
            return Err(reject("Wheel admission session"));
        }
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move{
   let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
   let auth=super::character_equipment::assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,None,digest).await.map_err(|_|DurabilityError::Unavailable)?;
   let equipment=super::character_equipment::read_equipment_in_transaction(&mut tx,&auth).await.map_err(|_|DurabilityError::Unavailable)?;
   let committed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_fresh_admission_receipts WHERE replay_key=$1 AND game_session_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND character_lease_generation=$4::text::numeric AND scope_ownership_generation=$5::text::numeric)")
    .bind(replay_key.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string()).fetch_one(&mut *tx).await?;
   if !committed{return Err(DurabilityError::Unavailable);}
   sqlx::query("INSERT INTO game_character_wheel_state(character_id,revision,content_digest,allocation,extra_points,maximum_grade_modifier,revelation_bonus,selected_gems) VALUES(encode($1,'hex')::uuid,1,$2,array_fill(0,ARRAY[36]),0,0,ARRAY[0,0,0,0],'[]'::jsonb) ON CONFLICT(character_id) DO NOTHING")
    .bind(equipment.character.as_slice()).bind(digest.as_slice()).execute(&mut *tx).await?;
   let snapshot=read_in_transaction(&mut tx,&equipment).await.map_err(|_|DurabilityError::Unavailable)?;
   super::db::commit_semantic_transaction(tx,deadline).await?;Ok(snapshot)
  })).await.map_err(WheelError::from)
    }
}
