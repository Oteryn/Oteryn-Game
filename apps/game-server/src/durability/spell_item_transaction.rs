//! Candidate SPELL-ITEM-1 writer on the existing durable ItemInstance owner.
//! Every helper runs inside the compositor's existing PostgreSQL transaction.
//! No helper commits, spawns a creature, or manufactures a creature-death cause.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_items_abi::*;
use super::{DurabilityError, DurabilityRoot};
use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug)]
pub(crate) enum SpellItemError {
    Rejected(&'static str),
    Durability(DurabilityError),
    Database(sqlx::Error),
    Training(super::character_progression::CharacterProgressionError),
}
impl From<DurabilityError> for SpellItemError {
    fn from(value: DurabilityError) -> Self {
        Self::Durability(value)
    }
}
impl From<sqlx::Error> for SpellItemError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}
type Result<T> = std::result::Result<T, SpellItemError>;

#[path = "spell_item_direct_companion.rs"]
mod direct_companion;
pub(crate) use direct_companion::{
    CommittedDirectCompanionAcquisition, DirectCompanionAcquisitionSource, DirectCompanionSource,
    PendingDirectCompanionAcquisition, PreparedDirectCompanionAcquisition,
    direct_companion_source_seal, reconcile_committed_direct_companion_acquisition_in_transaction,
    record_prepared_direct_companion_acquisition_in_transaction,
};

/// Read immutable source attribution only for an exact current live Ground item.
/// The property never authorizes an Item mutation or substitutes for custody.
pub(crate) async fn read_source_item_description_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    item: [u8; 16],
    state_revision: u64,
) -> Result<Option<String>> {
    check_transaction(tx, authority).await?;
    let row = sqlx::query("SELECT d.description FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) LEFT JOIN game_spell_item_source_descriptions d USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.state_revision=$3::text::numeric(20,0) AND i.lifecycle=1 AND i.quantity>0 AND g.channel_id=encode($4,'hex')::uuid AND g.runtime_scope_ownership_generation=$5::text::numeric(20,0) FOR SHARE OF i")
        .bind(item.as_slice()).bind(authority.world.as_slice())
        .bind(state_revision.to_string()).bind(authority.channel.as_slice())
        .bind(authority.ownership_generation.to_string())
        .fetch_optional(&mut **tx).await?
        .ok_or(SpellItemError::Rejected("source description requires current exact Item custody"))?;
    Ok(row.try_get("description")?)
}

pub(crate) async fn begin_spell_owner_transaction<'a>(
    holder: &'a mut sqlx::pool::PoolConnection<Postgres>,
    deadline: std::time::Instant,
) -> std::result::Result<Transaction<'a, Postgres>, DurabilityError> {
    super::db::begin_semantic_transaction(holder, deadline).await
}

pub(crate) struct WorldItemSpellCommit {
    pub(crate) committed: super::spell_owner_commit::CommittedSpellOwnerTransaction,
    pub(crate) training: Option<super::character_build::BuildCommitOutcome>,
    pub(crate) replayed: bool,
}

/// An exact optional D151 successor from the existing Character build owner.
/// The formula is Content-qualified before construction and revalidated by
/// that writer; there is no special spell-only training table.
pub(crate) struct WorldItemTraining {
    pub(crate) fence: super::character_progression::CurrentCharacterGameplayFence,
    pub(crate) request: super::character_build::BuildChangeRequest,
    pub(crate) formula: Box<dyn super::character_build::BuildFormula + Send + Sync>,
}

impl DurabilityRoot {
    /// One source-owned due pass; no caller event, caster cost or synthetic command.
    /// The live Channel owner must remain held around this await. A successful
    /// return proves the actual SQL COMMIT; an unknown COMMIT is reconciled by
    /// the same immutable source schedule on the next pass.
    pub(crate) async fn drain_spell_item_deadlines(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<usize> {
        let record = recovery
            .record_for(self)
            .map_err(|_| SpellItemError::Rejected("due pass recovery authority"))?;
        let root = self.clone();
        let node = node.clone();
        root.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<usize> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx, &record, &node, scope, generation,
                        )
                        .await?;
                        let fields =
                            expire_due_spell_items_bounded(&mut tx, &authority, 500).await?;
                        let temporal = super::spell_item_temporal::drain_due_items_in_transaction(
                            &mut tx,
                            &authority,
                            500 - fields.len(),
                        )
                        .await?;
                        let chained =
                            super::spell_item_temporal::drain_due_field_chains_in_transaction(
                                &mut tx,
                                &authority,
                                500 - fields.len() - temporal,
                            )
                            .await?;
                        Ok(fields.len() + temporal + chained)
                    }
                    .await;
                    match outcome {
                        Ok(changed) => {
                            super::db::commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(changed))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
    pub(crate) async fn read_standing_player_tile(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content_digest: [u8; 32],
        target: SpellGroundTarget,
    ) -> Result<StandingTileRead> {
        let record = recovery
            .record_for(self)
            .map_err(|_| SpellItemError::Rejected("standing read recovery authority"))?;
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let result = match read_standing_player_tile_with_recovery(
                        &mut tx,
                        &root,
                        &record,
                        &node,
                        &fence,
                        content_digest,
                        &target,
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }

    /// One bounded physical SQL transaction contains the actual Item source,
    /// caster-cost receipt and optional Character training/companion source.
    /// The outer Channel compositor must retain its physical preflight/owner
    /// lock and install only after this returns a genuine COMMIT capability.
    pub(crate) async fn commit_world_item_spell(
        &self,
        recovery_authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: SpellItemTransactionRequest,
        training: Option<WorldItemTraining>,
    ) -> Result<WorldItemSpellCommit> {
        let recovery = recovery_authority
            .record_for(self)
            .map_err(|_| SpellItemError::Rejected("unbound Character recovery authority"))?;
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let authority = match assert_spell_item_authority_with_recovery(
                        &mut tx,
                        &root,
                        &recovery,
                        &node,
                        &fence,
                        request.command,
                        request.catalog_digest,
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    let items =
                        match apply_spell_items_in_transaction(&mut tx, &authority, &request).await
                        {
                            Ok(value) => value,
                            Err(error) => return Ok(Err(error)),
                        };
                    if let SpellItemTransactionOutcome::AlreadyCommitted(items) = items {
                        // Reconcile the original training source; never turn a
                        // missing historical receipt into a fresh build write.
                        let companion = if request.companion.is_some() {
                            match reconcile_committed_companion_acquisition_in_transaction(
                                &mut tx,
                                &authority,
                                request.transaction_id,
                            )
                            .await
                            {
                                Ok(Some(value)) => Some(value),
                                Ok(None) => {
                                    return Ok(Err(SpellItemError::Rejected(
                                        "missing committed acquisition source",
                                    )));
                                }
                                Err(error) => return Ok(Err(error)),
                            }
                        } else {
                            None
                        };
                        let direct = if request.direct_companion.is_some() {
                            match reconcile_committed_direct_companion_acquisition_in_transaction(&mut tx,&authority,&request).await {
                                Ok(Some(value)) => Some(value),
                                Ok(None) => return Ok(Err(SpellItemError::Rejected("missing committed direct acquisition source"))),
                                Err(error) => return Ok(Err(error)),
                            }
                        } else { None };
                        let committed = super::spell_owner_commit::reconcile_committed_spell_owner_transaction_with_direct(
                            &mut tx,items,companion,direct).await?;
                        let training = if let Some(training) = training {
                            let pending =
                                match super::character_build::prepare_character_build_with_recovery(
                                    &mut tx,
                                    &recovery,
                                    &node,
                                    training.fence,
                                    training.request,
                                    training.formula.as_ref(),
                                )
                                .await
                                {
                                    Ok(value) => value,
                                    Err(error) => return Ok(Err(SpellItemError::Training(error))),
                                };
                            if pending.historical_outcome().is_none() {
                                return Ok(Err(SpellItemError::Rejected(
                                    "missing original committed training source",
                                )));
                            }
                            match pending.after_commit(&committed) {
                                Ok(value) => Some(value),
                                Err(error) => return Ok(Err(SpellItemError::Training(error))),
                            }
                        } else {
                            None
                        };
                        super::db::commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(WorldItemSpellCommit {
                            committed,
                            training,
                            replayed: true,
                        }));
                    }
                    let SpellItemTransactionOutcome::Applied(items) = items else {
                        unreachable!()
                    };
                    let companion = if request.companion.is_some() {
                        match record_prepared_companion_acquisition_in_transaction(
                            &mut tx, &authority, &request,
                        )
                        .await
                        {
                            Ok(value) => Some(value),
                            Err(error) => return Ok(Err(error)),
                        }
                    } else {
                        None
                    };
                    let direct = if request.direct_companion.is_some() {
                        match record_prepared_direct_companion_acquisition_in_transaction(&mut tx,&authority,&request).await {
                            Ok(value) => Some(value), Err(error) => return Ok(Err(error)),
                        }
                    } else { None };
                    let training = if let Some(training) = training {
                        if training.fence.character_id != fence.character_id
                            || training.fence.game_session_id != fence.game_session_id
                            || training.fence.connection_generation != fence.connection_generation
                            || training.fence.character_lease_generation
                                != fence.character_lease_generation
                            || training.fence.runtime_scope != fence.runtime_scope
                            || training.fence.scope_ownership_generation
                                != fence.scope_ownership_generation
                        {
                            return Ok(Err(SpellItemError::Rejected(
                                "training current caster fence mismatch",
                            )));
                        }
                        match super::character_build::prepare_character_build_with_recovery(
                            &mut tx,
                            &recovery,
                            &node,
                            training.fence,
                            training.request,
                            training.formula.as_ref(),
                        )
                        .await
                        {
                            Ok(value) => Some(value),
                            Err(error) => return Ok(Err(SpellItemError::Training(error))),
                        }
                    } else {
                        None
                    };
                    let pending = match super::spell_owner_commit::prepare_pending_spell_owner_transaction_with_direct(
                        &mut tx, items, companion, direct, None, deadline,
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(SpellItemError::Durability(error))),
                    };
                    let committed =
                        super::spell_owner_commit::commit_spell_owner_transaction(tx, pending)
                            .await?;
                    let training = match training
                        .map(|pending| pending.after_commit(&committed))
                        .transpose()
                    {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(SpellItemError::Training(error))),
                    };
                    Ok(Ok(WorldItemSpellCommit {
                        committed,
                        training,
                        replayed: false,
                    }))
                })
            })
            .await?
    }
}

pub(crate) mod companion_source_seal {
    pub(crate) trait Sealed {}
}
pub(crate) trait CompanionAcquisitionSource: companion_source_seal::Sealed {
    fn actor(&self) -> crate::foundation::ExactActorRef;
    fn master_actor(&self) -> crate::foundation::ExactActorRef;
    fn character_id(&self) -> crate::domain::CharacterId;
    fn game_session_id(&self) -> crate::foundation::GameSessionId;
    fn corpse_item_instance_id(&self) -> &[u8; 16];
    fn corpse_state_revision(&self) -> u64;
    fn spawn_cell(&self) -> SpellItemCell;
    fn creature_definition_key(&self) -> &str;
    fn creature_definition_revision(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompanionBinding {
    transaction: [u8; 16],
    corpse: [u8; 16],
    corpse_revision: u64,
    world: [u8; 16],
    channel: [u8; 16],
    ownership_generation: u64,
    actor_id: u32,
    actor_generation: u64,
    master_id: u32,
    master_generation: u64,
    character: [u8; 16],
    session: [u8; 16],
    cell: SpellItemCell,
    creature_key: String,
    creature_revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedCompanionAcquisition {
    binding: CompanionBinding,
    master_scope_matches: bool,
}
impl PreparedCompanionAcquisition {
    pub(crate) fn from_spawn(
        transaction: [u8; 16],
        spawn: &impl CompanionAcquisitionSource,
    ) -> Self {
        Self {
            binding: companion_binding(transaction, spawn),
            master_scope_matches: spawn.actor().world_id() == spawn.master_actor().world_id()
                && spawn.actor().channel_id() == spawn.master_actor().channel_id()
                && spawn.actor().scope_generation() == spawn.master_actor().scope_generation(),
        }
    }
}
impl CompanionBinding {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({"transaction":self.transaction,"corpse":self.corpse,"corpse_revision":self.corpse_revision,"world":self.world,"channel":self.channel,"ownership_generation":self.ownership_generation,"actor_id":self.actor_id,"actor_generation":self.actor_generation,"master_id":self.master_id,"master_generation":self.master_generation,"character":self.character,"session":self.session,"cell":[self.cell.x,self.cell.y,self.cell.z],"creature_key":self.creature_key,"creature_revision":self.creature_revision})
    }
}
pub(crate) struct PendingCompanionAcquisition {
    binding: CompanionBinding,
}
impl PendingCompanionAcquisition {
    pub(super) fn matches_transaction(&self, transaction: &[u8; 16]) -> bool {
        &self.binding.transaction == transaction
    }
    pub(super) async fn verify_staged_row(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> std::result::Result<(), DurabilityError> {
        let b = &self.binding;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_companion_acquisition_receipts c WHERE c.transaction_id=encode($1,'hex')::uuid AND c.corpse_item_instance_id=encode($2,'hex')::uuid AND c.corpse_state_revision=$3::text::numeric(20,0) AND c.actor_local_id=$4 AND c.actor_local_generation=$5::text::numeric(20,0) AND c.master_actor_local_id=$6 AND c.master_actor_local_generation=$7::text::numeric(20,0) AND c.creature_key=$8 AND c.creature_revision=$9 AND c.spawn_x=$10 AND c.spawn_y=$11 AND c.spawn_z=$12 AND c.created_xact_id=pg_current_xact_id())")
            .bind(b.transaction.as_slice()).bind(b.corpse.as_slice()).bind(b.corpse_revision.to_string()).bind(i64::from(b.actor_id)).bind(b.actor_generation.to_string()).bind(i64::from(b.master_id)).bind(b.master_generation.to_string()).bind(&b.creature_key).bind(&b.creature_revision).bind(b.cell.x).bind(b.cell.y).bind(b.cell.z).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
}
/// Constructed only from an observed committed DB receipt. A staged row or a
/// caller-supplied receipt id cannot authorize installing a physical creature.
pub(crate) struct CommittedCompanionAcquisition {
    binding: CompanionBinding,
}
impl CommittedCompanionAcquisition {
    /// Called exclusively by the real commit owner after observed COMMIT.
    pub(super) fn after_successful_commit(pending: PendingCompanionAcquisition) -> Self {
        Self {
            binding: pending.binding,
        }
    }
    pub(crate) fn matches_spawn(&self, spawn: &impl CompanionAcquisitionSource) -> bool {
        let b = &self.binding;
        let actor = spawn.actor();
        let master = spawn.master_actor();
        b.corpse == *spawn.corpse_item_instance_id()
            && b.corpse_revision == spawn.corpse_state_revision()
            && b.world == *actor.world_id().as_bytes()
            && b.channel == *actor.channel_id().as_bytes()
            && b.ownership_generation == actor.scope_generation().get()
            && actor.world_id() == master.world_id()
            && actor.channel_id() == master.channel_id()
            && actor.scope_generation() == master.scope_generation()
            && b.actor_id == actor.actor_local_id()
            && b.actor_generation == actor.actor_local_generation()
            && b.master_id == master.actor_local_id()
            && b.master_generation == master.actor_local_generation()
            && b.character == *spawn.character_id().as_bytes()
            && b.session == *spawn.game_session_id().as_bytes()
            && b.cell == spawn.spawn_cell()
            && b.creature_key == spawn.creature_definition_key()
            && b.creature_revision == spawn.creature_definition_revision()
    }
}

pub(crate) async fn stage_spell_owner_commit(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    items: CommittedSpellItems,
    companion: Option<PendingCompanionAcquisition>,
    familiar: Option<super::character_familiar::PendingCharacterFamiliar>,
    deadline: std::time::Instant,
) -> Result<super::spell_owner_commit::PendingSpellOwnerTransaction> {
    check_transaction(tx, authority).await?;
    if items.command != authority.command {
        return Err(SpellItemError::Rejected(
            "staged receipt command/current authority",
        ));
    }
    Ok(
        super::spell_owner_commit::prepare_pending_spell_owner_transaction(
            tx, items, companion, familiar, deadline,
        )
        .await?,
    )
}

pub(crate) async fn reconcile_spell_owner_commit_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    items: CommittedSpellItems,
    companion: Option<CommittedCompanionAcquisition>,
) -> Result<super::spell_owner_commit::CommittedSpellOwnerTransaction> {
    check_transaction(tx, authority).await?;
    if items.command != authority.command {
        return Err(SpellItemError::Rejected(
            "historical receipt/current read command mismatch",
        ));
    }
    Ok(
        super::spell_owner_commit::reconcile_committed_spell_owner_transaction(
            tx, items, companion,
        )
        .await?,
    )
}

pub(crate) async fn stage_spell_owner_commit_with_direct(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    items: CommittedSpellItems,
    direct: PendingDirectCompanionAcquisition,
    deadline: std::time::Instant,
) -> Result<super::spell_owner_commit::PendingSpellOwnerTransaction> {
    check_transaction(tx, authority).await?;
    if items.command != authority.command {
        return Err(SpellItemError::Rejected(
            "direct receipt/current authority command",
        ));
    }
    Ok(
        super::spell_owner_commit::prepare_pending_spell_owner_transaction_with_direct(
            tx,
            items,
            None,
            Some(direct),
            None,
            deadline,
        )
        .await?,
    )
}

pub(crate) async fn reconcile_spell_owner_commit_with_direct_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    items: CommittedSpellItems,
    direct: CommittedDirectCompanionAcquisition,
) -> Result<super::spell_owner_commit::CommittedSpellOwnerTransaction> {
    check_transaction(tx, authority).await?;
    if items.command != authority.command {
        return Err(SpellItemError::Rejected(
            "historical direct receipt/current authority command",
        ));
    }
    Ok(
        super::spell_owner_commit::reconcile_committed_spell_owner_transaction_with_direct(
            tx,
            items,
            None,
            Some(direct),
        )
        .await?,
    )
}
fn companion_binding(
    transaction: [u8; 16],
    spawn: &impl CompanionAcquisitionSource,
) -> CompanionBinding {
    let actor = spawn.actor();
    let master = spawn.master_actor();
    CompanionBinding {
        transaction,
        corpse: *spawn.corpse_item_instance_id(),
        corpse_revision: spawn.corpse_state_revision(),
        world: *actor.world_id().as_bytes(),
        channel: *actor.channel_id().as_bytes(),
        ownership_generation: actor.scope_generation().get(),
        actor_id: actor.actor_local_id(),
        actor_generation: actor.actor_local_generation(),
        master_id: master.actor_local_id(),
        master_generation: master.actor_local_generation(),
        character: *spawn.character_id().as_bytes(),
        session: *spawn.game_session_id().as_bytes(),
        cell: spawn.spawn_cell(),
        creature_key: spawn.creature_definition_key().into(),
        creature_revision: spawn.creature_definition_revision().into(),
    }
}

pub(crate) async fn record_companion_acquisition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    spawn: &impl CompanionAcquisitionSource,
) -> Result<PendingCompanionAcquisition> {
    let binding = companion_binding(request.transaction_id, spawn);
    if request.companion.as_ref().map(|p| &p.binding) != Some(&binding) {
        return Err(SpellItemError::Rejected(
            "companion absent from the exact compound cast intent",
        ));
    }
    record_prepared_companion_acquisition_in_transaction(tx, authority, request).await
}

