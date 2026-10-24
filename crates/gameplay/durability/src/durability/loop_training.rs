//! LOOP-TRAIN-1: DB-clock markers and native build participant in one physical transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_build::{
    self, BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
};
use super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::offline_training_state::{self, OfflineChange, OfflineTrainingState};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::content::training_formula::CompiledTrainingFormula;
use crate::domain::character_build::BuildFormula;
use crate::domain::{CharacterId, CharacterRevision};
use oteryn_gameplay_progression::loop_training::{TrainingRates, add_gain, plan_offline};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;

#[derive(Clone)]
pub struct TrainingContext {
    pub rates: Arc<TrainingRates>,
    pub formula: CompiledTrainingFormula,
    pub progression:
        Option<crate::content::character_progression_content::CharacterProgressionContent>,
}

/// Only the current fenced logout owner supplies this participant to terminal release.
#[derive(Clone)]
pub struct TrainingLogout {
    pub context: TrainingContext,
    pub fence: CurrentCharacterGameplayFence,
    pub node: NodeIncarnationProof,
    pub selected: Option<usize>,
    recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainingSettlement {
    pub revision: CharacterRevision,
    pub offline: OfflineTrainingState,
}

fn unavailable<T>(_: T) -> DurabilityError {
    DurabilityError::Unavailable
}

pub(super) fn occurrence(
    character: CharacterId,
    lease: u64,
    cause: &str,
) -> Result<BuildOccurrence, DurabilityError> {
    let mut bytes: [u8; 16] = Sha256::new()
        .chain_update(b"oteryn:loop-training:v1\0")
        .chain_update(character.as_bytes())
        .chain_update(lease.to_be_bytes())
        .chain_update(cause.as_bytes())
        .finalize()[..16]
        .try_into()
        .map_err(unavailable)?;
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    BuildOccurrence::from_bytes(bytes).map_err(unavailable)
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    cause: &str,
) -> Result<Option<TrainingSettlement>, DurabilityError> {
    if fence.character_lease_generation == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let row = sqlx::query("SELECT b.original_character_revision::text,b.committed_character_revision::text, \
                     b.stamina_minutes_after AS stamina_minutes,b.offline_pool_s_after AS \
                     offline_pool_s,b.offline_skill_after AS offline_skill FROM game_character_training_markers m JOIN \
                     game_character_build_receipts b USING(build_occurrence_id) WHERE \
                     m.character_id=encode($1,'hex')::uuid AND m.lease_generation=$2::text::numeric AND m.cause=$3 AND \
                     m.game_session_id=encode($4,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).bind(cause).bind(fence.game_session_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let original: u64 = row
            .try_get::<String, _>("original_character_revision")?
            .parse()
            .map_err(unavailable)?;
        let committed: u64 = row
            .try_get::<String, _>("committed_character_revision")?
            .parse()
            .map_err(unavailable)?;
        let offline = OfflineTrainingState::decode(&row)?;
        if original.checked_add(1) != Some(committed) || !offline.valid() {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(TrainingSettlement {
            revision: CharacterRevision::new(committed).map_err(unavailable)?,
            offline,
        })
    })
    .transpose()
}

pub(super) async fn load_build(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> Result<DurableBuildState, DurabilityError> {
    sqlx::query("SELECT * FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
        .map(|row| DurableBuildState::stored(&row,"vocation", "")).transpose().map(|state| state.unwrap_or_default())
}

async fn insert_marker(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    cause: &str,
    selected: Option<usize>,
    occurrence: BuildOccurrence,
) -> Result<i64, DurabilityError> {
    sqlx::query_scalar("INSERT INTO \
                     game_character_training_markers(character_id,game_session_id,lease_generation,cause,build_occurrence_id,selected_skill,committed_at_us,as_of_us) \
                     VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,$6,0,0) \
                     RETURNING committed_at_us")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.character_lease_generation.to_string()).bind(cause).bind(occurrence.as_bytes().as_slice())
        .bind(selected.map_or(Ok(-1),i16::try_from).map_err(unavailable)?)
        .fetch_one(&mut **tx).await.map_err(Into::into)
}

async fn write_build(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    context: &TrainingContext,
    request: BuildChangeRequest,
    offline: OfflineChange,
) -> Result<TrainingSettlement, DurabilityError> {
    let binding = Sha256::new()
        .chain_update(b"oteryn:offline-build:v2\0")
        .chain_update(request.occurrence.as_bytes())
        .chain_update(context.formula.digest())
        .chain_update(serde_json::to_vec(&*context.rates).map_err(unavailable)?)
        .chain_update(fence.expected_character_revision.get().to_be_bytes())
        .chain_update(offline.before.stamina_minutes.to_be_bytes())
        .chain_update(offline.before.pool_seconds.to_be_bytes())
        .chain_update(offline.before.skill_code()?.to_be_bytes())
        .chain_update(offline.after.stamina_minutes.to_be_bytes())
        .chain_update(offline.after.pool_seconds.to_be_bytes())
        .chain_update(offline.after.skill_code()?.to_be_bytes())
        .finalize();
    let mut versioned = vec![2];
    versioned.extend_from_slice(&binding);
    let outcome = character_build::apply_build_in_transaction(
        tx,
        node,
        &fence,
        request,
        context.formula.digest(),
        context.formula.content_revision().to_owned(),
        versioned,
        Some(offline),
    )
    .await?
    .map_err(unavailable)?;
    let revision = match outcome {
        character_build::BuildCommitOutcome::Committed(receipt)
        | character_build::BuildCommitOutcome::AlreadyCommitted(receipt) => {
            receipt.committed_character_revision
        }
    };
    Ok(TrainingSettlement {
        revision,
        offline: offline.after,
    })
}

impl DurabilityRoot {
    /// Historical completion only; this receipt never reacquires session authority.
    pub async fn read_training_logout(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<TrainingSettlement>, DurabilityError> {
        let recovery = authority.record_for(self).map_err(unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = replay(&mut tx, fence, "logout").await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
    /// Admission after progression/respawn, before loading live build values. First marker wins.
    pub async fn settle_offline_training(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        context: TrainingContext,
    ) -> Result<TrainingSettlement, DurabilityError> {
        let recovery = authority.record_for(self).map_err(unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| { Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Some(prior)=replay(&mut tx,fence,"offline_settlement").await? {
                commit_semantic_transaction(tx,deadline).await?; return Ok(prior);
            }
            if !context.rates.enabled || fence.character_lease_generation==0 { return Err(DurabilityError::Unavailable); }
            assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(unavailable)?;
            let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if pending { return Err(DurabilityError::Unavailable); }
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !exists {
                let root=assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(unavailable)?;
                let policy=context.progression.as_ref().ok_or(DurabilityError::Unavailable)?.policy_for(&root.profile_revision,&root.ruleset_revision,&root.content_revision).map_err(unavailable)?;
                let request=super::character_progression::ProgressionInitializationRequest {context:policy.context.clone(),policy_revision:policy.policy_revision.clone(),reward_revision:policy.reward_revision.clone(),policy};
                super::character_progression::initialize_in_transaction(&mut tx,&node,fence,&request).await?.map_err(unavailable)?;
            }
            let before=load_build(&mut tx,fence.character_id).await?;
            let offline=offline_training_state::load(&mut tx,fence.character_id.as_bytes()).await?;
            // The immediate predecessor is an explicit immutable DB-stamped session relation.
            // The latest build-carrying receipt must be that predecessor's logout marker.
            let elapsed:Option<i64>=sqlx::query_scalar("SELECT greatest(0,(s.training_created_at_us-m.committed_at_us)/1000000)::bigint FROM \
                     game_durability_reconnect_sessions s JOIN game_durability_reconnect_sessions p ON \
                     p.game_session_id=s.training_predecessor_session JOIN game_character_training_markers m ON \
                     m.game_session_id=p.game_session_id AND m.character_id=p.character_id AND \
                     m.lease_generation=p.character_lease_generation AND m.cause='logout' JOIN game_character_build_state \
                     b ON b.character_id=s.character_id AND b.last_build_occurrence_id=m.build_occurrence_id WHERE \
                     s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND \
                     s.training_created_at_us IS NOT NULL AND p.session_state=3 AND p.world_id=s.world_id AND \
                     m.committed_at_us<=s.training_created_at_us")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let elapsed=u64::try_from(elapsed.unwrap_or(0)).map_err(unavailable)?;
            let plan=plan_offline(offline.pool_seconds,offline.skill,elapsed).map_err(unavailable)?;
            let mut after=before.clone();
            if let Some(family)=offline.skill.filter(|_| plan.training_seconds>=60) {
                let gain=context.rates.offline_gain(before.vocation(),family,plan.training_seconds).map_err(unavailable)?;
                if gain>0 {
                    after=add_gain(&before,&context.formula,family,gain).map_err(unavailable)?;
                    let old=if family==0 { before.magic() } else { before.skills()[family-1] };
                    let new=if family==0 { after.magic() } else { after.skills()[family-1] };
                    let percent=|pair:(u16,u64)| -> Result<u64,DurabilityError> {
                        let required=context.formula.required(before.vocation(),family,pair.0+1).ok_or(DurabilityError::Unavailable)?;
                        u64::try_from(u128::from(pair.1)*u128::from(context.rates.percent_scale)/u128::from(required.max(1))).map_err(unavailable)
                    };
                    if old.0!=new.0 || percent(old)?!=percent(new)? {
                        after=add_gain(&after,&context.formula,6,u64::from(plan.training_seconds/context.rates.offline_divisors[6])).map_err(unavailable)?;
                    }
                }
            }
            let stamina=oteryn_gameplay_characters::stamina::regenerate_stamina_minutes(u32::from(offline.stamina_minutes),elapsed).map_err(unavailable)?;
            let after_offline=OfflineTrainingState { stamina_minutes:u16::try_from(stamina).map_err(unavailable)?, pool_seconds:plan.pool_after,skill:None };
            let occurrence=occurrence(fence.character_id,fence.character_lease_generation,"offline_settlement")?;
            insert_marker(&mut tx,fence,"offline_settlement",None,occurrence).await?;
            let result=write_build(&mut tx,&node,fence,&context,BuildChangeRequest { occurrence,cause:BuildCause::OfflineSettlement,before,after,pruned_stance:None },OfflineChange {before:offline,after:after_offline}).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        }) }).await
    }
}

impl TrainingLogout {
    pub fn new(
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        context: TrainingContext,
        selected: Option<usize>,
    ) -> Result<Self, DurabilityError> {
        Ok(Self {
            context,
            fence,
            node: node.clone(),
            selected,
            recovery: authority.record_for(root).map_err(unavailable)?,
        })
    }
    pub(super) async fn prepare(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), DurabilityError> {
        assert_recovery_fence(tx, &self.recovery).await?;
        if replay(tx, self.fence, "logout").await?.is_some() {
            return Ok(());
        }
        if !self.context.rates.enabled || self.selected.is_some_and(|family| family > 5) {
            return Err(DurabilityError::Unavailable);
        }
        assert_gameplay_fence(tx, &self.fence, &self.node)
            .await?
            .map_err(unavailable)?;
        if self.selected.is_some()
            && super::pvp_state::load_pvp_state(tx, self.fence.character_id)
                .await?
                .is_some_and(|state| state.readmission_clock.has_running_lock())
        {
            return Err(DurabilityError::Unavailable);
        }
        let before = load_build(tx, self.fence.character_id).await?;
        let offline = offline_training_state::load(tx, self.fence.character_id.as_bytes()).await?;
        let occurrence = occurrence(
            self.fence.character_id,
            self.fence.character_lease_generation,
            "logout",
        )?;
        let now = insert_marker(tx, self.fence, "logout", self.selected, occurrence).await?;
        let created:Option<i64>=sqlx::query_scalar("SELECT s.training_created_at_us FROM game_durability_reconnect_sessions s WHERE \
                     s.game_session_id=encode($1,'hex')::uuid AND EXISTS(SELECT 1 FROM game_character_training_markers m \
                     WHERE m.game_session_id=s.game_session_id AND m.cause='offline_settlement')")
            .bind(self.fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.flatten();
        let online = created.map_or(0, |created| now.saturating_sub(created).max(0) / 1_000_000);
        let after = OfflineTrainingState {
            pool_seconds: u32::try_from((i64::from(offline.pool_seconds) + online).min(43200))
                .map_err(unavailable)?,
            skill: self.selected,
            ..offline
        };
        write_build(
            tx,
            &self.node,
            self.fence,
            &self.context,
            BuildChangeRequest {
                occurrence,
                cause: BuildCause::Logout,
                after: before.clone(),
                before,
                pruned_stance: None,
            },
            OfflineChange {
                before: offline,
                after,
            },
        )
        .await?;
        Ok(())
    }
}

impl super::fresh_admission::TerminalReleaseParticipant for TrainingLogout {
    fn prepare<'a, 'c>(
        &'a self,
        tx: &'a mut Transaction<'c, Postgres>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), DurabilityError>> + Send + 'a>>
    where
        'c: 'a,
    {
        Box::pin(TrainingLogout::prepare(self, tx))
    }
}
