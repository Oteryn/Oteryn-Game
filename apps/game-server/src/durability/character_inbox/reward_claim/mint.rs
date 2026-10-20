//! Native fresh-mint bridge consumed inside MAIL's real Root transaction.
//! The source owns claim/line/binding writes; this module owns frozen mint
//! metadata, fresh identities, the Inbox destination receipt and tag-4 audit.
//! No value write opens a second transaction or delegates authority to input.

use super::*;
use crate::durability::character_authority::SERVER_BUILD_ID;
use crate::durability::item_mint::{TypedDefinitionRef, uuid_text};
use crate::durability::item_mint_audit::{
    self as common, AuditError, OneItemStateV1, OneItemTypedDefinitionRevisionV1,
    RL08_RETRY_WORK_UNITS_MAX,
};
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, stack_maximum, validate_facts,
};
use crate::durability::item_transfer_audit::OneItemCommandRefV1;
use crate::durability::reward_claim_mint_audit::{
    self as audit, OneItemRewardClaimCauseV1, OneItemRewardClaimMintV1,
    RewardClaimInboxMintOutputV1, RewardClaimInboxMintV1, RewardClaimMintEventIdentity,
};
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimInboxMintOutput {
    pub source_line: u32,
    pub item: ItemDefinitionFacts,
    pub quantity: u32,
    pub key_number: Option<u16>,
}

/// Server-resolved source facts. This is semantic input, never a Root or claim
/// capability. All fields are frozen into the durable reservation's exact spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimInboxMintRequest {
    pub claim: TypedDefinitionRef,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
    pub outputs: Vec<RewardClaimInboxMintOutput>,
}

#[derive(Debug)]
pub struct RewardClaimInboxMintCandidate {
    frozen: FrozenInboxMint,
    work_units_used: u8,
}

impl RewardClaimInboxMintCandidate {
    pub fn plan(&self) -> &RewardClaimInboxPlan {
        &self.frozen.plan
    }
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.frozen.event_id
    }
    pub const fn occurred_at_unix_ms(&self) -> i64 {
        self.frozen.occurred_at
    }
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }
    pub const fn source_binding(&self) -> &[u8; 32] {
        &self.frozen.source_binding
    }
}

#[derive(Debug, Clone)]
struct FrozenInboxMint {
    plan: RewardClaimInboxPlan,
    request: RewardClaimInboxMintRequest,
    source_binding: [u8; 32],
    mint_intent_sha256: [u8; 32],
    event_id: [u8; 16],
    occurred_at: i64,
}

fn audit_error(error: AuditError) -> RewardClaimInboxError {
    match error {
        AuditError::CapacityExceeded => RewardClaimInboxError::CapacityExceeded,
        AuditError::InvalidInput => RewardClaimInboxError::InvalidPlan,
    }
}

fn mint_sql_error(error: sqlx::Error) -> RewardClaimInboxError {
    if error.as_database_error().and_then(|e| e.code()).as_deref() == Some("OTI05") {
        RewardClaimInboxError::ConflictingDelivery
    } else {
        error.into()
    }
}