pub(super) async fn record_prepared_companion_acquisition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
) -> Result<PendingCompanionAcquisition> {
    check_transaction(tx, authority).await?;
    let prepared = request
        .companion
        .as_ref()
        .ok_or(SpellItemError::Rejected("missing sealed companion intent"))?;
    let binding = prepared.binding.clone();
    if binding.world!=authority.world || binding.channel!=authority.channel || binding.ownership_generation!=authority.ownership_generation
        || binding.character!=authority.character || binding.session!=*authority.command.game_session_id().as_bytes()
        || !prepared.master_scope_matches
        || !request.operations.iter().any(|op|matches!(op,SpellItemOperation::ConsumeCorpse(c) if c.item_instance_id==binding.corpse && c.state_revision==binding.corpse_revision)) {return Err(SpellItemError::Rejected("companion source/current owner binding"));}
    sqlx::query("INSERT INTO game_spell_companion_acquisition_receipts(transaction_id,corpse_item_instance_id,corpse_state_revision,world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation,master_character_id,master_game_session_id,master_actor_local_id,master_actor_local_generation,creature_key,creature_revision,spawn_x,spawn_y,spawn_z,binding_json) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),$7,$8::text::numeric(20,0),encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11,$12::text::numeric(20,0),$13,$14,$15,$16,$17,$18::text::jsonb)")
        .bind(binding.transaction.as_slice()).bind(binding.corpse.as_slice()).bind(binding.corpse_revision.to_string()).bind(binding.world.as_slice()).bind(binding.channel.as_slice()).bind(binding.ownership_generation.to_string()).bind(i64::from(binding.actor_id)).bind(binding.actor_generation.to_string()).bind(binding.character.as_slice()).bind(binding.session.as_slice()).bind(i64::from(binding.master_id)).bind(binding.master_generation.to_string()).bind(&binding.creature_key).bind(&binding.creature_revision).bind(binding.cell.x).bind(binding.cell.y).bind(binding.cell.z).bind(binding.json().to_string()).execute(&mut **tx).await?;
    Ok(PendingCompanionAcquisition { binding })
}

pub(crate) async fn reconcile_committed_companion_acquisition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    transaction: [u8; 16],
) -> Result<Option<CommittedCompanionAcquisition>> {
    check_transaction(tx, authority).await?;
    let row=sqlx::query("SELECT c.*,c.corpse_item_instance_id::text AS corpse_text,c.world_id::text AS world_text,c.channel_id::text AS channel_text,c.master_character_id::text AS character_text,c.master_game_session_id::text AS session_text,c.corpse_state_revision::text AS corpse_revision_text,c.ownership_generation::text AS ownership_text,c.actor_local_generation::text AS actor_generation_text,c.master_actor_local_generation::text AS master_generation_text FROM game_spell_companion_acquisition_receipts c JOIN game_spell_item_receipts r USING(transaction_id) WHERE c.transaction_id=encode($1,'hex')::uuid AND c.created_xact_id<>pg_current_xact_id() AND pg_xact_status(c.created_xact_id)='committed' AND r.created_xact_id=c.created_xact_id")
        .bind(transaction.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let binding = CompanionBinding {
        transaction,
        corpse: uuid(&row.try_get::<String, _>("corpse_text")?)?,
        corpse_revision: integer(row.try_get("corpse_revision_text")?)?,
        world: uuid(&row.try_get::<String, _>("world_text")?)?,
        channel: uuid(&row.try_get::<String, _>("channel_text")?)?,
        ownership_generation: integer(row.try_get("ownership_text")?)?,
        actor_id: u32::try_from(row.try_get::<i64, _>("actor_local_id")?)
            .map_err(|_| SpellItemError::Rejected("actor id"))?,
        actor_generation: integer(row.try_get("actor_generation_text")?)?,
        master_id: u32::try_from(row.try_get::<i64, _>("master_actor_local_id")?)
            .map_err(|_| SpellItemError::Rejected("master id"))?,
        master_generation: integer(row.try_get("master_generation_text")?)?,
        character: uuid(&row.try_get::<String, _>("character_text")?)?,
        session: uuid(&row.try_get::<String, _>("session_text")?)?,
        cell: SpellItemCell {
            x: row.try_get("spawn_x")?,
            y: row.try_get("spawn_y")?,
            z: row.try_get("spawn_z")?,
        },
        creature_key: row.try_get("creature_key")?,
        creature_revision: row.try_get("creature_revision")?,
    };
    if binding.world != authority.world
        || binding.channel != authority.channel
        || binding.ownership_generation != authority.ownership_generation
        || binding.character != authority.character
        || binding.session != *authority.command.game_session_id().as_bytes()
    {
        return Err(SpellItemError::Rejected(
            "committed historical acquisition is not current install authority",
        ));
    }
    Ok(Some(CommittedCompanionAcquisition { binding }))
}

/// Current, sealed proof for one physical transaction. A previous receipt or
/// frozen reservation cannot construct this proof, and a different transaction
/// cannot reuse it. The root compositor owns the transaction's lifetime.
pub(crate) struct SpellItemAuthority {
    due_read_only: bool,
    physical_transaction: String,
    world: [u8; 16],
    channel: [u8; 16],
    character: [u8; 16],
    ownership_generation: u64,
    command: CommandRef,
    compatible_content_digest: [u8; 32],
    runtime_scope: crate::foundation::RuntimeScopeRefV1,
    character_lease_generation: u64,
    connection_generation: crate::foundation::ConnectionGeneration,
}

pub(crate) mod due_read_seal {
    pub(crate) trait Sealed {}
}
/// Implemented only by the actual timer owner holding its committed payload
/// and retained original physical source batch. Stored payload data alone is
/// insufficient to produce this current-owner observation.
pub(crate) trait CommittedSpellDueEvidence: due_read_seal::Sealed {
    fn command(&self) -> CommandRef;
    fn caster_placement_identity(&self) -> [u8; 16];
    fn spell(&self) -> &TypedDefinitionRef;
    fn content_digest(&self) -> [u8; 32];
    fn verify_current_retained_source(&self) -> std::result::Result<(), &'static str>;
}
pub(crate) struct DueSpellItemReadAuthority {
    authority: SpellItemAuthority,
}
impl DueSpellItemReadAuthority {
    pub(crate) fn read_authority(&self) -> &SpellItemAuthority {
        &self.authority
    }
}

/// Read-only due callback authority. Reconnectable is admitted only after a
/// freshly observed complete canonical session and an originally committed
/// source cost are independently matched to the actual retained timer owner.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn assert_due_spell_item_read_authority_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    compatible_content_digest: [u8; 32],
    evidence: &impl CommittedSpellDueEvidence,
) -> Result<DueSpellItemReadAuthority> {
    evidence
        .verify_current_retained_source()
        .map_err(SpellItemError::Rejected)?;
    if evidence.command() != command
        || evidence.content_digest() != compatible_content_digest
        || evidence.spell().family != "Spell"
        || evidence.caster_placement_identity() == [0; 16]
    {
        return Err(SpellItemError::Rejected("due callback source identity"));
    }
    let mut authority = assert_spell_item_authority_in_transaction(
        tx,
        root,
        recovery,
        node,
        fence,
        command,
        compatible_content_digest,
    )
    .await?;
    let current = super::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session_in_transaction(tx, fence.game_session_id)
        .await?;
    if !matches!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
            | crate::foundation::GameSessionState::Reconnectable
    ) || current.commit().game_session_id() != fence.game_session_id
        || current.current_connection_generation() != fence.connection_generation
        || current.current_character_lease().generation() != fence.character_lease_generation
        || current.current_runtime_scope() != fence.runtime_scope
        || current.current_scope_generation() != fence.scope_ownership_generation
        || current.commit().character_id().as_bytes() != fence.character_id.as_bytes()
    {
        return Err(SpellItemError::Rejected(
            "due callback current canonical session",
        ));
    }
    verify_paid_due_source(tx, &authority, evidence).await?;
    evidence
        .verify_current_retained_source()
        .map_err(SpellItemError::Rejected)?;
    authority.due_read_only = true;
    Ok(DueSpellItemReadAuthority { authority })
}
async fn verify_paid_due_source(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    evidence: &impl CommittedSpellDueEvidence,
) -> Result<()> {
    check_transaction(tx, authority).await?;
    let paid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_receipts r WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric(20,0) AND r.character_id=encode($3,'hex')::uuid AND r.world_id=encode($4,'hex')::uuid AND r.channel_id=encode($5,'hex')::uuid AND r.ownership_generation=$6::text::numeric(20,0) AND r.caster_lease_generation=$7::text::numeric(20,0) AND r.caster_placement_digest=$8 AND r.catalog_digest=$9 AND r.spell_family='Spell' AND r.spell_production_key=$10 AND r.spell_revision=$11 AND r.binding=sha256(r.intent) AND r.cost_binding=sha256(r.cost) AND game_spell_cost_binding_valid(r.cost) AND r.created_xact_id<>pg_current_xact_id() AND pg_xact_status(r.created_xact_id)='committed')")
        .bind(authority.command.game_session_id().as_bytes().as_slice()).bind(authority.command.command_id().get().to_string())
        .bind(authority.character.as_slice()).bind(authority.world.as_slice()).bind(authority.channel.as_slice())
        .bind(authority.ownership_generation.to_string()).bind(authority.character_lease_generation.to_string())
        .bind(evidence.caster_placement_identity().as_slice()).bind(authority.compatible_content_digest.as_slice())
        .bind(&evidence.spell().production_key).bind(&evidence.spell().revision_ref).fetch_one(&mut **tx).await?;
    if !paid {
        return Err(SpellItemError::Rejected(
            "due callback original committed source absent",
        ));
    }
    Ok(())
}

pub(crate) struct SpellItemScopeAuthority {
    physical_transaction: String,
    world: [u8; 16],
    channel: [u8; 16],
    ownership_generation: u64,
}
impl SpellItemScopeAuthority {
    pub(super) fn world_bytes(&self) -> [u8; 16] {
        self.world
    }
    pub(super) fn channel_bytes(&self) -> [u8; 16] {
        self.channel
    }
    pub(super) fn generation(&self) -> u64 {
        self.ownership_generation
    }
}
pub(super) async fn check_scope_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
) -> Result<()> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    if current != authority.physical_transaction {
        return Err(SpellItemError::Rejected(
            "scope authority belongs to another transaction",
        ));
    }
    Ok(())
}

/// A current fenced read result, sealed by this reader. Its detached data is
/// not permission to advance a player; the actual periodic owner independently
/// compares all bindings and retains its owner borrow through consumption.
pub(crate) struct StandingTileRead {
    authority: SpellItemScopeAuthority,
    fence: CurrentCharacterItemFence,
    content_digest: [u8; 32],
    items: DurableTileItems,
    observed_at_unix_ms: i64,
}
impl StandingTileRead {
    /// A detached/historical read cannot be replayed as a new owner pass.
    pub(crate) async fn check_transaction(&self, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        check_scope_transaction(tx, &self.authority).await
    }
    pub(crate) fn observed_at_unix_ms(&self) -> i64 {
        self.observed_at_unix_ms
    }
    pub(crate) fn fence(&self) -> &CurrentCharacterItemFence {
        &self.fence
    }
    pub(crate) fn content_digest(&self) -> [u8; 32] {
        self.content_digest
    }
    pub(crate) fn target(&self) -> &SpellGroundTarget {
        &self.items.target
    }
    pub(crate) fn items(&self) -> &[DurableTileItem] {
        &self.items.items
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn read_standing_player_tile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
    target: &SpellGroundTarget,
) -> Result<StandingTileRead> {
    let record = recovery
        .record_for(root)
        .map_err(|_| SpellItemError::Rejected("standing read recovery authority"))?;
    read_standing_player_tile_with_recovery(tx, root, &record, node, fence, content_digest, target)
        .await
}

async fn read_standing_player_tile_with_recovery(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
    target: &SpellGroundTarget,
) -> Result<StandingTileRead> {
    if content_digest == [0; 32] {
        return Err(SpellItemError::Rejected(
            "standing read missing Content pin",
        ));
    }
    let authority = assert_spell_item_scope_with_recovery(
        tx,
        record,
        node,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await?;
    let session = super::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session_in_transaction(tx, fence.game_session_id)
        .await?;
    if session.session_state() != crate::foundation::GameSessionState::Active
        || session.current_connection_generation() != fence.connection_generation
        || session.current_character_lease().generation() != fence.character_lease_generation
        || session.current_runtime_scope() != fence.runtime_scope
        || session.current_scope_generation() != fence.scope_ownership_generation
        || session.commit().character_id().as_bytes() != fence.character_id.as_bytes()
    {
        return Err(SpellItemError::Rejected(
            "standing read current player fence mismatch",
        ));
    }
    let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND g.eligible AND g.holder_game_session_id=encode($2,'hex')::uuid AND g.lease_generation=$3::text::numeric(20,0))")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).fetch_one(&mut **tx).await?;
    if !live {
        return Err(SpellItemError::Rejected(
            "standing player root/lease is not current",
        ));
    }
    let rows = locked_tile_rows_for_scope(tx, &authority, target).await?;
    let items = decode_tile_rows(rows, authority.ownership_generation, target)?;
    let observed_at_unix_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    Ok(StandingTileRead {
        authority,
        fence: *fence,
        content_digest,
        items,
        observed_at_unix_ms,
    })
}

pub(crate) async fn assert_spell_item_scope_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
) -> Result<SpellItemScopeAuthority> {
    let record = recovery
        .record_for(root)
        .map_err(|_| SpellItemError::Rejected("recovery authority"))?;
    assert_spell_item_scope_with_recovery(tx, &record, node, scope, generation).await
}

