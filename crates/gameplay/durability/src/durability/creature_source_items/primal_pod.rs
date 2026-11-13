//! Explicit source Encounter callback through the one existing physical Item writer.
//! The prepared callback grants no Player, inventory, Ability or death-loot cause.
use super::*;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1,
};

const BOSS: &str = "oteryn:creature.the_primal_menace";
const POD: &str = "oteryn:item.tibia.i39176";

/// Sealed while the real source parent is alive. Crystal's delayed Pod closure
/// ignores parent liveness; current SQL scope/Node/content/map are re-proved at
/// execution, so this historical source token never supplies current authority.
#[derive(Debug)]
pub struct PrimalPodCallback {
    cause: CreatureGroundCause,
    operation: SpellItemOperation,
    due_us: u64,
    position: MovementLocalPosition,
    item: [u8; 16],
}
impl PrimalPodCallback {
    #[allow(clippy::too_many_arguments)]
    pub fn from_current_native_wave(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        parent: ExactActorRef,
        position: MovementLocalPosition,
        ordinal: u64,
        now_us: u64,
    ) -> Result<Self> {
        runtime
            .owner_fence()
            .map_err(|_| SpellItemError::Rejected("Primal Pod source owner"))?;
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || parent.runtime_scope() != runtime.binding().runtime_scope()
            || parent.scope_generation() != runtime.binding().scope_generation()
            || !runtime.matches_live_creature_identity(parent, BOSS.as_bytes())
            || runtime.native_summon_master(parent).is_some()
            || !native.qualifies_primal_callbacks(runtime.content_pin().server_artifact_digest())
            || ordinal == 0
            || position.floor != 15
            || !(33547..=33570).contains(&position.x)
            || !(32749..=32769).contains(&position.y)
        {
            return Err(SpellItemError::Rejected(
                "Primal Pod exact source Instance/closure",
            ));
        }
        let parent_snapshot = runtime
            .companion_snapshot(parent)
            .map_err(|_| SpellItemError::Rejected("Primal Pod actual source parent"))?;
        if parent_snapshot.state.master.is_some() || parent_snapshot.health <= 0 {
            return Err(SpellItemError::Rejected("Primal Pod source parent retired"));
        }
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let policy = native
            .item_policy(POD, "definition-r1")
            .ok_or(SpellItemError::Rejected("Primal Pod actual source Item"))?;
        let attrs = &policy.record().attributes;
        let definition = QualifiedItemDefinition::from_native_policy(policy)
            .map_err(SpellItemError::Rejected)?;
        if !definition.materializable
            || !definition.ground_destination
            || definition.movable
            || definition.stack_maximum != 1
            || definition.container_capacity.is_some()
            || definition.decay.is_some()
            || !definition.decay_chain.is_empty()
            || attrs.blocks_movement != Some(false)
            || attrs.blocks_projectile != Some(false)
            || attrs.immovable_block_solid != Some(false)
        {
            return Err(SpellItemError::Rejected(
                "Primal Pod nonpickupable nondecaying source policy",
            ));
        }
        let due_us = now_us
            .checked_add(3_000_000)
            .ok_or(SpellItemError::Rejected("Primal Pod due overflow"))?;
        let mut h = Sha256::new();
        h.update(b"oteryn:primal-pod-callback:v1");
        h.update(parent.placement_identity());
        h.update(parent.actor_local_generation().to_be_bytes());
        h.update(ordinal.to_be_bytes());
        h.update(due_us.to_be_bytes());
        h.update(position.x.to_be_bytes());
        h.update(position.y.to_be_bytes());
        h.update(position.floor.to_be_bytes());
        h.update(runtime.content_pin().server_artifact_digest());
        h.update(runtime.content_pin().map_revision_digest());
        h.update(runtime.content_pin().frame_binding_digest());
        let cast: [u8; 32] = h.finalize().into();
        let source = serde_json::to_vec(
            native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == BOSS)
                .ok_or(SpellItemError::Rejected("Primal Pod source body"))?,
        )
        .map_err(|_| SpellItemError::Rejected("Primal Pod source encoding"))?;
        let occurrence = format!(
            "primal-pod:{}:{ordinal}:{due_us}",
            destroy_instance_key(&parent.placement_identity())
        );
        let cause = CreatureGroundCause {
            world: *parent.world_id().as_bytes(),
            scope: parent.runtime_scope(),
            scope_generation: parent.scope_generation().get(),
            source_items: vec![TypedDefinitionRef {
                family: "Item".into(),
                production_key: POD.into(),
                revision_ref: "definition-r1".into(),
            }],
            placement: parent.placement_identity(),
            generation: parent.actor_local_generation(),
            creature: TypedDefinitionRef {
                family: "Creature".into(),
                production_key: BOSS.into(),
                revision_ref: "definition-r1".into(),
            },
            ability: TypedDefinitionRef {
                family: "Encounter".into(),
                production_key: "oteryn:encounter.the_primal_menace".into(),
                revision_ref: "definition-r1".into(),
            },
            occurrence,
            content: native.source_digest(),
            map: runtime.content_pin().map_revision_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            body: Sha256::digest(source).into(),
            cast,
            event_rule: None,
        };
        let item = identity(&cause, b"oteryn:primal-pod-item:v1");
        let operation = SpellItemOperation::MintGround {
            item_instance_id: item,
            definition,
            quantity: 1,
            placement: target.placement(cause.occurrence.as_bytes().to_vec()),
            lifetime_millis: None,
            blocks_movement: false,
            blocks_projectile: false,
            description: None,
        };
        Ok(Self {
            cause,
            operation,
            due_us,
            position,
            item,
        })
    }
    /// Stable source transaction identity; grants no clock or Item authority.
    pub fn source_transaction_id(&self) -> [u8; 16] {
        identity(&self.cause, b"oteryn:primal-pod-transaction:v1")
    }
    pub fn due_us(&self) -> u64 {
        self.due_us
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn item_instance_id(&self) -> [u8; 16] {
        self.item
    }
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.cause.scope
    }
    pub fn scope_generation(&self) -> u64 {
        self.cause.scope_generation
    }
}
pub(super) fn identity(cause: &CreatureGroundCause, domain: &[u8]) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(domain);
    h.update(cause.world);
    cause.hash_scope(&mut h);
    h.update(cause.scope_generation.to_be_bytes());
    h.update(cause.placement);
    h.update(cause.generation.to_be_bytes());
    h.update(cause.cast);
    let digest = h.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

