//! MAP-OVERLAY-1c bounded retirement of one real Ground item during an admitted reset.
//!
//! The private schema below is an UNINSTALLED candidate. Only the control role may publish
//! a reset/cohort; callers supply neither an epoch nor a current-generation claim. This is
//! step 3, not admission closure, assignment orchestration or content activation.
use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_decay_retire_audit::{
    self as audit, OneItemDecayRetireV1, OneItemGroundContainerEntryV1, OneItemWorldResetV1,
};
use super::item_mint_audit::{
    OneItemGroundV1, OneItemStateV1, OneItemTypedDefinitionRevisionV1, RL08_RETRY_WORK_UNITS_MAX,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug)]
pub enum WorldResetError {
    NotCurrent,
    NotInitialized,
    InvalidInitialization,
    UnqualifiedTarget,
    ConflictingCause,
    CapacityExceeded,
    InvalidStoredState,
    HouseSourceRefused,
    DrainBindingRefused,
    HouseTargetMembershipChanged { house_key: String },
    HouseCatalogueChanged { house_key: String },
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for WorldResetError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldResetRetirement {
    pub item_instance_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub reset_epoch: u64,
}
/// A durable reservation identity, never current authority. Retain it after every unknown
/// outcome; retry/reconciliation uses these exact IDs and the persisted work budget.
#[derive(Debug, Clone)]
pub struct WorldResetCandidate {
    world: WorldId,
    channel: ChannelId,
    generation: u64,
    retirement: WorldResetRetirement,
}
impl WorldResetCandidate {
    /// Frozen data for outcome lookup, never permission to perform a new transition.
    pub const fn identity(&self) -> &WorldResetRetirement {
        &self.retirement
    }
}
enum Phase {
    Freeze,
    Charge(WorldResetCandidate),
    Commit(WorldResetCandidate),
}
enum PhaseResult {
    Frozen(Option<WorldResetCandidate>),
    Charged,
    Committed(WorldResetRetirement),
}
fn uuid(value: String) -> Result<[u8; 16], DurabilityError> {
    super::item_mint::uuid_text(&value)
}

/// Durable census data, not a drain certificate or permission to activate content.
/// The caller must retain the transaction and its locks through its next operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldResetCensus {
    /// World bound by the actual census query, never relabelled by its consumer.
    pub world: WorldId,
    pub reset_epoch: u64,
    pub target_digest: [u8; 32],
    pub reset_revision: u64,
    pub channel_count: u64,
    pub ground_root_count: u64,
    pub ground_corpse_entry_count: u64,
}