fn definition(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

fn mint_spec(request: &RewardClaimInboxMintRequest) -> Result<Vec<u8>, RewardClaimInboxError> {
    audit::reward_inbox_usage(request.outputs.len()).map_err(audit_error)?;
    common::check_definition(Some(&definition(&request.claim))).map_err(audit_error)?;
    for text in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        common::check_content_key(text).map_err(audit_error)?;
    }
    let mut outputs = Vec::with_capacity(request.outputs.len());
    for output in &request.outputs {
        validate_facts(&output.item).map_err(|_| RewardClaimInboxError::InvalidPlan)?;
        if output.item.container_capacity.is_some() || output.item.container_slot_equip_pattern {
            return Err(RewardClaimInboxError::InvalidPlan);
        }
        let (maximum, _) =
            stack_maximum(output.item.stack).map_err(|_| RewardClaimInboxError::InvalidPlan)?;
        if output.quantity == 0
            || output.quantity > maximum
            || output.key_number.is_some_and(|key| {
                key == 0
                    || output.quantity != 1
                    || output.item.stack != ItemStackClass::NonStackable
            })
        {
            return Err(RewardClaimInboxError::InvalidPlan);
        }
        let (stack_kind, proven_maximum) = match output.item.stack {
            ItemStackClass::NonStackable => (0, None),
            ItemStackClass::Stackable { proven_maximum } => (1, proven_maximum),
            ItemStackClass::Unknown => return Err(RewardClaimInboxError::InvalidPlan),
        };
        outputs.push(json!({
            "source_line": output.source_line,
            "family": output.item.definition.family,
            "key": output.item.definition.production_key,
            "revision": output.item.definition.revision_ref,
            "quantity": output.quantity,
            "stack_kind": stack_kind,
            "stack_maximum": maximum,
            "proven_stack_maximum": proven_maximum,
            "container_capacity": output.item.container_capacity,
            "container_slot_equip_pattern": output.item.container_slot_equip_pattern,
            "key_number": output.key_number,
        }));
    }
    let spec = serde_json::to_vec(&json!({
        "version": 1,
        "claim": {"family": request.claim.family, "key": request.claim.production_key,
                  "revision": request.claim.revision_ref},
        "content_revision": request.content_revision,
        "ruleset_revision": request.ruleset_revision,
        "sim_revision": request.sim_revision,
        "outputs": outputs,
    }))
    .map_err(|_| RewardClaimInboxError::InvalidPlan)?;
    // Twenty bounded definitions with JSON escaping still fit this finite
    // reservation ceiling. The SQL boundary independently enforces it.
    if spec.len() > 65_536 {
        return Err(RewardClaimInboxError::CapacityExceeded);
    }
    Ok(spec)
}

fn bound_plan(plan: &RewardClaimInboxPlan, spec: &[u8]) -> RewardClaimInboxPlan {
    let mut bound = plan.clone();
    let mut digest = Sha256::new();
    digest.update(b"oteryn:reward_claim_mint:inbox-source:v1\0");
    digest.update(plan.source_binding);
    digest.update(Sha256::digest(spec));
    bound.source_binding = digest.finalize().into();
    bound
}