/// Current authority comes only from the issued SQL scope token. A committed
/// receipt is reconciled before mutation, and all replays re-prove the scope.
pub async fn execute_primal_pod_callback_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &PrimalPodCallback,
    now_us: u64,
) -> Result<CreatureGroundReceipt> {
    if now_us < callback.due_us
        || authority.runtime_scope() != callback.cause.scope
        || authority.generation() != callback.cause.scope_generation
    {
        return Err(SpellItemError::Rejected("Primal Pod current due/scope"));
    }
    check_scope_transaction(tx, authority).await?;
    let Some(instance) = callback.cause.instance_bytes() else {
        return Err(SpellItemError::Rejected("Primal Pod Instance required"));
    };
    // The durable accepted activity carries the actual source room pin. Its
    // current ownership is already locked by the issued scope authority.
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric(20,0) AND g.state=2 AND e.destination_server_artifact_digest=$4 AND e.destination_map_revision_digest=$5 AND e.destination_coordinate_frame_digest=$6)")
        .bind(callback.cause.world.as_slice()).bind(instance.as_slice()).bind(callback.cause.scope_generation.to_string())
        .bind(callback.cause.content.as_slice()).bind(callback.cause.map.as_slice()).bind(callback.cause.frame.as_slice())
        .fetch_one(&mut **tx).await?;
    if !active {
        return Err(SpellItemError::Rejected(
            "Primal Pod current accepted activity pin",
        ));
    }
    if let Some(prior) =
        reconcile_creature_ground_in_transaction(tx, authority, &callback.cause, || Ok(())).await?
    {
        return Ok(prior);
    }
    let prepared = PreparedCreatureGroundOperations {
        placement: callback.cause.placement,
        generation: callback.cause.generation,
        body: callback.cause.body,
        cast: callback.cause.cast,
        operations: vec![callback.operation.clone()],
    };
    let receipt = apply_creature_ground_in_transaction(
        tx,
        authority,
        &callback.cause,
        identity(&callback.cause, b"oteryn:primal-pod-transaction:v1"),
        identity(&callback.cause, b"oteryn:primal-pod-event:v1"),
        &prepared,
        || Ok(()),
    )
    .await?;
    sqlx::query("INSERT INTO game_primal_pod_callbacks(source_transaction_id,item_instance_id,world_id,instance_id,ownership_generation,spatial_position,content_digest,map_digest,frame_digest,source_timer_unix_ms,due_unix_ms) SELECT r.transaction_id,l.item_instance_id,r.world_id,r.instance_id,r.ownership_generation,l.spatial_position,r.catalog_digest,r.creature_map_digest,r.creature_frame_digest,(r.occurred_at_unix_ms/1000)*1000,r.occurred_at_unix_ms+4000 FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.transaction_id=encode($1,'hex')::uuid AND l.operation_kind=1")
        .bind(receipt.transaction_id.as_slice()).execute(&mut **tx).await?;
    Ok(receipt)
}

