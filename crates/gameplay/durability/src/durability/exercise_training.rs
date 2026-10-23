//! Atomic EXERCISE0 checkpoint: existing timed-item owner plus the native build writer.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_build::{
    self, BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
};
use super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::item_timed_state::{
    self, TimedCheckpointRequest, TimedDefinitionFacts, TimedExpiryRequest,
};
use super::loop_training::{TrainingContext, load_build};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::character_build::BuildFormula;
use crate::domain::{
    CharacterRevision,
    timed_item::{ExpireReason, TimedValues},
};
use oteryn_gameplay_progression::loop_training::add_gain;
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingWeapon {
    pub revision: u64,
    pub charges: u32,
}
#[derive(Debug, Clone)]
pub struct ExerciseCheckpoint {
    pub occurrence: BuildOccurrence,
    pub event_id: [u8; 16],
    pub weapon: [u8; 16],
    pub definition: TimedDefinitionFacts,
    pub before: TrainingWeapon,
    pub spent: u32,
    pub family: usize,
}
#[derive(Debug, Clone)]
pub struct ExerciseCommitted {
    pub revision: CharacterRevision,
    pub build: DurableBuildState,
    pub weapon: TrainingWeapon,
}
fn unavailable<T>(_: T) -> DurabilityError {
    DurabilityError::Unavailable
}

