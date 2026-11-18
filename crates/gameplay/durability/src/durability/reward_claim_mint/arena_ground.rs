use super::*;
use crate::durability::ground_cell_limit::check_ground_cell_root_insert_with_recovery;
use crate::durability::item_mint::GroundPlacement;
use crate::durability::item_mint_audit::OneItemGroundV1;
use crate::durability::spell_owner_commit::SpellLanePermit;

/// A server-resolved top-level trophy tile in the currently pinned bundle.
pub struct ArenaTrophyStep {
    pub command: CommandRef,
    pub uid: u16,
    pub x: u16,
    pub y: u16,
    pub floor: i8,
    pub map_revision: String,
    pub content_revision: String,
    pub frame: [u8; 32],
    pub item: TypedDefinitionRef,
}

impl DurabilityRoot {
    pub async fn claim_arena_trophy_on_step(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        gameplay: CurrentCharacterGameplayFence,
        step: ArenaTrophyStep,
    ) -> std::result::Result<bool, DurabilityError> {
        let rank = match step.uid {
            3264 => 1,
            3265 => 2,
            3266 => 3,
            _ => return Ok(false),
        };
        let Some(north) = step.y.checked_sub(1) else {
            return Ok(false);
        };
        let expected = arena::donor::reward(rank, 4, 1).ok_or(DurabilityError::InvalidStoredState)?;
        if step.command.game_session_id() != gameplay.game_session_id
            || step.item.family != "Item"
            || step.item.production_key != format!("oteryn:item.tibia.i{}", expected.root)
        {
            return Ok(false);
        }
        let fence = gameplay.item_fence();
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if crate::durability::character_progression::assert_gameplay_fence(&mut tx, &gameplay, &node).await?.is_err() {
                return Ok(false);
            }
            let state = arena::load_state(&mut tx, gameplay).await?;
            let masks = arena::claimed_masks(&mut tx, gameplay).await?;
            if state.completed < rank || masks[usize::from(rank - 1)] & 8 != 0 {
                return Ok(false);
            }

            let name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let inscription = trophy_inscription(rank, &name)?;
            let (transaction, event, item, occurred): (String, String, String, i64) = sqlx::query_as(
                "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,
                   floor(extract(epoch FROM clock_timestamp())*1000)::bigint"
            ).fetch_one(&mut **tx).await?;
            let placement = GroundPlacement {
                spatial_position: crate::durability::map_item_mint_audit::map_item_spatial_position(step.x, north, step.floor),
                corpse_ref: b"arena-trophy-v1".to_vec(),
                map_revision: step.map_revision,
                content_revision: step.content_revision,
                native_room_placement_context: step.frame.to_vec(),
            };
            match check_ground_cell_root_insert_with_recovery(&mut tx, permit, &recovery, &node,
                (fence.runtime_scope, fence.scope_ownership_generation.get()), &placement, &uuid_text(&item)?).await {
                Ok(()) => {}
                Err(crate::durability::ground_cell_limit::GroundCellAdmissionError::Limit(_)) => return Ok(false),
                Err(crate::durability::ground_cell_limit::GroundCellAdmissionError::Source(error)) => {
                    use crate::durability::spell_item_transaction::SpellItemError;
                    return match error {
                        SpellItemError::Durability(error) => Err(error),
                        SpellItemError::Database(error) => Err(error.into()),
                        _ => Ok(false),
                    };
                }
            }
            let world_bytes = world.as_bytes().as_slice();
            let channel_bytes = channel.as_bytes().as_slice();
            let generation = fence.scope_ownership_generation.get();
            let claim = TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: arena::donor::claim_key(rank, 4),
                revision_ref: placement.content_revision.clone(),
            };
            let tuple = tx.tuple();
            let envelope = audit::encode_reward_claim_mint_event(
                RewardClaimMintEventIdentity {
                    event_id: uuid_text(&event)?,
                    transaction_id: uuid_text(&transaction)?,
                    occurred_at_unix_ms: occurred,
                    channel_id: *channel.as_bytes(),
                    server_build_id: SERVER_BUILD_ID,
                },
                OneItemRewardClaimMintV1 {
                    after: Some(OneItemStateV1 {
                        item_instance_id: uuid_text(&item)?.to_vec(),
                        world_id: world_bytes.to_vec(),
                        definition: Some(definition_message(&step.item)),
                        quantity: 1,
                        lifecycle: ITEM_LIFECYCLE_LIVE,
                        key_number: 0,
                    }),
                    destination: None,
                    source: Some(OneItemRewardClaimCauseV1 {
                        typed_cause: REWARD_CLAIM_MINT_TYPED_CAUSE.into(),
                        content_revision: placement.content_revision.clone(),
                        ruleset_revision: "oteryn:svargrond-arena-r1".into(),
                        sim_revision: "oteryn:svargrond-arena-r1".into(),
                        command_ref: Some(OneItemCommandRefV1 {
                            game_session_id: step.command.game_session_id().as_bytes().to_vec(),
                            command_id: step.command.command_id().get(),
                        }),
                        reward_claim: Some(definition_message(&claim)),
                        boss_reward: None,
                    }),
                    before_semantically_absent: true,
                    inbox: None,
                    ground: Some(OneItemGroundV1 {
                        world_id: world_bytes.to_vec(),
                        channel_id: channel_bytes.to_vec(),
                        spatial_position: placement.spatial_position.clone(),
                        corpse_ref: placement.corpse_ref.clone(),
                        map_revision: placement.map_revision.clone(),
                        content_revision: placement.content_revision.clone(),
                        native_room_placement_context: placement.native_room_placement_context.clone(),
                        runtime_scope_ownership_generation: generation,
                        instance_id: None,
                    }),
                    ground_connection_generation: fence.connection_generation.get(),
                },
                tuple.tuple(),
            ).map_err(|_| DurabilityError::InvalidStoredState)?;