pub(super) async fn assert_spell_item_scope_with_recovery(
    tx: &mut Transaction<'_, Postgres>,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
) -> Result<SpellItemScopeAuthority> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        return Err(SpellItemError::Rejected("timer requires Channel scope"));
    };
    assert_recovery_fence(tx, record).await?;
    super::db::lock_admission_relations(tx).await?;
    let fact = node.fact();
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE s.world_id=encode($1,'hex')::uuid AND s.channel_id=encode($2,'hex')::uuid AND s.state=1 AND s.ownership_generation=$3::text::numeric(20,0) AND s.holder_node_id=encode($4,'hex')::uuid AND s.holder_registration_revision=$5::text::numeric(20,0) AND g.ready AND g.ownership_generation=s.ownership_generation)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
    if !current || !super::runtime_scope_assignment::prove_current_incarnation(tx, node).await? {
        return Err(SpellItemError::Rejected(
            "current timer scope/node authority",
        ));
    }
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'), 33))",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    let physical_transaction = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(SpellItemScopeAuthority {
        physical_transaction,
        world: *world_id.as_bytes(),
        channel: *channel_id.as_bytes(),
        ownership_generation: generation,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExpiredSpellItem {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) state_revision_before: u64,
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
}

/// A bounded durable timer pass. Expiry uses the DB clock and a fresh scope
/// owner/node/recovery fence, including after handoff. No original caster
/// session, creature death, or fake Character cost is required.
pub(crate) async fn expire_due_spell_items_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
) -> Result<Vec<ExpiredSpellItem>> {
    expire_due_spell_items_bounded(tx, authority, 500).await
}

async fn expire_due_spell_items_bounded(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<Vec<ExpiredSpellItem>> {
    if limit > 500 {
        return Err(SpellItemError::Rejected("field expiry batch bound"));
    }
    if limit == 0 {
        return Ok(Vec::new());
    }
    let physical: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    if physical != authority.physical_transaction {
        return Err(SpellItemError::Rejected(
            "timer authority belongs to another transaction",
        ));
    }
    let rows=sqlx::query("SELECT i.item_instance_id::text,i.state_revision::text,g.stack_ordinal,l.transaction_id::text AS source_transaction_id,l.ordinal AS source_ordinal,l.expires_at_unix_ms,game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_spell_item_lines l USING(item_instance_id) WHERE i.lifecycle=1 AND l.operation_kind=1 AND l.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid ORDER BY l.expires_at_unix_ms,i.item_instance_id LIMIT $3 FOR UPDATE OF i")
        .bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(i64::try_from(limit).map_err(|_|SpellItemError::Rejected("field expiry batch bound"))?).fetch_all(&mut **tx).await?;
    let mut output = Vec::new();
    for row in rows {
        let item = uuid(&row.try_get::<String, _>("item_instance_id")?)?;
        let transaction = uuid(&row.try_get::<String, _>("transaction_id")?)?;
        let event = uuid(&row.try_get::<String, _>("event_id")?)?;
        let source = uuid(&row.try_get::<String, _>("source_transaction_id")?)?;
        let revision = integer(row.try_get("state_revision")?)?;
        sqlx::query("INSERT INTO game_spell_item_expiry_receipts(item_instance_id,transaction_id,event_id,source_transaction_id,source_ordinal,state_revision_before,source_stack_ordinal,world_id,channel_id,ownership_generation,expires_at_unix_ms,occurred_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6::text::numeric(20,0),$7,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric(20,0),$11,floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
            .bind(item.as_slice()).bind(transaction.as_slice()).bind(event.as_slice()).bind(source.as_slice()).bind(row.try_get::<i32,_>("source_ordinal")?).bind(revision.to_string()).bind(row.try_get::<i64,_>("stack_ordinal")?).bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(authority.ownership_generation.to_string()).bind(row.try_get::<i64,_>("expires_at_unix_ms")?).execute(&mut **tx).await?;
        let update=sqlx::query("UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid AND lifecycle=1 AND state_revision=$3::text::numeric(20,0)").bind(transaction.as_slice()).bind(item.as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
        if update.rows_affected() != 1 {
            return Err(SpellItemError::Rejected("expiry item compare-and-retire"));
        }
        let removed=sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid AND stack_ordinal=$2").bind(item.as_slice()).bind(row.try_get::<i64,_>("stack_ordinal")?).execute(&mut **tx).await?;
        if removed.rows_affected() != 1 {
            return Err(SpellItemError::Rejected("expiry exact Ground remove"));
        }
        output.push(ExpiredSpellItem {
            item_instance_id: item,
            state_revision_before: revision,
            transaction_id: transaction,
            event_id: event,
        });
    }
    Ok(output)
}

impl SpellItemAuthority {
    pub(crate) fn connection_generation(&self) -> crate::foundation::ConnectionGeneration {
        self.connection_generation
    }
    pub(crate) fn character_lease_generation(&self) -> u64 {
        self.character_lease_generation
    }
    pub(crate) fn character_id_bytes(&self) -> [u8; 16] {
        self.character
    }
    pub(crate) fn command(&self) -> CommandRef {
        self.command
    }
    pub(crate) fn game_session_id(&self) -> crate::foundation::GameSessionId {
        self.command.game_session_id()
    }
    pub(crate) fn runtime_scope(&self) -> crate::foundation::RuntimeScopeRefV1 {
        self.runtime_scope
    }
    pub(crate) fn scope_generation(&self) -> u64 {
        self.ownership_generation
    }
    pub(crate) fn compatible_content_digest(&self) -> [u8; 32] {
        self.compatible_content_digest
    }
}

pub(crate) async fn assert_spell_item_authority_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    compatible_content_digest: [u8; 32],
) -> Result<SpellItemAuthority> {
    let record = recovery
        .record_for(root)
        .map_err(|_| SpellItemError::Rejected("recovery authority"))?;
    assert_spell_item_authority_with_recovery(
        tx,
        root,
        &record,
        node,
        fence,
        command,
        compatible_content_digest,
    )
    .await
}

pub(super) async fn assert_spell_item_authority_with_recovery(
    tx: &mut Transaction<'_, Postgres>,
    _root: &DurabilityRoot,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    compatible_content_digest: [u8; 32],
) -> Result<SpellItemAuthority> {
    if compatible_content_digest == [0; 32] {
        return Err(SpellItemError::Rejected(
            "missing compatible Content binding",
        ));
    }
    assert_recovery_fence(tx, record).await?;
    super::db::lock_admission_relations(tx).await?;
    let (world, channel) =
        scope_of(fence).map_err(|_| SpellItemError::Rejected("Channel scope"))?;
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
        return Err(SpellItemError::Rejected("current Character item fence"));
    }
    let physical_transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    // Serialize all dynamic item snapshots/mutations in this Channel in a
    // stable owner lock order. The lock lasts to the compositor's commit.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'), 33))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(SpellItemAuthority {
        due_read_only: false,
        physical_transaction,
        world: *world.as_bytes(),
        channel: *channel.as_bytes(),
        character: *fence.character_id.as_bytes(),
        ownership_generation: fence.scope_ownership_generation.get(),
        command,
        compatible_content_digest,
        runtime_scope: fence.runtime_scope,
        character_lease_generation: fence.character_lease_generation,
        connection_generation: fence.connection_generation,
    })
}

pub(crate) async fn check_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<()> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    if current != authority.physical_transaction {
        return Err(SpellItemError::Rejected(
            "authority belongs to another physical transaction",
        ));
    }
    Ok(())
}

fn uuid(value: &str) -> Result<[u8; 16]> {
    let compact: String = value.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 {
        return Err(SpellItemError::Rejected("stored UUID"));
    }
    let mut output = [0; 16];
    for (index, out) in output.iter_mut().enumerate() {
        *out = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
            .map_err(|_| SpellItemError::Rejected("stored UUID"))?;
    }
    Ok(output)
}
fn integer(value: String) -> Result<u64> {
    value
        .parse()
        .map_err(|_| SpellItemError::Rejected("stored unsigned value"))
}
fn decoded_definition(row: &sqlx::postgres::PgRow) -> Result<TypedDefinitionRef> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}
fn decoded_placement(row: &sqlx::postgres::PgRow) -> Result<GroundPlacement> {
    Ok(GroundPlacement {
        spatial_position: row.try_get("spatial_position")?,
        corpse_ref: row.try_get("corpse_ref")?,
        map_revision: row.try_get("map_revision")?,
        content_revision: row.try_get("content_revision")?,
        native_room_placement_context: row.try_get("native_room_placement_context")?,
    })
}

// The Channel owner lock serializes Ground custody; retain row locks on
// mutable ItemInstances without requiring UPDATE on immutable locations.
const TILE_ROWS: &str = "SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.state_revision::text,i.minted_transaction_id::text,g.stack_ordinal::text,g.runtime_scope_ownership_generation::text AS ground_scope_generation,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,g.native_room_placement_context,COALESCE(fc.blocks_movement,l.blocks_movement,nm.blocks_movement) AS blocks_movement,COALESCE(fc.blocks_projectile,l.blocks_projectile,nm.blocks_projectile) AS blocks_projectile,COALESCE(fc.immovable_block_solid,l.immovable_block_solid,nm.immovable_block_solid) AS immovable_block_solid,l.expires_at_unix_ms,r.character_id::text AS origin_character,r.game_session_id::text AS origin_session,r.command_id::text AS origin_command,r.occurred_at_unix_ms AS origin_created_unix_ms,r.ownership_generation::text AS origin_generation,r.caster_lease_generation::text AS origin_lease,r.caster_placement_digest AS origin_placement,r.catalog_digest AS origin_catalog,l.content_generation_digest AS origin_content, EXISTS (SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=i.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION') AS corpse FROM game_item_ground_locations g JOIN game_item_instances i ON i.item_instance_id=g.item_instance_id LEFT JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id AND l.operation_kind=1 LEFT JOIN game_spell_item_receipts r ON r.transaction_id=l.transaction_id LEFT JOIN game_spell_field_temporal_schedules fc ON fc.item_instance_id=i.item_instance_id AND fc.source_transaction_id=i.minted_transaction_id AND (fc.definition_family,fc.definition_production_key,fc.definition_revision)=(i.definition_family,i.definition_production_key,i.definition_revision_ref) AND fc.content_digest=l.content_generation_digest LEFT JOIN game_native_map_item_receipts nm ON nm.item_instance_id=i.item_instance_id AND nm.transaction_id=i.minted_transaction_id AND (nm.definition_family,nm.definition_key,nm.definition_revision)=(i.definition_family,i.definition_production_key,i.definition_revision_ref) AND nm.quantity=i.quantity AND i.state_revision=1 AND i.last_transaction_id IS NULL AND (nm.world_id,nm.channel_id,nm.spatial_position,nm.map_revision,nm.content_revision,nm.placement_context)=(g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context) AND (nm.scope_generation=g.runtime_scope_ownership_generation OR EXISTS(SELECT 1 FROM game_native_map_scope_adoptions ad WHERE ad.source_transaction_id=nm.transaction_id AND ad.ownership_generation=g.runtime_scope_ownership_generation)) AND nm.content_revision='sha256:'||encode(nm.content_digest,'hex') AND nm.map_revision='sha256:'||encode(nm.map_digest,'hex') AND nm.placement_context=nm.frame_digest AND (nm.owner_kind IS NULL OR nm.owner_kind IN('container','door')) AND EXISTS(SELECT 1 FROM game_native_map_item_audit na WHERE na.transaction_id=nm.transaction_id AND na.item_instance_id=nm.item_instance_id AND na.event_id=nm.event_id AND na.envelope=nm.source_intent AND na.envelope_digest=nm.binding AND na.created_xact_id=nm.created_xact_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6 AND i.lifecycle=1 ORDER BY g.stack_ordinal DESC NULLS FIRST LIMIT 501 FOR UPDATE OF i";

async fn locked_tile_rows(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: &SpellGroundTarget,
) -> Result<Vec<sqlx::postgres::PgRow>> {
    check_transaction(tx, authority).await?;
    let scope = SpellItemScopeAuthority {
        physical_transaction: authority.physical_transaction.clone(),
        world: authority.world,
        channel: authority.channel,
        ownership_generation: authority.ownership_generation,
    };
    locked_tile_rows_for_scope(tx, &scope, target).await
}

async fn locked_tile_rows_for_scope(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
) -> Result<Vec<sqlx::postgres::PgRow>> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    if current != authority.physical_transaction {
        return Err(SpellItemError::Rejected(
            "tile read requires same physical owner transaction",
        ));
    }
    let unknown:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND (octet_length(g.spatial_position)<>10 OR (g.spatial_position=$3 AND (g.map_revision<>$4 OR g.content_revision<>$5 OR g.native_room_placement_context<>$6))))")
        .bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(&target.spatial_position).bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).fetch_one(&mut **tx).await?;
    if unknown {
        return Err(SpellItemError::Rejected(
            "unreconciled opaque or incompatible Ground placement",
        ));
    }
    let rows = sqlx::query(TILE_ROWS)
        .bind(authority.world.as_slice())
        .bind(authority.channel.as_slice())
        .bind(&target.spatial_position)
        .bind(&target.map_revision)
        .bind(&target.content_revision)
        .bind(&target.placement_context)
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() > 500 {
        return Err(SpellItemError::Rejected(
            "tile item snapshot exceeds 500 visited items",
        ));
    }
    for row in &rows {
        if integer(row.try_get("ground_scope_generation")?)? != authority.ownership_generation {
            return Err(SpellItemError::Rejected(
                "Ground custody belongs to an unreconciled scope generation",
            ));
        }
    }
    if rows.iter().any(|r| {
        r.try_get::<Option<String>, _>("stack_ordinal")
            .ok()
            .flatten()
            .is_none()
    }) {
        return Err(SpellItemError::Rejected(
            "legacy ground ordering has not been reconciled",
        ));
    }
    Ok(rows)
}

pub(crate) async fn prepare_corpse_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: &SpellGroundTarget,
    qualified: &QualifiedItemDefinition,
) -> Result<DurableCorpseReservation> {
    if !qualified.materializable || !qualified.movable || qualified.definition.family != "Item" {
        return Err(SpellItemError::Rejected(
            "corpse requires proven materializable movable Item",
        ));
    }
    let rows = locked_tile_rows(tx, authority, target).await?;
    let row = rows
        .first()
        .ok_or(SpellItemError::Rejected("no top item"))?;
    if !row.try_get::<bool, _>("corpse")? || decoded_definition(row)? != qualified.definition {
        return Err(SpellItemError::Rejected(
            "top item is not the qualified real corpse",
        ));
    }
    let item = uuid(&row.try_get::<String, _>("item_instance_id")?)?;
    let placement = decoded_placement(row)?;
    let contents = prepare_contained_retirements(tx, authority, item, &placement).await?;
    Ok(DurableCorpseReservation {
        item_instance_id: item,
        definition: decoded_definition(row)?,
        state_revision: integer(row.try_get("state_revision")?)?,
        top_down_ordinal: integer(row.try_get("stack_ordinal")?)?,
        placement,
        mint_transaction_id: uuid(&row.try_get::<String, _>("minted_transaction_id")?)?,
        quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| SpellItemError::Rejected("quantity"))?,
        contents,
    })
}

pub(crate) async fn read_spell_tile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: &SpellGroundTarget,
) -> Result<DurableTileItems> {
    decode_tile_rows(
        locked_tile_rows(tx, authority, target).await?,
        authority.ownership_generation,
        target,
    )
}

/// Native qualification spawn reads the actual scope-owned Item tile, without
/// fabricating a player command or describing it as a standing-player read.
#[cfg(test)]
pub(crate) async fn read_qualification_scope_tile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
) -> Result<DurableTileItems> {
    check_scope_transaction(tx, authority).await?;
    decode_tile_rows(
        locked_tile_rows_for_scope(tx, authority, target).await?,
        authority.ownership_generation,
        target,
    )
}