/// D1134: census the complete durable Channel cohort in the caller's transaction.
///
/// Epoch/revision/target come from the locked control-published job, never the request.
/// Current incarnation and every ordinary Channel's replacement/closed-readiness/session
/// predicates come from the native database owners. House assignments are not Channels.
/// This follows retirement's admission -> job -> assignment lock order. Ground and corpse
/// entry relation locks then keep this census stable until the caller ends the transaction.
/// Call before taking Item locks; never retain this value as current proof after COMMIT.
///
/// `None` means no RETIRING job. A stale incarnation, incomplete cohort, foreign holder,
/// open readiness or live/reconnectable session refuses. This does not prove the transport's
/// in-memory actor/pending-work drain, target House/property eligibility, content activation
/// issuance or live quiescence. Those are additional required final-activation boundaries.
pub async fn census_retiring_world_reset(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    world: WorldId,
) -> Result<Option<WorldResetCensus>, WorldResetError> {
    lock_admission_relations(tx).await?;
    let job = sqlx::query(
        "SELECT reset_epoch::text,target_digest,revision::text,active_map_revision FROM game_world_reset_jobs \
         WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING' FOR UPDATE",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let Some(job) = job else {
        read_active_world_revision(tx, world).await?;
        return Ok(None);
    };
    let active = read_active_world_revision(tx, world).await?;
    let epoch: String = job.try_get(0).map_err(DurabilityError::from)?;
    if epoch
        .parse::<u64>()
        .map_err(|_| WorldResetError::InvalidStoredState)?
        != active.epoch
        || job.try_get::<String, _>(3).map_err(DurabilityError::from)? != active.map_revision
    {
        return Err(WorldResetError::NotCurrent);
    }
    sqlx::query("LOCK TABLE game_runtime_scope_assignments IN SHARE MODE")
        .execute(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    if !prove_current_incarnation(tx, node).await? {
        return Err(WorldResetError::NotCurrent);
    }
    let fact = node.fact();
    let current: bool = sqlx::query_scalar(
        "SELECT game_world_reset_cohort_current(encode($1,'hex')::uuid,$2::text::numeric,\
         encode($3,'hex')::uuid,$4::text::numeric)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(&epoch)
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    if !current {
        return Err(WorldResetError::NotCurrent);
    }
    sqlx::query(
        "LOCK TABLE game_item_ground_locations,game_item_corpse_container_entries IN SHARE MODE",
    )
    .execute(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let counts = sqlx::query(
        "SELECT \
         (SELECT count(*) FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1),\
         (SELECT count(*) FROM game_item_ground_locations WHERE world_id=encode($1,'hex')::uuid),\
         (SELECT count(*) FROM game_item_corpse_container_entries e JOIN game_item_ground_locations g \
          ON g.item_instance_id=e.parent_item_instance_id WHERE g.world_id=encode($1,'hex')::uuid)",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let count = |index| -> Result<u64, WorldResetError> {
        u64::try_from(
            counts
                .try_get::<i64, _>(index)
                .map_err(DurabilityError::from)?,
        )
        .map_err(|_| WorldResetError::InvalidStoredState)
    };
    Ok(Some(WorldResetCensus {
        world,
        reset_epoch: epoch
            .parse()
            .map_err(|_| WorldResetError::InvalidStoredState)?,
        target_digest: job
            .try_get::<Vec<u8>, _>(1)
            .map_err(DurabilityError::from)?
            .try_into()
            .map_err(|_| WorldResetError::InvalidStoredState)?,
        reset_revision: job
            .try_get::<String, _>(2)
            .map_err(DurabilityError::from)?
            .parse()
            .map_err(|_| WorldResetError::InvalidStoredState)?,
        channel_count: count(0)?,
        ground_root_count: count(1)?,
        ground_corpse_entry_count: count(2)?,
    }))
}

fn check_world_drain_data(
    binding: &super::world_reset_port::WorldDrainBinding,
    world: WorldId,
    node: super::runtime_scope_assignment::NodeRegistrationFact,
) -> Result<(), WorldResetError> {
    if binding.world != world
        || binding.node != node
        || super::world_reset_port::world_drain_cohort_digest(world, node, &binding.channels)
            .map_err(|_| WorldResetError::DrainBindingRefused)?
            != binding.cohort_digest
    {
        return Err(WorldResetError::DrainBindingRefused);
    }
    Ok(())
}

fn check_world_drain_channel(
    expected: Option<&super::world_reset_port::DrainedChannelBinding>,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
) -> Result<(), WorldResetError> {
    if !expected.is_some_and(|entry| entry.scope == scope && entry.generation == generation) {
        return Err(WorldResetError::DrainBindingRefused);
    }
    Ok(())
}

/// D1233 consumer-only check. No witness implementation or proof constructor lives here.
/// Reacquires native admission -> RETIRING job -> assignments locks through the existing
/// census; independently verifies current incarnation, complete replacement cohort, closed
/// readiness and no live/reconnect sessions. Then streams every World Channel assignment,
/// comparing its independently read holder/generation against the entire borrowed proof.
/// Keep this same TX/locks through final activation; success is not a reusable token.
/// I's sealed proof must additionally establish completed in-memory logout/work drain.
pub(crate) async fn check_world_drain_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    world: WorldId,
    witness: &(dyn super::world_reset_port::WorldDrainWitness + Sync),
) -> Result<(), WorldResetError> {
    if census_retiring_world_reset(tx, node, world)
        .await?
        .is_none()
    {
        return Err(WorldResetError::NotCurrent);
    }
    let binding = witness.drained();
    let fact = node.fact();
    check_world_drain_data(binding, world, fact)?;
    let query = sqlx::query("SELECT channel_id::text,ownership_generation::text,state,holder_node_id::text,holder_registration_revision::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 ORDER BY uuid_send(channel_id)")
        .bind(world.as_bytes().as_slice());
    let mut rows = query.fetch(&mut **tx);
    let mut index = 0;
    while let Some(row) = std::future::poll_fn(|cx| rows.as_mut().poll_next(cx)).await {
        let row = row.map_err(DurabilityError::from)?;
        let channel = ChannelId::decode(&uuid(row.try_get(0).map_err(DurabilityError::from)?)?)
            .map_err(|_| WorldResetError::InvalidStoredState)?;
        let generation = row
            .try_get::<String, _>(1)
            .map_err(DurabilityError::from)?
            .parse::<u64>()
            .map_err(|_| WorldResetError::InvalidStoredState)?;
        let state: i16 = row.try_get(2).map_err(DurabilityError::from)?;
        let holder: Option<String> = row.try_get(3).map_err(DurabilityError::from)?;
        let revision: Option<String> = row.try_get(4).map_err(DurabilityError::from)?;
        if state != 1
            || holder.map(uuid).transpose()?.as_ref() != Some(fact.node_id().as_bytes())
            || revision
                .map(|r| r.parse::<u64>())
                .transpose()
                .map_err(|_| WorldResetError::InvalidStoredState)?
                != Some(fact.registration_revision())
        {
            return Err(WorldResetError::DrainBindingRefused);
        }
        check_world_drain_channel(
            binding.channels.get(index),
            crate::foundation::RuntimeScopeRefV1::channel(world, channel),
            generation,
        )?;
        index += 1;
    }
    if index != binding.channels.len() {
        return Err(WorldResetError::DrainBindingRefused);
    }
    Ok(())
}

// A source catalogue keeps opaque native position bytes; this owner never decodes or
// recreates C's position encoder. One House definition digest must agree across its tiles.
type ResetHouseCatalogue =
    std::collections::BTreeMap<String, ([u8; 32], std::collections::BTreeSet<Vec<u8>>)>;

fn reset_house_catalogue(
    facts: Vec<super::world_reset_port::HouseResetFact>,
) -> Result<ResetHouseCatalogue, WorldResetError> {
    // Bounds from the actual qualified native House source (1024 Houses,4096 cells each).
    if facts.len() > 1024 * 4096 {
        return Err(WorldResetError::HouseSourceRefused);
    }
    let mut catalogue = ResetHouseCatalogue::new();
    let mut positions = std::collections::BTreeSet::new();
    for fact in facts {
        let suffix = fact.house_key.strip_prefix("oteryn:content.house.");
        if fact.house_key.len() > 512
            || suffix.is_none_or(|value| {
                value.is_empty()
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
            || !(1..=128).contains(&fact.spatial_position.len())
            || !positions.insert(fact.spatial_position.clone())
        {
            return Err(WorldResetError::HouseSourceRefused);
        }
        let entry = catalogue
            .entry(fact.house_key)
            .or_insert_with(|| (fact.definition_digest, std::collections::BTreeSet::new()));
        if entry.0 != fact.definition_digest
            || entry.1.len() >= 4096
            || !entry.1.insert(fact.spatial_position)
        {
            return Err(WorldResetError::HouseSourceRefused);
        }
    }
    if catalogue.len() > 1024 {
        return Err(WorldResetError::HouseSourceRefused);
    }
    Ok(catalogue)
}

/// Compare source-derived immutable identity with independently read database facts.
/// A caller-selected Active/Target label alone never supplies the binding.
fn bound_reset_house_catalogue(
    catalogue: super::world_reset_port::QualifiedHouseResetCatalogue,
    world: WorldId,
    map_revision: &str,
    server_artifact_digest: [u8; 32],
) -> Result<ResetHouseCatalogue, WorldResetError> {
    if catalogue.world != world
        || catalogue.map_revision.is_empty()
        || catalogue.map_revision != map_revision
        || catalogue.server_artifact_digest != server_artifact_digest
    {
        return Err(WorldResetError::HouseSourceRefused);
    }
    reset_house_catalogue(catalogue.facts)
}

fn check_reset_house_membership(
    target: &ResetHouseCatalogue,
    house_key: &str,
    position: &[u8],
) -> Result<(), WorldResetError> {
    if !target
        .get(house_key)
        .is_some_and(|(_, positions)| positions.contains(position))
    {
        return Err(WorldResetError::HouseTargetMembershipChanged {
            house_key: house_key.to_owned(),
        });
    }
    Ok(())
}

fn check_reset_house_definition(
    active: &ResetHouseCatalogue,
    target: &ResetHouseCatalogue,
    house_key: &str,
) -> Result<(), WorldResetError> {
    match (active.get(house_key), target.get(house_key)) {
        (Some((before, _)), Some((after, _))) if before == after => Ok(()),
        _ => Err(WorldResetError::HouseCatalogueChanged {
            house_key: house_key.to_owned(),
        }),
    }
}

/// D1189/D1215/D1218 read-only House preflight in the caller's transaction.
/// D1272 final preflight: the RETIRING job must already be locked in this TX.
/// Source identity is compared with actual active history and locked target facts.
/// The three SHARE relation locks prevent concurrent custody/ownership/Instance inserts,
/// updates and deletes until that TX ends. Success is not a retained authority token.
/// The source implementation must bind both catalogues to this World and call C's
/// genuine native encoder; no such production implementation is supplied here.
pub(crate) async fn preflight_world_reset_houses(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
    source: &(dyn super::world_reset_port::WorldResetHouseSource + Sync),
) -> Result<(), WorldResetError> {
    use super::world_reset_port::CatalogueSide;
    lock_admission_relations(tx).await?;
    let target_record = read_locked_world_reset_target(tx, world).await?;
    let active_record = read_active_world_revision(tx, world).await?;
    sqlx::query("LOCK TABLE game_item_house_interior_locations,game_house_ownership,game_world_house_instances IN SHARE MODE")
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    let active_source = source
        .reset_houses(world, CatalogueSide::Active)
        .map_err(|_| WorldResetError::HouseSourceRefused)?;
    let target_source = source
        .reset_houses(world, CatalogueSide::Target)
        .map_err(|_| WorldResetError::HouseSourceRefused)?;
    let active = bound_reset_house_catalogue(
        active_source,
        world,
        &active_record.map_revision,
        active_record.digests.server,
    )?;
    let target = bound_reset_house_catalogue(
        target_source,
        world,
        &target_record.map_revision,
        target_record.digests.server,
    )?;
    {
        let query = sqlx::query("SELECT DISTINCT house_key,spatial_position FROM game_item_house_interior_locations WHERE world_id=encode($1,'hex')::uuid ORDER BY house_key,spatial_position")
            .bind(world.as_bytes().as_slice());
        let mut rows = query.fetch(&mut **tx);
        while let Some(row) = std::future::poll_fn(|cx| rows.as_mut().poll_next(cx)).await {
            let row = row.map_err(DurabilityError::from)?;
            let house_key: String = row.try_get(0).map_err(DurabilityError::from)?;
            let position: Vec<u8> = row.try_get(1).map_err(DurabilityError::from)?;
            check_reset_house_membership(&target, &house_key, &position)?;
        }
    }
    {
        let query = sqlx::query_scalar::<_, String>("SELECT house_key FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid UNION SELECT house_key FROM game_world_house_instances WHERE world_id=encode($1,'hex')::uuid ORDER BY house_key")
            .bind(world.as_bytes().as_slice());
        let mut rows = query.fetch(&mut **tx);
        while let Some(row) = std::future::poll_fn(|cx| rows.as_mut().poll_next(cx)).await {
            let house_key = row.map_err(DurabilityError::from)?;
            check_reset_house_definition(&active, &target, &house_key)?;
        }
    }
    Ok(())
}

impl DurabilityRoot {
    /// Persist one frozen candidate before work, or resume this generation's exact bytes.
    /// Later ownership generations reserve afresh, like native corpse decay; old generation
    /// candidates cannot mutate. No caller supplies an epoch or a generation.
    pub async fn freeze_next_world_reset_item(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
    ) -> Result<Option<WorldResetCandidate>, WorldResetError> {
        match self
            .world_reset_phase(permit, authority, node, world, channel, Phase::Freeze)
            .await?
        {
            PhaseResult::Frozen(candidate) => Ok(candidate),
            _ => Err(WorldResetError::InvalidStoredState),
        }
    }

    /// Charge a durable RL-08 unit before atomic retirement. A committed replay does not
    /// charge again. An unknown charge/COMMIT retains this candidate: never select new IDs.
    pub async fn commit_world_reset_item(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &WorldResetCandidate,
    ) -> Result<WorldResetRetirement, WorldResetError> {
        let charge = self
            .world_reset_phase(
                permit,
                authority,
                node,
                candidate.world,
                candidate.channel,
                Phase::Charge(candidate.clone()),
            )
            .await?;
        if let PhaseResult::Committed(receipt) = charge {
            return Ok(receipt);
        }
        match self
            .world_reset_phase(
                permit,
                authority,
                node,
                candidate.world,
                candidate.channel,
                Phase::Commit(candidate.clone()),
            )
            .await?
        {
            PhaseResult::Committed(receipt) => Ok(receipt),
            _ => Err(WorldResetError::InvalidStoredState),
        }
    }

    async fn world_reset_phase(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        phase: Phase,
    ) -> Result<PhaseResult, WorldResetError> {
        permit.check_channel(world, channel)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| WorldResetError::NotCurrent)?;
        let root = self.clone();
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Reconcile a known candidate before any new work, including after owner turnover.
            if let Phase::Charge(candidate)|Phase::Commit(candidate)=&phase {
                let receipt=sqlx::query("SELECT transaction_id::text,event_id::text,reset_epoch::text,cause_kind FROM game_item_decay_retire_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(candidate.retirement.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
                if let Some(row)=receipt {
                    if row.try_get::<String,_>(3)?!="WORLD_RESET" || uuid(row.try_get(0)?)?!=candidate.retirement.transaction_id || uuid(row.try_get(1)?)?!=candidate.retirement.event_id || row.try_get::<String,_>(2)?!=candidate.retirement.reset_epoch.to_string() { return Ok(Err(WorldResetError::ConflictingCause)); }
                    tx.commit(deadline).await?;return Ok(Ok(PhaseResult::Committed(candidate.retirement.clone())));
                }
            }

            let job = sqlx::query("SELECT reset_epoch::text,revision::text,active_map_revision FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING' FOR UPDATE")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let Some(job) = job else { return Ok(Err(WorldResetError::NotCurrent)); };
            let epoch: String = job.try_get(0)?;
            let revision: String = job.try_get(1)?;
            // Stable native relations are retained through COMMIT; no channel can join the
            // World or attach a player between the cohort check and retirement.
            sqlx::query("LOCK TABLE game_runtime_scope_assignments IN SHARE MODE").execute(&mut **tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? { return Ok(Err(WorldResetError::NotCurrent)); }
            let fact = node.fact();
            let current: bool = sqlx::query_scalar("SELECT game_world_reset_cohort_current(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(&epoch).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
            if !current { return Ok(Err(WorldResetError::NotCurrent)); }
            let guard = super::admission_authority_guards::AdmissionGuardStore::from_root(root)
                .load_locked(&mut tx, &crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(crate::foundation::RuntimeScopeRefV1::channel(world,channel))).await?;
            let Some(guard) = guard else { return Ok(Err(WorldResetError::NotCurrent)); };
            let crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1::Runtime{ownership_generation, ready:false, map_revision,..}=guard.state else {return Ok(Err(WorldResetError::NotCurrent));};
            if map_revision != job.try_get::<String,_>(2)? { return Ok(Err(WorldResetError::NotCurrent)); }
            let generation = ownership_generation.to_string();
            let local:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_kind=1 AND world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 AND ownership_generation=$3::text::numeric AND holder_node_id=encode($4,'hex')::uuid AND holder_registration_revision=$5::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&generation).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
            if !local {return Ok(Err(WorldResetError::NotCurrent));}

            // Select a root, then its first actual live corpse entry. Character backpack
            // entries cannot have Ground parents under the current native FK.
            let ground = sqlx::query("SELECT g.item_instance_id::text,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,g.native_room_placement_context,g.runtime_scope_ownership_generation::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND i.lifecycle=1 ORDER BY g.item_instance_id LIMIT 1")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let Some(ground) = ground else { if !matches!(phase,Phase::Freeze) {return Ok(Err(WorldResetError::NotCurrent));} tx.commit(deadline).await?; return Ok(Ok(PhaseResult::Frozen(None))); };
            let parent = uuid(ground.try_get(0)?)?;
            let entry = sqlx::query("SELECT e.item_instance_id::text,e.placement_ordinal::text FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 ORDER BY e.placement_ordinal,e.item_instance_id LIMIT 1")
                .bind(parent.as_slice()).fetch_optional(&mut **tx).await?;
            let item = match &entry { Some(row)=>uuid(row.try_get(0)?)?,None=>parent };
            let ordinal: Option<String> = entry.as_ref().map(|row|row.try_get(1)).transpose()?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-decay:' || encode($1,'hex'),0))").bind(item.as_slice()).execute(&mut **tx).await?;
            let before = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref,quantity,COALESCE((SELECT key_number FROM game_item_keys k WHERE k.item_instance_id=game_item_instances.item_instance_id),0) AS key_number FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
                .bind(item.as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
            let quantity: i64 = before.try_get(3)?;
            let key_number=u32::try_from(before.try_get::<i32,_>("key_number")?).map_err(|_|DurabilityError::InvalidStoredState)?;
            let definition=OneItemTypedDefinitionRevisionV1{family:before.try_get(0)?,production_key:before.try_get(1)?,revision_ref:before.try_get(2)?};
            let state = |quantity, lifecycle| OneItemStateV1{item_instance_id:item.to_vec(),world_id:world.as_bytes().to_vec(),definition:Some(definition.clone()),quantity,lifecycle,key_number};
            let location = OneItemGroundV1{world_id:world.as_bytes().to_vec(),channel_id:channel.as_bytes().to_vec(),spatial_position:ground.try_get(1)?,corpse_ref:ground.try_get(2)?,map_revision:ground.try_get(3)?,content_revision:ground.try_get(4)?,native_room_placement_context:ground.try_get(5)?,runtime_scope_ownership_generation:ground.try_get::<String,_>(6)?.parse().map_err(|_|DurabilityError::InvalidStoredState)?};
            let reserved=sqlx::query("SELECT cause_kind,reset_epoch::text,reset_parent_item::text,transaction_id::text,event_id::text,occurred_at,envelope,work_units_used,fence_holder_node_id::text,fence_holder_registration_revision::text FROM game_item_decay_retire_reservations WHERE item_instance_id=encode($1,'hex')::uuid AND fence_scope_ownership_generation=$2::text::numeric FOR UPDATE")
                .bind(item.as_slice()).bind(&generation).fetch_optional(&mut **tx).await?;
            let (transaction,event,at)=if let Some(row)=&reserved {
                if row.try_get::<String,_>(0)?!="WORLD_RESET" || row.try_get::<String,_>(1)?!=epoch || uuid(row.try_get(2)?)?!=parent || uuid(row.try_get(8)?)?!=*fact.node_id().as_bytes() || row.try_get::<String,_>(9)?!=fact.registration_revision().to_string() { return Ok(Err(WorldResetError::ConflictingCause)); }
                (uuid(row.try_get(3)?)?,uuid(row.try_get(4)?)?,row.try_get::<i64,_>(5)?)
            } else {
                if !matches!(phase,Phase::Freeze) {return Ok(Err(WorldResetError::NotCurrent));}
                let ids=sqlx::query("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
                (uuid(ids.try_get(0)?)?,uuid(ids.try_get(1)?)?,ids.try_get::<i64,_>(2)?)
            };
            let epoch_value = epoch.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
            let ordinal_value = ordinal.as_ref().map(|n|n.parse::<u64>()).transpose().map_err(|_|DurabilityError::InvalidStoredState)?;
            let message = OneItemDecayRetireV1{before:Some(state(u32::try_from(quantity).map_err(|_|DurabilityError::InvalidStoredState)?,1)),after:Some(state(0,2)),ground:ordinal.is_none().then_some(location.clone()),corpse_entry:None,cause:None,runtime_scope_ownership_generation:ownership_generation,world_reset:Some(OneItemWorldResetV1{world_id:world.as_bytes().to_vec(),reset_epoch:epoch_value,item_instance_id:item.to_vec()}),ground_container_entry:ordinal_value.map(|placement_ordinal|OneItemGroundContainerEntryV1{container_item_instance_id:parent.to_vec(),placement_ordinal,container_ground:Some(location)})};
            let tuple = if let Some(row)=&reserved { tx.frozen_tuple(&row.try_get::<Vec<u8>,_>(6)?).map_err(|_|DurabilityError::InvalidStoredState)? } else {tx.tuple()};
            let envelope = audit::encode_decay_retire_event(audit::DecayRetireEventIdentity{event_id:event,transaction_id:transaction,occurred_at_unix_ms:at,server_build_id:SERVER_BUILD_ID},message,tuple.tuple()).map_err(|_|DurabilityError::InvalidStoredState)?;
            if reserved.is_none() {
            sqlx::query("INSERT INTO game_item_decay_retire_reservations(item_instance_id,fence_scope_ownership_generation,corpse_item_instance_id,world_id,channel_id,transaction_id,event_id,quantity_before,placement_ordinal,deadline,occurred_at,envelope,fence_holder_node_id,fence_holder_registration_revision,work_units_used,reserved_at,cause_kind,reset_epoch,reset_parent_item,type2_schema_revision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,NULL,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8::text::numeric,NULL,$9,$10,encode($11,'hex')::uuid,$12::text::numeric,0,$9,'WORLD_RESET',$13::text::numeric,encode($14,'hex')::uuid,$15)")
                .bind(item.as_slice()).bind(&generation).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(transaction.as_slice()).bind(event.as_slice()).bind(quantity).bind(&ordinal).bind(at).bind(&envelope).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(&epoch).bind(parent.as_slice()).bind(i16::try_from(tuple.schema_revision()).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?;
            } else if reserved.as_ref().ok_or(DurabilityError::InvalidStoredState)?.try_get::<Vec<u8>,_>(6)?!=envelope {
                return Ok(Err(WorldResetError::NotCurrent));
            }
            let candidate=WorldResetCandidate{world,channel,generation:ownership_generation,retirement:WorldResetRetirement{item_instance_id:item,transaction_id:transaction,event_id:event,reset_epoch:epoch_value}};
            if let Phase::Freeze=phase { tx.commit(deadline).await?;return Ok(Ok(PhaseResult::Frozen(Some(candidate)))); }
            let expected=match &phase {Phase::Charge(value)|Phase::Commit(value)=>value,Phase::Freeze=>return Err(DurabilityError::InvalidStoredState)};
            if expected.generation!=candidate.generation || expected.retirement!=candidate.retirement {return Ok(Err(WorldResetError::NotCurrent));}
            if let Phase::Charge(_)=phase {
                let changed=sqlx::query("UPDATE game_item_decay_retire_reservations SET work_units_used=work_units_used+1 WHERE item_instance_id=encode($1,'hex')::uuid AND fence_scope_ownership_generation=$2::text::numeric AND transaction_id=encode($3,'hex')::uuid AND work_units_used<$4")
                    .bind(item.as_slice()).bind(&generation).bind(transaction.as_slice()).bind(i16::from(RL08_RETRY_WORK_UNITS_MAX)).execute(&mut **tx).await?;
                if changed.rows_affected()!=1{return Ok(Err(WorldResetError::CapacityExceeded));}
                tx.commit(deadline).await?;return Ok(Ok(PhaseResult::Charged));
            }
            if reserved.as_ref().ok_or(DurabilityError::InvalidStoredState)?.try_get::<i16,_>(7)?<1 {return Err(DurabilityError::InvalidStoredState);}
            permit.check_channel(world,channel)?;
            let deleted=if ordinal.is_some(){sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid").bind(item.as_slice()).bind(parent.as_slice()).execute(&mut **tx).await?}else{sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut **tx).await?};
            if deleted.rows_affected()!=1{return Err(DurabilityError::InvalidStoredState);}
            let changed_item=sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=$3").bind(item.as_slice()).bind(transaction.as_slice()).bind(quantity).execute(&mut **tx).await?;
            if changed_item.rows_affected()!=1{return Err(DurabilityError::InvalidStoredState);}
            let hash:[u8;32]=Sha256::digest(&envelope).into();
            sqlx::query("INSERT INTO game_item_decay_retire_receipts(item_instance_id,corpse_item_instance_id,world_id,channel_id,fence_scope_ownership_generation,transaction_id,event_id,quantity_before,placement_ordinal,deadline,occurred_at,envelope_sha256,committed_at,cause_kind,reset_epoch,reset_parent_item) VALUES(encode($1,'hex')::uuid,NULL,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8::text::numeric,NULL,$9,$10,$9,'WORLD_RESET',$11::text::numeric,encode($12,'hex')::uuid)")
                .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&generation).bind(transaction.as_slice()).bind(event.as_slice()).bind(quantity).bind(&ordinal).bind(at).bind(hash.as_slice()).bind(&epoch).bind(parent.as_slice()).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$7,$8,$9,1)")
                .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(item.as_slice()).bind(at).bind(at.checked_add(super::item_mint_audit::AUDIT_RETENTION_P90D_MS).ok_or(DurabilityError::InvalidStoredState)?).bind(envelope).bind(hash.as_slice()).execute(&mut **tx).await?;
            let changed=sqlx::query("UPDATE game_world_reset_jobs SET revision=revision+1 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND revision=$3::text::numeric AND phase='RETIRING'").bind(world.as_bytes().as_slice()).bind(&epoch).bind(revision).execute(&mut **tx).await?;
            if changed.rows_affected()!=1{return Err(DurabilityError::InvalidStoredState);}
            tx.commit(deadline).await?;
            Ok(Ok(PhaseResult::Committed(candidate.retirement)))
        })).await?
    }

    /// Historical outcome only: never authorizes a fresh retirement or physical install.
    pub async fn read_world_reset_retirement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item: [u8; 16],
    ) -> Result<Option<WorldResetRetirement>, WorldResetError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| WorldResetError::NotCurrent)?;
        Ok(self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let row=sqlx::query("SELECT transaction_id::text,event_id::text,reset_epoch::text FROM game_item_decay_retire_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND cause_kind='WORLD_RESET'").bind(item.as_slice()).fetch_optional(&mut *tx).await?;
            let value=row.map(|row|Ok::<_,DurabilityError>(WorldResetRetirement{item_instance_id:item,transaction_id:uuid(row.try_get(0)?)?,event_id:uuid(row.try_get(1)?)?,reset_epoch:row.try_get::<String,_>(2)?.parse().map_err(|_|DurabilityError::InvalidStoredState)?})).transpose()?;
            super::db::commit_semantic_transaction(tx,deadline).await?;Ok(value)
        })).await?)
    }
}

