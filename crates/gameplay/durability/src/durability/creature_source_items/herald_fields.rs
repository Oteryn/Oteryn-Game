//! Crystal's delayed 25-cell refresh uses the instance's existing Ground Item transaction.
use super::{rotten_portal::hex, *};
use crate::content::bosses::activity_template::herald::{HeraldFireFieldAttack, HeraldRedFloors};
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::native_world_items::{qualify_source_field_chain, source_magic_field};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1,
};

#[derive(Debug)]
pub struct HeraldFieldRefresh {
    cause: CreatureGroundCause,
    field: QualifiedItemDefinition,
    targets: Vec<SpellGroundTarget>,
    due_us: u64,
}

impl HeraldFieldRefresh {
    pub fn from_live_cast(
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        source: HeraldFireFieldAttack,
        actor: ExactActorRef,
        cast_us: u64,
    ) -> Result<Self> {
        let pin = runtime.content_pin();
        let scope = runtime.binding().runtime_scope();
        let fence = runtime
            .owner_fence()
            .map_err(|_| SpellItemError::Rejected("Herald field owner"))?;
        if !matches!(scope, RuntimeScopeRefV1::Instance { .. })
            || !fence.is_current_for_scope(scope, actor.scope_generation())
            || actor.runtime_scope() != scope
            || !runtime
                .matches_live_creature_identity(actor, HeraldFireFieldAttack::CREATURE.as_bytes())
            || !runtime.activity_herald_is_alive()
            || native.source_digest() != pin.server_artifact_digest()
        {
            return Err(SpellItemError::Rejected("Herald source field owner"));
        }
        let rule = CreatureEventRule::HeraldFieldRefresh;
        let item = native
            .item_policy(HeraldFireFieldAttack::ITEM, "definition-r1")
            .ok_or(SpellItemError::Rejected("Herald fire field definition"))?;
        if !source_magic_field(item)? {
            return Err(SpellItemError::Rejected("Herald fire field type"));
        }
        let field = qualify_source_field_chain(
            native,
            QualifiedItemDefinition::from_native_policy(item).map_err(SpellItemError::Rejected)?,
        )?;
        if field.decay_chain.is_empty() {
            return Err(SpellItemError::Rejected("Herald fire field decay chain"));
        }
        let targets = source
            .cells()
            .map(|cell| {
                let position = MovementLocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: i16::try_from(cell.z)
                        .map_err(|_| SpellItemError::Rejected("Herald field floor"))?,
                };
                SpellGroundTarget::from_native_owner(room, runtime, position)
                    .map_err(SpellItemError::Rejected)
            })
            .collect::<Result<_>>()?;
        let cause = herald_cause(runtime, actor, rule, HeraldFireFieldAttack::BODY, cast_us);
        let due_us = cast_us
            .checked_add(HeraldFireFieldAttack::DELAY_US)
            .ok_or(SpellItemError::Rejected("Herald cast clock overflow"))?;
        Ok(Self {
            cause,
            field,
            targets,
            due_us,
        })
    }

    pub const fn due_us(&self) -> u64 {
        self.due_us
    }

    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        authority: &SpellItemScopeAuthority,
    ) -> Result<CreatureGroundReceipt> {
        let cause = &self.cause;
        // Reuse the current activity and immutable room/content pin checks on the scoped owner.
        rotten_portal::accepted(tx, authority, cause).await?;
        if let Some(receipt) =
            reconcile_creature_ground_in_transaction(tx, authority, cause, || Ok(())).await?
        {
            return Ok(receipt);
        }
        let mut operations = Vec::new();
        let transaction = primal_pod::identity(cause, b"oteryn:herald-fields-transaction:v1");
        let event = primal_pod::identity(cause, b"oteryn:herald-fields-event:v1");
        for target in &self.targets {
            let current = read_creature_scope_tile_in_transaction(tx, authority, target).await?;
            for definition in &cause.source_items {
                if let Some(old) = current
                    .items
                    .iter()
                    .find(|item| &item.definition == definition)
                {
                    operations.push(SpellItemOperation::RemoveGround(
                        prepare_creature_scope_removal_in_transaction(
                            tx,
                            authority,
                            target,
                            old.item_instance_id,
                        )
                        .await?,
                    ));
                }
            }
            let id: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut **tx)
                .await?;
            operations.push(SpellItemOperation::MintGround {
                item_instance_id: uuid(&id)?,
                definition: self.field.clone(),
                quantity: 1,
                placement: target.placement(cause.occurrence.as_bytes().to_vec()),
                lifetime_millis: None,
                blocks_movement: false,
                blocks_projectile: false,
                description: None,
            });
        }
        let prepared = PreparedCreatureGroundOperations {
            placement: cause.placement,
            generation: cause.generation,
            body: cause.body,
            cast: cause.cast,
            operations,
        };
        apply_creature_ground_in_transaction(
            tx,
            authority,
            cause,
            transaction,
            event,
            &prepared,
            || Ok(()),
        )
        .await
    }
}