            sqlx::query("INSERT INTO game_arena_trophy_mints(transaction_id,event_id,item_instance_id,character_id,world_id,channel_id,scope_generation,rank,claim_revision,item_revision,ground_position,map_revision,content_revision,placement_context,inscription,occurred_at,envelope_sha256)
                VALUES($1::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8,$9,$10,$11,$12,$13,$14,$15,$16,sha256($17))")
                .bind(&transaction).bind(&event).bind(&item).bind(fence.character_id.as_bytes().as_slice()).bind(world_bytes)
                .bind(channel_bytes).bind(generation.to_string()).bind(i16::from(rank)).bind(&claim.revision_ref)
                .bind(&step.item.revision_ref).bind(&placement.spatial_position).bind(&placement.map_revision)
                .bind(&placement.content_revision).bind(&placement.native_room_placement_context).bind(&inscription)
                .bind(occurred).bind(&envelope).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,$5::uuid)")
                .bind(&item).bind(world_bytes).bind(&step.item.production_key).bind(&step.item.revision_ref).bind(&transaction)
                .execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
                VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
                .bind(&item).bind(world_bytes).bind(channel_bytes).bind(generation.to_string())
                .bind(&placement.spatial_position).bind(&placement.corpse_ref).bind(&placement.map_revision)
                .bind(&placement.content_revision).bind(&placement.native_room_placement_context).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state)
                VALUES($1::uuid,$2::uuid,1,1,$3,$4,$5,$6::uuid,$7,$7+$8,$9,sha256($9),1)")
                .bind(&event).bind(&transaction).bind(EVENT_TYPE_ID).bind(i64::from(tuple.schema_revision()))
                .bind(tuple.retention_profile_id()).bind(&item).bind(occurred).bind(mint_audit::AUDIT_RETENTION_P90D_MS)
                .bind(&envelope).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_reward_claims(character_id,claim_family,claim_production_key,claim_revision_ref,next_allowed_at,claimed_transaction_id,claimed_at)
                VALUES(encode($1,'hex')::uuid,'RewardClaim',$2,$3,NULL,$4::uuid,$5)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&claim.production_key).bind(&claim.revision_ref)
                .bind(&transaction).bind(occurred).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(true)
        })).await
    }
}
