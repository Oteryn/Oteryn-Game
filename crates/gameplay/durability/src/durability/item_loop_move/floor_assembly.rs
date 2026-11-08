//! An AddItem consequence shares the ordinary move's fence, receipt and physical transaction.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::terrain_tool::TerrainScope;
use oteryn_gameplay_items::floor_assembly::recipe;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct FloorAssemblyTarget {
    pub action: u16,
    pub placement: Vec<u8>,
    pub scope: TerrainScope,
    #[cfg(any(test, feature = "test-support"))]
    test_occurred_at: Option<i64>,
}

impl FloorAssemblyTarget {
    pub fn new(action: u16, placement: Vec<u8>, scope: TerrainScope) -> Self {
        Self {
            action,
            placement,
            scope,
            #[cfg(any(test, feature = "test-support"))]
            test_occurred_at: None,
        }
    }

    /// Seed an original historical occurrence; the actual expiry owner still
    /// uses PostgreSQL's current clock and all production fences/constraints.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_test_occurred_at(mut self, occurred_at: i64) -> Self {
        self.test_occurred_at = Some(occurred_at);
        self
    }

    pub(super) fn source_occurrence(&self, database_now: i64) -> i64 {
        #[cfg(any(test, feature = "test-support"))]
        {
            self.test_occurred_at.unwrap_or(database_now)
        }
        #[cfg(not(any(test, feature = "test-support")))]
        {
            database_now
        }
    }
}

