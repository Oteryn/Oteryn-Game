//! Crystal's delayed 25-cell refresh uses the instance's existing Ground Item transaction.
use super::*;
use crate::content::bosses::activity_template::herald::HeraldFireFieldAttack;
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
        let item_ref = |key: &str| TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        };
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
        let creature = TypedDefinitionRef {
            family: "Creature".into(),
            production_key: HeraldFireFieldAttack::CREATURE.into(),
            revision_ref: "definition-r1".into(),
        };
        let cast = Sha256::new()
            .chain_update(HeraldFireFieldAttack::BODY)
            .chain_update(actor.placement_identity())
            .chain_update(cast_us.to_be_bytes())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .finalize()
            .into();
        let cause = CreatureGroundCause {
            world: *scope.world_id().as_bytes(),
            scope,
            scope_generation: actor.scope_generation().get(),
            source_items: HeraldFireFieldAttack::FIRE_ITEMS
                .iter()
                .map(|key| item_ref(key))
                .collect(),
            placement: actor.placement_identity(),
            generation: actor.actor_local_generation(),
            ability: creature.clone(),
            creature,
            event_rule: Some(rule),
            occurrence: format!("{}:{cast_us}", rule.key()),
            content: pin.server_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
            body: HeraldFireFieldAttack::BODY,
            cast,
        };
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

impl DurabilityRoot {
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