struct PodScopeContext<'a> {
    permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    callback: &'a PrimalPodCallback,
    now_us: u64,
}
impl DurabilityRoot {
    /// Only sealed immutable source data crosses this await. The current Node,
    /// character recovery and tagged Instance authority are issued inside SQL;
    /// callers release native runtime/object guards before entering this owner.
    pub async fn commit_primal_pod_callback(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &PrimalPodCallback,
        now_us: u64,
    ) -> Result<CreatureGroundReceipt> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let scope = callback.runtime_scope();
        let generation = callback.scope_generation();
        let mut context = PodScopeContext {
            permit,
            callback,
            now_us,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<CreatureGroundReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.permit,
                            &record,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        let receipt = execute_primal_pod_callback_in_transaction(
                            &mut tx,
                            &authority,
                            context.callback,
                            context.now_us,
                        )
                        .await?;
                        Ok(receipt)
                    }
                    .await;
                    match outcome {
                        Ok(value) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// A committed source callback's native effect. Retried until the actual native
/// owner acknowledges its single physical spawn attempt.
#[derive(Debug, Clone, Copy)]
pub struct PrimalPodExpiry {
    pub source_transaction_id: [u8; 16],
    pub position: MovementLocalPosition,
    pub requires_spawn: bool,
}

async fn expired_cause(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    source: [u8; 16],
) -> Result<CreatureGroundCause> {
    let row=sqlx::query("SELECT r.creature_placement,r.creature_actor_generation::text,r.creature_source_body_digest,r.creature_cast_digest,r.catalog_digest,r.creature_map_digest,r.creature_frame_digest FROM game_spell_item_receipts r WHERE r.transaction_id=encode($1,'hex')::uuid AND r.cause_kind=2 AND r.world_id=encode($2,'hex')::uuid AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id=encode($3,'hex')::uuid AND r.ownership_generation=$4::text::numeric(20,0)")
        .bind(source.as_slice()).bind(authority.world_bytes().as_slice())
        .bind(match authority.runtime_scope(){RuntimeScopeRefV1::Instance{instance_id,..}=>instance_id,_=>return Err(SpellItemError::Rejected("Pod expiry Instance"))}.as_slice())
        .bind(authority.generation().to_string()).fetch_optional(&mut **tx).await?
        .ok_or(SpellItemError::Rejected("Pod expiry actual source receipt"))?;
    let digest = |name: &str| -> Result<[u8; 32]> {
        row.try_get::<Vec<u8>, _>(name)?
            .try_into()
            .map_err(|_| SpellItemError::Rejected("Pod source receipt digest"))
    };
    let cast = Sha256::new()
        .chain_update(b"oteryn:primal-pod-expiry:v1")
        .chain_update(source)
        .chain_update(digest("creature_cast_digest")?)
        .finalize()
        .into();
    Ok(CreatureGroundCause {
        world: authority.world_bytes(),
        scope: authority.runtime_scope(),
        scope_generation: authority.generation(),
        source_items: vec![TypedDefinitionRef {
            family: "Item".into(),
            production_key: POD.into(),
            revision_ref: "definition-r1".into(),
        }],
        placement: row
            .try_get::<Vec<u8>, _>("creature_placement")?
            .try_into()
            .map_err(|_| SpellItemError::Rejected("Pod source parent placement"))?,
        generation: integer(row.try_get("creature_actor_generation")?)?,
        creature: TypedDefinitionRef {
            family: "Creature".into(),
            production_key: BOSS.into(),
            revision_ref: "definition-r1".into(),
        },
        ability: TypedDefinitionRef {
            family: "Encounter".into(),
            production_key: "oteryn:encounter.the_primal_menace".into(),
            revision_ref: "definition-r1".into(),
        },
        occurrence: format!("primal-pod-expire:{}", destroy_instance_key(&source)),
        content: digest("catalog_digest")?,
        map: digest("creature_map_digest")?,
        frame: digest("creature_frame_digest")?,
        body: digest("creature_source_body_digest")?,
        cast,
        event_rule: None,
    })
}

pub async fn drain_primal_pod_expiry_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
) -> Result<Option<PrimalPodExpiry>> {
    drain_primal_pod_expiry_selected_in_transaction(tx, authority, None, None).await
}
async fn drain_primal_pod_expiry_selected_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    source: Option<[u8; 16]>,
    profiles: Option<&crate::content::native_gameplay::NativeItemProfileSource>,
) -> Result<Option<PrimalPodExpiry>> {
    check_scope_transaction(tx, authority).await?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = authority.runtime_scope() else {
        return Ok(None);
    };
    let row=sqlx::query("SELECT p.source_transaction_id::text,p.spatial_position,p.content_digest,p.map_digest,p.frame_digest,p.completion_kind FROM game_primal_pod_callbacks p WHERE p.world_id=encode($1,'hex')::uuid AND p.instance_id=encode($2,'hex')::uuid AND p.ownership_generation=$3::text::numeric(20,0) AND ($4::bytea IS NULL OR p.source_transaction_id=encode($4,'hex')::uuid) AND (p.completion_kind=0 OR (p.completion_kind=2 AND NOT p.effect_acknowledged) OR ($4::bytea IS NOT NULL AND p.completion_kind=3 AND p.effect_acknowledged)) AND p.due_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=p.world_id AND g.instance_id=p.instance_id AND g.ownership_generation=p.ownership_generation AND g.state=2 AND e.destination_server_artifact_digest=p.content_digest AND e.destination_map_revision_digest=p.map_digest AND e.destination_coordinate_frame_digest=p.frame_digest) ORDER BY p.due_unix_ms,p.source_transaction_id LIMIT 1 FOR UPDATE OF p")
        .bind(authority.world_bytes().as_slice()).bind(instance_id.as_slice()).bind(authority.generation().to_string()).bind(source.as_ref().map(|id| id.as_slice()))
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let source = uuid(&row.try_get::<String, _>("source_transaction_id")?)?;
    let spatial = row.try_get::<Vec<u8>, _>("spatial_position")?;
    let position = decode_position(&spatial)?;
    if row.try_get::<i16, _>("completion_kind")? == 3 {
        return Ok(Some(PrimalPodExpiry {
            source_transaction_id: source,
            position,
            requires_spawn: false,
        }));
    }
    if row.try_get::<i16, _>("completion_kind")? == 2 {
        return Ok(Some(PrimalPodExpiry {
            source_transaction_id: source,
            position,
            requires_spawn: true,
        }));
    }
    let cause = expired_cause(tx, authority, source).await?;
    let target = SpellGroundTarget {
        item_profile_source: profiles.cloned(),
        spatial_position: spatial,
        map_revision: format!(
            "sha256:{}",
            cause
                .map
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>()
        ),
        content_revision: format!(
            "sha256:{}",
            cause
                .content
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>()
        ),
        placement_context: cause.frame.to_vec(),
    };
    let current = read_creature_scope_tile_in_transaction(tx, authority, &target).await?;
    let item = current.items.iter().find(|item| {
        item.definition.family == "Item"
            && item.definition.production_key == POD
            && item.definition.revision_ref == "definition-r1"
    });
    let Some(item) = item else {
        sqlx::query("UPDATE game_primal_pod_callbacks SET completion_kind=3,effect_acknowledged=true WHERE source_transaction_id=encode($1,'hex')::uuid AND completion_kind=0")
            .bind(source.as_slice()).execute(&mut **tx).await?;
        return Ok(Some(PrimalPodExpiry {
            source_transaction_id: source,
            position,
            requires_spawn: false,
        }));
    };
    let removal = prepare_creature_scope_removal_in_transaction(
        tx,
        authority,
        &target,
        item.item_instance_id,
    )
    .await?;
    let prepared = PreparedCreatureGroundOperations {
        placement: cause.placement,
        generation: cause.generation,
        body: cause.body,
        cast: cause.cast,
        operations: vec![SpellItemOperation::RemoveGround(removal)],
    };
    let receipt = apply_creature_ground_in_transaction(
        tx,
        authority,
        &cause,
        identity(&cause, b"oteryn:primal-pod-expire-transaction:v1"),
        identity(&cause, b"oteryn:primal-pod-expire-event:v1"),
        &prepared,
        || Ok(()),
    )
    .await?;
    sqlx::query("UPDATE game_primal_pod_callbacks SET completion_kind=2,completed_transaction_id=encode($2,'hex')::uuid WHERE source_transaction_id=encode($1,'hex')::uuid AND completion_kind=0")
        .bind(source.as_slice()).bind(receipt.transaction_id.as_slice()).execute(&mut **tx).await?;
    Ok(Some(PrimalPodExpiry {
        source_transaction_id: source,
        position,
        requires_spawn: true,
    }))
}
fn decode_position(bytes: &[u8]) -> Result<MovementLocalPosition> {
    if bytes.len() != 10 {
        return Err(SpellItemError::Rejected("Pod source position encoding"));
    }
    Ok(MovementLocalPosition {
        x: i32::from_be_bytes(
            bytes[..4]
                .try_into()
                .map_err(|_| SpellItemError::Rejected("Pod position x"))?,
        ),
        y: i32::from_be_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| SpellItemError::Rejected("Pod position y"))?,
        ),
        floor: i16::from_be_bytes(
            bytes[8..]
                .try_into()
                .map_err(|_| SpellItemError::Rejected("Pod position floor"))?,
        ),
    })
}