impl DurabilityRoot {
    // This boundary needs the existing move fences plus the actual content and resolved map target.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_item_loop_floor_assembly<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        native: &NativeGameplayState,
        request: LoopMoveRequest,
        target: FloorAssemblyTarget,
    ) -> Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content, native) = *context;
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            let attempt: LoopResult<LoopMoveReceipt> = async {
                assert_recovery_fence(&mut tx, &record).await?;
                let intent = intent_bytes(fence.character_id, &request)?;
                lock_loop_command(&mut tx, command).await?;
                if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                    return replay.map_err(LoopError::Refused);
                }
                let assembly_recipe = recipe(target.action);
                if !oteryn_gameplay_items::floor_assembly::actions().any(|action| action == target.action) { return refuse(LoopMoveRefusal::Rejected); }
                let LoopDestination::Ground(ground) = &request.destination else { return refuse(LoopMoveRefusal::Rejected); };
                if target.scope.content_generation != content.loop_content_digest()
                    || native.source_digest() != content.loop_content_digest()
                    || target.scope.map_revision != ground.map_revision || target.placement.len() != 40
                    || target.scope.reset_epoch < 0 {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let current: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                    .bind(world.as_bytes().as_slice()).bind(target.scope.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
                if !current { return refuse(LoopMoveRefusal::Stale); }
                // Keep the existing Item-before-Ground lock order. Additional floor
                // operands below use NOWAIT, so a concurrent pickup cannot form a cycle.
                let source = lock_live_item(&mut tx, request.item, *world.as_bytes()).await?
                    .ok_or(LoopError::Refused(LoopMoveRefusal::Stale))?;
                let transform = oteryn_gameplay_items::floor_assembly::transform(target.action, &source.0.production_key);
                let eligible = transform.is_some() || assembly_recipe.is_some_and(|recipe| recipe.inputs.iter().any(|(key, _)| *key == source.0.production_key));
                if eligible {
                    // Keep the source Item before the original Ground-owner lock. Never
                    // wait for Ground while holding additional destination Items.
                    let ground_available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    if !ground_available { return refuse(LoopMoveRefusal::Rejected); }
                    // The original Ground census also locks Items. Prelock its bounded
                    // destination set without waiting before its unchanged validation,
                    // so contention cannot exhaust the semantic pass deadline.
                    sqlx::query("SELECT i.item_instance_id FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=$3 AND i.lifecycle=1 ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i NOWAIT")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position)
                        .fetch_all(&mut **tx).await?;
                }
                let mut moved = apply_loop_move(&mut tx, permit, &record, &node, &fence, command, content, &request, &intent, None, None, None, None).await?;
                if let Some(recipe) = transform {
                    if super::floor_transform::apply(&mut tx, permit, &fence, command, native, &moved, &target, ground, recipe).await? {
                        moved.floor_effect = recipe.effect;
                    }
                    return Ok(moved);
                }
                if !eligible { return Ok(moved); }
                let Some(recipe) = assembly_recipe else { return Ok(moved); };
                let mut inputs = Vec::new();
                for (key, count) in recipe.inputs {
                    let row = sqlx::query("SELECT to_jsonb(i) AS item,to_jsonb(g) AS ground FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6 AND g.runtime_scope_ownership_generation=$7::text::numeric AND game_item_loop_drop_ref(g.corpse_ref) AND g.stack_ordinal IS NOT NULL AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$8 AND i.definition_revision_ref='definition-r1' AND i.quantity=$9 ORDER BY g.stack_ordinal,i.item_instance_id LIMIT 1 FOR UPDATE OF i NOWAIT")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position)
                        .bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.placement_context)
                        .bind(fence.scope_ownership_generation.get().to_string()).bind(key).bind(i64::from(*count))
                        .fetch_optional(&mut **tx).await;
                    let row = match row {
                        Ok(row) => row,
                        Err(sqlx::Error::Database(error)) if error.code().is_some_and(|code| code == "55P03") => return refuse(LoopMoveRefusal::Rejected),
                        Err(error) => return Err(LoopError::Database(error)),
                    };
                    let Some(row) = row else { return Ok(moved); };
                    inputs.push(json!({"item": row.try_get::<serde_json::Value, _>("item")?, "ground": row.try_get::<serde_json::Value, _>("ground")?}));
                }
                let output = TypedDefinitionRef { family: "Item".into(), production_key: recipe.output.into(), revision_ref: "definition-r1".into() };
                let policy = native.item_policy(&output.production_key, &output.revision_ref).ok_or(DurabilityError::InvalidStoredState)?;
                let charge_free = policy.record().semantics.charges == crate::content::ReferenceItemField::NotApplicable;
                let qualified = crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if !qualified.ground_destination || qualified.container_capacity.is_some() || qualified.decay.is_some()
                    || qualified.stack_maximum < u32::from(recipe.output_count)
                    || !charge_free {
                    return refuse(LoopMoveRefusal::NotSupported);
                }
                let (transaction, event, output_id, occurred): (String, String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple = tx.tuple();
                let inputs_intent = serde_json::to_vec(&inputs).map_err(|_| DurabilityError::InvalidStoredState)?;
                let inputs_sha256: [u8; 32] = Sha256::digest(&inputs_intent).into();
                let input_balances: Vec<_> = inputs.iter().map(|input| json!({
                    "item": input["item"]["item_instance_id"],
                    "key": input["item"]["definition_production_key"],
                    "revision": input["item"]["definition_revision_ref"],
                    "quantity": input["item"]["quantity"],
                    "state_revision": input["item"]["state_revision"],
                })).collect();
                let proof = json!({
                    "recipe": recipe.key, "inputs": input_balances, "inputs_sha256": inputs_sha256, "output": {"item": output_id,"definition": {"family": output.family, "key": output.production_key, "revision": output.revision_ref},"quantity": recipe.output_count},
                    "intent": intent,"move_transaction": moved.transaction_id,"placement": target.placement,
                    "reset_epoch": target.scope.reset_epoch,"content_digest": native.source_digest(),
                });
                let envelope = crate::durability::item_craft::audit::encode_map_harvest(fence, command, proof,
                    &transaction, &event, occurred, tuple.tuple())?;
                // The ordinary move's original deferred checks have already run. Only the
                // new assembly successor remains deferred until all inputs/output/audit exist.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_floor_assembly_receipts(transaction_id,event_id,move_transaction_id,recipe,inputs,inputs_intent,inputs_sha256,output_item_instance_id,placement,reset_epoch,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,$8::uuid,$9,$10,$11,sha256($12))")
                    .bind(&transaction).bind(&event).bind(moved.transaction_id.as_slice()).bind(recipe.key).bind(json!(inputs)).bind(&inputs_intent).bind(inputs_sha256.as_slice())
                    .bind(&output_id).bind(&target.placement).bind(target.scope.reset_epoch).bind(occurred).bind(&envelope).execute(&mut **tx).await?;
                for input in &inputs {
                    let item = input["item"]["item_instance_id"].as_str().ok_or(DurabilityError::InvalidStoredState)?;
                    sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid")
                        .bind(item).bind(&transaction).execute(&mut **tx).await?;
                    sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid").bind(item).execute(&mut **tx).await?;
                }
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,$5::uuid)")
                    .bind(&output_id).bind(world.as_bytes().as_slice()).bind(recipe.output).bind(i64::from(recipe.output_count)).bind(&transaction).execute(&mut **tx).await?;
                let mut drop_ref = ITEM_LOOP_DROP_REF_PREFIX.to_vec();
                drop_ref.extend_from_slice(&super::super::item_mint::uuid_text(&transaction)?);
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
                    .bind(&output_id).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(&ground.spatial_position).bind(drop_ref).bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.placement_context).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(&output_id).bind(occurred)
                    .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                moved.floor_effect = recipe.effect;
                Ok(moved)
            }.await;
            match attempt {
                Ok(receipt) => { tx.commit(deadline).await?; Ok(Ok(receipt)) },
                Err(LoopError::Database(sqlx::Error::Database(error)))
                    if error.code().is_some_and(|code| code == "55P03") => Ok(Err(LoopMoveRefusal::Rejected)),
                Err(error) => {
                    loop_error(error).map(Err)
                },
            }
        })).await
    }
}
