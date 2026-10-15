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
    ConflictingCause,
    CapacityExceeded,
    InvalidStoredState,
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
    let mut sql = RESET_SCHEMA.to_owned();
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

const RESET_SCHEMA: &str = r#"
-- These are reset records/cohorts, NOT a second retirement ledger. Only control publishes them.
CREATE TABLE game_world_reset_jobs (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551614),
 target_digest BYTEA NOT NULL CHECK(octet_length(target_digest)=32),
 active_map_revision TEXT NOT NULL CHECK(octet_length(active_map_revision) BETWEEN 1 AND 512),
 phase TEXT NOT NULL CHECK(phase IN('RETIRING','ACTIVATED')),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,reset_epoch)
);
CREATE UNIQUE INDEX game_world_reset_one_retiring ON game_world_reset_jobs(world_id) WHERE phase='RETIRING';
CREATE TABLE game_world_reset_scopes (
 world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL, channel_id UUID NOT NULL,
 predecessor_generation NUMERIC(20,0) NOT NULL CHECK(predecessor_generation BETWEEN 1 AND 18446744073709551614),
 PRIMARY KEY(world_id,reset_epoch,channel_id),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_jobs(world_id,reset_epoch)
);
CREATE FUNCTION game_world_reset_job_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.revision=OLD.revision+1
 AND (to_jsonb(NEW)-'revision')=(to_jsonb(OLD)-'revision') THEN RETURN NEW; END IF;
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
 FOREACH f IN ARRAY ARRAY['game_world_reset_cohort_current(uuid,numeric,uuid,numeric)','game_world_reset_item_proven(uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
"#;