impl DurabilityRoot {
    pub async fn drain_primal_pod_expiry(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<Option<PrimalPodExpiry>> {
        self.drain_primal_pod_expiry_selected(permit, recovery, node, scope, generation, None, None)
            .await
    }
    /// Exact retained source callback only. SQL's original receipt clock and
    /// due condition remain authoritative independently of native eligibility.
    #[allow(
        clippy::too_many_arguments,
        reason = "same scoped source owner with one exact retained transaction filter"
    )]
    pub async fn drain_primal_pod_expiry_for_source(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
        source: [u8; 16],
        profiles: crate::content::native_gameplay::NativeItemProfileSource,
    ) -> Result<Option<PrimalPodExpiry>> {
        self.drain_primal_pod_expiry_selected(
            permit,
            recovery,
            node,
            scope,
            generation,
            Some(source),
            Some(profiles),
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "common scoped SQL owner preserves legacy query and exact source filtering"
    )]
    async fn drain_primal_pod_expiry_selected(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
        source: Option<[u8; 16]>,
        profiles: Option<crate::content::native_gameplay::NativeItemProfileSource>,
    ) -> Result<Option<PrimalPodExpiry>> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<Option<PrimalPodExpiry>> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await?;
                        let expired = drain_primal_pod_expiry_selected_in_transaction(
                            &mut tx,
                            &authority,
                            source,
                            profiles.as_ref(),
                        )
                        .await?;
                        Ok(expired)
                    }
                    .await;
                    match outcome {
                        Ok(value) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
    /// The native source owner calls only after its memoized physical spawn
    /// attempt. Receipt replay cannot make a second attempt in the same runtime.
    #[allow(clippy::too_many_arguments)]
    pub async fn acknowledge_primal_pod_expiry(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
        source: [u8; 16],
    ) -> Result<()> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,permit| {
            Box::pin(async move {
                let mut tx=begin_spell_owner_transaction(holder,deadline).await?;
                    let outcome:Result<()>=async {

                let authority=assert_spell_item_scope_with_recovery(&mut tx,permit,&record,&node,scope,generation).await?;
                let RuntimeScopeRefV1::Instance{instance_id,..}=authority.runtime_scope() else{return Err(SpellItemError::Rejected("Pod effect Instance"));};
                sqlx::query("UPDATE game_primal_pod_callbacks SET effect_acknowledged=true WHERE source_transaction_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND instance_id=encode($3,'hex')::uuid AND ownership_generation=$4::text::numeric(20,0) AND completion_kind=2 AND NOT effect_acknowledged")
                    .bind(source.as_slice()).bind(authority.world_bytes().as_slice()).bind(instance_id.as_slice()).bind(authority.generation().to_string())
                    .execute(&mut *tx).await?;
                Ok(())
                    }.await;
                    match outcome {
                        Ok(value)=>{
                            commit_creature_item_owner_transaction(tx,deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error)=>Ok(Err(error)),
                    }
            })
        }).await?
    }
}