impl DurabilityRoot {
    /// Persist the complete mint spec, exact caller-frozen transaction/items and
    /// native-generated EventId/server time before the first value attempt. A
    /// changed source, output fact or plan conflicts with the first reservation.
    /// Fresh or uncommitted preparations require the actual live fence; complete
    /// immutable replay needs only this Root's current recovery proof.
    pub async fn prepare_reward_claim_inbox_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        source_plan: &RewardClaimInboxPlan,
        request: RewardClaimInboxMintRequest,
    ) -> Result<RewardClaimInboxMintCandidate, RewardClaimInboxError> {
        source_plan.validate()?;
        let spec = mint_spec(&request)?;
        if source_plan.item_instance_ids.len() != request.outputs.len() {
            return Err(RewardClaimInboxError::InvalidPlan);
        }
        let source_binding = source_plan.source_binding;
        let plan = bound_plan(source_plan, &spec);
        let binding = plan.binding();
        let digest: [u8; 32] = Sha256::digest(&spec).into();
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimInboxError::AuthorityRejected)?;
        let node = node.clone();
        let query_plan = plan.clone();
        let (event_id, occurred_at, used) = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_reward_claim_cause(&mut tx, query_plan.command).await?;
                    let replay = match lookup_plan(&mut tx, &query_plan, &binding).await? {
                        Ok(Some(_)) => true,
                        Ok(None) => false,
                        Err(error) => return Ok(Err(error)),
                    };
                    if !replay
                        && !character_item_fence_is_current(
                            &mut tx,
                            &node,
                            &fence,
                            query_plan.command,
                            query_plan.character_id,
                            *query_plan.world_id.as_bytes(),
                            *query_plan.channel_id.as_bytes(),
                        )
                        .await?
                    {
                        return Ok(Err(RewardClaimInboxError::AuthorityRejected));
                    }
                    let item_ids: Vec<String> = query_plan
                        .item_instance_ids
                        .iter()
                        .map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect())
                        .collect();
                    let row = match sqlx::query(
                        "SELECT event_id::text AS event_id, occurred_at, work_units_used \
                     FROM game_reward_claim_inbox_mint_reserve(\
                       encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,\
                       encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,\
                       $7,$8,$9::text[]::uuid[],$10)",
                    )
                    .bind(query_plan.command.game_session_id().as_bytes().as_slice())
                    .bind(query_plan.command.command_id().get().to_string())
                    .bind(query_plan.character_id.as_bytes().as_slice())
                    .bind(query_plan.world_id.as_bytes().as_slice())
                    .bind(query_plan.channel_id.as_bytes().as_slice())
                    .bind(query_plan.transaction_id.as_slice())
                    .bind(source_binding.as_slice())
                    .bind(binding.as_slice())
                    .bind(item_ids)
                    .bind(spec)
                    .fetch_one(&mut *tx)
                    .await
                    {
                        Ok(row) => row,
                        Err(error) => return Ok(Err(mint_sql_error(error))),
                    };
                    let event_id = uuid_text(row.try_get("event_id")?)?;
                    let occurred_at: i64 = row.try_get("occurred_at")?;
                    let used = u8::try_from(row.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if occurred_at <= 0
                        || occurred_at
                            .checked_add(common::AUDIT_RETENTION_P90D_MS)
                            .is_none()
                        || used > RL08_RETRY_WORK_UNITS_MAX
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((event_id, occurred_at, used)))
                })
            })
            .await??;
        Ok(RewardClaimInboxMintCandidate {
            frozen: FrozenInboxMint {
                plan,
                request,
                source_binding,
                mint_intent_sha256: digest,
                event_id,
                occurred_at,
            },
            work_units_used: used,
        })
    }

    /// Perform the actual fresh mint, Inbox placement and registered audit, then
    /// let the source record its claim and item bindings in the same transaction.
    /// The MAIL runner commits all of them together or rolls all value back.
    pub async fn commit_reward_claim_inbox_mint<T, C, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        candidate: &mut RewardClaimInboxMintCandidate,
        owner: &mut C,
        operation: F,
    ) -> Result<RewardClaimInboxOutcome<T>, RewardClaimInboxError>
    where
        T: Send + 'static,
        C: Send,
        F: for<'a, 'connection> FnOnce(
                &'a mut RewardClaimInboxTransaction<'connection>,
                &'a mut C,
            ) -> Pin<
                Box<dyn Future<Output = Result<T, RewardClaimInboxError>> + Send + 'a>,
            > + Send
            + 'static,
    {
        if let Some(positions) = self
            .reconcile_reward_claim_inbox(authority, candidate.plan())
            .await?
        {
            return Ok(RewardClaimInboxOutcome::AlreadyCommitted(positions));
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimInboxError::AuthorityRejected)?;
        let frozen = candidate.frozen.clone();
        let charge = frozen.clone();
        let used = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = match sqlx::query(
                        "SELECT work_units_used FROM game_reward_claim_inbox_mint_charge(\
                encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4)",
                    )
                    .bind(charge.plan.command.game_session_id().as_bytes().as_slice())
                    .bind(charge.plan.command.command_id().get().to_string())
                    .bind(charge.plan.transaction_id.as_slice())
                    .bind(charge.plan.binding().as_slice())
                    .fetch_optional(&mut *tx)
                    .await
                    {
                        Ok(row) => row,
                        Err(error) => return Ok(Err(mint_sql_error(error))),
                    };
                    let Some(row) = row else {
                        return Ok(Err(RewardClaimInboxError::CapacityExceeded));
                    };
                    let used = u8::try_from(row.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(used))
                })
            })
            .await??;
        candidate.work_units_used = used;
        self.with_reward_claim_inbox_transaction(
            authority,
            node,
            fence,
            candidate.plan(),
            owner,
            move |context, owner| {
                Box::pin(async move {
                    mint_inbox(context, &frozen).await?;
                    operation(context, owner).await
                })
            },
        )
        .await
    }
}

