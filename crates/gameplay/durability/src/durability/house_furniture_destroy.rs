//! Fenced House destruction owner; SQL owns custody, receipts and timed arming.
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use serde_json::{Value, json};

// Exact source/target definition triples followed by item and ACL revisions.
type ObservationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

#[derive(Debug, Clone)]
pub struct HouseDestroyCommand {
    pub house: HouseId,
    pub command: u64,
    pub game_session_id: crate::foundation::GameSessionId,
    pub source: [u8; 16],
    pub source_definition: TypedDefinitionRef,
    pub target: [u8; 16],
    pub target_definition: TypedDefinitionRef,
    pub expected_revision: u64,
    pub expected_acl_revision: u64,
    pub roll: u8,
    pub attack: i32,
    pub expected_success: bool,
    pub remains: TypedDefinitionRef,
    pub decay_target: TypedDefinitionRef,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseDestroyOutcome {
    Committed,
    FailedRoll,
    NoAccess,
    Stale,
}

/// Observations only: the writer rechecks every field under its locks.
#[derive(Debug, Clone)]
pub struct HouseDestroyObservation {
    pub source: TypedDefinitionRef,
    pub target: TypedDefinitionRef,
    pub revision: u64,
    pub acl_revision: u64,
}
fn definition(value: &TypedDefinitionRef) -> Value {
    json!({"family":value.family,"key":value.production_key,"revision":value.revision_ref})
}
fn hex(value: &[u8; 16]) -> String {
    value.iter().map(|v| format!("{v:02x}")).collect()
}
impl DurabilityRoot {
    pub async fn observe_house_destroy(
        &self,
        house: HouseId,
        source: [u8; 16],
        target: [u8; 16],
        session: crate::foundation::GameSessionId,
        command: u64,
    ) -> Result<Option<HouseDestroyObservation>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<ObservationRow> = sqlx::query_as(
                "SELECT s.definition_family,s.definition_production_key,s.definition_revision_ref,t.definition_family,t.definition_production_key,t.definition_revision_ref,t.state_revision::text,p.acl_revision::text FROM game_item_instances s CROSS JOIN game_item_instances t JOIN game_house_properties p ON p.world_id=t.world_id AND p.house_key=$2 WHERE s.item_instance_id=encode($3,'hex')::uuid AND t.item_instance_id=encode($4,'hex')::uuid AND s.world_id=encode($1,'hex')::uuid AND t.world_id=s.world_id AND (EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=t.item_instance_id AND h.world_id=p.world_id AND h.house_key=p.house_key) OR EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r WHERE r.game_session_id=encode($5,'hex')::uuid AND r.command_id=$6::text::numeric AND r.target=t.item_instance_id AND r.source=s.item_instance_id))")
                .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key())
                .bind(source.as_slice()).bind(target.as_slice()).bind(session.as_bytes().as_slice())
                .bind(command.to_string()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            row.map(|(sf,sk,sr,tf,tk,tr,revision,acl)| Ok(HouseDestroyObservation {
                source: TypedDefinitionRef { family:sf, production_key:sk, revision_ref:sr },
                target: TypedDefinitionRef { family:tf, production_key:tk, revision_ref:tr },
                revision:revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                acl_revision:acl.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
            })).transpose()
        })).await
    }

    pub async fn destroy_house_furniture(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        input: HouseDestroyCommand,
    ) -> Result<HouseDestroyOutcome, DurabilityError> {
        if fence.runtime_scope != input.house.runtime_scope()?
            || fence.game_session_id != input.game_session_id
        {
            return Ok(HouseDestroyOutcome::Stale);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? {
                return Ok(HouseDestroyOutcome::Stale);
            }
            let payload = json!({"source":hex(&input.source),"target":hex(&input.target),
                "source_definition":definition(&input.source_definition),
                "target_definition":definition(&input.target_definition),
                "revision":input.expected_revision.to_string(),"acl":input.expected_acl_revision.to_string(),
                "roll":input.roll,"attack":input.attack,"succeeds":input.expected_success,"remains":definition(&input.remains),
                "decay":definition(&input.decay_target),"duration":input.duration_ms});
            let fact = node.fact();
            let result: String = sqlx::query_scalar("SELECT game_house_furniture_destroy(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,$10::text::numeric,$11::jsonb)->>'status'")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
                .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(input.house.world_id().as_bytes().as_slice())
                .bind(input.house.house_key()).bind(input.command.to_string()).bind(payload)
                .fetch_one(&mut *tx).await?;
            let outcome = match result.as_str() {
                "COMMITTED" => HouseDestroyOutcome::Committed,
                "FAILED_ROLL" => HouseDestroyOutcome::FailedRoll,
                "NO_ACCESS" => HouseDestroyOutcome::NoAccess,
                "STALE" => HouseDestroyOutcome::Stale,
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(outcome)
        })).await
    }
}

impl DurabilityRoot {
    /// Settle bounded batches before the first House map read after an idle period.
    /// Crystal src/items/decay/decay.cpp:29-54 arms absolute deadlines independently
    /// of observers; the SQL owner catches up the closed eight-stage animation.
    pub async fn settle_house_timed_decorations(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
    ) -> Result<bool, DurabilityError> {
        // Each call processes up to sixteen native timers and sixteen destruction
        // timers in its own fenced transaction; no runtime guard spans this work.
        for _ in 0..=(super::house_interior::HOUSE_INTERIOR_ITEMS_MAX / 16) {
            if self
                .expire_house_furniture(node, fence, house.clone())
                .await?
                < 16
            {
                return Ok(true);
            }
            tokio::task::yield_now().await;
        }
        Ok(false)
    }

    /// Bounded deadline work over the existing timed-state deadline index.
    pub async fn expire_house_furniture(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
    ) -> Result<u32, DurabilityError> {
        if fence.runtime_scope != house.runtime_scope()? {
            return Ok(0);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await? { return Ok(0); }
            let fact=node.fact();
            let count:i32=sqlx::query_scalar("SELECT game_house_furniture_destroy_expire(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
                .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(house.world_id().as_bytes().as_slice())
                .bind(house.house_key()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            count.try_into().map_err(|_| DurabilityError::InvalidStoredState)
        })).await
    }
}