/// Actual committed native Player STEP, sealed before SQL. This is a source
/// callback descriptor, not Player cost, inventory, command or current authority.
#[derive(Debug)]
pub struct PrimalPodPlayerStep {
    scope: RuntimeScopeRefV1,
    generation: u64,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    target: SpellGroundTarget,
    source: [u8; 32],
    wall_unix_ms: u64,
}
impl PrimalPodPlayerStep {
    #[allow(clippy::too_many_arguments)]
    pub fn from_current_player_step(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        committed: crate::foundation::MovementPositionSnapshot,
        now_us: u64,
    ) -> Result<Self> {
        runtime
            .owner_fence()
            .map_err(|_| SpellItemError::Rejected("Pod step owner"))?;
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
            || !native.qualifies_primal_callbacks(runtime.content_pin().server_artifact_digest())
            || runtime
                .read_actor_position(actor)
                .map_err(|_| SpellItemError::Rejected("Pod step actor"))?
                != committed
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || actor.runtime_scope() != runtime.binding().runtime_scope()
            || actor.scope_generation() != runtime.binding().scope_generation()
        {
            return Err(SpellItemError::Rejected("Pod exact committed Player step"));
        }
        runtime
            .assert_actor_spell_unreserved(actor)
            .map_err(|_| SpellItemError::Rejected("Pod step reserved native actor"))?;
        runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| SpellItemError::Rejected("Pod step current Player lease"))?;
        let position = committed.position();
        if position.floor != 15
            || !(33547..=33570).contains(&position.x)
            || !(32749..=32769).contains(&position.y)
        {
            return Err(SpellItemError::Rejected("Pod step source bounds"));
        }
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let wall_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| SpellItemError::Rejected("Pod source wall clock"))?
            .as_secs()
            .checked_mul(1_000)
            .ok_or(SpellItemError::Rejected("Pod source wall clock bounds"))?;
        let source = Sha256::new()
            .chain_update(b"oteryn:primal-pod-player-step:v1")
            .chain_update(actor.placement_identity())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .chain_update(session.as_bytes())
            .chain_update(now_us.to_be_bytes())
            .chain_update(wall_unix_ms.to_be_bytes())
            .chain_update(format!("{committed:?}").as_bytes())
            .chain_update(runtime.content_pin().server_artifact_digest())
            .chain_update(runtime.content_pin().map_revision_digest())
            .chain_update(runtime.content_pin().frame_binding_digest())
            .finalize()
            .into();
        Ok(Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            content: native.source_digest(),
            map: runtime.content_pin().map_revision_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            target,
            source,
            wall_unix_ms,
        })
    }
}
#[derive(Debug, Clone)]
pub struct PrimalPodStepReceipt {
    pub source_timer_unix_ms: u64,
    pub additional_source_timers_unix_ms: Vec<u64>,
    pub step_unix_ms: u64,
}
async fn prior_step(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    step: &PrimalPodPlayerStep,
) -> Result<Option<PrimalPodStepReceipt>> {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = authority.runtime_scope() else {
        return Err(SpellItemError::Rejected("Pod step Instance"));
    };
    let prefix = format!(
        "primal-pod-step:{}:",
        step.source
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>()
    );
    let rows=sqlx::query("SELECT p.source_timer_unix_ms FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id JOIN game_primal_pod_callbacks p ON p.source_transaction_id=i.minted_transaction_id WHERE r.cause_kind=2 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.world_id=encode($1,'hex')::uuid AND r.instance_id=encode($2,'hex')::uuid AND r.ownership_generation=$3::text::numeric(20,0) AND r.creature_occurrence LIKE $4||'%' AND r.catalog_digest=$5 AND r.creature_map_digest=$6 AND r.creature_frame_digest=$7 AND l.operation_kind=2 AND l.spatial_position=$8 ORDER BY r.creature_occurrence")
        .bind(authority.world_bytes().as_slice()).bind(instance_id.as_slice()).bind(authority.generation().to_string()).bind(prefix)
        .bind(step.content.as_slice()).bind(step.map.as_slice()).bind(step.frame.as_slice()).bind(&step.target.spatial_position)
        .fetch_all(&mut **tx).await?;
    let mut timers = rows
        .iter()
        .map(|row| {
            u64::try_from(row.try_get::<i64, _>("source_timer_unix_ms")?)
                .map_err(|_| SpellItemError::Rejected("Pod source timer"))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter();
    Ok(timers
        .next()
        .map(|source_timer_unix_ms| PrimalPodStepReceipt {
            source_timer_unix_ms,
            additional_source_timers_unix_ms: timers.collect(),
            step_unix_ms: step.wall_unix_ms,
        }))
}
pub async fn execute_primal_pod_step_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    step: &PrimalPodPlayerStep,
) -> Result<Option<PrimalPodStepReceipt>> {
    check_scope_transaction(tx, authority).await?;
    if authority.runtime_scope() != step.scope || authority.generation() != step.generation {
        return Err(SpellItemError::Rejected("Pod step current scope"));
    }
    let RuntimeScopeRefV1::Instance { instance_id, .. } = authority.runtime_scope() else {
        return Err(SpellItemError::Rejected("Pod Player step Instance"));
    };
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric(20,0) AND g.state=2 AND e.destination_server_artifact_digest=$4 AND e.destination_map_revision_digest=$5 AND e.destination_coordinate_frame_digest=$6)")
        .bind(authority.world_bytes().as_slice()).bind(instance_id.as_slice()).bind(authority.generation().to_string())
        .bind(step.content.as_slice()).bind(step.map.as_slice()).bind(step.frame.as_slice()).fetch_one(&mut **tx).await?;
    if !active {
        return Err(SpellItemError::Rejected(
            "Pod Player step current accepted activity pin",
        ));
    }
    if let Some(prior) = prior_step(tx, authority, step).await? {
        return Ok(Some(prior));
    }
    let tile = read_creature_scope_tile_in_transaction(tx, authority, &step.target).await?;
    let pods = tile
        .items
        .iter()
        .filter(|item| {
            item.definition.family == "Item"
                && item.definition.production_key == POD
                && item.definition.revision_ref == "definition-r1"
        })
        .collect::<Vec<_>>();
    for index in primal_pod_step_item_indices(pods.len(), true) {
        let item = pods[index];
        let source:String=sqlx::query_scalar("SELECT minted_transaction_id::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
            .bind(item.item_instance_id.as_slice()).fetch_one(&mut **tx).await?;
        let mut cause = expired_cause(tx, authority, uuid(&source)?).await?;
        if cause.content != step.content || cause.map != step.map || cause.frame != step.frame {
            return Err(SpellItemError::Rejected("Pod step exact source pin"));
        }
        let mut cast = Sha256::new();
        cast.update(b"oteryn:primal-pod-per-item-step:v1");
        cast.update(step.source);
        cast.update(item.item_instance_id);
        cause.cast = cast.finalize().into();
        cause.occurrence = format!(
            "primal-pod-step:{}:{}",
            step.source
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>(),
            destroy_instance_key(&item.item_instance_id)
        );
        let removal = prepare_creature_scope_removal_in_transaction(
            tx,
            authority,
            &step.target,
            item.item_instance_id,
        )
        .await?;
        let prepared = PreparedCreatureGroundOperations {
            placement: cause.placement,
            generation: cause.generation,
            body: cause.body,
            cast: cause.cast,
            operations: vec![SpellItemOperation::RemoveGround(removal)],
        };
        apply_creature_ground_in_transaction(
            tx,
            authority,
            &cause,
            identity(&cause, b"oteryn:primal-pod-step-transaction:v1"),
            identity(&cause, b"oteryn:primal-pod-step-event:v1"),
            &prepared,
            || Ok(()),
        )
        .await?;
    }
    // The old position callback deliberately remains scheduled. A replacement
    // Pod at that location can be consumed by this older callback, as in Lua.
    prior_step(tx, authority, step).await
}

impl DurabilityRoot {
    pub async fn commit_primal_pod_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        step: &PrimalPodPlayerStep,
    ) -> Result<Option<PrimalPodStepReceipt>> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, step);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<Option<PrimalPodStepReceipt>> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.0,
                            &record,
                            &node,
                            context.1.scope,
                            context.1.generation,
                        )
                        .await?;
                        let receipt =
                            execute_primal_pod_step_in_transaction(&mut tx, &authority, context.1)
                                .await?;
                        Ok(receipt)
                    }
                    .await;
                    match outcome {
                        Ok(value) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// Crystal movement.cpp:385-398 scans live tile indices; removing a top-order3
/// Pod shifts its neighbour behind the next index (tile.cpp:1297-1313). A
/// nonplayer returns nil from Lua, stopping the movement-event loop without removal.
#[must_use]
pub fn primal_pod_step_item_indices(pods: usize, is_player: bool) -> Vec<usize> {
    if !is_player {
        return (pods != 0).then_some(0).into_iter().collect();
    }
    (0..pods).step_by(2).collect()
}