/// UNINSTALLED candidate DDL. Kept here until CP leases a migration and integrates the
/// control-owned reset publisher. It extends exact native guard bodies, never disables a
/// trigger or replaces their existing positive/refusal clauses. Fixtures install this same SQL.
pub fn candidate_schema_sql() -> Result<String, &'static str> {
    let mut sql = RESET_EPOCH_SCHEMA.to_owned();
    sql.push_str(RESET_SCHEMA);
    sql.push_str(&candidate_reset_issuance_sql()?);
    sql.push_str(RESET_ACTIVATION_SCHEMA);
    for (source, name, record) in [
        (
            include_str!("../../migrations/0015_corpse_decay_retire.sql"),
            "game_item_decay_retire_consistency_guard",
            "NEW",
        ),
        (
            include_str!("../../migrations/0083_item_use.sql"),
            "game_item_instance_change_proven",
            "NEW",
        ),
        (
            include_str!("../../migrations/0050_spell_item_temporal_chain.sql"),
            "game_item_ground_removal_proven",
            "OLD",
        ),
        (
            include_str!("../../migrations/0042_spell_inventory_consumption.sql"),
            "game_item_corpse_entry_removal_proven",
            "OLD",
        ),
    ] {
        let marker = format!("FUNCTION {name}() RETURNS trigger");
        let start = source.find(&marker).ok_or("native guard missing")?;
        let body = &source[start..];
        let end = body
            .find("\n$$;")
            .ok_or("native guard terminator missing")?
            + 4;
        let body = &body[..end];
        if body.matches("\nBEGIN\n").count() != 1 {
            return Err("native guard body changed");
        }
        let extension = format!(
            "\nBEGIN\n    IF game_world_reset_item_proven({record}.item_instance_id) THEN RETURN NULL; END IF;\n"
        );
        sql.push_str("\nCREATE OR REPLACE ");
        sql.push_str(&body.replacen("\nBEGIN\n", &extension, 1));
        sql.push_str(&format!("\nDO $$ BEGIN EXECUTE format('ALTER FUNCTION {name}() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;\n"));
    }
    Ok(sql)
}