fn decode_tile_rows(
    rows: Vec<sqlx::postgres::PgRow>,
    ownership_generation: u64,
    target: &SpellGroundTarget,
) -> Result<DurableTileItems> {
    let mut items = Vec::new();
    for row in rows {
        let blocks_movement =
            row.try_get::<Option<bool>, _>("blocks_movement")?
                .ok_or(SpellItemError::Rejected(
                    "item has no qualified current movement flags",
                ))?;
        let blocks_projectile = row.try_get::<Option<bool>, _>("blocks_projectile")?.ok_or(
            SpellItemError::Rejected("item has no qualified current projectile flags"),
        )?;
        items.push(DurableTileItem {
            item_instance_id: uuid(&row.try_get::<String, _>("item_instance_id")?)?,
            definition: decoded_definition(&row)?,
            state_revision: integer(row.try_get("state_revision")?)?,
            top_down_ordinal: integer(row.try_get("stack_ordinal")?)?,
            blocks_movement,
            blocks_projectile,
            immovable_block_solid: row
                .try_get::<Option<bool>, _>("immovable_block_solid")?
                .ok_or(SpellItemError::Rejected("unknown immovable blocking fact"))?,
            expires_at_unix_ms: row.try_get("expires_at_unix_ms")?,
            field_origin: match row.try_get::<Option<String>, _>("origin_lease")? {
                None => None,
                Some(lease) => Some(DurableFieldOrigin {
                    created_at_unix_ms: row.try_get("origin_created_unix_ms")?,
                    character_id: uuid(&row.try_get::<String, _>("origin_character")?)?,
                    game_session_id: uuid(&row.try_get::<String, _>("origin_session")?)?,
                    character_lease_generation: integer(lease)?,
                    actor_placement_digest: row
                        .try_get::<Vec<u8>, _>("origin_placement")?
                        .try_into()
                        .map_err(|_| SpellItemError::Rejected("field origin placement bytes"))?,
                    source_scope_generation: integer(row.try_get("origin_generation")?)?,
                    content_digest: row
                        .try_get::<Vec<u8>, _>("origin_content")?
                        .try_into()
                        .map_err(|_| SpellItemError::Rejected("field origin content digest"))?,
                    catalog_digest: row
                        .try_get::<Vec<u8>, _>("origin_catalog")?
                        .try_into()
                        .map_err(|_| SpellItemError::Rejected("field origin catalog digest"))?,
                    creation_command_id: integer(row.try_get("origin_command")?)?,
                }),
            },
        });
    }
    Ok(DurableTileItems {
        items,
        ownership_generation,
        target: target.clone(),
    })
}

/// Qualify source action/unique protection on a currently locked tile instance.
/// Native map attributes are immutable source data; original spell creation has
/// neither tag. Other mint causes need their own explicit source closure.
pub(crate) async fn current_tile_protection_tags_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    item: &super::spell_items_abi::DurableTileItem,
) -> Result<(bool, bool)> {
    check_transaction(tx, authority).await?;
    let row = sqlx::query("SELECT nm.source_attributes, EXISTS(SELECT 1 FROM game_spell_item_lines sl WHERE sl.transaction_id=i.minted_transaction_id AND sl.item_instance_id=i.item_instance_id AND sl.operation_kind=1) AS spell_origin FROM game_item_instances i LEFT JOIN game_native_map_item_receipts nm ON nm.transaction_id=i.minted_transaction_id AND nm.item_instance_id=i.item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.state_revision=$3::text::numeric(20,0) AND i.definition_family=$4 AND i.definition_production_key=$5 AND i.definition_revision_ref=$6 FOR UPDATE OF i")
        .bind(item.item_instance_id.as_slice()).bind(authority.world.as_slice())
        .bind(item.state_revision.to_string()).bind(&item.definition.family)
        .bind(&item.definition.production_key).bind(&item.definition.revision_ref)
        .fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("source tile instance changed"))?;
    if let Some(attributes) = row.try_get::<Option<serde_json::Value>, _>("source_attributes")? {
        return source_map_protection_tags(&attributes);
    }
    if row.try_get::<bool, _>("spell_origin")? {
        return Ok((false, false));
    }
    Err(SpellItemError::Rejected(
        "source Item protection tags unknown",
    ))
}

pub(crate) async fn prepare_ground_removal_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: &SpellGroundTarget,
    item: [u8; 16],
) -> Result<DurableGroundItemReservation> {
    let rows = locked_tile_rows(tx, authority, target).await?;
    for row in rows {
        if uuid(&row.try_get::<String, _>("item_instance_id")?)? == item {
            let placement = decoded_placement(&row)?;
            let contents = prepare_contained_retirements(tx, authority, item, &placement).await?;
            return Ok(DurableGroundItemReservation {
                item_instance_id: item,
                definition: decoded_definition(&row)?,
                state_revision: integer(row.try_get("state_revision")?)?,
                top_down_ordinal: integer(row.try_get("stack_ordinal")?)?,
                placement,
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| SpellItemError::Rejected("quantity"))?,
                contents,
            });
        }
    }
    Err(SpellItemError::Rejected("item no longer in target custody"))
}

fn reference(value: &TypedDefinitionRef) -> serde_json::Value {
    serde_json::json!({"family":value.family,"key":value.production_key,"revision":value.revision_ref})
}
fn ground(value: &GroundPlacement) -> serde_json::Value {
    serde_json::json!({"position":value.spatial_position,"corpse_ref":value.corpse_ref,"map_revision":value.map_revision,"content_revision":value.content_revision,"context":value.native_room_placement_context})
}
pub(crate) fn encode_cast_cost(cost: &CastCostBinding) -> Result<Vec<u8>> {
    let valid_cooldowns = |values: &[(String, u64)]| {
        values.windows(2).all(|pair| pair[0].0 < pair[1].0)
            && values.iter().all(|(key, _)| {
                (key.starts_with("spell:") && key.len() > 6)
                    || (key.starts_with("group:") && key.len() > 6)
            })
    };
    if cost.vitals_revision_before.checked_add(1) != Some(cost.vitals_revision_after)
        || cost.mana_after > cost.mana_before
        || cost.soul_after > cost.soul_before
        || !valid_cooldowns(&cost.cooldowns_before)
        || !valid_cooldowns(&cost.cooldowns_after)
        || cost.caster_digest_before == [0; 32]
        || cost.caster_digest_after == [0; 32]
        || cost.caster_digest_before == cost.caster_digest_after
    {
        return Err(SpellItemError::Rejected(
            "invalid complete caster cost successor",
        ));
    }
    serde_json::to_vec(&serde_json::json!({"version":1,"before_revision":cost.vitals_revision_before,"after_revision":cost.vitals_revision_after,"mana_before":cost.mana_before,"mana_after":cost.mana_after,"soul_before":cost.soul_before,"soul_after":cost.soul_after,"cooldowns_before":cost.cooldowns_before,"cooldowns_after":cost.cooldowns_after,"caster_before":cost.caster_digest_before.as_slice(),"caster_after":cost.caster_digest_after.as_slice()})).map_err(|_|SpellItemError::Rejected("cost encoding"))
}
fn uuid_v7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}
fn item_definition_valid(value: &TypedDefinitionRef) -> bool {
    value.family == "Item"
        && value
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|id| id > 0)
        && !value.revision_ref.is_empty()
        && value.revision_ref.len() <= 512
}

