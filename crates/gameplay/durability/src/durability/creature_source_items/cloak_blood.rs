//! Accepted Q6a HP events compose with the existing scoped physical Item owner.
use super::*;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_entities::carrier::soul_war_native_health_events::CommittedCloakBloodEvent;

const BLOOD: &str = "oteryn:item.tibia.i33854";
#[derive(Debug)]
pub struct CloakBloodCallback {
    event: CommittedCloakBloodEvent,
    cause: CreatureGroundCause,
    target: SpellGroundTarget,
    definition: QualifiedItemDefinition,
    source_present: bool,
}
impl CloakBloodCallback {
    pub fn from_committed_health(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        event: &CommittedCloakBloodEvent,
    ) -> Result<Self> {
        if !event.current_for(runtime)
            || event.creature().key != "oteryn:creature.cloak_of_terror"
            || event.creature().revision != "definition-r1"
            || !native.qualifies_soul_war_callbacks(event.server_digest())
        {
            return Err(SpellItemError::Rejected(
                "Cloak committed native health source",
            ));
        }
        let position = event.position();
        let cell = crate::content::LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        let cells = room.movement_cells();
        let tile = if let Some(world) = room.source_world() {
            std::borrow::Cow::Borrowed(
                world
                    .immutable_tile(world.scope(), cell)
                    .map_err(|_| SpellItemError::Rejected("Cloak source tile"))?,
            )
        } else {
            cells
                .spell_tiles()
                .lookup(cells.scope(), cell)
                .map_err(|_| SpellItemError::Rejected("Cloak source tile"))?
        };
        let source_present =
            tile.ground_source_id() == Some(33854) || tile.top_source_ids().contains(&33854);
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let policy = native
            .item_policy(BLOOD, "definition-r1")
            .ok_or(SpellItemError::Rejected("Cloak blood source Item policy"))?;
        let definition = crate::durability::native_world_items::qualify_source_item_chain(
            native,
            QualifiedItemDefinition::from_native_policy(policy)
                .map_err(SpellItemError::Rejected)?,
        )?;
        let expected = [
            (BLOOD, 600_000),
            ("oteryn:item.tibia.i34006", 300_000),
            ("oteryn:item.tibia.i34007", 300_000),
        ];
        if definition.movable
            || !definition.ground_destination
            || definition.stack_maximum != 1
            || definition.container_capacity.is_some()
            || definition.decay_chain.len() != expected.len()
            || definition
                .decay_chain
                .iter()
                .zip(expected)
                .any(|(stage, (key, duration))| {
                    stage.definition.production_key != key
                        || stage.duration_millis != duration
                        || stage.blocks_movement
                        || stage.blocks_projectile
                })
        {
            return Err(SpellItemError::Rejected(
                "Cloak exact native blood decay chain",
            ));
        }
        let actor = event.actor();
        let cause = CreatureGroundCause {
            world: *actor.world_id().as_bytes(),
            scope: event.runtime_scope(),
            scope_generation: event.scope_generation(),
            source_items: vec![definition.definition.clone()],
            placement: actor.placement_identity(),
            generation: actor.actor_local_generation(),
            creature: TypedDefinitionRef {
                family: "Creature".into(),
                production_key: event.creature().key.clone(),
                revision_ref: event.creature().revision.clone(),
            },
            ability: TypedDefinitionRef {
                family: "Encounter".into(),
                production_key: "oteryn:encounter.soul_war_taint_zones".into(),
                revision_ref: "definition-r1".into(),
            },
            event_rule: None,
            occurrence: format!(
                "cloak-blood:{}",
                super::rotten_portal::hex(event.occurrence())
            ),
            content: event.server_digest(),
            map: event.map_digest(),
            frame: event.frame_digest(),
            body: event.source_body_digest(),
            cast: *event.occurrence(),
        };
        Ok(Self {
            event: event.clone(),
            cause,
            target,
            definition,
            source_present,
        })
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.event.current_for(runtime)
    }
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &CloakBloodCallback,
) -> Result<(CreatureGroundReceipt, bool)> {
    if authority.runtime_scope() != callback.cause.scope
        || authority.generation() != callback.cause.scope_generation
    {
        return Err(SpellItemError::Rejected("Cloak current physical scope"));
    }
    // Resolve an unknown prior COMMIT before interpreting its pool as a new no-op.
    if let Some(receipt) =
        reconcile_creature_ground_in_transaction(tx, authority, &callback.cause, || Ok(())).await?
    {
        let created: bool = sqlx::query_scalar("SELECT operation_count=1 FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(receipt.transaction_id.as_slice()).fetch_one(&mut **tx).await?;
        return Ok((receipt, created));
    }
    // This hash serializes the complete scoped tile key; exact equality remains
    // with the canonical Ground reader and source receipt checks.
    let mut key = b"oteryn:cloak-blood:tile:v1".to_vec();
    key.extend_from_slice(&callback.cause.world);
    key.extend_from_slice(
        &callback
            .cause
            .channel_bytes()
            .or(callback.cause.instance_bytes())
            .ok_or(SpellItemError::Rejected("Cloak scope identity"))?,
    );
    key.extend_from_slice(&callback.cause.scope_generation.to_be_bytes());
    key.extend_from_slice(&callback.target.spatial_position);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),73))")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    let current = read_creature_scope_tile_in_transaction(tx, authority, &callback.target).await?;
    let present = callback.source_present
        || current
            .items
            .iter()
            .any(|item| item.definition.production_key == BLOOD);
    let operations = if present {
        Vec::new()
    } else {
        vec![SpellItemOperation::MintGround {
            item_instance_id: super::primal_pod::identity(
                &callback.cause,
                b"oteryn:cloak-blood:item:v1",
            ),
            definition: callback.definition.clone(),
            quantity: 1,
            placement: callback
                .target
                .placement(callback.cause.occurrence.as_bytes().to_vec()),
            lifetime_millis: None,
            blocks_movement: false,
            blocks_projectile: false,
            description: None,
        }]
    };
    let prepared = PreparedCreatureGroundOperations {
        placement: callback.cause.placement,
        generation: callback.cause.generation,
        body: callback.cause.body,
        cast: callback.cause.cast,
        operations,
    };
    let receipt = apply_creature_ground_in_transaction(
        tx,
        authority,
        &callback.cause,
        super::primal_pod::identity(&callback.cause, b"oteryn:cloak-blood:transaction:v1"),
        super::primal_pod::identity(&callback.cause, b"oteryn:cloak-blood:event:v1"),
        &prepared,
        || Ok(()),
    )
    .await?;
    Ok((receipt, !present))
}

impl DurabilityRoot {
    pub async fn commit_cloak_blood(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &CloakBloodCallback,
    ) -> Result<(CreatureGroundReceipt, bool)> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, callback);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let (permit, callback) = context;
                    let outcome: Result<(CreatureGroundReceipt, bool)> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            permit,
                            &record,
                            &node,
                            callback.cause.scope,
                            callback.cause.scope_generation,
                        )
                        .await?;
                        execute(&mut tx, &authority, callback).await
                    }
                    .await;
                    match outcome {
                        Ok(receipt) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