/// D1189a/0096 uninstalled candidate: one native issuance body, two separately
/// authorized entry points. The reset entry point is not installed here yet.
/// Preserve the native replay/CAS/INSERT bytes and the public op4 check exactly.
fn candidate_reset_issuance_sql() -> Result<String, &'static str> {
    const SOURCE: &str = include_str!("../../migrations/0008_native_entry_content_activation.sql");
    const START: &str = "CREATE FUNCTION game_content_record_activation(";
    const END: &str = "\nEND; $$;";
    const WRITE_START: &str = "    PERFORM pg_advisory_xact_lock(";
    if SOURCE.matches(START).count() != 1 {
        return Err("native issuance definition changed");
    }
    let start = SOURCE.find(START).ok_or("native issuance missing")?;
    let native = &SOURCE[start..];
    let end = native
        .find(END)
        .ok_or("native issuance terminator missing")?
        + END.len();
    let (header, body) = native[..end]
        .split_once("BEGIN\n")
        .ok_or("native issuance body missing")?;
    let (grant, write_body) = body
        .split_once(WRITE_START)
        .ok_or("native issuance write boundary missing")?;
    if !grant.starts_with("    IF NOT EXISTS (\n")
        || !grant.ends_with("    END IF;\n")
        || !grant.contains("g.control_role = session_user")
        || !grant.contains("g.operation = 4")
    {
        return Err("native issuance authorization changed");
    }
    let core = format!(
        "{}BEGIN\n{WRITE_START}{write_body}",
        header.replacen(
            START,
            "CREATE FUNCTION game_content_record_activation_core(",
            1
        )
    );
    let (signature, _) = header
        .split_once("DECLARE\n")
        .ok_or("native issuance declaration changed")?;
    let wrapper = format!(
        "{}BEGIN\n{grant}    PERFORM game_content_record_activation_core(\
         p_world_id,p_channel_id,p_sequence,p_previous,p_server,p_client,p_frame);\nEND; $$;",
        signature.replacen("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1)
    );
    Ok(format!(
        "\n{core}\n{wrapper}\n\
         REVOKE ALL ON FUNCTION game_content_record_activation_core(UUID,UUID,NUMERIC,NUMERIC,BYTEA,BYTEA,BYTEA) \
         FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;\n\
         DO $$ BEGIN\n\
         EXECUTE format('ALTER FUNCTION game_content_record_activation_core(uuid,uuid,numeric,numeric,bytea,bytea,bytea) \
         SET search_path = pg_catalog, %I, pg_temp',current_schema());\n\
         EXECUTE format('ALTER FUNCTION game_content_record_activation(uuid,uuid,numeric,numeric,bytea,bytea,bytea) \
         SET search_path = pg_catalog, %I, pg_temp',current_schema());\n\
         END $$;\n"
    ))
}

const RESET_SCHEMA: &str = r#"
-- These are reset records/cohorts, NOT a second retirement ledger. Only control publishes them.
CREATE TABLE game_world_reset_jobs (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551614),
 target_digest BYTEA NOT NULL CHECK(octet_length(target_digest)=32),
 active_map_revision TEXT NOT NULL CHECK(octet_length(active_map_revision) BETWEEN 1 AND 512),
 -- Separate target facts, never aliases of target_digest or active_map_revision.
 -- All NULL is a legacy retirement-only job; final activation must refuse it.
 target_map_revision TEXT CHECK(octet_length(target_map_revision) BETWEEN 1 AND 512),
 target_server_digest BYTEA CHECK(octet_length(target_server_digest)=32),
 target_client_digest BYTEA CHECK(octet_length(target_client_digest)=32),
 target_frame_digest BYTEA CHECK(octet_length(target_frame_digest)=32),
 target_calendar_policy TEXT CHECK(target_calendar_policy='TIBIA_SERVER_SAVE_BERLIN_10'),
 target_calendar_revision NUMERIC(20,0) CHECK(target_calendar_revision BETWEEN 1 AND 18446744073709551615),
 target_calendar_parity TEXT CHECK(target_calendar_parity='PARITY_PENDING'),
 CHECK ((target_map_revision IS NULL AND target_server_digest IS NULL AND target_client_digest IS NULL AND target_frame_digest IS NULL AND target_calendar_policy IS NULL AND target_calendar_revision IS NULL AND target_calendar_parity IS NULL)
 OR (target_map_revision IS NOT NULL AND target_server_digest IS NOT NULL AND target_client_digest IS NOT NULL AND target_frame_digest IS NOT NULL AND target_calendar_policy IS NOT NULL AND target_calendar_revision IS NOT NULL AND target_calendar_parity IS NOT NULL)),
 phase TEXT NOT NULL CHECK(phase IN('RETIRING','ACTIVATED')),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE FUNCTION game_world_reset_job_initialized() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.phase IS DISTINCT FROM 'RETIRING' THEN
  RAISE EXCEPTION 'World reset jobs must start RETIRING' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch)
 WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch AND h.map_revision=NEW.active_map_revision FOR SHARE OF a;
 IF NOT FOUND THEN
 RAISE EXCEPTION 'World reset epoch not initialized/current' USING ERRCODE='OTR02'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_world_reset_job_initialized BEFORE INSERT ON game_world_reset_jobs FOR EACH ROW EXECUTE FUNCTION game_world_reset_job_initialized();
CREATE UNIQUE INDEX game_world_reset_one_retiring ON game_world_reset_jobs(world_id) WHERE phase='RETIRING';
CREATE TABLE game_world_reset_scopes (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL, channel_id UUID NOT NULL,
 predecessor_generation NUMERIC(20,0) NOT NULL CHECK(predecessor_generation BETWEEN 1 AND 18446744073709551614),
 PRIMARY KEY(world_id,reset_epoch,channel_id),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch)
);
CREATE FUNCTION game_world_reset_job_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.phase='RETIRING' AND NEW.revision=OLD.revision+1
 AND (to_jsonb(NEW)-'revision')=(to_jsonb(OLD)-'revision') THEN RETURN NEW; END IF;
 IF TG_OP='UPDATE' AND OLD.phase='RETIRING' AND NEW.phase='ACTIVATED' AND NEW.revision=OLD.revision+1
 AND (to_jsonb(NEW)-'revision'-'phase')=(to_jsonb(OLD)-'revision'-'phase')
 AND EXISTS(SELECT 1 FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch)
 WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch+1
 AND (h.map_revision,h.server_digest,h.client_digest,h.frame_digest,h.calendar_policy,h.calendar_revision,h.calendar_parity)
 IS NOT DISTINCT FROM (NEW.target_map_revision,NEW.target_server_digest,NEW.target_client_digest,NEW.target_frame_digest,NEW.target_calendar_policy,NEW.target_calendar_revision,NEW.target_calendar_parity))
 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'reset job binding is immutable; activation belongs to its inseparable owner' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_world_reset_job_change BEFORE UPDATE OR DELETE ON game_world_reset_jobs FOR EACH ROW EXECUTE FUNCTION game_world_reset_job_change();
CREATE TRIGGER game_world_reset_scope_immutable BEFORE UPDATE OR DELETE ON game_world_reset_scopes FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_jobs_no_truncate BEFORE TRUNCATE ON game_world_reset_jobs FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_reset_scopes_no_truncate BEFORE TRUNCATE ON game_world_reset_scopes FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_world_reset_jobs,game_world_reset_scopes FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_reset_jobs,game_world_reset_scopes TO oteryn_game_control;
GRANT SELECT ON game_world_reset_jobs,game_world_reset_scopes TO oteryn_game_runtime;
GRANT UPDATE(revision) ON game_world_reset_jobs TO oteryn_game_runtime;