fn encode_intent(
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    cost: &[u8],
) -> Result<Vec<u8>> {
    if request.command != authority.command
        || request.spell.family != "Spell"
        || request.spell.production_key.is_empty()
        || request.spell.revision_ref.is_empty()
        || request.catalog_digest == [0; 32]
        || !uuid_v7(&request.transaction_id)
        || !uuid_v7(&request.event_id)
        || request.operations.len() > 500
    {
        return Err(SpellItemError::Rejected("spell item request shape"));
    }
    if let Some(origin) = &request.caster_origin {
        if *origin.actor.world_id().as_bytes() != authority.world
            || *origin.actor.channel_id().as_bytes() != authority.channel
            || origin.actor.scope_generation().get() != authority.ownership_generation
            || origin.character_lease_generation != authority.character_lease_generation
        {
            return Err(SpellItemError::Rejected(
                "caster creation source/current fence mismatch",
            ));
        }
    }
    let corpse_ops: Vec<_> = request
        .operations
        .iter()
        .filter_map(|op| {
            if let SpellItemOperation::ConsumeCorpse(c) = op {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    match (&request.companion, corpse_ops.as_slice()) {
        (None, []) => {}
        (Some(p), [corpse])
            if p.binding.transaction == request.transaction_id
                && p.binding.corpse == corpse.item_instance_id
                && p.binding.corpse_revision == corpse.state_revision => {}
        _ => {
            return Err(SpellItemError::Rejected(
                "complete paired corpse/companion intent",
            ));
        }
    }
    if request
        .direct_companion
        .as_ref()
        .is_some_and(|p| !p.valid_intent(request))
    {
        return Err(SpellItemError::Rejected(
            "complete sealed direct acquisition intent",
        ));
    }
    let mut identities = std::collections::BTreeSet::new();
    let mut operations = Vec::new();
    for op in &request.operations {
        let (identity, value) = match op {
            SpellItemOperation::ConsumeInventory(r) => {
                if !r.definition.materializable
                    || !r.definition.inventory_destination
                    || !r.definition.movable
                    || !item_definition_valid(&r.definition.definition)
                    || r.definition.content_generation_digest != authority.compatible_content_digest
                    || r.quantity_before <= r.quantity_after
                    || r.quantity_before > r.definition.stack_maximum
                    || r.state_revision == 0
                {
                    return Err(SpellItemError::Rejected(
                        "unqualified inventory consumption",
                    ));
                }
                let custody = match r.custody {
                    InventoryCustody::Container { parent, ordinal } if ordinal > 0 => {
                        serde_json::json!({"container":parent,"ordinal":ordinal})
                    }
                    InventoryCustody::Equipment {
                        slot,
                        equipment_revision,
                    } if (1..=10).contains(&slot) && slot != 9 && equipment_revision > 0 => {
                        serde_json::json!({"equipment_slot":slot,"equipment_revision":equipment_revision})
                    }
                    _ => return Err(SpellItemError::Rejected("invalid source inventory custody")),
                };
                (
                    r.item_instance_id,
                    serde_json::json!({"kind":"consume_inventory","id":r.item_instance_id,
                    "definition":reference(&r.definition.definition),"content_digest":r.definition.content_generation_digest,
                    "revision":r.state_revision,"before":r.quantity_before,"after":r.quantity_after,
                    "custody":custody,"caster_ground":ground(&r.caster_ground)}),
                )
            }
            SpellItemOperation::RetireContained(r) => {
                if r.item_instance_id == r.root
                    || r.item_instance_id == r.parent
                    || r.ordinal == 0
                    || r.quantity == 0
                    || r.state_revision == 0
                    || r.depth == 0
                    || r.depth > 500
                    || !item_definition_valid(&r.definition)
                    || !request.operations.iter().any(|op| match op {
                        SpellItemOperation::ConsumeCorpse(root) => {
                            root.item_instance_id == r.root && root.placement == r.root_placement
                        }
                        SpellItemOperation::RemoveGround(root) => {
                            root.item_instance_id == r.root && root.placement == r.root_placement
                        }
                        _ => false,
                    })
                {
                    return Err(SpellItemError::Rejected(
                        "contained retirement lacks its exact source root",
                    ));
                }
                (
                    r.item_instance_id,
                    serde_json::json!({"kind":"retire_contained","id":r.item_instance_id,
                    "definition":reference(&r.definition),"revision":r.state_revision,"quantity":r.quantity,
                    "parent":r.parent,"ordinal":r.ordinal,"root":r.root,"depth":r.depth,
                    "placement":ground(&r.root_placement),"corpse_entry":r.corpse_entry}),
                )
            }
            SpellItemOperation::MergeInventory(r) => {
                if !r.definition.materializable
                    || !r.definition.inventory_destination
                    || !r.definition.movable
                    || !item_definition_valid(&r.definition.definition)
                    || r.quantity_before == 0
                    || r.quantity_after <= r.quantity_before
                    || r.quantity_after > r.definition.stack_maximum
                    || r.definition.stack_maximum > 100
                    || r.state_revision == 0
                    || r.placement_ordinal == 0
                    || r.definition.content_generation_digest != authority.compatible_content_digest
                {
                    return Err(SpellItemError::Rejected(
                        "unqualified existing-stack inventory grant",
                    ));
                }
                (
                    r.item_instance_id,
                    serde_json::json!({"kind":"merge_inventory","id":r.item_instance_id,"definition":reference(&r.definition.definition),"content_digest":r.definition.content_generation_digest,"revision":r.state_revision,"before":r.quantity_before,"after":r.quantity_after,"parent":r.parent,"ordinal":r.placement_ordinal,"caster_ground":ground(&r.caster_ground)}),
                )
            }
            SpellItemOperation::ConsumeCorpse(r) => (
                r.item_instance_id,
                serde_json::json!({"kind":"consume_corpse","id":r.item_instance_id,"definition":reference(&r.definition),"revision":r.state_revision,"ordinal":r.top_down_ordinal,"placement":ground(&r.placement),"mint":r.mint_transaction_id,"quantity":r.quantity}),
            ),
            SpellItemOperation::RemoveGround(r) => (
                r.item_instance_id,
                serde_json::json!({"kind":"remove_ground","id":r.item_instance_id,"definition":reference(&r.definition),"revision":r.state_revision,"ordinal":r.top_down_ordinal,"placement":ground(&r.placement),"quantity":r.quantity}),
            ),
            SpellItemOperation::MintInventory {
                item_instance_id,
                definition,
                quantity,
                backpack,
                overflow_placement,
                allow_ground_overflow,
            } => {
                if !definition.materializable
                    || !definition.inventory_destination
                    || (*allow_ground_overflow && !definition.ground_destination)
                    || !item_definition_valid(&definition.definition)
                    || *quantity == 0
                    || *quantity > definition.stack_maximum
                    || definition.stack_maximum > 100
                    || definition.content_generation_digest != authority.compatible_content_digest
                    || backpack.content_generation_digest != authority.compatible_content_digest
                    || backpack.container_capacity.is_none_or(|n| n == 0 || n > 20)
                {
                    return Err(SpellItemError::Rejected(
                        "unqualified bounded inventory grant",
                    ));
                }
                (
                    *item_instance_id,
                    serde_json::json!({"kind":"mint_inventory","id":item_instance_id,"definition":reference(&definition.definition),"content_digest":definition.content_generation_digest.as_slice(),"quantity":quantity,"backpack":reference(&backpack.definition),"capacity":backpack.container_capacity,"overflow":allow_ground_overflow,"overflow_placement":ground(overflow_placement),"decay":definition.decay.as_ref().map(|d|serde_json::json!({"duration_millis":d.duration_millis,"target":d.target.as_ref().map(reference)}))}),
                )
            }
            SpellItemOperation::MintGround {
                item_instance_id,
                definition,
                quantity,
                placement,
                lifetime_millis,
                blocks_movement,
                blocks_projectile,
                description,
            } => {
                if !definition.materializable
                    || !definition.ground_destination
                    || request.caster_origin.is_none()
                    || !item_definition_valid(&definition.definition)
                    || *quantity == 0
                    || *quantity > definition.stack_maximum
                    || definition.stack_maximum > 100
                    || definition.content_generation_digest != authority.compatible_content_digest
                    || matches!(lifetime_millis, Some(0))
                    || description.as_ref().is_some_and(|text| {
                        text.len() > 1024
                            || !text.starts_with("Casted by: ")
                            || text.as_bytes().contains(&0)
                    })
                {
                    return Err(SpellItemError::Rejected(
                        "unqualified or unbounded spell Item MINT",
                    ));
                }
                (
                    *item_instance_id,
                    serde_json::json!({"kind":"mint_ground","id":item_instance_id,"definition":reference(&definition.definition),"content_digest":definition.content_generation_digest.as_slice(),"quantity":quantity,"placement":ground(placement),"lifetime_millis":lifetime_millis,"blocks_movement":blocks_movement,"blocks_projectile":blocks_projectile,"description":description,"decay_chain":definition.decay_chain.iter().map(|stage|serde_json::json!({"definition":reference(&stage.definition),"duration_millis":stage.duration_millis,"target":stage.target.as_ref().map(reference),"blocks_movement":stage.blocks_movement,"blocks_projectile":stage.blocks_projectile,"immovable_block_solid":stage.immovable_block_solid})).collect::<Vec<_>>()}),
                )
            }
        };
        if !uuid_v7(&identity) || !identities.insert(identity) {
            return Err(SpellItemError::Rejected(
                "duplicate or invalid item identity",
            ));
        }
        operations.push(value);
    }
    let mut intent = serde_json::json!({"version":1,"world":authority.world,"channel":authority.channel,"character":authority.character,"session":authority.command.game_session_id().as_bytes(),"command":authority.command.command_id().get(),"ownership_generation":authority.ownership_generation,"spell":reference(&request.spell),"catalog_digest":request.catalog_digest.as_slice(),"transaction":request.transaction_id,"event":request.event_id,"cost":cost,"operations":operations,"companion":request.companion.as_ref().map(|p|p.binding.json()),"caster_origin":request.caster_origin.as_ref().map(|o|serde_json::json!({"placement":o.actor.placement_identity(),"lease":o.character_lease_generation}))});
    if let Some(direct) = &request.direct_companion {
        intent
            .as_object_mut()
            .expect("intent object")
            .insert("direct_companion".into(), direct.binding.json());
    }
    let bytes =
        serde_json::to_vec(&intent).map_err(|_| SpellItemError::Rejected("intent encoding"))?;
    if bytes.len() > 524288 {
        return Err(SpellItemError::Rejected(
            "spell item audit resource ceiling",
        ));
    }
    Ok(bytes)
}

/// The caller commits this transaction together with all companion/caster
/// source receipts. Errors require rollback. An AlreadyCommitted result is a
/// historical classification, never authorization to reapply runtime effects.
pub(crate) async fn apply_spell_items_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
) -> Result<SpellItemTransactionOutcome> {
    apply_spell_items_in_transaction_guarded(tx, authority, request, || Ok(())).await
}

/// Current eligibility gates new source writes. An exact prior occurrence is
/// classified first and still requires its original sealed COMMIT proof before
/// physical reconciliation; historical success does not reacquire authority.
pub(crate) async fn apply_spell_items_in_transaction_guarded<F>(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    new_write_guard: F,
) -> Result<SpellItemTransactionOutcome>
where
    F: FnOnce() -> Result<()>,
{
    if authority.due_read_only {
        return Err(SpellItemError::Rejected(
            "due callback cannot grant a new cast or Item mutation",
        ));
    }
    check_transaction(tx, authority).await?;
    let expanded = expand_retirement_operations(request)?;
    let request = &expanded;
    let cost = encode_cast_cost(&request.cost)?;
    let cost_binding: [u8; 32] = Sha256::digest(&cost).into();
    let intent = encode_intent(authority, request, &cost)?;
    let binding: [u8; 32] = Sha256::digest(&intent).into();
    // A prior unknown COMMIT may still be in flight. Serialize the exact
    // occurrence before reading its receipt or rejecting a new source write.
    // The existing Channel owner lock is acquired first by every caller.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || ':' || $2,44))")
        .bind(authority.command.game_session_id().as_bytes().as_slice())
        .bind(authority.command.command_id().get().to_string())
        .execute(&mut **tx)
        .await?;
    let previous=sqlx::query("SELECT transaction_id::text,event_id::text,binding,cost_binding FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(authority.command.game_session_id().as_bytes().as_slice()).bind(authority.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    let ids: Vec<[u8; 16]> = request
        .operations
        .iter()
        .map(|op| match op {
            SpellItemOperation::ConsumeInventory(r) => r.item_instance_id,
            SpellItemOperation::RetireContained(r) => r.item_instance_id,
            SpellItemOperation::ConsumeCorpse(r) => r.item_instance_id,
            SpellItemOperation::RemoveGround(r) => r.item_instance_id,
            SpellItemOperation::MergeInventory(r) => r.item_instance_id,
            SpellItemOperation::MintGround {
                item_instance_id, ..
            } => *item_instance_id,
            SpellItemOperation::MintInventory {
                item_instance_id, ..
            } => *item_instance_id,
        })
        .collect();
    let receipt = CommittedSpellItems {
        transaction_id: request.transaction_id,
        event_id: request.event_id,
        command: request.command,
        binding,
        cost_binding,
        item_instance_ids: ids,
    };
    if let Some(row) = previous {
        if uuid(&row.try_get::<String, _>("transaction_id")?)? != request.transaction_id
            || uuid(&row.try_get::<String, _>("event_id")?)? != request.event_id
            || row.try_get::<Vec<u8>, _>("binding")?.as_slice() != binding
            || row.try_get::<Vec<u8>, _>("cost_binding")?.as_slice() != cost_binding
        {
            return Err(SpellItemError::Rejected(
                "conflicting spell CommandRef replay",
            ));
        }
        return Ok(SpellItemTransactionOutcome::AlreadyCommitted(receipt));
    }
    new_write_guard()?;
    let occurred_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query("INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms,caster_lease_generation,caster_placement_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric(20,0),$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19::text::numeric(20,0),$20)")
        .bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(authority.character.as_slice()).bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(authority.ownership_generation.to_string()).bind(&request.spell.family).bind(&request.spell.production_key).bind(&request.spell.revision_ref).bind(request.catalog_digest.as_slice()).bind(binding.as_slice()).bind(&intent).bind(&cost).bind(cost_binding.as_slice()).bind(i32::try_from(request.operations.len()).map_err(|_|SpellItemError::Rejected("operation count"))?).bind(occurred_at).bind(request.caster_origin.as_ref().map(|o|o.character_lease_generation.to_string())).bind(request.caster_origin.as_ref().map(|o|o.actor.placement_identity().to_vec())).execute(&mut **tx).await?;
    for (index, op) in request.operations.iter().enumerate() {
        apply_operation(tx, authority, request, index + 1, occurred_at, op).await?;
    }
    apply_consumed_equipment_epoch(tx, authority, request).await?;
    sqlx::query("INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$3+7776000000,$4,$5)")
        .bind(request.event_id.as_slice()).bind(request.transaction_id.as_slice()).bind(occurred_at).bind(intent).bind(binding.as_slice()).execute(&mut **tx).await?;
    Ok(SpellItemTransactionOutcome::Applied(receipt))
}

async fn inventory_destination(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    definition: &QualifiedItemDefinition,
    quantity: u32,
    backpack: &QualifiedItemDefinition,
    overflow: bool,
) -> Result<Option<([u8; 16], u64)>> {
    let row=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(authority.character.as_slice()).bind(authority.world.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return if overflow {
            Ok(None)
        } else {
            Err(SpellItemError::Rejected("no equipped main backpack"))
        };
    };
    if decoded_definition(&row)? != backpack.definition {
        return Err(SpellItemError::Rejected(
            "backpack Content revision mismatch",
        ));
    }
    let parent = uuid(&row.try_get::<String, _>("item_instance_id")?)?;
    // Source stack behavior must never silently allocate another slot/drop an
    // item when the actual compatible owner stack would receive it.
    if definition.stack_maximum > 1 {
        let merge:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_family=$2 AND i.definition_production_key=$3 AND i.definition_revision_ref=$4 AND i.quantity+$5<=$6)")
            .bind(parent.as_slice()).bind(&definition.definition.family).bind(&definition.definition.production_key).bind(&definition.definition.revision_ref).bind(i64::from(quantity)).bind(i64::from(definition.stack_maximum)).fetch_one(&mut **tx).await?;
        if merge {
            return Err(SpellItemError::Rejected(
                "inventory grant requires exact existing-stack MINT line",
            ));
        }
    }
    let row=sqlx::query("SELECT count(*) AS count,COALESCE(max(placement_ordinal),0)::text AS last FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(parent.as_slice()).fetch_one(&mut **tx).await?;
    let capacity = backpack
        .container_capacity
        .ok_or(SpellItemError::Rejected("unknown backpack capacity"))?;
    if row.try_get::<i64, _>("count")? >= i64::from(capacity) {
        return if overflow {
            Ok(None)
        } else {
            Err(SpellItemError::Rejected("backpack capacity exceeded"))
        };
    }
    let next = integer(row.try_get("last")?)?
        .checked_add(1)
        .ok_or(SpellItemError::Rejected("backpack ordinal exhausted"))?;
    Ok(Some((parent, next)))
}

/// Reserve the real compatible direct backpack stack. Outputs of the same
/// definition in one source cast must first be grouped into one exact grant;
/// there is one quantity successor per actual ItemInstance per transaction.
pub(crate) async fn prepare_inventory_merge_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    definition: &QualifiedItemDefinition,
    quantity: u32,
    backpack: &QualifiedItemDefinition,
    caster_ground: &GroundPlacement,
) -> Result<Option<DurableInventoryGrantReservation>> {
    check_transaction(tx, authority).await?;
    if quantity == 0 || definition.stack_maximum <= 1 || quantity > definition.stack_maximum {
        return if quantity > 0 && definition.stack_maximum == 1 {
            Ok(None)
        } else {
            Err(SpellItemError::Rejected("invalid inventory merge amount"))
        };
    }
    if !definition.materializable
        || !definition.inventory_destination
        || !definition.movable
        || definition.content_generation_digest != authority.compatible_content_digest
        || backpack.content_generation_digest != authority.compatible_content_digest
    {
        return Err(SpellItemError::Rejected("inventory merge Content mismatch"));
    }
    let row=sqlx::query("SELECT i.item_instance_id::text,i.state_revision::text,i.quantity,e.placement_ordinal::text,e.parent_item_instance_id::text AS parent,b.definition_family AS backpack_family,b.definition_production_key AS backpack_key,b.definition_revision_ref AS backpack_revision FROM game_item_container_slots s JOIN game_item_instances b ON b.item_instance_id=s.item_instance_id JOIN game_item_container_entries e ON e.parent_item_instance_id=b.item_instance_id JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND e.character_id=s.character_id AND i.world_id=s.world_id AND b.lifecycle=1 AND i.lifecycle=1 AND i.definition_family=$3 AND i.definition_production_key=$4 AND i.definition_revision_ref=$5 AND i.quantity<$7 AND $6::bigint>0 ORDER BY e.placement_ordinal LIMIT 1 FOR UPDATE OF b,i")
        .bind(authority.character.as_slice()).bind(authority.world.as_slice()).bind(&definition.definition.family).bind(&definition.definition.production_key).bind(&definition.definition.revision_ref).bind(i64::from(quantity)).bind(i64::from(definition.stack_maximum)).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    if row.try_get::<String, _>("backpack_family")? != backpack.definition.family
        || row.try_get::<String, _>("backpack_key")? != backpack.definition.production_key
        || row.try_get::<String, _>("backpack_revision")? != backpack.definition.revision_ref
    {
        return Err(SpellItemError::Rejected(
            "actual inventory merge backpack changed",
        ));
    }
    let before = u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| SpellItemError::Rejected("corrupt inventory quantity"))?;
    Ok(Some(DurableInventoryGrantReservation {
        item_instance_id: uuid(&row.try_get::<String, _>("item_instance_id")?)?,
        definition: definition.clone(),
        state_revision: integer(row.try_get("state_revision")?)?,
        quantity_before: before,
        quantity_after: before
            .checked_add(quantity.min(definition.stack_maximum - before))
            .ok_or(SpellItemError::Rejected(
                "inventory merge quantity overflow",
            ))?,
        parent: uuid(&row.try_get::<String, _>("parent")?)?,
        placement_ordinal: integer(row.try_get("placement_ordinal")?)?,
        caster_ground: caster_ground.clone(),
    }))
}

/// Resolve all compatible direct-backpack stacks in source inventory order.
/// The remaining quantity is explicit; callers must mint it, never discard it.
/// One reservation per instance permits a single exact quantity successor even
/// when a cast produces two equal food items.
pub(crate) async fn prepare_inventory_merges_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    definition: &QualifiedItemDefinition,
    quantity: u32,
    backpack: &QualifiedItemDefinition,
    caster_ground: &GroundPlacement,
) -> Result<(Vec<DurableInventoryGrantReservation>, u32)> {
    check_transaction(tx, authority).await?;
    if quantity == 0
        || quantity > 50_000
        || definition.stack_maximum == 0
        || definition.stack_maximum > 100
        || !definition.materializable
        || !definition.inventory_destination
        || !definition.movable
        || definition.content_generation_digest != authority.compatible_content_digest
        || backpack.content_generation_digest != authority.compatible_content_digest
    {
        return Err(SpellItemError::Rejected("invalid bounded inventory grant"));
    }
    if definition.stack_maximum == 1 {
        return Ok((Vec::new(), quantity));
    }
    let rows=sqlx::query("SELECT i.item_instance_id::text,i.state_revision::text,i.quantity,e.placement_ordinal::text,e.parent_item_instance_id::text AS parent,b.definition_family AS backpack_family,b.definition_production_key AS backpack_key,b.definition_revision_ref AS backpack_revision FROM game_item_container_slots s JOIN game_item_instances b ON b.item_instance_id=s.item_instance_id JOIN game_item_container_entries e ON e.parent_item_instance_id=b.item_instance_id JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND e.character_id=s.character_id AND i.world_id=s.world_id AND b.lifecycle=1 AND i.lifecycle=1 AND i.definition_family=$3 AND i.definition_production_key=$4 AND i.definition_revision_ref=$5 AND i.quantity<$6 ORDER BY e.placement_ordinal LIMIT 501 FOR UPDATE OF b,i")
        .bind(authority.character.as_slice()).bind(authority.world.as_slice())
        .bind(&definition.definition.family).bind(&definition.definition.production_key)
        .bind(&definition.definition.revision_ref).bind(i64::from(definition.stack_maximum))
        .fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Err(SpellItemError::Rejected("inventory grant work bound"));
    }
    let mut remaining = quantity;
    let mut reservations = Vec::new();
    for row in rows {
        if row.try_get::<String, _>("backpack_family")? != backpack.definition.family
            || row.try_get::<String, _>("backpack_key")? != backpack.definition.production_key
            || row.try_get::<String, _>("backpack_revision")? != backpack.definition.revision_ref
        {
            return Err(SpellItemError::Rejected(
                "actual inventory grant backpack changed",
            ));
        }
        let before = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| SpellItemError::Rejected("corrupt inventory quantity"))?;
        if before == 0 || before >= definition.stack_maximum {
            return Err(SpellItemError::Rejected("invalid live inventory stack"));
        }
        if remaining == 0 {
            continue;
        }
        let granted = remaining.min(definition.stack_maximum - before);
        reservations.push(DurableInventoryGrantReservation {
            item_instance_id: uuid(&row.try_get::<String, _>("item_instance_id")?)?,
            definition: definition.clone(),
            state_revision: integer(row.try_get("state_revision")?)?,
            quantity_before: before,
            quantity_after: before + granted,
            parent: uuid(&row.try_get::<String, _>("parent")?)?,
            placement_ordinal: integer(row.try_get("placement_ordinal")?)?,
            caster_ground: caster_ground.clone(),
        });
        remaining -= granted;
    }
    Ok((reservations, remaining))
}

