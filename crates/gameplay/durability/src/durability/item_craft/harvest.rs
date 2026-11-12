//! Literal map USE shares terrain CAS, current custody, item audit and achievement owners.
use super::super::terrain_tool::{MapMutation, TerrainScope};
use super::*;
use oteryn_gameplay_items::event_planting as fishing;

#[derive(Debug, Clone)]
pub struct MapHarvestOperand {
    pub scope: TerrainScope,
    pub mutation: MapMutation,
    /// Current server-resolved actor tile; Player:addItem defaults canDropOnMap=true.
    pub actor_position: Vec<u8>,
    pub source_position: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MapHarvestRequest {
    pub command: CommandRef,
    pub intent: Vec<u8>,
    /// None probes exact history before a current handle or map revision is consulted.
    pub operand: Option<MapHarvestOperand>,
}

#[derive(Debug, Clone, Copy)]
pub struct MapHarvestResult {
    pub disposition: UseDisposition,
    pub fresh: bool,
}

struct HarvestOwner<'a> {
    permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    recovery: &'a crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &'a NodeIncarnationProof,
}

fn result(disposition: UseDisposition, fresh: bool) -> MapHarvestResult {
    MapHarvestResult { disposition, fresh }
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn harvest_map_item<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MapHarvestRequest,
        native: &NativeGameplayState,
        content: &C,
    ) -> Result<Option<MapHarvestResult>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(Some(result(UseDisposition::Rejected, false)));
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:terrain-command:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>, bool)> = sqlx::query_as("SELECT intent,uuid_send(character_id),tool_item_instance_id IS NULL FROM game_terrain_tool_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character, no_tool)) = previous {
                let disposition = if intent == request.intent && character == fence.character_id.as_bytes() && no_tool {
                    UseDisposition::Committed
                } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(Some(result(disposition, false)));
            }
            let Some(operand) = &request.operand else { tx.commit(deadline).await?; return Ok(None); };
            if operand.scope.content_generation != context.0.source_digest()
                || assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence,
                    Some(request.command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(result(UseDisposition::Rejected, false)));
            }
            let owner = HarvestOwner { permit: context.2, recovery: &record, node: &node };
            let disposition = write(context.2, &mut tx, fence, &request, operand, context.0, context.1, &owner).await?;
            if disposition == UseDisposition::Committed { tx.commit(deadline).await?; }
            Ok(Some(result(disposition, disposition == UseDisposition::Committed)))
        })).await
    }
}

