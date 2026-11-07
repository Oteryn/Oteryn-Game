//! Temporary character-only contracts; the native NPC composes the existing audited fee owner.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, burn_fee_in_transaction,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
pub use oteryn_gameplay_world_change::horse_station as rules;
use serde::{Deserialize, Serialize};
use sqlx::Row;
#[derive(Clone, Debug)]
pub struct HorseRentalRequest {
    pub command: CommandRef,
    pub npc: String,
    pub fee: FeeBurnRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum HorseRentalOutcome {
    Rented { mount: u16, expires: i64 },
    AlreadyOwned,
    HorsesLoose,
    InsufficientGold,
}
mod earned;
mod world;
pub(crate) use earned::earned;
pub use world::{HorseStationChannel, HorseStationWorld};
fn binding(request: &HorseRentalRequest) -> Result<Vec<u8>, DurabilityError> {
    use sha2::{Digest, Sha256};
    let encoded = serde_json::to_vec(&(
        &request.npc,
        request.command.game_session_id().as_bytes(),
        request.command.command_id().get(),
        request.fee.transaction_id,
        request.fee.event_id,
    ))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Sha256::new()
        .chain_update(b"oteryn:horse-rental:v1\0")
        .chain_update(encoded)
        .finalize()
        .to_vec())
}
impl DurabilityRoot {
    /// Source rental and 500gold BURN are one Type2 transaction. Replayed history
    /// reproduces its original reply but never renews a deadline or publishes again.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent current Character, Node, command, item lane and active content proofs"
    )]
    pub async fn commit_horse_rental(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: HorseRentalRequest,
        permit: &SpellLanePermit,
        native: &NativeGameplayState,
    ) -> Result<HorseRentalOutcome, CharacterProgressionError> {
        if !rules::supported_npc(&request.npc)
            || request.command.game_session_id() != fence.game_session_id
            || request.fee.fee_gold_units != rules::RENTAL_GOLD
            || !matches!(request.fee.cause, FeeBurnCause::HorseRental { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (permit, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,(_,native)|Box::pin(async move{
            let native = *native;
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let request_binding = binding(&request)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:horse-rental:'||encode($1,'hex'),0))")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
            let prior:Option<(Vec<u8>,i32,i64)>=sqlx::query_as("SELECT request_binding,mount_id,expires_seconds FROM game_horse_rental_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
            .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((old, mount, expires)) = prior {
                if old != request_binding {
                    return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                }
                let mount = u16::try_from(mount).map_err(|_| DurabilityError::InvalidStoredState)?;
                if rules::Horse::from_source_id(mount).is_none() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                tx.commit(deadline).await?;
                return Ok(Ok(HorseRentalOutcome::Rented { mount, expires }));
            }
            match assert_current_equipment_in_transaction(
                &mut tx,
                &recovery,
                &node,
                &fence.item_fence(),
                Some(request.command),
                native.source_digest(),
            )
            .await
            {
                Ok(_) => {}
                Err(EquipmentError::Rejected(_)) => {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                }
                Err(EquipmentError::Database(e)) => return Err(e.into()),
                Err(EquipmentError::Durability(e)) => return Err(e),
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(e) => return Ok(Err(e)),
            };
            let progress=sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if super::character_progression::numeric_u64(&progress, "character_revision")? != root.revision
                || !super::character_progression::state_matches_root(&progress, &root)
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM public.game_daily_effect_now()))::bigint",
            )
            .fetch_one(&mut **tx)
            .await?;
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid AND expires_seconds >= $2)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(now).fetch_one(&mut **tx).await?;
            if active {
                tx.commit(deadline).await?;
                return Ok(Ok(HorseRentalOutcome::AlreadyOwned));
            }
            let (world, channel) = super::item_transfer::scope_of(&fence.item_fence())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:horse-station:'||encode($1,'hex'),0))")
            .bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
            let loose:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_horse_station_worlds WHERE world_id=encode($1,'hex')::uuid AND NOT ended AND expires_seconds>$2)")
            .bind(world.as_bytes().as_slice()).bind(now).fetch_one(&mut **tx).await?;
            if loose {
                tx.commit(deadline).await?;
                return Ok(Ok(HorseRentalOutcome::HorsesLoose));
            }
            let fact = node.fact();
            let row=sqlx::query("SELECT mount_id,expires_seconds FROM game_horse_rental_arm(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12,$13,encode($14,'hex')::uuid)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
            .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
            .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string())
            .bind(fence.character_lease_generation.to_string()).bind(request.command.command_id().get().to_string()).bind(&request.npc).bind(&request_binding)
            .bind(request.fee.transaction_id.as_slice()).fetch_one(&mut **tx).await?;
            let mount: i32 = row.try_get("mount_id")?;
            let expires: i64 = row.try_get("expires_seconds")?;
            let horse = rules::Horse::from_source_id(
                u16::try_from(mount).map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
            let cosmetics = native
                .cosmetics()
                .ok_or(DurabilityError::InvalidStoredState)?;
            use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
            let key = horse.key().to_owned();
            if cosmetics.earned_unlock(
                horse.grant(),
                rules::PROVENANCE,
                EarnedCosmetic::Mount(&key),
            ) != UnlockFact::Compatible
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let burned = match burn_fee_in_transaction(&mut tx, &fence, &request.fee).await {
                Ok(FeeBurnOutcome::Burned(fee)) => fee,
                Ok(FeeBurnOutcome::AlreadyBurned(_)) => return Err(DurabilityError::InvalidStoredState),
                Err(FeeBurnError::InsufficientFunds) => {
                    return Ok(Ok(HorseRentalOutcome::InsufficientGold));
                }
                Err(FeeBurnError::Unavailable(e)) => return Err(e),
                Err(_) => return Ok(Err(CharacterProgressionError::InvalidInput)),
            };
            if burned.committed_character_revision.get()
                != root
                    .revision
                    .checked_add(1)
                    .ok_or(DurabilityError::InvalidStoredState)?
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            sqlx::query("INSERT INTO game_horse_rental_receipts(transaction_id,character_id,world_id,game_session_id,command_id,npc_key,mount_id,issued_seconds,expires_seconds,fee_transaction_id,request_binding,committed_character_revision,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9,encode($1,'hex')::uuid,$10,$11::text::numeric,$12::text::numeric,$13,$13,$14,$14,$15,$16,$17,$18,$19,$20,$21,$22)")
            .bind(request.fee.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
            .bind(request.command.command_id().get().to_string()).bind(&request.npc).bind(mount).bind(expires-rules::RENTAL_SECONDS).bind(expires).bind(&request_binding).bind(burned.committed_character_revision.get().to_string())
            .bind(root.revision.to_string()).bind(progress.try_get::<i64,_>("level")?).bind(progress.try_get::<i64,_>("total_experience")?)
            .bind(progress.try_get::<String,_>("profile_revision")?).bind(progress.try_get::<String,_>("ruleset_revision")?)
            .bind(progress.try_get::<String,_>("content_revision")?).bind(progress.try_get::<String,_>("simulation_revision")?)
            .bind(progress.try_get::<String,_>("evidence_revision")?).bind(progress.try_get::<String,_>("declaration_revision")?)
            .bind(progress.try_get::<String,_>("policy_revision")?).bind(progress.try_get::<String,_>("reward_revision")?)
            .execute(&mut **tx).await?;
            super::account_achievement::grant_missing(
                &mut tx,
                fence.character_id,
                &["oteryn:achievement/natural_born_cowboy"],
                "npc",
                request.fee.transaction_id.as_slice(),
            )
            .await?;
            // The fee owns the one revision; its service receipt composes no second advancement.
            if sqlx::query("UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric")
            .bind(fence.character_id.as_bytes().as_slice()).bind(burned.committed_character_revision.get().to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected()!=1{return Err(DurabilityError::InvalidStoredState)}
            if sqlx::query("UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric")
            .bind(fence.character_id.as_bytes().as_slice()).bind(burned.committed_character_revision.get().to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected()!=1{return Err(DurabilityError::InvalidStoredState)}
            tx.commit(deadline).await?;
            Ok(Ok(HorseRentalOutcome::Rented {
                mount: horse.source_id(),
                expires,
            }))
        })).await?
    }
}

impl DurabilityRoot {
    pub async fn horse_station_rent_available(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: crate::foundation::WorldId,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let open:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_horse_station_worlds WHERE world_id=encode($1,'hex')::uuid AND NOT ended AND expires_seconds>floor(extract(epoch FROM public.game_daily_effect_now()))::bigint)").bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(open)
        })).await
    }
}

impl DurabilityRoot {
    pub async fn observe_horse_rental_expiry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: super::item_transfer::CurrentCharacterItemFence,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let fact = node.fact();
            let fresh:bool=sqlx::query_scalar("SELECT game_horse_rental_arm_notice(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::text::numeric,$6::text::numeric,encode($7,'hex')::uuid,$8::text::numeric,$9)")
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
            .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
            .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).fetch_one(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(fresh)
        })).await
    }
}