async fn apply_operation(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    ordinal: usize,
    occurred_at: i64,
    operation: &SpellItemOperation,
) -> Result<()> {
    if matches!(
        operation,
        SpellItemOperation::ConsumeInventory(_) | SpellItemOperation::RetireContained(_)
    ) {
        return apply_consumption_operation(tx, authority, request, ordinal, operation).await;
    }
    let destination = if let SpellItemOperation::MergeInventory(r) = operation {
        Some((r.parent, r.placement_ordinal))
    } else if let SpellItemOperation::MintInventory {
        definition,
        quantity,
        backpack,
        allow_ground_overflow,
        ..
    } = operation
    {
        inventory_destination(
            tx,
            authority,
            definition,
            *quantity,
            backpack,
            *allow_ground_overflow,
        )
        .await?
    } else {
        None
    };
    let (
        kind,
        item,
        definition,
        quantity_before,
        quantity_after,
        revision,
        stack,
        placement,
        content_digest,
        blocks_movement,
        blocks_projectile,
        lifetime,
    ) = match operation {
        SpellItemOperation::ConsumeInventory(_) | SpellItemOperation::RetireContained(_) => {
            return Err(SpellItemError::Rejected(
                "consumption must use its exact source owner route",
            ));
        }
        SpellItemOperation::MergeInventory(r) => (
            4,
            r.item_instance_id,
            &r.definition.definition,
            r.quantity_before,
            r.quantity_after,
            r.state_revision,
            None,
            &r.caster_ground,
            r.definition.content_generation_digest,
            false,
            false,
            None,
        ),
        SpellItemOperation::MintGround {
            item_instance_id,
            definition,
            quantity,
            placement,
            lifetime_millis,
            blocks_movement,
            blocks_projectile,
            ..
        } => (
            1i16,
            *item_instance_id,
            &definition.definition,
            0,
            *quantity,
            0,
            None,
            placement,
            definition.content_generation_digest,
            *blocks_movement,
            *blocks_projectile,
            *lifetime_millis,
        ),
        SpellItemOperation::ConsumeCorpse(r) => (
            3i16,
            r.item_instance_id,
            &r.definition,
            r.quantity,
            0,
            r.state_revision,
            Some(r.top_down_ordinal),
            &r.placement,
            authority.compatible_content_digest,
            false,
            false,
            None,
        ),
        SpellItemOperation::RemoveGround(r) => (
            2i16,
            r.item_instance_id,
            &r.definition,
            r.quantity,
            0,
            r.state_revision,
            Some(r.top_down_ordinal),
            &r.placement,
            authority.compatible_content_digest,
            false,
            false,
            None,
        ),
        SpellItemOperation::MintInventory {
            item_instance_id,
            definition,
            quantity,
            overflow_placement,
            ..
        } => (
            1,
            *item_instance_id,
            &definition.definition,
            0,
            *quantity,
            0,
            None,
            overflow_placement,
            definition.content_generation_digest,
            false,
            false,
            None,
        ),
    };
    if !item_definition_valid(definition) {
        return Err(SpellItemError::Rejected("unqualified Item identity"));
    }
    let expires = lifetime
        .map(|value| {
            occurred_at
                .checked_add(i64::from(value))
                .ok_or(SpellItemError::Rejected("field deadline overflow"))
        })
        .transpose()?;
    if let SpellItemOperation::MergeInventory(r) = operation {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.state_revision=$3::text::numeric(20,0) AND i.quantity=$4 AND i.definition_family=$5 AND i.definition_production_key=$6 AND i.definition_revision_ref=$7 AND e.character_id=encode($8,'hex')::uuid AND e.parent_item_instance_id=encode($9,'hex')::uuid AND e.placement_ordinal=$10::text::numeric(20,0) AND s.character_id=e.character_id)")
            .bind(item.as_slice()).bind(authority.world.as_slice()).bind(revision.to_string()).bind(i64::from(quantity_before)).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(authority.character.as_slice()).bind(r.parent.as_slice()).bind(r.placement_ordinal.to_string()).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(SpellItemError::Rejected(
                "reserved existing stack/custody changed",
            ));
        }
    } else if kind != 1 {
        let target = SpellGroundTarget {
            spatial_position: placement.spatial_position.clone(),
            map_revision: placement.map_revision.clone(),
            content_revision: placement.content_revision.clone(),
            placement_context: placement.native_room_placement_context.clone(),
        };
        let rows = locked_tile_rows(tx, authority, &target).await?;
        let index = rows
            .iter()
            .position(|row| {
                row.try_get::<String, _>("item_instance_id")
                    .ok()
                    .and_then(|s| uuid(&s).ok())
                    == Some(item)
            })
            .ok_or(SpellItemError::Rejected("reserved item left its Ground"))?;
        let row = &rows[index];
        if decoded_definition(row)? != *definition
            || decoded_placement(row)? != *placement
            || integer(row.try_get("state_revision")?)? != revision
            || Some(integer(row.try_get("stack_ordinal")?)?) != stack
            || row.try_get::<i64, _>("quantity")? != i64::from(quantity_before)
        {
            return Err(SpellItemError::Rejected(
                "reserved item state or exact custody changed",
            ));
        }
        if let SpellItemOperation::ConsumeCorpse(reservation) = operation {
            if index != 0
                || !row.try_get::<bool, _>("corpse")?
                || uuid(&row.try_get::<String, _>("minted_transaction_id")?)?
                    != reservation.mint_transaction_id
            {
                return Err(SpellItemError::Rejected(
                    "reserved corpse is no longer the exact top corpse",
                ));
            }
        }
        let children:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&mut **tx).await?;
        if children != 0 {
            return Err(SpellItemError::Rejected(
                "item owns unretired corpse entries",
            ));
        }
    }
    sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,source_stack_ordinal,content_generation_digest,blocks_movement,blocks_projectile,expires_at_unix_ms,immovable_block_solid,destination_parent_item_instance_id,destination_ordinal) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10::text::numeric(20,0),encode($11,'hex')::uuid,encode($12,'hex')::uuid,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,CASE WHEN $23::bytea IS NULL THEN NULL ELSE encode($23,'hex')::uuid END,$24::text::numeric(20,0))")
        .bind(request.transaction_id.as_slice()).bind(i32::try_from(ordinal).map_err(|_|SpellItemError::Rejected("line ordinal"))?).bind(kind).bind(item.as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(quantity_before)).bind(i64::from(quantity_after)).bind(revision.to_string()).bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(&placement.spatial_position).bind(&placement.map_revision).bind(&placement.content_revision).bind(&placement.native_room_placement_context).bind(stack.map(|n|i64::try_from(n).map_err(|_|SpellItemError::Rejected("Ground ordinal overflow"))).transpose()?).bind(content_digest.as_slice()).bind(blocks_movement).bind(blocks_projectile).bind(expires).bind(match operation {SpellItemOperation::MintGround{definition,blocks_movement,..}=> !definition.movable && *blocks_movement, _=>false}).bind(destination.as_ref().map(|(id,_)|id.as_slice())).bind(destination.as_ref().map(|(_,n)|n.to_string())).execute(&mut **tx).await?;
    if kind == 1 {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid)")
            .bind(item.as_slice()).bind(authority.world.as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(quantity_after)).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
        if let SpellItemOperation::MintGround { definition, .. } = operation {
            super::spell_item_temporal::schedule_field_chain_in_transaction(
                tx,
                authority,
                request,
                definition,
                item,
                u32::try_from(ordinal)
                    .map_err(|_| SpellItemError::Rejected("field stage ordinal"))?,
                occurred_at,
            )
            .await?;
        }
        if let SpellItemOperation::MintGround {
            description: Some(description),
            ..
        } = operation
        {
            sqlx::query("INSERT INTO game_spell_item_source_descriptions(item_instance_id,transaction_id,ordinal,description) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)")
                .bind(item.as_slice())
                .bind(request.transaction_id.as_slice())
                .bind(i32::try_from(ordinal).map_err(|_| SpellItemError::Rejected("description ordinal"))?)
                .bind(description)
                .execute(&mut **tx).await?;
        }
        if let SpellItemOperation::MintInventory { definition, .. } = operation {
            super::spell_item_temporal::schedule_minted_item_in_transaction(
                tx,
                authority,
                request,
                definition,
                item,
                u32::try_from(ordinal)
                    .map_err(|_| SpellItemError::Rejected("temporal source ordinal"))?,
                occurred_at,
            )
            .await?;
        }
        if let Some((parent, ordinal)) = destination {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric(20,0),encode($6,'hex')::uuid)")
                .bind(item.as_slice()).bind(authority.world.as_slice()).bind(authority.character.as_slice()).bind(parent.as_slice()).bind(ordinal.to_string()).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6,$7,$8,$9)")
            .bind(item.as_slice()).bind(authority.world.as_slice()).bind(authority.channel.as_slice()).bind(authority.ownership_generation.to_string()).bind(&placement.spatial_position).bind(&placement.corpse_ref).bind(&placement.map_revision).bind(&placement.content_revision).bind(&placement.native_room_placement_context).execute(&mut **tx).await?;
        }
    } else if kind == 4 {
        let updated=sqlx::query("UPDATE game_item_instances SET quantity=$1,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($3,'hex')::uuid AND lifecycle=1 AND state_revision=$4::text::numeric(20,0) AND quantity=$5")
            .bind(i64::from(quantity_after)).bind(request.transaction_id.as_slice()).bind(item.as_slice()).bind(revision.to_string()).bind(i64::from(quantity_before)).execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(SpellItemError::Rejected(
                "existing inventory stack compare-and-grant refused",
            ));
        }
    } else {
        let updated=sqlx::query("UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid AND lifecycle=1 AND state_revision=$3::text::numeric(20,0)")
            .bind(request.transaction_id.as_slice()).bind(item.as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(SpellItemError::Rejected("item compare-and-retire refused"));
        }
        let removed=sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid AND stack_ordinal=$2")
            .bind(item.as_slice()).bind(stack.map(|n|i64::try_from(n).map_err(|_|SpellItemError::Rejected("Ground ordinal overflow"))).transpose()?).execute(&mut **tx).await?;
        if removed.rows_affected() != 1 {
            return Err(SpellItemError::Rejected(
                "Ground compare-and-remove refused",
            ));
        }
    }
    Ok(())
}