async fn mint_inbox(
    context: &mut RewardClaimInboxTransaction<'_>,
    frozen: &FrozenInboxMint,
) -> Result<(), RewardClaimInboxError> {
    context.assert_same_transaction().await?;
    if context.plan != frozen.plan {
        return Err(RewardClaimInboxError::ConflictingDelivery);
    }
    let plan = context.plan.clone();
    let binding = plan.binding();
    // This narrow function reads the immutable reservation; it accepts no new
    // item definition, quantity or source facts from the commit caller.
    sqlx::query(
        "SELECT game_reward_claim_inbox_mint_begin(encode($1,'hex')::uuid,\
        $2::text::numeric,encode($3,'hex')::uuid,$4)",
    )
    .bind(plan.command.game_session_id().as_bytes().as_slice())
    .bind(plan.command.command_id().get().to_string())
    .bind(plan.transaction_id.as_slice())
    .bind(binding.as_slice())
    .execute(&mut **context.transaction())
    .await
    .map_err(mint_sql_error)?;
    let mut outputs = Vec::with_capacity(plan.item_instance_ids.len());
    for (item, source) in plan.item_instance_ids.iter().zip(&frozen.request.outputs) {
        let position = context.deliver(*item).await?;
        outputs.push(RewardClaimInboxMintOutputV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: item.to_vec(),
                world_id: plan.world_id.as_bytes().to_vec(),
                definition: Some(definition(&source.item.definition)),
                quantity: source.quantity,
                lifecycle: common::ITEM_LIFECYCLE_LIVE,
                key_number: source.key_number.map(u32::from).unwrap_or(0),
            }),
            ordinal: position.ordinal,
            source_line: source.source_line,
        });
    }
    let payload = OneItemRewardClaimMintV1 {
        after: None,
        destination: None,
        before_semantically_absent: true,
        source: Some(OneItemRewardClaimCauseV1 {
            typed_cause: context.typed_cause().into(),
            content_revision: frozen.request.content_revision.clone(),
            ruleset_revision: frozen.request.ruleset_revision.clone(),
            sim_revision: frozen.request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: plan.command.game_session_id().as_bytes().to_vec(),
                command_id: plan.command.command_id().get(),
            }),
            reward_claim: Some(definition(&frozen.request.claim)),
        }),
        inbox: Some(RewardClaimInboxMintV1 {
            character_id: plan.character_id.as_bytes().to_vec(),
            world_id: plan.world_id.as_bytes().to_vec(),
            expected_session_generation: context.connection_generation,
            source_binding: frozen.source_binding.to_vec(),
            outputs,
            plan_binding: binding.to_vec(),
            mint_intent_sha256: frozen.mint_intent_sha256.to_vec(),
            expected_character_lease_generation: context.character_lease_generation,
        }),
    };
    let tuple = context.selected_type2_tuple();
    let envelope = audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: frozen.event_id,
            transaction_id: plan.transaction_id,
            occurred_at_unix_ms: frozen.occurred_at,
            channel_id: *plan.channel_id.as_bytes(),
            server_build_id: SERVER_BUILD_ID,
        },
        payload,
        tuple.tuple(),
    )
    .map_err(audit_error)?;
    let envelope_hash: [u8; 32] = Sha256::digest(&envelope).into();
    sqlx::query(
        "SELECT game_reward_claim_inbox_mint_finish(encode($1,'hex')::uuid,\
        $2::text::numeric,encode($3,'hex')::uuid,$4,$5::text::numeric,$6::text::numeric,$7)",
    )
    .bind(plan.command.game_session_id().as_bytes().as_slice())
    .bind(plan.command.command_id().get().to_string())
    .bind(plan.transaction_id.as_slice())
    .bind(binding.as_slice())
    .bind(context.connection_generation.to_string())
    .bind(context.character_lease_generation.to_string())
    .bind(envelope_hash.as_slice())
    .execute(&mut **context.transaction())
    .await
    .map_err(mint_sql_error)?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,\
        transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,\
        occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(\
        encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,\
        $6,$6+$7,$8,sha256($8),1)",
    )
    .bind(frozen.event_id.as_slice())
    .bind(plan.transaction_id.as_slice())
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(plan.item_instance_ids[0].as_slice())
    .bind(frozen.occurred_at)
    .bind(common::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **context.transaction())
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