fn herald_cause(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    rule: CreatureEventRule,
    body: [u8; 32],
    ordinal: u64,
) -> CreatureGroundCause {
    let pin = runtime.content_pin();
    let scope = runtime.binding().runtime_scope();
    let creature = TypedDefinitionRef {
        family: "Creature".into(),
        production_key: HeraldFireFieldAttack::CREATURE.into(),
        revision_ref: "definition-r1".into(),
    };
    let cast = Sha256::new()
        .chain_update(body)
        .chain_update(actor.placement_identity())
        .chain_update(ordinal.to_be_bytes())
        .chain_update(actor.actor_local_generation().to_be_bytes())
        .finalize()
        .into();
    CreatureGroundCause {
        world: *scope.world_id().as_bytes(),
        scope,
        scope_generation: actor.scope_generation().get(),
        source_items: HeraldFireFieldAttack::FIRE_ITEMS
            .into_iter()
            .map(|key| TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            })
            .collect(),
        placement: actor.placement_identity(),
        generation: actor.actor_local_generation(),
        ability: creature.clone(),
        creature,
        event_rule: Some(rule),
        occurrence: format!("{}:{ordinal}", rule.key()),
        content: pin.server_artifact_digest(),
        map: pin.map_revision_digest(),
        frame: pin.frame_binding_digest(),
        body,
        cast,
    }
}

#[derive(Debug)]
pub struct HeraldFieldClear {
    cause: CreatureGroundCause,
}

impl HeraldFieldClear {
    pub fn from_committed_step(
        runtime: &ChannelRuntimeV1,
        _source: HeraldRedFloors,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        step: crate::foundation::MovementPositionSnapshot,
    ) -> Result<Self> {
        let at = step.position();
        let cell = crate::content::LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        };
        if !HeraldRedFloors::contains_plate(cell)
            || runtime.read_actor_position(actor).ok() != Some(step)
            || runtime.bound_attacker_lease(actor, session).is_err()
            || !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || !runtime.owner_fence().is_ok_and(|fence| {
                fence.is_current_for_scope(actor.runtime_scope(), actor.scope_generation())
            })
        {
            return Err(SpellItemError::Rejected("Herald red-floor StepIn owner"));
        }
        Ok(Self {
            cause: herald_cause(
                runtime,
                actor,
                CreatureEventRule::HeraldFieldClear,
                HeraldRedFloors::BODY,
                step.revision(),
            ),
        })
    }

    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        authority: &SpellItemScopeAuthority,
    ) -> Result<CreatureGroundReceipt> {
        let cause = &self.cause;
        rotten_portal::accepted(tx, authority, cause).await?;
        if let Some(receipt) =
            reconcile_creature_ground_in_transaction(tx, authority, cause, || Ok(())).await?
        {
            return Ok(receipt);
        }
        // Crystal clears the full arena, including player-created fire away from the attack grid.
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT DISTINCT ON (g.spatial_position,i.definition_production_key)
               i.item_instance_id::text,g.spatial_position
             FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
             WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid
               AND g.scope_kind=2 AND g.runtime_scope_ownership_generation=$3::text::numeric
               AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6
               AND g.spatial_position BETWEEN int4send(32487)||int4send(32649)||int2send(15::smallint)
                                         AND int4send(32504)||int4send(32664)||int2send(15::smallint)
               AND substring(g.spatial_position from 5 for 4) BETWEEN int4send(32649) AND int4send(32664)
               AND substring(g.spatial_position from 9) = int2send(15::smallint)
               AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
               AND i.definition_production_key=ANY($7)
             ORDER BY g.spatial_position,i.definition_production_key,g.stack_ordinal DESC LIMIT 501",
        )
        .bind(cause.world.as_slice())
        .bind(cause.instance_bytes().as_ref().map(|id| id.as_slice()))
        .bind(cause.scope_generation.to_string())
        .bind(format!("sha256:{}", hex(&cause.map)))
        .bind(format!("sha256:{}", hex(&cause.content)))
        .bind(cause.frame.as_slice())
        .bind(HeraldFireFieldAttack::FIRE_ITEMS.as_slice())
        .fetch_all(&mut **tx)
        .await?;
        let mut operations = Vec::with_capacity(rows.len());
        for (id, position) in rows {
            let target = SpellGroundTarget {
                item_profile_source: None,
                spatial_position: position,
                map_revision: format!("sha256:{}", hex(&cause.map)),
                content_revision: format!("sha256:{}", hex(&cause.content)),
                placement_context: cause.frame.to_vec(),
            };
            operations.push(SpellItemOperation::RemoveGround(
                prepare_creature_scope_removal_in_transaction(tx, authority, &target, uuid(&id)?)
                    .await?,
            ));
        }
        let prepared = PreparedCreatureGroundOperations {
            placement: cause.placement,
            generation: cause.generation,
            body: cause.body,
            cast: cause.cast,
            operations,
        };
        apply_creature_ground_in_transaction(
            tx,
            authority,
            cause,
            primal_pod::identity(cause, b"oteryn:herald-red-floors-transaction:v1"),
            primal_pod::identity(cause, b"oteryn:herald-red-floors-event:v1"),
            &prepared,
            || Ok(()),
        )
        .await
    }
}

impl DurabilityRoot {
    pub async fn commit_herald_field_clear(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        clear: &HeraldFieldClear,
    ) -> Result<CreatureGroundReceipt> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, clear);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<CreatureGroundReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.0,
                            &record,
                            &node,
                            context.1.cause.scope,
                            context.1.cause.scope_generation,
                        )
                        .await?;
                        context.1.apply(&mut tx, &authority).await
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

    pub async fn commit_herald_field_refresh(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        refresh: &HeraldFieldRefresh,
        now_us: u64,
    ) -> Result<CreatureGroundReceipt> {
        if now_us < refresh.due_us() {
            return Err(SpellItemError::Rejected("Herald fire field is not due"));
        }
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, refresh);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<CreatureGroundReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.0,
                            &record,
                            &node,
                            context.1.cause.scope,
                            context.1.cause.scope_generation,
                        )
                        .await?;
                        context.1.apply(&mut tx, &authority).await
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
