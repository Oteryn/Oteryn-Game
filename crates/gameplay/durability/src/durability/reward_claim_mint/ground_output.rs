//! Ground outputs participate in the same quest transaction as its existing input BURNs.
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::durability::{
    DurabilityError,
    character_progression::CurrentCharacterGameplayFence,
    item_mint::{GroundPlacement, TypedDefinitionRef},
    item_mint_audit::SelectedType2Tuple,
    quest_state::{QuestCause, QuestTransitionRequest},
    quest_world_callback::QuestWorldToolContext,
    runtime_scope_assignment::NodeIncarnationProof,
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use sqlx::{Postgres, Transaction};

/// No backpack allocation or carried-weight branch: Crystal creates the output on Ground.
#[allow(
    clippy::too_many_arguments,
    reason = "the participant borrows the quest writer's transaction, audit tuple and independently current scope fences"
)]
pub(crate) async fn create(
    tx: &mut Transaction<'_, Postgres>,
    tuple: SelectedType2Tuple,
    permit: &SpellLanePermit,
    scope: (&CharacterRecoveryFenceV1, &NodeIncarnationProof),
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
    native: &NativeGameplayState,
) -> Result<bool, DurabilityError> {
    let (Some(output), QuestCause::Use(command)) = (&callback.effects.ground_output, request.cause)
    else {
        return Ok(false);
    };
    let revision = format!(
        "sha256:{}",
        native
            .source_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if revision != context.content_revision {
        return Ok(false);
    }
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: output.item.clone(),
        revision_ref: "definition-r1".into(),
    };
    let Some(policy) = native.item_policy(&definition.production_key, &definition.revision_ref)
    else {
        return Ok(false);
    };
    let qualified =
        super::super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    // Initially support plain portable roots only; containers, charge state and
    // timed transforms need their existing specialized participants.
    if !qualified.materializable
        || !qualified.movable
        || !qualified.inventory_destination
        || !qualified.ground_destination
        || qualified.container_capacity.is_some()
        || qualified.decay.is_some()
        || qualified.stack_maximum < u32::from(output.count)
        || policy.record().semantics.charges
            != oteryn_gameplay_content::content::ReferenceItemField::NotApplicable
    {
        return Ok(false);
    }
    let (transaction, event, item, occurred): (String,String,String,i64) = sqlx::query_as(
        "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM statement_timestamp())*1000)::bigint"
    ).fetch_one(&mut **tx).await?;
    let item_bytes = super::super::item_mint::uuid_text(&item)?;
    let mut spatial = i32::from(output.position.x).to_be_bytes().to_vec();
    spatial.extend_from_slice(&i32::from(output.position.y).to_be_bytes());
    spatial.extend_from_slice(&i16::from(output.position.floor).to_be_bytes());
    let mut drop_ref = super::super::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX.to_vec();
    drop_ref.extend_from_slice(&super::super::item_mint::uuid_text(&transaction)?);
    let ground = GroundPlacement {
        spatial_position: spatial,
        corpse_ref: drop_ref,
        map_revision: context.map_revision.clone(),
        content_revision: context.content_revision.clone(),
        native_room_placement_context: context.placement_context.clone(),
    };
    match super::super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
        tx,
        permit,
        scope.0,
        scope.1,
        (fence.runtime_scope, fence.scope_ownership_generation.get()),
        &ground,
        &item_bytes,
    )
    .await
    {
        Ok(()) => {}
        Err(super::super::ground_cell_limit::GroundCellAdmissionError::Limit(_)) => {
            return Ok(false);
        }
        Err(error) => match error.into_spell_error() {
            super::super::spell_item_transaction::SpellItemError::Database(error) => {
                return Err(error.into());
            }
            super::super::spell_item_transaction::SpellItemError::Durability(error) => {
                return Err(error);
            }
            _ => return Ok(false),
        },
    }
    let proof = serde_json::json!({"callback": callback,"context":context,"output":output,
        "item":item,"transaction":transaction,"character":fence.character_id.as_bytes(),
        "session":command.game_session_id().as_bytes(),"command":command.command_id().get()});
    let envelope = super::super::item_craft::audit::encode_map_harvest(
        fence.item_fence(),
        command,
        proof,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    let world = fence.runtime_scope.world_id();
    let channel = match fence.runtime_scope {
        crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => channel_id,
        _ => return Ok(false),
    };
    sqlx::query("INSERT INTO game_quest_ground_outputs(transaction_id,event_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,callback,context,world_id,channel_id,scope_generation,spatial_position,corpse_ref,occurred_at,envelope) VALUES($1::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12::text::numeric,$13,$14,$15,$16)")
        .bind(&transaction).bind(&event).bind(&item).bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(&callback.key)
        .bind(serde_json::to_value(callback).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(serde_json::to_value(context).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(&ground.spatial_position).bind(&ground.corpse_ref)
        .bind(occurred).bind(&envelope).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,$5::uuid)")
        .bind(&item).bind(world.as_bytes().as_slice()).bind(&output.item).bind(i64::from(output.count)).bind(&transaction).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
        .bind(&item).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(&ground.spatial_position).bind(&ground.corpse_ref).bind(&ground.map_revision).bind(&ground.content_revision)
        .bind(&ground.native_room_placement_context).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .bind(&item).bind(occurred).bind(super::super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
    Ok(true)
}