// Keep the lane proof explicit at the key-33 boundary alongside the harvest inputs.
#[allow(clippy::too_many_arguments)]
async fn write<C: ItemLoopContent>(
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &MapHarvestRequest,
    operand: &MapHarvestOperand,
    native: &NativeGameplayState,
    content: &C,
    owner: &HarvestOwner<'_>,
) -> Result<UseDisposition, DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mutation = &operand.mutation;
    if mutation.placement.len() != 40
        || operand.actor_position.len() != 10
        || operand.source_position.len() != 10
        || mutation.expected_revision < 0
    {
        return Ok(UseDisposition::Rejected);
    }
    let Some(before) = u16::try_from(mutation.before).ok() else {
        return Ok(UseDisposition::Rejected);
    };
    let key = u64::from_be_bytes(
        mutation.placement[32..]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    if let Some((appearance, x, y, z)) = fishing::world_change_item_source(key) {
        let position = (x, y, z as i8);
        // The ID is provenance from the native daily constructor, not appearance/coordinate
        // guessing: a player's ordinary flower at this same tile never enters this branch.
        if appearance != 13842
            || before != 13842
            || !oteryn_gameplay_world_change::daily::THAWING_ICE_FLOWERS.contains(&position)
            || operand.source_position
                != super::super::map_item_mint_audit::map_item_spatial_position(x, y, -(z as i8))
        {
            return Ok(UseDisposition::Rejected);
        }
        let Ok(epoch) = u64::try_from(operand.scope.reset_epoch) else {
            return Ok(UseDisposition::Rejected);
        };
        let Some(day) = super::super::world_event::changes::load(tx, world, epoch).await? else {
            return Ok(UseDisposition::Rejected);
        };
        if !oteryn_gameplay_world_change::owner::is_active(&day, "thawing") {
            return Ok(UseDisposition::Rejected);
        }
    }
    let roll = fishing::map_harvest_roll(
        native.source_digest(),
        *world.as_bytes(),
        *channel.as_bytes(),
        &mutation.placement,
        operand.scope.reset_epoch,
        mutation.expected_revision,
    )
    .ok_or(DurabilityError::InvalidStoredState)?;
    let Some(plan) = fishing::map_harvest(before, roll) else {
        return Ok(UseDisposition::NothingToUse);
    };
    if mutation.after != u32::from(plan.after)
        || mutation.decay != plan.decay.into_iter().collect::<Vec<_>>()
    {
        return Ok(UseDisposition::Rejected);
    }
    let Some((source_id, count)) = plan.yield_item else {
        return Ok(
            if super::super::terrain_tool::apply_map_in_transaction(
                tx,
                fence,
                request.command,
                &request.intent,
                &operand.scope,
                mutation,
            )
            .await?
            {
                UseDisposition::Committed
            } else {
                UseDisposition::StaleState
            },
        );
    };
    let Some(policy) = native
        .item_policy_for_source_id(u32::from(source_id))
        .map(|p| p.record())
    else {
        return Ok(UseDisposition::NotPossible);
    };
    let definition = TypedDefinitionRef {
        family: policy.production_definition.family.clone(),
        production_key: policy.production_definition.production_key.clone(),
        revision_ref: policy.production_definition.revision_ref.clone(),
    };
    let Some(facts) = content.loop_facts(&definition) else {
        return Ok(UseDisposition::NotPossible);
    };
    if !policy.admission.materializable
        || facts.container_capacity.is_some()
        || stack_maximum(facts.stack)
            .ok()
            .is_none_or(|(max, _)| u32::from(count) > max)
    {
        return Ok(UseDisposition::NotPossible);
    }
    let mut parent: Option<Vec<u8>> = None;
    let mut ordinal: Option<String> = None;
    if !plan.source_ground {
        let bag = sqlx::query("SELECT uuid_send(i.item_instance_id) AS id,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.world_id=b.world_id AND i.lifecycle=1 FOR UPDATE OF i")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
        if let Some(bag) = bag {
            let bag_id: Vec<u8> = bag.try_get("id")?;
            let bag_definition = TypedDefinitionRef {
                family: bag.try_get("definition_family")?,
                production_key: bag.try_get("definition_production_key")?,
                revision_ref: bag.try_get("definition_revision_ref")?,
            };
            let capacity = content
                .loop_facts(&bag_definition)
                .and_then(|f| f.container_capacity)
                .unwrap_or(0)
                .min(20);
            let (occupied, highest): (i64, String) = sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
                .bind(&bag_id).fetch_one(&mut ***tx).await?;
            let tree: i64 = sqlx::query_scalar("WITH RECURSIVE t(item_instance_id,depth) AS (SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT e.item_instance_id,t.depth+1 FROM t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8) SELECT count(*) FROM (SELECT 1 FROM t LIMIT 501) b")
                .bind(&bag_id).fetch_one(&mut ***tx).await?;
            let carry =
                match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await {
                    Ok(weight) => weight,
                    Err(_) => return Ok(UseDisposition::NotPossible),
                };
            let maximum =
                read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
            if occupied < i64::from(capacity)
                && tree < 500
                && facts.weight.is_some_and(|weight| {
                    carry
                        .checked_add(u64::from(weight) * u64::from(count))
                        .is_some_and(|weight| weight <= maximum)
                })
            {
                parent = Some(bag_id);
                ordinal = highest
                    .parse::<u64>()
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .map(|value| value.to_string());
                if ordinal.is_none() {
                    return Ok(UseDisposition::Exhausted);
                }
            }
        }
    }
    let ground = if parent.is_none() {
        Some(if plan.source_ground {
            &operand.source_position
        } else {
            &operand.actor_position
        })
    } else {
        None
    };
    let (transaction, event, item, occurred): (String, String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut ***tx).await?;
    if let Some(position) = ground {
        use super::super::ground_cell_limit::{GroundCellAdmissionError, GroundCellLimitError};
        use super::super::spell_item_transaction::SpellItemError;
        let placement = super::super::item_mint::GroundPlacement {
            spatial_position: position.clone(),
            corpse_ref: b"map-harvest-v1".to_vec(),
            map_revision: operand.scope.map_revision.clone(),
            content_revision: format!("sha256:{}", hex(&native.source_digest())),
            native_room_placement_context: mutation.placement[..32].to_vec(),
        };
        // Reuse the actual native census and same-transaction scope locks. No
        // independent cap, optimistic count, reservation DTO or cleanup is added.
        if let Err(error) =
            super::super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
                tx,
                owner.permit,
                owner.recovery,
                owner.node,
                (fence.runtime_scope, fence.scope_ownership_generation.get()),
                &placement,
                &super::super::item_mint::uuid_text(&item)?,
            )
            .await
        {
            return match error {
                GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom) => {
                    Ok(UseDisposition::NoRoom)
                }
                GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput) => {
                    Ok(UseDisposition::NotPossible)
                }
                GroundCellAdmissionError::Source(
                    SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused,
                ) => Ok(UseDisposition::Rejected),
                GroundCellAdmissionError::Source(SpellItemError::Database(error)) => {
                    Err(error.into())
                }
                GroundCellAdmissionError::Source(SpellItemError::Durability(error)) => Err(error),
                GroundCellAdmissionError::Source(SpellItemError::Training(_)) => {
                    Err(DurabilityError::Unavailable)
                }
            };
        }
    }
    if !super::super::terrain_tool::apply_map_in_transaction(
        tx,
        fence,
        request.command,
        &request.intent,
        &operand.scope,
        mutation,
    )
    .await?
    {
        return Ok(UseDisposition::StaleState);
    }
    let output = json!({"item":item,"key":definition.production_key,"revision":definition.revision_ref,"count":count,
        "parent":parent.as_ref().map(|id| hex(id)),"ordinal":ordinal,"position":ground});
    let proof = json!({"recipe":"oteryn:map-harvest.crystal","intent":request.intent,
        "character":fence.character_id.as_bytes(),"placement":mutation.placement,
        "before":mutation.before,"after":mutation.after,"expected_revision":mutation.expected_revision,
        "reset_epoch":operand.scope.reset_epoch,"content_digest":native.source_digest(),
        "scope_generation":fence.scope_ownership_generation.get(),"outputs":[output.clone()]});
    let envelope = audit::encode_map_harvest(
        fence,
        request.command,
        proof,
        &transaction,
        &event,
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_map_harvest_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,scope_generation,placement,reset_epoch,object_revision,before_item_id,after_item_id,output_item_instance_id,output_key,output_revision,output_quantity,parent_item_instance_id,placement_ordinal,ground_position,map_revision,content_revision,intent,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9,$10,$11,$12,$13,$14::uuid,$15,$16,$17,encode($18,'hex')::uuid,$19::text::numeric,$20,$21,$22,$23,$24,sha256($25))")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(&mutation.placement).bind(operand.scope.reset_epoch)
        .bind(mutation.expected_revision).bind(i64::from(before)).bind(i64::from(plan.after)).bind(&item)
        .bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(count)).bind(&parent).bind(&ordinal)
        .bind(ground).bind(&operand.scope.map_revision).bind(format!("sha256:{}", hex(&native.source_digest())))
        .bind(&request.intent).bind(occurred).bind(&envelope).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,$6::uuid)")
        .bind(&item).bind(world.as_bytes().as_slice()).bind(&definition.production_key).bind(&definition.revision_ref)
        .bind(i64::from(count)).bind(&transaction).execute(&mut ***tx).await?;
    if let (Some(parent), Some(ordinal)) = (&parent, &ordinal) {
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::uuid)")
            .bind(&item).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(parent).bind(ordinal).bind(&transaction)
            .execute(&mut ***tx).await?;
    } else if let Some(position) = ground {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
            .bind(&item).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
            .bind(position).bind(b"map-harvest-v1".as_slice()).bind(&operand.scope.map_revision)
            .bind(format!("sha256:{}", hex(&native.source_digest()))).bind(&mutation.placement[..32]).execute(&mut ***tx).await?;
    }
    let tuple = tx.tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(&item)
        .bind(occurred).bind(super::super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    let (achievement, total) = if before == 3699 {
        ("oteryn:achievement/bluebarian", 500)
    } else {
        ("oteryn:achievement/ice_harvester", 10)
    };
    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key=$2)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(achievement).fetch_one(&mut ***tx).await?;
    if before == 3699 || !held {
        super::super::account_achievement::add_progress(
            tx,
            fence.character_id,
            achievement,
            total,
            "map_item_harvest",
            &super::super::item_mint::uuid_text(&event)?,
        )
        .await?;
    }
    Ok(UseDisposition::Committed)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