/// Historical source integrity only. This cannot grant current gameplay,
/// session, scope, Content or recovery authority.
pub(crate) async fn verify_spell_item_chain(
    tx: &mut Transaction<'_, Postgres>,
) -> std::result::Result<(), DurabilityError> {
    let corrupt:bool=sqlx::query_scalar(r#"
      SELECT EXISTS(
        SELECT 1 FROM game_spell_item_receipts r
        LEFT JOIN game_spell_item_audit_outbox a USING(transaction_id)
        WHERE r.binding<>sha256(r.intent) OR r.cost_binding<>sha256(r.cost)
           OR a.event_id IS DISTINCT FROM r.event_id OR a.envelope IS DISTINCT FROM r.intent
           OR a.envelope_sha256 IS DISTINCT FROM r.binding OR a.created_xact_id IS DISTINCT FROM r.created_xact_id
           OR a.occurred_at_unix_ms IS DISTINCT FROM r.occurred_at_unix_ms
           OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=r.transaction_id)<>r.operation_count
        UNION ALL
        SELECT 1 FROM game_spell_item_lines l
        LEFT JOIN game_spell_item_receipts r USING(transaction_id)
        LEFT JOIN game_item_instances i USING(item_instance_id)
        WHERE r.transaction_id IS NULL OR i.item_instance_id IS NULL
           OR l.created_xact_id<>r.created_xact_id OR l.ordinal>r.operation_count
           OR (l.world_id,l.channel_id)<>(r.world_id,r.channel_id)
           OR ((l.definition_family,l.definition_production_key,l.definition_revision)<>(i.definition_family,i.definition_production_key,i.definition_revision_ref)
               AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
                 JOIN game_spell_item_temporal_receipts t USING(item_instance_id)
                 JOIN game_spell_item_temporal_before b USING(transaction_id)
                 JOIN game_spell_item_temporal_audit a USING(transaction_id)
                 WHERE p.item_instance_id=i.item_instance_id AND p.target_family IS NOT NULL
                   AND (p.definition_family,p.definition_production_key,p.definition_revision)=(l.definition_family,l.definition_production_key,l.definition_revision)
                   AND (p.target_family,p.target_production_key,p.target_revision)=(i.definition_family,i.definition_production_key,i.definition_revision_ref)
                   AND b.item_instance_id=i.item_instance_id AND b.created_xact_id=t.created_xact_id
                   AND a.created_xact_id=t.created_xact_id AND a.envelope_sha256=sha256(a.envelope)
                   AND convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(t),'schedule',to_jsonb(p)))
               AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
                 JOIN game_spell_field_temporal_receipts t ON t.item_instance_id=p.item_instance_id AND t.source_stage=p.source_stage
                 JOIN game_spell_field_temporal_before b USING(transaction_id)
                 JOIN game_spell_field_temporal_audit a USING(transaction_id)
                 WHERE p.item_instance_id=i.item_instance_id AND p.source_transaction_id=i.minted_transaction_id
                   AND p.target_family IS NOT NULL
                   AND (p.target_family,p.target_production_key,p.target_revision)=(i.definition_family,i.definition_production_key,i.definition_revision_ref)
                   AND b.item_instance_id=i.item_instance_id AND b.created_xact_id=t.created_xact_id
                   AND a.created_xact_id=t.created_xact_id AND a.envelope_sha256=sha256(a.envelope)
                   AND convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(t),'schedule',to_jsonb(p))))
           OR (l.operation_kind=1 AND i.minted_transaction_id<>l.transaction_id)
           OR (l.operation_kind IN(2,3,6) AND (i.lifecycle<>2 OR i.last_transaction_id<>l.transaction_id OR i.quantity<>0
               OR i.state_revision<>l.state_revision_before+1
               OR EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)))
           OR (l.operation_kind IN(4,5) AND (i.state_revision<l.state_revision_before+1
               OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.transaction_id=l.transaction_id
                 AND q.item_instance_id=l.item_instance_id AND q.quantity_before=l.quantity_before)
               OR (i.last_transaction_id=l.transaction_id AND (i.quantity<>l.quantity_after
                 OR i.state_revision<>l.state_revision_before+1 OR i.lifecycle<>CASE WHEN l.quantity_after=0 THEN 2 ELSE 1 END))))
           OR (l.operation_kind IN(5,6) AND NOT EXISTS(SELECT 1 FROM game_spell_item_source_custody c
               WHERE c.transaction_id=l.transaction_id AND c.item_instance_id=l.item_instance_id AND c.quantity_before=l.quantity_before
                 AND c.state_revision_before=l.state_revision_before AND c.source_parent IS NOT DISTINCT FROM l.source_parent_item_instance_id
                 AND c.source_ordinal IS NOT DISTINCT FROM l.source_placement_ordinal AND c.source_slot IS NOT DISTINCT FROM l.source_equipment_slot
                 AND c.created_xact_id=l.created_xact_id))
           OR (l.operation_kind=5 AND l.quantity_after=0 AND (i.lifecycle<>2 OR i.last_transaction_id<>l.transaction_id
                 OR i.quantity<>0 OR i.state_revision<>l.state_revision_before+1))
           OR (l.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=l.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION'))
        UNION ALL
        SELECT 1 FROM game_spell_equipment_consumptions c
        JOIN game_spell_item_receipts r USING(transaction_id)
        LEFT JOIN game_character_equipment_state s ON s.character_id=c.character_id
        WHERE c.created_xact_id<>r.created_xact_id OR c.character_id<>r.character_id
          OR s.character_id IS NULL OR s.revision<c.revision_after
          OR (s.last_transaction_id=c.transaction_id AND (s.revision<>c.revision_after OR s.combat_mode IS DISTINCT FROM c.combat_mode))
          OR NOT EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=c.transaction_id AND l.operation_kind=5
               AND l.source_equipment_slot IS NOT NULL AND l.source_equipment_revision=c.revision_before AND l.created_xact_id=c.created_xact_id)
        UNION ALL
        SELECT 1 FROM game_spell_item_expiry_receipts e
        LEFT JOIN game_spell_item_lines l ON l.transaction_id=e.source_transaction_id AND l.ordinal=e.source_ordinal
        LEFT JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
        WHERE l.operation_kind IS DISTINCT FROM 1 OR l.item_instance_id IS DISTINCT FROM e.item_instance_id
          OR l.expires_at_unix_ms IS DISTINCT FROM e.expires_at_unix_ms OR e.occurred_at_unix_ms<e.expires_at_unix_ms
          OR i.lifecycle IS DISTINCT FROM 2 OR i.quantity IS DISTINCT FROM 0 OR i.last_transaction_id IS DISTINCT FROM e.transaction_id
          OR i.state_revision IS DISTINCT FROM e.state_revision_before+1
          OR EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=e.item_instance_id)
        UNION ALL
        SELECT 1 FROM game_spell_field_temporal_receipts t
        LEFT JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=t.item_instance_id AND p.source_stage=t.source_stage
        LEFT JOIN game_spell_field_temporal_before b USING(transaction_id)
        LEFT JOIN game_spell_field_temporal_audit a USING(transaction_id)
        WHERE p.item_instance_id IS NULL OR b.item_instance_id IS DISTINCT FROM t.item_instance_id
          OR b.state_revision_before IS DISTINCT FROM t.state_revision_before OR b.quantity_before IS DISTINCT FROM t.quantity_before
          OR (b.definition_family,b.definition_production_key,b.definition_revision) IS DISTINCT FROM (p.definition_family,p.definition_production_key,p.definition_revision)
          OR b.created_xact_id IS DISTINCT FROM t.created_xact_id OR a.created_xact_id IS DISTINCT FROM t.created_xact_id
          OR a.event_id IS DISTINCT FROM t.event_id OR a.envelope_sha256 IS DISTINCT FROM sha256(a.envelope)
          OR t.occurred_at_unix_ms<p.expires_at_unix_ms
          OR convert_from(a.envelope,'UTF8')::jsonb IS DISTINCT FROM jsonb_build_object('receipt',to_jsonb(t),'schedule',to_jsonb(p))
        UNION ALL
        SELECT 1 FROM game_spell_companion_acquisition_receipts c
        LEFT JOIN game_spell_item_receipts r USING(transaction_id)
        LEFT JOIN game_spell_item_lines l ON l.transaction_id=c.transaction_id AND l.operation_kind=3
        WHERE c.created_xact_id IS DISTINCT FROM r.created_xact_id OR l.created_xact_id IS DISTINCT FROM c.created_xact_id
          OR l.item_instance_id IS DISTINCT FROM c.corpse_item_instance_id OR l.state_revision_before IS DISTINCT FROM c.corpse_state_revision
          OR (r.world_id,r.channel_id,r.ownership_generation,r.character_id,r.game_session_id)
              IS DISTINCT FROM (c.world_id,c.channel_id,c.ownership_generation,c.master_character_id,c.master_game_session_id)
          OR (convert_from(r.intent,'UTF8')::jsonb)->'companion' IS DISTINCT FROM c.binding_json
      )
    "#).fetch_one(&mut **tx).await?;
    if corrupt {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut cursor = String::from("00000000-0000-0000-0000-000000000000");
    loop {
        let rows=sqlx::query("SELECT transaction_id::text,cost,intent,binding,cost_binding FROM game_spell_item_receipts WHERE transaction_id>$1::uuid ORDER BY transaction_id LIMIT 256")
        .bind(&cursor).fetch_all(&mut **tx).await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            cursor = row.try_get("transaction_id")?;
            let bytes: Vec<u8> = row.try_get("cost")?;
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
            let number = |key: &str| {
                value
                    .get(key)
                    .and_then(|v| v.as_u64())
                    .ok_or(DurabilityError::InvalidStoredState)
            };
            let digest = |key: &str| -> std::result::Result<[u8; 32], DurabilityError> {
                let bytes: Vec<u8> = serde_json::from_value(
                    value
                        .get(key)
                        .cloned()
                        .ok_or(DurabilityError::InvalidStoredState)?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?;
                bytes
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)
            };
            let cooldowns = |key: &str| {
                serde_json::from_value(
                    value
                        .get(key)
                        .cloned()
                        .ok_or(DurabilityError::InvalidStoredState)?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)
            };
            let cost = CastCostBinding {
                vitals_revision_before: number("before_revision")?,
                vitals_revision_after: number("after_revision")?,
                mana_before: u32::try_from(number("mana_before")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                mana_after: u32::try_from(number("mana_after")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                soul_before: u32::try_from(number("soul_before")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                soul_after: u32::try_from(number("soul_after")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                cooldowns_before: cooldowns("cooldowns_before")?,
                cooldowns_after: cooldowns("cooldowns_after")?,
                caster_digest_before: digest("caster_before")?,
                caster_digest_after: digest("caster_after")?,
            };
            if encode_cast_cost(&cost).map_err(|_| DurabilityError::InvalidStoredState)? != bytes {
                return Err(DurabilityError::InvalidStoredState);
            }
            let intent: serde_json::Value =
                serde_json::from_slice(&row.try_get::<Vec<u8>, _>("intent")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            let intent_cost: Vec<u8> = serde_json::from_value(
                intent
                    .get("cost")
                    .cloned()
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            if intent_cost != bytes {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cost() -> CastCostBinding {
        CastCostBinding {
            vitals_revision_before: 9,
            vitals_revision_after: 10,
            mana_before: 100,
            mana_after: 60,
            soul_before: 5,
            soul_after: 3,
            cooldowns_before: vec![("spell:old".into(), 123)],
            cooldowns_after: vec![("group:attack".into(), 2000), ("spell:old".into(), 123)],
            caster_digest_before: [1; 32],
            caster_digest_after: [2; 32],
        }
    }
    #[test]
    fn costs_bind_all_namespaced_integer_deadlines() {
        let before = encode_cast_cost(&cost()).unwrap();
        let mut changed = cost();
        changed.cooldowns_after[0].1 += 1;
        assert_ne!(
            Sha256::digest(&before),
            Sha256::digest(encode_cast_cost(&changed).unwrap())
        );
    }
    #[test]
    fn invalid_cost_successors_fail_closed() {
        let mut c = cost();
        c.vitals_revision_after = 11;
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.mana_after = 101;
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.soul_after = 6;
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.cooldowns_after.reverse();
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.cooldowns_after
            .push(c.cooldowns_after.last().unwrap().clone());
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.cooldowns_after[0].0 = "attack".into();
        assert!(encode_cast_cost(&c).is_err());
        c = cost();
        c.caster_digest_after = c.caster_digest_before;
        assert!(encode_cast_cost(&c).is_err());
    }
}

async fn apply_consumption_operation(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    ordinal: usize,
    operation: &SpellItemOperation,
) -> Result<()> {
    let (
        kind,
        id,
        definition,
        before,
        after,
        revision,
        placement,
        parent,
        parent_ordinal,
        slot,
        equipment_revision,
        root,
        corpse_entry,
    ) = match operation {
        SpellItemOperation::ConsumeInventory(r) => {
            let (parent, parent_ordinal, slot, equipment_revision) = match r.custody {
                InventoryCustody::Container { parent, ordinal } => {
                    (Some(parent), Some(ordinal), None, None)
                }
                InventoryCustody::Equipment {
                    slot,
                    equipment_revision,
                } => (None, None, Some(slot), Some(equipment_revision)),
            };
            (
                5i16,
                r.item_instance_id,
                &r.definition.definition,
                r.quantity_before,
                r.quantity_after,
                r.state_revision,
                &r.caster_ground,
                parent,
                parent_ordinal,
                slot,
                equipment_revision,
                None,
                None,
            )
        }
        SpellItemOperation::RetireContained(r) => (
            6i16,
            r.item_instance_id,
            &r.definition,
            r.quantity,
            0,
            r.state_revision,
            &r.root_placement,
            Some(r.parent),
            Some(r.ordinal),
            None,
            None,
            Some(r.root),
            Some(r.corpse_entry),
        ),
        _ => return Err(SpellItemError::Rejected("wrong consumption route")),
    };
    let row=sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref,quantity,state_revision::text,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(id.as_slice()).bind(authority.world.as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(SpellItemError::Rejected("source consumption ItemInstance missing"))?;
    if decoded_definition(&row)? != *definition
        || row.try_get::<i16, _>("lifecycle")? != 1
        || row.try_get::<i64, _>("quantity")? != i64::from(before)
        || integer(row.try_get("state_revision")?)? != revision
    {
        return Err(SpellItemError::Rejected(
            "source consumption ItemInstance changed",
        ));
    }
    if after == 0 {
        let has_children:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
            .bind(id.as_slice()).fetch_one(&mut **tx).await?;
        if has_children {
            return Err(SpellItemError::Rejected(
                "retirement requires complete deepest-first contents plan",
            ));
        }
    }
    // The Item BEFORE UPDATE trigger captures actual OLD custody itself.
    // Payloads cannot fabricate a parent or slot in lieu of that evidence.
    sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,immovable_block_solid,source_parent_item_instance_id,source_placement_ordinal,source_equipment_slot,source_equipment_revision,source_corpse_root,source_entry_is_corpse) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10::text::numeric(20,0),encode($11,'hex')::uuid,encode($12,'hex')::uuid,$13,$14,$15,$16,$17,false,CASE WHEN $18::bytea IS NULL THEN NULL ELSE encode($18,'hex')::uuid END,$19::text::numeric(20,0),$20,$21::text::numeric(20,0),CASE WHEN $22::bytea IS NULL THEN NULL ELSE encode($22,'hex')::uuid END,$23)")
        .bind(request.transaction_id.as_slice()).bind(i32::try_from(ordinal).map_err(|_|SpellItemError::Rejected("line work bound"))?)
        .bind(kind).bind(id.as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref)
        .bind(i64::from(before)).bind(i64::from(after)).bind(revision.to_string()).bind(authority.world.as_slice()).bind(authority.channel.as_slice())
        .bind(&placement.spatial_position).bind(&placement.map_revision).bind(&placement.content_revision).bind(&placement.native_room_placement_context)
        .bind(authority.compatible_content_digest.as_slice()).bind(parent.as_ref().map(|v|v.as_slice())).bind(parent_ordinal.map(|v|v.to_string()))
        .bind(slot.map(i16::from)).bind(equipment_revision.map(|v|v.to_string())).bind(root.as_ref().map(|v|v.as_slice())).bind(corpse_entry)
        .execute(&mut **tx).await?;
    let changed=sqlx::query("UPDATE game_item_instances SET quantity=$1,lifecycle=$2,last_transaction_id=encode($3,'hex')::uuid WHERE item_instance_id=encode($4,'hex')::uuid AND lifecycle=1 AND quantity=$5 AND state_revision=$6::text::numeric(20,0)")
        .bind(i64::from(after)).bind(if after==0{2i16}else{1i16}).bind(request.transaction_id.as_slice())
        .bind(id.as_slice()).bind(i64::from(before)).bind(revision.to_string()).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(SpellItemError::Rejected(
            "source consumption compare/update failed",
        ));
    }
    if after == 0 {
        let changed = if let Some(slot) = slot {
            sqlx::query("DELETE FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND slot=$3")
                .bind(id.as_slice()).bind(authority.character.as_slice()).bind(i16::from(slot)).execute(&mut **tx).await?
        } else if corpse_entry == Some(true) {
            sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid AND placement_ordinal=$3::text::numeric(20,0)")
                .bind(id.as_slice()).bind(parent.ok_or(SpellItemError::Rejected("missing corpse parent"))?.as_slice())
                .bind(parent_ordinal.ok_or(SpellItemError::Rejected("missing corpse ordinal"))?.to_string()).execute(&mut **tx).await?
        } else {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid AND placement_ordinal=$3::text::numeric(20,0)")
                .bind(id.as_slice()).bind(parent.ok_or(SpellItemError::Rejected("missing container parent"))?.as_slice())
                .bind(parent_ordinal.ok_or(SpellItemError::Rejected("missing container ordinal"))?.to_string()).execute(&mut **tx).await?
        };
        if changed.rows_affected() != 1 {
            return Err(SpellItemError::Rejected(
                "source consumption exact custody disappeared",
            ));
        }
    }
    Ok(())
}

async fn apply_consumed_equipment_epoch(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
) -> Result<()> {
    let revisions: std::collections::BTreeSet<u64> = request
        .operations
        .iter()
        .filter_map(|op| match op {
            SpellItemOperation::ConsumeInventory(r) => match r.custody {
                InventoryCustody::Equipment {
                    equipment_revision, ..
                } => Some(equipment_revision),
                _ => None,
            },
            _ => None,
        })
        .collect();
    if revisions.is_empty() {
        return Ok(());
    }
    if revisions.len() != 1 {
        return Err(SpellItemError::Rejected(
            "inconsistent actual equipment source epochs",
        ));
    }
    let before = *revisions
        .first()
        .ok_or(SpellItemError::Rejected("missing equipment epoch"))?;
    let after = before
        .checked_add(1)
        .ok_or(SpellItemError::Rejected("equipment epoch exhausted"))?;
    let row=sqlx::query("SELECT revision::text,combat_mode FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(authority.character.as_slice()).fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("actual equipment state absent"))?;
    if integer(row.try_get("revision")?)? != before {
        return Err(SpellItemError::Rejected("actual equipment epoch changed"));
    }
    let mode: Option<i16> = row.try_get("combat_mode")?;
    sqlx::query("INSERT INTO game_spell_equipment_consumptions(transaction_id,character_id,revision_before,revision_after,combat_mode) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4::text::numeric(20,0),$5)")
        .bind(request.transaction_id.as_slice()).bind(authority.character.as_slice()).bind(before.to_string()).bind(after.to_string()).bind(mode).execute(&mut **tx).await?;
    let changed=sqlx::query("UPDATE game_character_equipment_state SET revision=$1::text::numeric(20,0),last_transaction_id=encode($2,'hex')::uuid WHERE character_id=encode($3,'hex')::uuid AND revision=$4::text::numeric(20,0)")
        .bind(after.to_string()).bind(request.transaction_id.as_slice()).bind(authority.character.as_slice()).bind(before.to_string()).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(SpellItemError::Rejected(
            "equipment consumption epoch compare/update failed",
        ));
    }
    Ok(())
}

/// Existing real custody rows only. The root and every visited ItemInstance
/// stay locked; all corpse-entry writers share its Channel scope lock (0042).
async fn prepare_contained_retirements(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    root: [u8; 16],
    placement: &GroundPlacement,
) -> Result<Vec<DurableContainedRetirement>> {
    check_transaction(tx, authority).await?;
    let mut pending = std::collections::VecDeque::from([(root, 0u16)]);
    let mut seen = std::collections::BTreeSet::from([root]);
    let mut result = Vec::new();
    while let Some((parent, depth)) = pending.pop_front() {
        let rows=sqlx::query("SELECT item_instance_id::text,parent_item_instance_id::text,placement_ordinal::text,true AS corpse_entry FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id::text,parent_item_instance_id::text,placement_ordinal::text,false AS corpse_entry FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal LIMIT 501")
            .bind(parent.as_slice()).fetch_all(&mut **tx).await?;
        if result.len() + rows.len() > 499 {
            return Err(SpellItemError::Rejected(
                "complete contained retirement exceeds source transaction work bound",
            ));
        }
        for row in rows {
            let id = uuid(&row.try_get::<String, _>("item_instance_id")?)?;
            if !seen.insert(id) {
                return Err(SpellItemError::Rejected(
                    "actual container custody cycle or duplicate",
                ));
            }
            let item=sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref,quantity,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
                .bind(id.as_slice()).bind(authority.world.as_slice()).fetch_optional(&mut **tx).await?
                .ok_or(SpellItemError::Rejected("contained ItemInstance no longer live in root World"))?;
            let definition = decoded_definition(&item)?;
            if !item_definition_valid(&definition) {
                return Err(SpellItemError::Rejected(
                    "unqualified contained Item identity",
                ));
            }
            let child_depth = depth
                .checked_add(1)
                .ok_or(SpellItemError::Rejected("container depth exhausted"))?;
            let corpse_entry: bool = row.try_get("corpse_entry")?;
            // Recheck exact immutable custody under the locked ItemInstance.
            // The Channel owner lock also prevents entry phantoms.
            let present=if corpse_entry{
                sqlx::query_scalar::<_,String>("SELECT placement_ordinal::text FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
                    .bind(id.as_slice()).bind(parent.as_slice()).fetch_optional(&mut **tx).await?
            }else{
                sqlx::query_scalar::<_,String>("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
                    .bind(id.as_slice()).bind(parent.as_slice()).fetch_optional(&mut **tx).await?
            }.ok_or(SpellItemError::Rejected("contained exact custody changed"))?;
            if present != row.try_get::<String, _>("placement_ordinal")? {
                return Err(SpellItemError::Rejected("contained ordinal changed"));
            }
            result.push(DurableContainedRetirement {
                item_instance_id: id,
                definition,
                state_revision: integer(item.try_get("state_revision")?)?,
                quantity: u32::try_from(item.try_get::<i64, _>("quantity")?)
                    .map_err(|_| SpellItemError::Rejected("contained quantity"))?,
                parent,
                ordinal: integer(present)?,
                root,
                depth: child_depth,
                root_placement: placement.clone(),
                corpse_entry,
            });
            pending.push_back((id, child_depth));
        }
    }
    if !result.is_empty() {
        let real:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND loot_purpose_key='CORPSE_MATERIALIZATION')")
            .bind(root.as_slice()).fetch_one(&mut **tx).await?;
        if !real {
            return Err(SpellItemError::Rejected(
                "contained retirement requires actual materialized corpse source",
            ));
        }
    }
    result.sort_by(|a, b| {
        b.depth
            .cmp(&a.depth)
            .then(a.ordinal.cmp(&b.ordinal))
            .then(a.item_instance_id.cmp(&b.item_instance_id))
    });
    Ok(result)
}

fn expand_retirement_operations(
    request: &SpellItemTransactionRequest,
) -> Result<SpellItemTransactionRequest> {
    let mut expanded = request.clone();
    expanded.operations.clear();
    for operation in &request.operations {
        let contents = match operation {
            SpellItemOperation::ConsumeCorpse(r) => r.contents.as_slice(),
            SpellItemOperation::RemoveGround(r) => r.contents.as_slice(),
            _ => &[],
        };
        expanded.operations.extend(
            contents
                .iter()
                .cloned()
                .map(SpellItemOperation::RetireContained),
        );
        let mut root = operation.clone();
        match &mut root {
            SpellItemOperation::ConsumeCorpse(r) => r.contents.clear(),
            SpellItemOperation::RemoveGround(r) => r.contents.clear(),
            _ => {}
        }
        expanded.operations.push(root);
        if expanded.operations.len() > 500 {
            return Err(SpellItemError::Rejected(
                "complete Item source transaction exceeds work bound",
            ));
        }
    }
    Ok(expanded)
}

/// Resolve an actual carried donor, including equipment. A missing wire Item
/// handle can only select a reagent by its explicit source-qualified type; rune
/// usage callers must provide the independently negotiated exact instance.
pub(crate) async fn prepare_inventory_consumption_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    qualified: &QualifiedItemDefinition,
    quantity: u32,
    exact_instance: Option<[u8; 16]>,
    caster_ground: &GroundPlacement,
) -> Result<DurableInventoryConsumption> {
    check_transaction(tx, authority).await?;
    if quantity == 0
        || !qualified.materializable
        || !qualified.inventory_destination
        || !qualified.movable
        || qualified.content_generation_digest != authority.compatible_content_digest
        || !item_definition_valid(&qualified.definition)
    {
        return Err(SpellItemError::Rejected(
            "unqualified carried consumption definition",
        ));
    }
    let row=sqlx::query("SELECT i.item_instance_id::text,i.quantity,i.state_revision::text,e.parent_item_instance_id::text AS parent,e.placement_ordinal::text AS ordinal,s.slot,es.revision::text AS equipment_revision FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id LEFT JOIN game_item_container_slots bag ON bag.item_instance_id=e.parent_item_instance_id LEFT JOIN game_character_equipment_slots s ON s.item_instance_id=i.item_instance_id LEFT JOIN game_character_equipment_state es ON es.character_id=s.character_id WHERE i.world_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_family=$2 AND i.definition_production_key=$3 AND i.definition_revision_ref=$4 AND i.quantity>=$5 AND ($6::bytea IS NULL OR i.item_instance_id=encode($6,'hex')::uuid) AND ((e.character_id=encode($7,'hex')::uuid AND bag.character_id=e.character_id) OR (s.character_id=encode($7,'hex')::uuid AND s.world_id=i.world_id)) ORDER BY s.slot ASC NULLS LAST,e.placement_ordinal DESC LIMIT 1 FOR UPDATE OF i")
        .bind(authority.world.as_slice()).bind(&qualified.definition.family).bind(&qualified.definition.production_key)
        .bind(&qualified.definition.revision_ref).bind(i64::from(quantity)).bind(exact_instance.as_ref().map(|v|v.as_slice()))
        .bind(authority.character.as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(SpellItemError::Rejected("source carried donor absent"))?;
    let before = u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| SpellItemError::Rejected("source donor quantity"))?;
    if before > qualified.stack_maximum {
        return Err(SpellItemError::Rejected(
            "source donor stack policy mismatch",
        ));
    }
    let parent: Option<String> = row.try_get("parent")?;
    let slot: Option<i16> = row.try_get("slot")?;
    let custody = match (parent, slot) {
        (Some(parent), None) => InventoryCustody::Container {
            parent: uuid(&parent)?,
            ordinal: integer(row.try_get("ordinal")?)?,
        },
        (None, Some(slot)) => InventoryCustody::Equipment {
            slot: u8::try_from(slot).map_err(|_| SpellItemError::Rejected("source donor slot"))?,
            equipment_revision: integer(row.try_get("equipment_revision")?)?,
        },
        _ => {
            return Err(SpellItemError::Rejected(
                "source donor custody missing or ambiguous",
            ));
        }
    };
    Ok(DurableInventoryConsumption {
        item_instance_id: uuid(&row.try_get::<String, _>("item_instance_id")?)?,
        definition: qualified.clone(),
        state_revision: integer(row.try_get("state_revision")?)?,
        quantity_before: before,
        quantity_after: before - quantity,
        custody,
        caster_ground: caster_ground.clone(),
    })
}

/// The pinned RuneSpell::executeUse consumes one actual ItemCount after a
/// successful cast when REMOVE_RUNE_CHARGES is enabled. Candidate source import
/// binds that configuration; quantity is the source counter, not invented loot.
pub(crate) async fn prepare_rune_instance_consumption_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    qualified: &QualifiedRuneDefinition,
    item_instance_id: [u8; 16],
    expected_state_revision: u64,
    caster_ground: &GroundPlacement,
    qualify_parent: &(dyn Fn(&TypedDefinitionRef) -> Option<QualifiedItemDefinition> + Send + Sync),
) -> Result<DurableRuneUseReservation> {
    check_transaction(tx, authority).await?;
    if expected_state_revision == 0
        || item_instance_id[6] >> 4 != 7
        || item_instance_id[8] & 192 != 128
        || qualified.source_item_id == 0
    {
        return Err(SpellItemError::Rejected("Rune exact identity/revision"));
    }
    let q = &qualified.item;
    if !q.materializable
        || !q.inventory_destination
        || !q.movable
        || q.content_generation_digest != authority.compatible_content_digest
    {
        return Err(SpellItemError::Rejected(
            "unqualified Rune source definition",
        ));
    }
    let row=sqlx::query("SELECT i.item_instance_id::text,i.quantity,i.state_revision::text,e.parent_item_instance_id::text AS parent,e.placement_ordinal::text AS ordinal,s.slot,es.revision::text AS equipment_revision FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id LEFT JOIN game_character_equipment_slots s ON s.item_instance_id=i.item_instance_id LEFT JOIN game_character_equipment_state es ON es.character_id=s.character_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=($3,$4,$5) AND i.quantity>=1 AND ((e.character_id=encode($6,'hex')::uuid AND e.world_id=i.world_id) OR (s.character_id=encode($6,'hex')::uuid AND s.world_id=i.world_id)) FOR UPDATE OF i")
        .bind(item_instance_id.as_slice()).bind(authority.world.as_slice()).bind(&q.definition.family)
        .bind(&q.definition.production_key).bind(&q.definition.revision_ref).bind(authority.character.as_slice())
        .fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("exact carried Rune absent"))?;
    let parent: Option<String> = row.try_get("parent")?;
    let slot: Option<i16> = row.try_get("slot")?;
    let custody = match (parent, slot) {
        (Some(parent), None) => {
            verify_rune_container_chain(tx, authority, uuid(&parent)?, qualify_parent).await?;
            InventoryCustody::Container {
                parent: uuid(&parent)?,
                ordinal: integer(row.try_get("ordinal")?)?,
            }
        }
        (None, Some(slot)) => InventoryCustody::Equipment {
            slot: u8::try_from(slot)
                .map_err(|_| SpellItemError::Rejected("Rune equipment slot"))?,
            equipment_revision: integer(row.try_get("equipment_revision")?)?,
        },
        _ => {
            return Err(SpellItemError::Rejected(
                "Rune custody is missing or ambiguous",
            ));
        }
    };
    let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| SpellItemError::Rejected("Rune source ItemCount"))?;
    if quantity == 0 || quantity > q.stack_maximum {
        return Err(SpellItemError::Rejected(
            "Rune source ItemCount violates policy",
        ));
    }
    let source = DurableInventoryConsumption {
        item_instance_id,
        definition: q.clone(),
        state_revision: integer(row.try_get("state_revision")?)?,
        quantity_before: quantity,
        quantity_after: quantity - 1,
        custody,
        caster_ground: caster_ground.clone(),
    };
    if source.state_revision != expected_state_revision {
        return Err(SpellItemError::Rejected(
            "Rune source state revision changed",
        ));
    }
    Ok(DurableRuneUseReservation {
        physical_transaction: authority.physical_transaction.clone(),
        command: authority.command,
        source,
    })
}

pub(crate) async fn validate_rune_reservation_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    reservation: &DurableRuneUseReservation,
) -> Result<()> {
    check_transaction(tx, authority).await?;
    if reservation.physical_transaction != authority.physical_transaction
        || reservation.command != authority.command
    {
        return Err(SpellItemError::Rejected(
            "Rune reservation transaction/command mismatch",
        ));
    }
    Ok(())
}

/// Every container ancestry hop is an actual locked live Item and exact current
/// Character custody. The native compositor's callback resolves each actual
/// DefinitionRef from the same activated Content; a missing policy is unknown.
async fn verify_rune_container_chain(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    mut parent: [u8; 16],
    qualify_parent: &(dyn Fn(&TypedDefinitionRef) -> Option<QualifiedItemDefinition> + Send + Sync),
) -> Result<()> {
    let mut visited = std::collections::BTreeSet::new();
    for _ in 0..500 {
        if !visited.insert(parent) {
            return Err(SpellItemError::Rejected("Rune container ancestry cycle"));
        }
        let row=sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,bag.item_instance_id::text AS root,e.parent_item_instance_id::text AS parent,(SELECT count(*) FROM game_item_container_entries children WHERE children.parent_item_instance_id=i.item_instance_id) AS children FROM game_item_instances i LEFT JOIN game_item_container_slots bag ON bag.item_instance_id=i.item_instance_id AND bag.character_id=encode($3,'hex')::uuid AND bag.world_id=i.world_id LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($3,'hex')::uuid AND e.world_id=i.world_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 FOR UPDATE OF i")
            .bind(parent.as_slice()).bind(authority.world.as_slice()).bind(authority.character.as_slice())
            .fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("Rune ancestor Item not live/current"))?;
        let definition = decoded_definition(&row)?;
        let policy = qualify_parent(&definition)
            .ok_or(SpellItemError::Rejected("Rune ancestor Content unknown"))?;
        let capacity =
            policy
                .container_capacity
                .filter(|c| *c > 0)
                .ok_or(SpellItemError::Rejected(
                    "Rune ancestor is not qualified container",
                ))?;
        if policy.definition != definition
            || policy.content_generation_digest != authority.compatible_content_digest
            || !policy.materializable
            || !policy.inventory_destination
            || row.try_get::<i64, _>("children")? > i64::from(capacity)
        {
            return Err(SpellItemError::Rejected(
                "Rune ancestor exact policy/capacity mismatch",
            ));
        }
        let root: Option<String> = row.try_get("root")?;
        let next: Option<String> = row.try_get("parent")?;
        match (root, next) {
            (Some(_), None) => return Ok(()),
            (None, Some(next)) => parent = uuid(&next)?,
            _ => {
                return Err(SpellItemError::Rejected(
                    "Rune ancestor custody missing/ambiguous",
                ));
            }
        }
    }
    Err(SpellItemError::Rejected(
        "Rune ancestry exceeds bounded source owner pass",
    ))
}

/// Exact current carried target, minted by the Item reader in this transaction.
/// It describes custody; it cannot debit, mint or authorize a cast by itself.
#[derive(Debug)]
pub(crate) struct CarriedSpellTarget {
    definition: TypedDefinitionRef,
    instance: [u8; 16],
    state_revision: u64,
    custody: InventoryCustody,
    physical_transaction: String,
    command: CommandRef,
}
impl CarriedSpellTarget {
    pub(crate) fn definition(&self) -> &TypedDefinitionRef {
        &self.definition
    }
    pub(crate) fn item_instance(&self) -> [u8; 16] {
        self.instance
    }
    pub(crate) fn state_revision(&self) -> u64 {
        self.state_revision
    }
    pub(crate) fn custody(&self) -> &InventoryCustody {
        &self.custody
    }
    pub(crate) async fn check_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        authority: &SpellItemAuthority,
    ) -> Result<()> {
        check_transaction(tx, authority).await?;
        if self.physical_transaction != authority.physical_transaction
            || self.command != authority.command
        {
            return Err(SpellItemError::Rejected(
                "carried Item target transaction changed",
            ));
        }
        Ok(())
    }
}
/// No client slot/ownership assertion is accepted. Every current parent is
/// locked and qualified to its actual activated container policy, including
/// nested custody, cycle, capacity, live world and Character binding.
pub(crate) async fn read_carried_spell_target_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    item_instance: [u8; 16],
    expected_state_revision: u64,
    qualify_parent: &(dyn Fn(&TypedDefinitionRef) -> Option<QualifiedItemDefinition> + Send + Sync),
) -> Result<CarriedSpellTarget> {
    check_transaction(tx, authority).await?;
    if expected_state_revision == 0 || !uuid_v7(&item_instance) {
        return Err(SpellItemError::Rejected("carried target identity/revision"));
    }
    let row=sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.state_revision::text,e.parent_item_instance_id::text AS parent,e.placement_ordinal::text AS ordinal,s.slot,es.revision::text AS equipment_revision FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id LEFT JOIN game_character_equipment_slots s ON s.item_instance_id=i.item_instance_id LEFT JOIN game_character_equipment_state es ON es.character_id=s.character_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND ((e.character_id=encode($3,'hex')::uuid AND e.world_id=i.world_id) OR (s.character_id=encode($3,'hex')::uuid AND s.world_id=i.world_id)) FOR UPDATE OF i")
        .bind(item_instance.as_slice()).bind(authority.world.as_slice()).bind(authority.character.as_slice()).fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("current carried target absent"))?;
    let definition = decoded_definition(&row)?;
    let policy = qualify_parent(&definition).ok_or(SpellItemError::Rejected(
        "carried target active Item policy missing",
    ))?;
    let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| SpellItemError::Rejected("carried target quantity"))?;
    let state_revision = integer(row.try_get("state_revision")?)?;
    if policy.definition != definition
        || !policy.inventory_destination
        || !policy.movable
        || policy.content_generation_digest != authority.compatible_content_digest
        || quantity > policy.stack_maximum
        || state_revision != expected_state_revision
    {
        return Err(SpellItemError::Rejected(
            "carried target exact policy/predecessor changed",
        ));
    }
    let parent: Option<String> = row.try_get("parent")?;
    let slot: Option<i16> = row.try_get("slot")?;
    let custody = match (parent, slot) {
        (Some(parent), None) => {
            let parent = uuid(&parent)?;
            verify_rune_container_chain(tx, authority, parent, qualify_parent).await?;
            InventoryCustody::Container {
                parent,
                ordinal: integer(row.try_get("ordinal")?)?,
            }
        }
        (None, Some(slot)) => InventoryCustody::Equipment {
            slot: u8::try_from(slot)
                .map_err(|_| SpellItemError::Rejected("carried target slot"))?,
            equipment_revision: integer(row.try_get("equipment_revision")?)?,
        },
        _ => return Err(SpellItemError::Rejected("carried target custody ambiguous")),
    };
    Ok(CarriedSpellTarget {
        definition,
        instance: item_instance,
        state_revision,
        custody,
        physical_transaction: authority.physical_transaction.clone(),
        command: authority.command,
    })
}