ALTER TABLE game_item_decay_retire_reservations ADD COLUMN cause_kind TEXT NOT NULL DEFAULT 'CORPSE_DECAY' CHECK(cause_kind IN('CORPSE_DECAY','WORLD_RESET'));
ALTER TABLE game_item_decay_retire_reservations ADD COLUMN reset_epoch NUMERIC(20,0), ADD COLUMN reset_parent_item UUID;
ALTER TABLE game_item_decay_retire_reservations ALTER COLUMN corpse_item_instance_id DROP NOT NULL, ALTER COLUMN deadline DROP NOT NULL;
ALTER TABLE game_item_decay_retire_receipts ADD COLUMN cause_kind TEXT NOT NULL DEFAULT 'CORPSE_DECAY' CHECK(cause_kind IN('CORPSE_DECAY','WORLD_RESET'));
ALTER TABLE game_item_decay_retire_receipts ADD COLUMN reset_epoch NUMERIC(20,0), ADD COLUMN reset_parent_item UUID;
ALTER TABLE game_item_decay_retire_receipts ALTER COLUMN corpse_item_instance_id DROP NOT NULL, ALTER COLUMN deadline DROP NOT NULL;
ALTER TABLE game_item_decay_retire_reservations ADD CHECK (
 (cause_kind='CORPSE_DECAY' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND reset_epoch IS NULL AND reset_parent_item IS NULL) OR
 (cause_kind='WORLD_RESET' AND corpse_item_instance_id IS NULL AND deadline IS NULL AND reset_epoch IS NOT NULL AND reset_parent_item IS NOT NULL AND ((item_instance_id=reset_parent_item)=(placement_ordinal IS NULL)))
);
ALTER TABLE game_item_decay_retire_receipts ADD CHECK (
 (cause_kind='CORPSE_DECAY' AND corpse_item_instance_id IS NOT NULL AND deadline IS NOT NULL AND reset_epoch IS NULL AND reset_parent_item IS NULL) OR
 (cause_kind='WORLD_RESET' AND corpse_item_instance_id IS NULL AND deadline IS NULL AND reset_epoch IS NOT NULL AND reset_parent_item IS NOT NULL AND ((item_instance_id=reset_parent_item)=(placement_ordinal IS NULL)))
);
ALTER TABLE game_item_decay_retire_reservations ADD FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch);
ALTER TABLE game_item_decay_retire_receipts ADD FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch);

CREATE FUNCTION game_world_reset_cohort_current(w UUID,e NUMERIC,n UUID,r NUMERIC) RETURNS BOOLEAN
LANGUAGE SQL SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=w AND j.reset_epoch=e AND j.phase='RETIRING')
 AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=w AND a.scope_kind=1)
 AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
 LEFT JOIN game_world_reset_scopes c ON (c.world_id,c.reset_epoch,c.channel_id)=(a.world_id,e,a.channel_id)
 LEFT JOIN game_durability_admission_runtime_guards g USING(scope_key)
 WHERE a.world_id=w AND a.scope_kind=1 AND (c.channel_id IS NULL OR a.state<>1 OR a.ownership_generation<=c.predecessor_generation OR a.holder_node_id<>n OR a.holder_registration_revision<>r OR g.scope_key IS NULL OR g.ready OR g.ownership_generation<>a.ownership_generation))
 AND NOT EXISTS(SELECT 1 FROM game_world_reset_scopes c WHERE c.world_id=w AND c.reset_epoch=e AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=w AND a.scope_kind=1 AND a.channel_id=c.channel_id))
 AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.world_id=w AND s.session_state IN(1,2));
$$;
-- This proof is shared by native deferred guards. Every positive reset clause still requires
-- its complete reservation, quantity evidence, actual pre-DELETE location and audit in THIS xact.
CREATE FUNCTION game_world_reset_item_proven(p UUID) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_item_decay_retire_receipts%ROWTYPE; v game_item_decay_retire_reservations%ROWTYPE;
BEGIN
 SELECT * INTO d FROM game_item_decay_retire_receipts WHERE item_instance_id=p AND cause_kind='WORLD_RESET' AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO v FROM game_item_decay_retire_reservations WHERE item_instance_id=p AND fence_scope_ownership_generation=d.fence_scope_ownership_generation;
 IF NOT FOUND OR v.cause_kind<>'WORLD_RESET' OR v.work_units_used<1
 OR (v.world_id,v.channel_id,v.transaction_id,v.event_id,v.quantity_before,v.occurred_at,v.reset_epoch,v.reset_parent_item)
 IS DISTINCT FROM (d.world_id,d.channel_id,d.transaction_id,d.event_id,d.quantity_before,d.occurred_at,d.reset_epoch,d.reset_parent_item)
 OR v.placement_ordinal IS DISTINCT FROM d.placement_ordinal OR sha256(v.envelope)<>d.envelope_sha256
 OR NOT game_world_reset_cohort_current(d.world_id,d.reset_epoch,v.fence_holder_node_id,v.fence_holder_registration_revision)
 OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=d.world_id AND a.channel_id=d.channel_id AND a.state=1 AND a.ownership_generation=d.fence_scope_ownership_generation AND a.holder_node_id=v.fence_holder_node_id AND a.holder_registration_revision=v.fence_holder_registration_revision)
 OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p AND i.world_id=d.world_id AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=d.transaction_id)
 OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=p AND q.transaction_id=d.transaction_id AND q.quantity_before=d.quantity_before)
 OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=d.event_id AND a.transaction_id=d.transaction_id AND a.item_instance_id=p AND a.envelope_sha256=d.envelope_sha256 AND a.envelope=v.envelope AND a.occurred_at=d.occurred_at AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
 OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p)
 OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p)
 THEN RAISE EXCEPTION 'WorldReset requires exact current reservation/receipt/retirement/audit' USING ERRCODE='23514'; END IF;
 IF d.placement_ordinal IS NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_removal_evidence g WHERE g.item_instance_id=p AND g.world_id=d.world_id AND g.channel_id=d.channel_id)
   OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=p)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=p)
  THEN RAISE EXCEPTION 'WorldReset root requires actual Ground removal and no contents' USING ERRCODE='23514'; END IF;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_item_corpse_entry_removal_evidence c JOIN game_item_ground_locations g ON g.item_instance_id=c.parent_item_instance_id JOIN game_item_instances i ON i.item_instance_id=g.item_instance_id
    WHERE c.item_instance_id=p AND c.parent_item_instance_id=d.reset_parent_item AND c.placement_ordinal=d.placement_ordinal AND c.world_id=d.world_id AND c.channel_id=d.channel_id AND c.created_xact_id=pg_current_xact_id() AND g.world_id=d.world_id AND g.channel_id=d.channel_id AND i.lifecycle=1)
  THEN RAISE EXCEPTION 'WorldReset entry requires actual live Ground parent' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN TRUE;
END $$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_world_reset_cohort_current(uuid,numeric,uuid,numeric)','game_world_reset_item_proven(uuid)','game_world_reset_job_initialized()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
"#;

#[cfg(test)]
mod reset_house_preflight_tests {
    use super::super::world_reset_port::HouseResetFact;
    use super::*;
    fn fact(key: &str, position: u8, digest: u8) -> HouseResetFact {
        HouseResetFact {
            house_key: format!("oteryn:content.house.{key}"),
            spatial_position: vec![position],
            definition_digest: [digest; 32],
        }
    }
    #[test]
    fn catalogue_identity_must_match_independent_world_map_and_artifact()
    -> Result<(), WorldResetError> {
        use super::super::world_reset_port::QualifiedHouseResetCatalogue;
        let make_world = |last| {
            WorldId::decode(&[
                1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, last,
            ])
            .map_err(|_| WorldResetError::InvalidStoredState)
        };
        let world = make_world(1)?;
        let source = || QualifiedHouseResetCatalogue {
            world,
            map_revision: "map-current".to_owned(),
            server_artifact_digest: [7; 32],
            facts: Vec::new(),
        };
        // Data binding only; explicit empty facts do not grant activation authority.
        assert!(bound_reset_house_catalogue(source(), world, "map-current", [7; 32])?.is_empty());
        for changed in 0..4 {
            let mut catalogue = source();
            match changed {
                0 => catalogue.world = make_world(2)?,
                1 => catalogue.map_revision = "map-other".to_owned(),
                2 => catalogue.server_artifact_digest = [8; 32],
                _ => catalogue.map_revision.clear(),
            }
            assert!(matches!(
                bound_reset_house_catalogue(catalogue, world, "map-current", [7; 32]),
                Err(WorldResetError::HouseSourceRefused)
            ));
        }
        Ok(())
    }
    #[test]
    fn only_owned_or_instance_house_definition_changes_refuse() -> Result<(), WorldResetError> {
        let active = reset_house_catalogue(vec![fact("owned", 1, 1), fact("unowned", 2, 2)])?;
        let target = reset_house_catalogue(vec![fact("owned", 1, 1), fact("unowned", 3, 9)])?;
        assert!(
            check_reset_house_definition(&active, &target, "oteryn:content.house.owned").is_ok()
        );
        assert!(
            matches!(check_reset_house_definition(&active, &target, "oteryn:content.house.unowned"), Err(WorldResetError::HouseCatalogueChanged { house_key }) if house_key == "oteryn:content.house.unowned")
        );
        let target = reset_house_catalogue(vec![fact("unowned", 2, 2)])?;
        assert!(matches!(
            check_reset_house_definition(&active, &target, "oteryn:content.house.owned"),
            Err(WorldResetError::HouseCatalogueChanged { .. })
        ));
        Ok(())
    }
    #[test]
    fn live_custody_requires_exact_target_key_and_opaque_position() -> Result<(), WorldResetError> {
        let active = reset_house_catalogue(vec![fact("a", 1, 1)])?;
        let key = "oteryn:content.house.a";
        let position = [1];
        assert!(check_reset_house_membership(&active, key, &position).is_ok());
        for target in [vec![], vec![fact("a", 2, 1)], vec![fact("b", 1, 1)]] {
            let target = reset_house_catalogue(target)?;
            assert!(matches!(
                check_reset_house_membership(&target, key, &position),
                Err(WorldResetError::HouseTargetMembershipChanged { .. })
            ));
        }
        Ok(())
    }
    #[test]
    fn malformed_or_ambiguous_native_source_refuses() {
        for facts in [
            vec![fact("a", 1, 1), fact("a", 1, 1)],
            vec![fact("a", 1, 1), fact("b", 1, 2)],
            vec![fact("a", 1, 1), fact("a", 2, 2)],
            vec![fact("INVALID", 1, 1)],
        ] {
            assert!(matches!(
                reset_house_catalogue(facts),
                Err(WorldResetError::HouseSourceRefused)
            ));
        }
        for len in [0, 129] {
            let mut malformed = fact("a", 1, 1);
            malformed.spatial_position = vec![0; len];
            assert!(reset_house_catalogue(vec![malformed]).is_err());
        }
        assert!(reset_house_catalogue(vec![fact("a", 1, 1), fact("a", 2, 1)]).is_ok());
    }
}