impl DurabilityRoot {
    /// Read a current held weapon through the same fences as its later checkpoint.
    pub async fn read_training_weapon(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        weapon: [u8; 16],
        definition: TimedDefinitionFacts,
    ) -> Result<TrainingWeapon, DurabilityError> {
        let recovery = authority.record_for(self).map_err(unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| { Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(unavailable)?;
            let held:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid \
                     AND i.lifecycle=1 AND i.definition_family=$3 AND i.definition_production_key=$4 AND \
                     i.definition_revision_ref=$5 AND (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE \
                     e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid)))")
                .bind(weapon.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&definition.definition.family)
                .bind(&definition.definition.production_key).bind(&definition.definition.revision_ref).fetch_one(&mut *tx).await?;
            if !held || definition.full.remaining_ms().is_some() {return Err(DurabilityError::Unavailable);}
            let state=item_timed_state::load_timed_state(&mut tx,&weapon).await.map_err(unavailable)?;
            let result=match state {
                Some(state) if state.deadline_at.is_none()=>TrainingWeapon {revision:state.revision,charges:state.values.and_then(|v|v.charges()).ok_or(DurabilityError::Unavailable)?},
                None=>TrainingWeapon {revision:0,charges:definition.full.charges().ok_or(DurabilityError::Unavailable)?},
                _=>return Err(DurabilityError::Unavailable),
            };
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        }) }).await
    }

    /// A frozen checkpoint is retained across unknown COMMIT outcomes; exact replay returns the
    /// original build/charge receipt before fresh fence or clock checks. No split charge write.
    pub async fn commit_exercise_training(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        context: TrainingContext,
        request: ExerciseCheckpoint,
    ) -> Result<ExerciseCommitted, DurabilityError> {
        let recovery = authority.record_for(self).map_err(unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| { Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let binding=Sha256::new().chain_update(request.weapon).chain_update(request.before.revision.to_be_bytes())
                .chain_update(request.before.charges.to_be_bytes()).chain_update(request.spent.to_be_bytes())
                .chain_update(u64::try_from(request.family).map_err(unavailable)?.to_be_bytes())
                .chain_update(fence.character_id.as_bytes()).chain_update(context.formula.digest())
                .chain_update(request.definition.definition.family.as_bytes()).chain_update([0])
                .chain_update(request.definition.definition.production_key.as_bytes()).chain_update([0])
                .chain_update(request.definition.definition.revision_ref.as_bytes()).chain_update([0])
                .chain_update(request.definition.full.charges().unwrap_or(0).to_be_bytes())
                .chain_update(if request.spent==request.before.charges {request.event_id} else {[0;16]})
                .chain_update(serde_json::to_vec(&*context.rates).map_err(unavailable)?).finalize().to_vec();
            let prior=sqlx::query("SELECT binding,committed_character_revision::text,charges_after,weapon_revision::text FROM \
                     game_character_exercise_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid")
                .bind(request.occurrence.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if let Some(row)=prior {
                if row.try_get::<Vec<u8>,_>("binding")?!=binding {return Err(DurabilityError::Unavailable);}
                // Reload this immutable occurrence, never a newer mutable build projection.
                let stored=sqlx::query("SELECT vocation_after AS vocation,magic_level_after AS magic_level,mana_spent_after AS \
                     mana_spent,fist_level_after AS fist_level,fist_tries_after AS fist_tries,club_level_after AS \
                     club_level,club_tries_after AS club_tries,sword_level_after AS sword_level,sword_tries_after AS \
                     sword_tries,axe_level_after AS axe_level,axe_tries_after AS axe_tries,distance_level_after AS \
                     distance_level,distance_tries_after AS distance_tries,shielding_level_after AS \
                     shielding_level,shielding_tries_after AS shielding_tries,fishing_level_after AS \
                     fishing_level,fishing_tries_after AS fishing_tries FROM game_character_build_receipts WHERE \
                     build_occurrence_id=encode($1,'hex')::uuid")
                    .bind(request.occurrence.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                let result=ExerciseCommitted {revision:CharacterRevision::new(row.try_get::<String,_>("committed_character_revision")?.parse().map_err(unavailable)?).map_err(unavailable)?,build:DurableBuildState::stored(&stored,"vocation", "")?,weapon:TrainingWeapon {revision:row.try_get::<String,_>("weapon_revision")?.parse().map_err(unavailable)?,charges:u32::try_from(row.try_get::<i32,_>("charges_after")?).map_err(unavailable)?}};
                tx.commit(deadline).await?;return Ok(result);
            }
            if !context.rates.enabled || request.family>6 || request.spent==0 || request.spent>request.before.charges
                || request.definition.full.remaining_ms().is_some() {return Err(DurabilityError::Unavailable);}
            assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(unavailable)?;
            let before=load_build(&mut tx,fence.character_id).await?;
            let gain=context.rates.exercise_gains[request.family].checked_mul(u64::from(request.spent)).ok_or(DurabilityError::Unavailable)?;
            let after=add_gain(&before,&context.formula,request.family,gain).map_err(unavailable)?;
            let remaining=request.before.charges-request.spent;
            let state=item_timed_state::load_timed_state(&mut tx,&request.weapon).await.map_err(unavailable)?;
            let current: Result<TrainingWeapon, DurabilityError> = state.map_or_else(||Ok(TrainingWeapon {revision:0,charges:request.definition.full.charges().ok_or(DurabilityError::Unavailable)?}),|state|Ok(TrainingWeapon {revision:state.revision,charges:state.values.and_then(|v|v.charges()).ok_or(DurabilityError::Unavailable)?}));
            if current?!=request.before {return Err(DurabilityError::Unavailable);}
            // Root lock precedes both participants. A semantic refusal drops the whole TX.
            if remaining>0 {
                let checkpoint=TimedCheckpointRequest {item_instance_id:request.weapon,expected_revision:request.before.revision,transaction_id:*request.occurrence.as_bytes(),definition:request.definition.clone(),values:TimedValues::new(Some(remaining),None).map_err(unavailable)?};
                item_timed_state::validate_checkpoint(&checkpoint).map_err(unavailable)?;
                item_timed_state::write_checkpoint(&mut tx,fence.character_id,&checkpoint).await.map_err(unavailable)?;
            } else {
                let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
                let expiry=TimedExpiryRequest {item_instance_id:request.weapon,expected_revision:request.before.revision,transaction_id:*request.occurrence.as_bytes(),event_id:request.event_id,reason:ExpireReason::ChargesExhausted,definition:request.definition.clone(),target:None,occurred_at_unix_ms:now,server_build_id:super::character_authority::SERVER_BUILD_ID.to_owned()};
                item_timed_state::validate_expiry(&expiry).map_err(unavailable)?;
                let tuple=tx.tuple();
                item_timed_state::write_expiry(&mut tx,&fence,&expiry,tuple).await.map_err(unavailable)?;
            }
            let mut build_binding=vec![2];build_binding.extend_from_slice(&binding);
            let outcome=character_build::apply_build_in_transaction(&mut tx,&node,&fence,BuildChangeRequest {occurrence:request.occurrence,cause:BuildCause::Training,before,after:after.clone(),pruned_stance:None},context.formula.digest(),context.formula.content_revision().to_owned(),build_binding,None).await?.map_err(unavailable)?;
            let revision=match outcome {character_build::BuildCommitOutcome::Committed(receipt)|character_build::BuildCommitOutcome::AlreadyCommitted(receipt)=>receipt.committed_character_revision};
            let weapon_revision=if remaining==0 {request.before.revision} else {request.before.revision.checked_add(1).ok_or(DurabilityError::Unavailable)?};
            sqlx::query("INSERT INTO \
                     game_character_exercise_receipts(build_occurrence_id,character_id,game_session_id,item_instance_id,binding,committed_character_revision,charges_before,charges_after,weapon_revision,family,gain,gain_per_charge,expected_weapon_revision) \
                     VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6::text::numeric,$7,$8, \
                     $9::text::numeric,$10,$11,$12,$13::text::numeric)")
                .bind(request.occurrence.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.weapon.as_slice()).bind(binding)
                .bind(revision.get().to_string()).bind(i32::try_from(request.before.charges).map_err(unavailable)?).bind(i32::try_from(remaining).map_err(unavailable)?)
                .bind(weapon_revision.to_string()).bind(i16::try_from(request.family).map_err(unavailable)?).bind(i64::try_from(gain).map_err(unavailable)?).bind(i64::try_from(context.rates.exercise_gains[request.family]).map_err(unavailable)?).bind(request.before.revision.to_string()).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(ExerciseCommitted {revision,build:after,weapon:TrainingWeapon {revision:weapon_revision,charges:remaining}})
        }) }).await
    }
}