/// Read the exact top-down source corpse Definition under the current tile
/// locks. Qualification and the later consumption reservation remain separate.
pub(crate) async fn current_top_corpse_definition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: &SpellGroundTarget,
) -> Result<TypedDefinitionRef> {
    let rows = locked_tile_rows(tx, authority, target).await?;
    let row = rows
        .first()
        .ok_or(SpellItemError::Rejected("source corpse tile empty"))?;
    if !row.try_get::<bool, _>("corpse")? {
        return Err(SpellItemError::Rejected(
            "source top Item is not a materialized corpse",
        ));
    }
    decoded_definition(row)
}

/// build_source_world preserves numeric OTBM tags as decimal JSON keys:
/// ATTR_ACTION_ID=4 and ATTR_UNIQUE_ID=5, not the separate region-codec names.
fn source_map_protection_tags(attributes: &serde_json::Value) -> Result<(bool, bool)> {
    let attributes = attributes
        .as_object()
        .ok_or(SpellItemError::Rejected("source item attributes shape"))?;
    if attributes.contains_key("unique") || attributes.contains_key("action") {
        return Err(SpellItemError::Rejected(
            "source map attribute codec substitution",
        ));
    }
    let tag = |name: &str| -> Result<bool> {
        match attributes.get(name) {
            None => Ok(false),
            Some(value) => value
                .as_u64()
                .filter(|n| *n <= u64::from(u16::MAX))
                .map(|n| n != 0)
                .ok_or(SpellItemError::Rejected("source item tag shape")),
        }
    };
    Ok((tag("5")?, tag("4")?))
}
#[cfg(test)]
mod map_protection_tag_tests {
    use super::*;
    #[test]
    fn source_otbm_action_unique_tags_are_not_lost_by_named_codec() {
        assert_eq!(
            source_map_protection_tags(&serde_json::json!({"5":42,"4":3,"15":0})).unwrap(),
            (true, true)
        );
        assert_eq!(
            source_map_protection_tags(&serde_json::json!({"5":0,"4":0})).unwrap(),
            (false, false)
        );
        assert!(source_map_protection_tags(&serde_json::json!({"unique":42})).is_err());
        for value in [
            serde_json::json!(-1),
            serde_json::json!(65536),
            serde_json::json!("42"),
            serde_json::json!(true),
        ] {
            assert!(source_map_protection_tags(&serde_json::json!({"5":value})).is_err());
        }
    }
}