#[cfg(test)]
mod world_drain_binding_data_tests {
    use super::super::runtime_scope_assignment::NodeRegistrationFact;
    use super::super::world_reset_port::{
        DrainedChannelBinding, WorldDrainBinding, world_drain_cohort_digest,
    };
    use super::*;
    use crate::foundation::{NodeId, RuntimeScopeRefV1};

    fn identity(value: u8) -> [u8; 16] {
        let mut bytes = [value; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }
    fn channel(world: WorldId, id: u8) -> Result<DrainedChannelBinding, WorldResetError> {
        Ok(DrainedChannelBinding {
            scope: RuntimeScopeRefV1::channel(
                world,
                ChannelId::decode(&identity(id))
                    .map_err(|_| WorldResetError::InvalidStoredState)?,
            ),
            generation: 9,
        })
    }
    fn compare_channels(
        binding: &WorldDrainBinding,
        observed_data: &[DrainedChannelBinding],
    ) -> Result<(), WorldResetError> {
        for (index, row) in observed_data.iter().enumerate() {
            check_world_drain_channel(binding.channels.get(index), row.scope, row.generation)?;
        }
        if observed_data.len() != binding.channels.len() {
            return Err(WorldResetError::DrainBindingRefused);
        }
        Ok(())
    }
    #[test]
    fn drain_binding_data_rejects_each_independent_mismatch() -> Result<(), WorldResetError> {
        // Data equality/digest only: no witness impl, current-incarnation or DB proof.
        let world =
            WorldId::decode(&identity(1)).map_err(|_| WorldResetError::InvalidStoredState)?;
        let other_world =
            WorldId::decode(&identity(2)).map_err(|_| WorldResetError::InvalidStoredState)?;
        let node_id =
            NodeId::decode(&identity(3)).map_err(|_| WorldResetError::InvalidStoredState)?;
        let node = NodeRegistrationFact::new(node_id, 7);
        let channels = vec![channel(world, 4)?, channel(world, 5)?];
        let digest = world_drain_cohort_digest(world, node, &channels)
            .map_err(|_| WorldResetError::DrainBindingRefused)?;
        let mut binding = WorldDrainBinding {
            world,
            node,
            channels: channels.clone(),
            cohort_digest: digest,
        };
        check_world_drain_data(&binding, world, node)?;
        compare_channels(&binding, &channels)?;
        assert!(compare_channels(&binding, &channels[..1]).is_err());
        assert!(compare_channels(&binding, &[channels[0], channels[1], channels[1]]).is_err());
        let mut changed = channels.clone();
        changed[1].generation += 1;
        assert!(compare_channels(&binding, &changed).is_err());
        assert!(check_world_drain_data(&binding, other_world, node).is_err());
        assert!(
            check_world_drain_data(&binding, world, NodeRegistrationFact::new(node_id, 8)).is_err()
        );
        let other_node =
            NodeId::decode(&identity(6)).map_err(|_| WorldResetError::InvalidStoredState)?;
        assert!(
            check_world_drain_data(&binding, world, NodeRegistrationFact::new(other_node, 7))
                .is_err()
        );
        binding.cohort_digest[0] ^= 1;
        assert!(check_world_drain_data(&binding, world, node).is_err());
        binding.cohort_digest = digest;
        binding.channels.reverse();
        assert!(check_world_drain_data(&binding, world, node).is_err());
        assert!(world_drain_cohort_digest(world, node, &[channels[0], channels[0]]).is_err());
        changed[0] = channel(other_world, 4)?;
        assert!(world_drain_cohort_digest(world, node, &changed).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod reset_issuance_core_source_tests {
    use super::candidate_reset_issuance_sql;

    #[test]
    fn one_private_native_issuance_body_preserves_control_authorization() -> Result<(), &'static str>
    {
        let native = include_str!("../../migrations/0008_native_entry_content_activation.sql");
        let sql = candidate_reset_issuance_sql()?;
        let (core, wrapper) = sql
            .split_once("CREATE OR REPLACE FUNCTION game_content_record_activation(")
            .ok_or("public wrapper missing")?;
        let native_write = &native[native
            .find("    PERFORM pg_advisory_xact_lock(")
            .ok_or("native write missing")?..];
        let native_write = &native_write[..native_write
            .find("\nEND; $$;")
            .ok_or("native write terminator missing")?
            + "\nEND; $$;".len()];
        assert!(core.ends_with(&format!("{native_write}\n")));
        assert_eq!(
            sql.matches("INSERT INTO game_content_activations").count(),
            1
        );
        assert!(!wrapper.contains("INSERT INTO game_content_activations"));
        assert!(wrapper.contains("g.control_role = session_user"));
        assert!(wrapper.contains("g.operation = 4"));
        assert!(wrapper.contains("PERFORM game_content_record_activation_core("));
        assert!(wrapper.contains("FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;"));
        assert_eq!(
            wrapper
                .matches("SET search_path = pg_catalog, %I, pg_temp")
                .count(),
            2
        );
        Ok(())
    }
}

/// D1257 explicit policy identity; every calendar result remains PARITY_PENDING.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetCalendarPolicy {
    TibiaServerSaveBerlin10,
}
impl ResetCalendarPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TibiaServerSaveBerlin10 => "TIBIA_SERVER_SAVE_BERLIN_10",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetCalendar {
    pub policy: ResetCalendarPolicy,
    pub revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldResetArtifactDigests {
    pub server: [u8; 32],
    pub client: [u8; 32],
    pub frame: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldResetEpochRecord {
    pub epoch: u64,
    pub map_revision: String,
    pub digests: WorldResetArtifactDigests,
    pub calendar: ResetCalendar,
    pub month_key: String,
    pub recorded_at_utc_ms: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetMonthObservation {
    pub month_key: String,
    pub calendar: ResetCalendar,
    pub observed_at_utc_ms: i64,
}
fn stored_calendar(
    policy: String,
    revision: String,
    parity: String,
) -> Result<ResetCalendar, WorldResetError> {
    let revision = revision
        .parse::<u64>()
        .map_err(|_| WorldResetError::InvalidStoredState)?;
    if policy != ResetCalendarPolicy::TibiaServerSaveBerlin10.as_str()
        || revision == 0
        || parity != "PARITY_PENDING"
    {
        return Err(WorldResetError::InvalidStoredState);
    }
    Ok(ResetCalendar {
        policy: ResetCalendarPolicy::TibiaServerSaveBerlin10,
        revision,
    })
}
fn decode_epoch(row: sqlx::postgres::PgRow) -> Result<WorldResetEpochRecord, WorldResetError> {
    let bad = || WorldResetError::InvalidStoredState;
    Ok(WorldResetEpochRecord {
        epoch: row
            .try_get::<String, _>(0)
            .map_err(DurabilityError::from)?
            .parse()
            .map_err(|_| bad())?,
        map_revision: row.try_get(1).map_err(DurabilityError::from)?,
        digests: WorldResetArtifactDigests {
            server: row
                .try_get::<Vec<u8>, _>(2)
                .map_err(DurabilityError::from)?
                .try_into()
                .map_err(|_| bad())?,
            client: row
                .try_get::<Vec<u8>, _>(3)
                .map_err(DurabilityError::from)?
                .try_into()
                .map_err(|_| bad())?,
            frame: row
                .try_get::<Vec<u8>, _>(4)
                .map_err(DurabilityError::from)?
                .try_into()
                .map_err(|_| bad())?,
        },
        calendar: stored_calendar(
            row.try_get(5).map_err(DurabilityError::from)?,
            row.try_get(6).map_err(DurabilityError::from)?,
            row.try_get(7).map_err(DurabilityError::from)?,
        )?,
        month_key: row.try_get(8).map_err(DurabilityError::from)?,
        recorded_at_utc_ms: row.try_get(9).map_err(DurabilityError::from)?,
    })
}

/// Actual immutable history; absence never synthesizes epoch0 or a calendar.
pub async fn read_world_reset_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Option<WorldResetEpochRecord>, WorldResetError> {
    let row=sqlx::query("SELECT reset_epoch::text,map_revision,server_digest,client_digest,frame_digest,calendar_policy,calendar_revision::text,calendar_parity,month_key,floor(extract(epoch FROM recorded_at)*1000)::bigint FROM game_world_reset_epoch_history WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    match row {
        Some(row) => Ok(Some(decode_epoch(row)?)),
        None => {
            read_active_world_revision(tx, world).await?;
            Ok(None)
        }
    }
}
/// The Control-initialized active pointer. Missing means NotInitialized (DAILY
/// must expose Unknown), never an implicit zero. This read grants no current
/// authority: reset consumers retain their native admission/job locks separately.
pub async fn read_active_world_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
) -> Result<WorldResetEpochRecord, WorldResetError> {
    let row=sqlx::query("SELECT h.reset_epoch::text,h.map_revision,h.server_digest,h.client_digest,h.frame_digest,h.calendar_policy,h.calendar_revision::text,h.calendar_parity,h.month_key,floor(extract(epoch FROM h.recorded_at)*1000)::bigint FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?.ok_or(WorldResetError::NotInitialized)?;
    decode_epoch(row)
}
/// Read PG's clock exactly once, with the actual stored active calendar in the
/// caller's locked TX. No process clock or request-supplied instant/calendar.
pub async fn read_reset_month_key_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
) -> Result<ResetMonthObservation, WorldResetError> {
    let row=sqlx::query("WITH instant AS MATERIALIZED(SELECT clock_timestamp() AS value) SELECT game_world_reset_month_key(i.value,h.calendar_policy,h.calendar_revision),h.calendar_policy,h.calendar_revision::text,h.calendar_parity,floor(extract(epoch FROM i.value)*1000)::bigint FROM game_world_reset_active_epochs a JOIN game_world_reset_epoch_history h USING(world_id,reset_epoch) CROSS JOIN instant i WHERE a.world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?.ok_or(WorldResetError::NotInitialized)?;
    Ok(ResetMonthObservation {
        month_key: row.try_get(0).map_err(DurabilityError::from)?,
        calendar: stored_calendar(
            row.try_get(1).map_err(DurabilityError::from)?,
            row.try_get(2).map_err(DurabilityError::from)?,
            row.try_get(3).map_err(DurabilityError::from)?,
        )?,
        observed_at_utc_ms: row.try_get(4).map_err(DurabilityError::from)?,
    })
}
impl DurabilityRoot {
    /// Explicit Control operation, matching native descriptor issuance's semantic
    /// transaction and definitive-conflict result. Runtime EXECUTE is not granted.
    pub async fn initialize_world_reset_epoch(
        &self,
        world: WorldId,
        map_revision: &str,
        digests: WorldResetArtifactDigests,
        calendar: ResetCalendar,
    ) -> Result<bool, WorldResetError> {
        if map_revision.is_empty() || map_revision.len() > 512 || calendar.revision == 0 {
            return Err(WorldResetError::InvalidInitialization);
        }
        let map_revision = map_revision.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
            let initialized=sqlx::query("SELECT game_world_reset_initialize_epoch(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(map_revision).bind(digests.server.as_slice()).bind(digests.client.as_slice()).bind(digests.frame.as_slice())
                .bind(calendar.policy.as_str()).bind(calendar.revision.to_string()).execute(&mut *tx).await;
            match initialized {
                Ok(_) => {},
                Err(sqlx::Error::Database(error)) if matches!(error.code().as_deref(),Some("OTR01"|"42501"))=>return Ok(false),
                Err(error)=>return Err(error.into()),
            }
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(true)
        })).await.map_err(WorldResetError::from)
    }
}

const RESET_EPOCH_SCHEMA: &str = r#"
-- D1257/0096 UNINSTALLED candidate. Explicit Control initialization only.
CREATE FUNCTION game_world_reset_month_key(p_instant TIMESTAMPTZ,p_policy TEXT,p_revision NUMERIC)
RETURNS TEXT LANGUAGE SQL IMMUTABLE STRICT AS $$
 SELECT CASE WHEN isfinite(p_instant) AND p_policy='TIBIA_SERVER_SAVE_BERLIN_10'
 AND p_revision BETWEEN 1 AND 18446744073709551615 AND trunc(p_revision)=p_revision
 THEN to_char((p_instant AT TIME ZONE 'Europe/Berlin')-INTERVAL '10 hours','YYYY-MM') ELSE NULL END;
$$;
CREATE TABLE game_world_reset_epoch_history (
 world_id UUID NOT NULL CHECK(game_node_is_uuid_v7(world_id)),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 map_revision TEXT NOT NULL CHECK(octet_length(map_revision) BETWEEN 1 AND 512),
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 calendar_policy TEXT NOT NULL CHECK(calendar_policy='TIBIA_SERVER_SAVE_BERLIN_10'),
 calendar_revision NUMERIC(20,0) NOT NULL CHECK(calendar_revision BETWEEN 1 AND 18446744073709551615),
 calendar_parity TEXT NOT NULL CHECK(calendar_parity='PARITY_PENDING'),
 month_key TEXT NOT NULL CHECK(month_key ~ '^[0-9]{4}-[0-9]{2}$'),
 recorded_at TIMESTAMPTZ NOT NULL CHECK(isfinite(recorded_at)),
 issued_by TEXT NOT NULL,
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE TABLE game_world_reset_active_epochs (
 world_id UUID PRIMARY KEY, reset_epoch NUMERIC(20,0) NOT NULL,
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epoch_history(world_id,reset_epoch)
);
CREATE TRIGGER game_world_reset_history_immutable BEFORE UPDATE OR DELETE ON game_world_reset_epoch_history FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_history_no_truncate BEFORE TRUNCATE ON game_world_reset_epoch_history FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_reset_active_no_delete BEFORE DELETE ON game_world_reset_active_epochs FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_world_reset_active_no_truncate BEFORE TRUNCATE ON game_world_reset_active_epochs FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_world_reset_epoch_history,game_world_reset_active_epochs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_world_reset_epoch_history,game_world_reset_active_epochs TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_world_reset_initialize_epoch(p_world UUID,p_map TEXT,p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_policy TEXT,p_revision NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE v_existing game_world_reset_epoch_history%ROWTYPE; v_instant TIMESTAMPTZ; v_month TEXT;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:'||p_world::text,0));
 SELECT * INTO v_existing FROM game_world_reset_epoch_history WHERE world_id=p_world AND reset_epoch=0;
 IF FOUND THEN
  IF (v_existing.map_revision,v_existing.server_digest,v_existing.client_digest,v_existing.frame_digest,v_existing.calendar_policy,v_existing.calendar_revision)
   IS NOT DISTINCT FROM (p_map,p_server,p_client,p_frame,p_policy,p_revision)
   AND EXISTS(SELECT 1 FROM game_world_reset_active_epochs WHERE world_id=p_world) THEN RETURN; END IF;
  RAISE EXCEPTION 'World reset initialization conflicts' USING ERRCODE='OTR01';
 END IF;
 IF EXISTS(SELECT 1 FROM game_world_reset_epoch_history WHERE world_id=p_world) THEN
  RAISE EXCEPTION 'World reset initialization is not fresh' USING ERRCODE='OTR01'; END IF;
 v_instant:=clock_timestamp();
 v_month:=game_world_reset_month_key(v_instant,p_policy,p_revision);
 IF p_world IS NULL OR NOT game_node_is_uuid_v7(p_world) OR p_server IS NULL OR p_client IS NULL OR p_frame IS NULL OR p_map IS NULL OR octet_length(p_map) NOT BETWEEN 1 AND 512 OR octet_length(p_server)<>32 OR octet_length(p_client)<>32 OR octet_length(p_frame)<>32 OR v_month IS NULL THEN
  RAISE EXCEPTION 'Invalid World reset initialization' USING ERRCODE='OTR01'; END IF;
 INSERT INTO game_world_reset_epoch_history(world_id,reset_epoch,map_revision,server_digest,client_digest,frame_digest,calendar_policy,calendar_revision,calendar_parity,month_key,recorded_at,issued_by)
 VALUES(p_world,0,p_map,p_server,p_client,p_frame,p_policy,p_revision,'PARITY_PENDING',v_month,v_instant,session_user);
 INSERT INTO game_world_reset_active_epochs(world_id,reset_epoch) VALUES(p_world,0);
END $$;
REVOKE ALL ON FUNCTION game_world_reset_initialize_epoch(UUID,TEXT,BYTEA,BYTEA,BYTEA,TEXT,NUMERIC) FROM PUBLIC,oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_world_reset_initialize_epoch(UUID,TEXT,BYTEA,BYTEA,BYTEA,TEXT,NUMERIC) TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_world_reset_month_key(TIMESTAMPTZ,TEXT,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_reset_month_key(TIMESTAMPTZ,TEXT,NUMERIC) TO oteryn_game_runtime,oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_initialize_epoch(uuid,text,bytea,bytea,bytea,text,numeric) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_world_reset_month_key(timestamptz,text,numeric) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
"#;

/// Activation target facts read independently from the immutable locked job.
/// `target_digest` and the predecessor's `active_map_revision` are not these fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldResetTarget {
    pub world: WorldId,
    pub reset_epoch: u64,
    pub map_revision: String,
    pub digests: WorldResetArtifactDigests,
    pub calendar: ResetCalendar,
}
pub async fn read_locked_world_reset_target(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
) -> Result<WorldResetTarget, WorldResetError> {
    let row=sqlx::query("SELECT reset_epoch::text,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_calendar_policy,target_calendar_revision::text,target_calendar_parity FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING' FOR UPDATE")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?.ok_or(WorldResetError::NotCurrent)?;
    let required_text = |index| {
        row.try_get::<Option<String>, _>(index)
            .map_err(DurabilityError::from)?
            .ok_or(WorldResetError::UnqualifiedTarget)
    };
    let required_digest = |index| {
        row.try_get::<Option<Vec<u8>>, _>(index)
            .map_err(DurabilityError::from)?
            .ok_or(WorldResetError::UnqualifiedTarget)?
            .try_into()
            .map_err(|_| WorldResetError::InvalidStoredState)
    };
    Ok(WorldResetTarget {
        world,
        reset_epoch: row
            .try_get::<String, _>(0)
            .map_err(DurabilityError::from)?
            .parse()
            .map_err(|_| WorldResetError::InvalidStoredState)?,
        map_revision: required_text(1)?,
        digests: WorldResetArtifactDigests {
            server: required_digest(2)?,
            client: required_digest(3)?,
            frame: required_digest(4)?,
        },
        calendar: stored_calendar(required_text(5)?, required_text(6)?, required_text(7)?)?,
    })
}

/// Frozen occurrence data, not authority. Retain this exact identity across unknown
/// commit outcomes; never rediscover a later RETIRING job by World on retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorldResetActivationIdentity {
    world: WorldId,
    epoch: u64,
    revision: u64,
    retirement_binding: [u8; 32],
}
impl WorldResetCensus {
    pub(crate) fn activation_identity(&self) -> WorldResetActivationIdentity {
        WorldResetActivationIdentity {
            world: self.world,
            epoch: self.reset_epoch,
            revision: self.reset_revision,
            retirement_binding: self.target_digest,
        }
    }
}

/// Caller-TX owner. Success is provisional until the caller commits this same TX.
/// Any error requires rollback, including errors after the first SQL mutation.
/// The source and witness must be genuine C/I implementations; this module creates neither.
/// Historical exact replay is checked first and does not require today's drain/cohort.
/// No publication or in-memory readiness transition may precede the commit receipt.
pub(crate) async fn activate_world_reset_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    identity: &WorldResetActivationIdentity,
    houses: &(dyn super::world_reset_port::WorldResetHouseSource + Sync),
    drain: &(dyn super::world_reset_port::WorldDrainWitness + Sync),
) -> Result<WorldResetEpochRecord, WorldResetError> {
    let next = identity
        .epoch
        .checked_add(1)
        .ok_or(WorldResetError::CapacityExceeded)?;
    let next_revision = identity
        .revision
        .checked_add(1)
        .ok_or(WorldResetError::CapacityExceeded)?;
    lock_admission_relations(tx).await?;
    // Lock the exact occurrence, never whichever job is currently RETIRING.
    let replay: Option<bool> = sqlx::query_scalar(
        "SELECT phase='ACTIVATED' AND revision=$3::text::numeric AND target_digest=$4 \
         FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid \
         AND reset_epoch=$2::text::numeric FOR UPDATE",
    )
    .bind(identity.world.as_bytes().as_slice())
    .bind(identity.epoch.to_string())
    .bind(next_revision.to_string())
    .bind(identity.retirement_binding.as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    if replay == Some(true) {
        return read_world_reset_history(tx, identity.world, next)
            .await?
            .ok_or(WorldResetError::InvalidStoredState);
    }
    let census = census_retiring_world_reset(tx, node, identity.world)
        .await?
        .ok_or(WorldResetError::NotCurrent)?;
    if census.activation_identity() != *identity
        || census.ground_root_count != 0
        || census.ground_corpse_entry_count != 0
    {
        return Err(WorldResetError::NotCurrent);
    }
    // All actual database locks are retained through both source checks and the writer.
    let target = read_locked_world_reset_target(tx, identity.world).await?;
    if target.reset_epoch != identity.epoch {
        return Err(WorldResetError::NotCurrent);
    }
    check_world_drain_in_transaction(tx, node, identity.world, drain).await?;
    preflight_world_reset_houses(tx, identity.world, houses).await?;
    let fact = node.fact();
    sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,$2::text::numeric,$3::text::numeric,$4,encode($5,'hex')::uuid,$6::text::numeric)")
        .bind(identity.world.as_bytes().as_slice()).bind(identity.epoch.to_string())
        .bind(identity.revision.to_string()).bind(identity.retirement_binding.as_slice())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    read_world_reset_history(tx, identity.world, next)
        .await?
        .ok_or(WorldResetError::InvalidStoredState)
}

impl DurabilityRoot {
    /// Bounded owner commit. Borrow the genuine sources/proof through the ordinary
    /// semantic pass; never publish an epoch before its transaction commits.
    /// On CommitOutcomeUnknown the caller retains this exact identity and retries
    /// this operation, whose historical replay precedes current-authority checks.
    pub(crate) async fn commit_world_reset_activation(
        &self,
        node: &NodeIncarnationProof,
        identity: &WorldResetActivationIdentity,
        houses: &(dyn super::world_reset_port::WorldResetHouseSource + Sync),
        drain: &(dyn super::world_reset_port::WorldDrainWitness + Sync),
    ) -> Result<WorldResetEpochRecord, WorldResetError> {
        let mut context = (node, identity, houses, drain);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let record = match activate_world_reset_in_transaction(
                        &mut tx, context.0, context.1, context.2, context.3,
                    )
                    .await
                    {
                        Ok(record) => record,
                        Err(error) => {
                            tx.rollback().await?;
                            return Ok(Err(error));
                        }
                    };
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await
            .map_err(WorldResetError::from)?
    }
}

// Private successor: UNINSTALLED 0096 only, no migration/root mounting.
const RESET_ACTIVATION_SCHEMA: &str = r#"
CREATE FUNCTION game_world_reset_activate(w UUID,e NUMERIC,r NUMERIC,b BYTEA,n UUID,nr NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE j game_world_reset_jobs%ROWTYPE; a NUMERIC; v_channel RECORD;
 v_history game_world_reset_epoch_history%ROWTYPE; v_current game_content_activations%ROWTYPE;
 v_previous NUMERIC; v_sequence NUMERIC; v_instant TIMESTAMPTZ; v_month TEXT;
BEGIN
 -- SQL NULL must never turn a current-authority predicate into UNKNOWN/false IF.
 -- Refuse malformed occurrence/node data before taking any mutation fence.
 IF w IS NULL OR e IS NULL OR r IS NULL OR b IS NULL OR n IS NULL OR nr IS NULL
 OR NOT game_node_is_uuid_v7(w) OR NOT game_node_is_uuid_v7(n) OR octet_length(b)<>32
 OR e<0 OR e>=18446744073709551615 OR trunc(e)<>e
 OR r<1 OR r>=18446744073709551615 OR trunc(r)<>r
 OR nr<1 OR nr>18446744073709551615 OR trunc(nr)<>nr
 THEN RAISE EXCEPTION 'World reset occurrence/node data refused' USING ERRCODE='OTR03'; END IF;
 -- Independently acquire the exact native admission relation fence, in its lexical
 -- order, even on direct runtime SQL invocation. These durable locks are not I/C proof.
 LOCK TABLE game_durability_admission_account_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_character_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_guard_history IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_lifecycle_receipts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_runtime_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_admission_signing_trust_guards IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_control_loss_continuity IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_executor_custody IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_fresh_admission_receipts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_attempts IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_pending_commands IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_reconnect_sessions IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_recovery_grant_consumptions IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_replacements IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_use_ledgers IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_session_use_memberships IN EXCLUSIVE MODE;
 LOCK TABLE game_durability_transport_ref_reservations IN EXCLUSIVE MODE;
 SELECT * INTO j FROM game_world_reset_jobs WHERE world_id=w AND reset_epoch=e FOR UPDATE;
 IF NOT FOUND OR j.revision<>r OR j.target_digest IS DISTINCT FROM b OR j.phase<>'RETIRING'
 OR e>=18446744073709551615 OR r>=18446744073709551615 OR j.target_map_revision IS NULL
 THEN RAISE EXCEPTION 'World reset occurrence/target stale or unqualified' USING ERRCODE='OTR03'; END IF;
 LOCK TABLE game_runtime_scope_assignments IN SHARE MODE;
 SELECT reset_epoch INTO a FROM game_world_reset_active_epochs WHERE world_id=w FOR UPDATE;
 IF NOT FOUND OR a<>e OR NOT EXISTS(SELECT 1 FROM game_world_reset_epoch_history h
 WHERE h.world_id=w AND h.reset_epoch=e AND h.map_revision=j.active_map_revision)
 OR NOT game_node_lock_current_registration(n,nr)
 THEN RAISE EXCEPTION 'World reset active epoch/incarnation refused' USING ERRCODE='OTR03'; END IF;
 SELECT * INTO v_history FROM game_world_reset_epoch_history WHERE world_id=w AND reset_epoch=e;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset predecessor history absent' USING ERRCODE='OTR03'; END IF;
 LOCK TABLE game_item_ground_locations,game_item_corpse_container_entries IN SHARE MODE;
 LOCK TABLE game_item_house_interior_locations,game_house_ownership,game_world_house_instances IN SHARE MODE;
 -- Re-read durable prerequisites only after all cohort, admission, Ground and House fences.
 IF NOT game_world_reset_cohort_current(w,e,n,nr)
 OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE world_id=w)
 OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries ce JOIN game_item_ground_locations g
 ON g.item_instance_id=ce.parent_item_instance_id WHERE g.world_id=w)
 THEN RAISE EXCEPTION 'World reset current durable prerequisites refused' USING ERRCODE='OTR03'; END IF;
 FOR v_channel IN SELECT channel_id FROM game_runtime_scope_assignments WHERE world_id=w AND scope_kind=1 ORDER BY uuid_send(channel_id) LOOP
  -- Native sequence is per Channel and is NOT the World epoch. Serialize with op4.
  PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:content-activation:'||w::text||':'||v_channel.channel_id::text,0));
  SELECT * INTO v_current FROM game_content_activations WHERE world_id=w AND channel_id=v_channel.channel_id ORDER BY activation_sequence DESC LIMIT 1;
  IF NOT FOUND OR (v_current.server_artifact_digest,v_current.client_artifact_digest,v_current.frame_binding_digest)
   IS DISTINCT FROM (v_history.server_digest,v_history.client_digest,v_history.frame_digest)
  THEN RAISE EXCEPTION 'World reset current Channel activation disagrees with predecessor history' USING ERRCODE='OTR03'; END IF;
  v_previous:=v_current.activation_sequence;
  v_sequence:=v_previous+1;
  IF v_sequence>18446744073709551615 THEN RAISE EXCEPTION 'Content issuance sequence exhausted' USING ERRCODE='OTR03'; END IF;
  PERFORM game_content_record_activation_core(w,v_channel.channel_id,v_sequence,v_previous,j.target_server_digest,j.target_client_digest,j.target_frame_digest);
 END LOOP;
 v_instant:=clock_timestamp();
 v_month:=game_world_reset_month_key(v_instant,j.target_calendar_policy,j.target_calendar_revision);
 IF v_month IS NULL THEN RAISE EXCEPTION 'World reset calendar refused' USING ERRCODE='OTR03'; END IF;
 INSERT INTO game_world_reset_epoch_history(world_id,reset_epoch,map_revision,server_digest,client_digest,frame_digest,calendar_policy,calendar_revision,calendar_parity,month_key,recorded_at,issued_by)
 VALUES(w,e+1,j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest,j.target_calendar_policy,j.target_calendar_revision,'PARITY_PENDING',v_month,v_instant,session_user);
 UPDATE game_world_reset_active_epochs SET reset_epoch=e+1 WHERE world_id=w AND reset_epoch=e;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset active CAS refused' USING ERRCODE='OTR03'; END IF;
 UPDATE game_world_reset_jobs SET phase='ACTIVATED',revision=r+1 WHERE world_id=w AND reset_epoch=e AND phase='RETIRING' AND revision=r;
 IF NOT FOUND THEN RAISE EXCEPTION 'World reset job CAS refused' USING ERRCODE='OTR03'; END IF;
END $$;
REVOKE ALL ON FUNCTION game_world_reset_activate(UUID,NUMERIC,NUMERIC,BYTEA,UUID,NUMERIC) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_world_reset_activate(UUID,NUMERIC,NUMERIC,BYTEA,UUID,NUMERIC) TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_reset_activate(uuid,numeric,numeric,bytea,uuid,numeric) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
"#;
