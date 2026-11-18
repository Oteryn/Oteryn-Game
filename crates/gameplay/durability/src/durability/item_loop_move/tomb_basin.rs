//! Crystal 201189f8: coin retirement and the flame occupant's arrival share the drop transaction.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};

pub const TOMB_BASIN_COIN: &str = "oteryn:item.tibia.i3042";

#[derive(Clone, Copy, Debug)]
pub struct TombBasinRule {
    pub uid: u16,
    /// Legacy movement coordinates.
    pub flame: (i32, i32, i16),
    pub destination: (i32, i32, i16),
}

pub const TOMB_BASINS: [TombBasinRule; 8] = [
    TombBasinRule { uid: 9033, flame: (33097, 32816, 13), destination: (33093, 32824, 13) },
    TombBasinRule { uid: 9034, flame: (33293, 32742, 13), destination: (33299, 32742, 13) },
    TombBasinRule { uid: 9035, flame: (33073, 32590, 13), destination: (33080, 32588, 13) },
    TombBasinRule { uid: 9036, flame: (33240, 32856, 13), destination: (33246, 32850, 13) },
    TombBasinRule { uid: 9037, flame: (33276, 32553, 14), destination: (33271, 32553, 14) },
    TombBasinRule { uid: 9038, flame: (33234, 32692, 13), destination: (33234, 32687, 13) },
    TombBasinRule { uid: 9039, flame: (33135, 32683, 12), destination: (33130, 32683, 12) },
    TombBasinRule { uid: 9040, flame: (33162, 32831, 10), destination: (33158, 32832, 10) },
];

pub struct TombBasinTarget {
    pub uid: u16,
    pub placement: Vec<u8>,
    pub revision: u64,
    pub reset_epoch: u64,
    /// The flame was empty, had a qualified player, or had a player whose landing was refused.
    pub occupant: Result<Option<CurrentCharacterGameplayFence>, LoopMoveRefusal>,
}

pub struct TombBasinCommit {
    pub receipt: LoopMoveReceipt,
    pub sacrifice: bool,
    pub arrival: Option<PendingArrival>,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "the item-loop writer retains the full source and arrival fences")]
    pub async fn commit_item_loop_tomb_basin<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: TombBasinTarget,
        catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    ) -> Result<Option<Result<TombBasinCommit, LoopMoveRefusal>>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content, catalogue);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content, catalogue) = *context;
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            let attempt: LoopResult<Option<TombBasinCommit>> = async {
                assert_recovery_fence(&mut tx, &record).await?;
                let intent = intent_bytes(fence.character_id, &request)?;
                lock_loop_command(&mut tx, command).await?;
                if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                    return Ok(Some(TombBasinCommit { receipt: replay.map_err(LoopError::Refused)?, sacrifice: false, arrival: None }));
                }
                let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let LoopDestination::Ground(ground) = &request.destination else { return refuse(LoopMoveRefusal::Rejected); };
                let rule = TOMB_BASINS.iter().find(|rule| rule.uid == target.uid)
                    .ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let revision: i64 = target.revision.try_into().map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let epoch: i64 = target.reset_epoch.try_into().map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                if target.placement.len() != 40 {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                // Hold the two Character roots in their UUID order before either writer acquires one.
                if let Ok(Some(occupant)) = target.occupant {
                    if occupant.runtime_scope != fence.runtime_scope
                        || occupant.scope_ownership_generation != fence.scope_ownership_generation {
                        return refuse(LoopMoveRefusal::Stale);
                    }
                    crate::durability::db::lock_admission_relations(&mut tx).await?;
                    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY character_id FOR UPDATE")
                        .bind(fence.character_id.as_bytes().as_slice()).bind(occupant.character_id.as_bytes().as_slice())
                        .fetch_all(&mut **tx).await?;
                }
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let Some((definition, _, _)) = lock_live_item(&mut tx, request.item, *world.as_bytes()).await? else {
                    return refuse(LoopMoveRefusal::Stale);
                };
                if definition.production_key != TOMB_BASIN_COIN {
                    return Ok(None);
                }
                if definition.family != "Item" || definition.revision_ref != "definition-r1" {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let occupant = target.occupant.map_err(LoopError::Refused)?;
                let recipient = if let Some(occupant) = occupant {
                    let Ok(root) = crate::durability::character_progression::assert_gameplay_fence(&mut tx, &occupant, &node).await? else {
                        return refuse(LoopMoveRefusal::Stale);
                    };
                    let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                        .bind(occupant.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    if busy {
                        return refuse(LoopMoveRefusal::Stale);
                    }
                    Some((occupant.character_id, root))
                } else {
                    None
                };
                let receipt = apply_loop_move(&mut tx, permit, &record, &node, &fence, command, content, &request, &intent, None, None, None, None).await?;
                let row = sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,uuid_send(i.item_instance_id) AS item FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.spatial_position=$4 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                    .bind(&ground.spatial_position).fetch_one(&mut **tx).await?;
                let item: Vec<u8> = row.try_get("item")?;
                let before: serde_json::Value = row.try_get("before_item")?;
                let before_ground: serde_json::Value = row.try_get("before_ground")?;
                let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple = tx.tuple();
                let occupant_id = occupant.as_ref().map(|f| f.character_id.as_bytes().as_slice());
                let occupant_session = occupant.as_ref().map(|f| f.game_session_id.as_bytes().as_slice());
                let envelope = crate::durability::item_craft::audit::encode_map_harvest(fence, command, json!({
                    "basin_uid": rule.uid, "before": before, "ground": before_ground, "outputs": [],
                    "move_transaction": receipt.transaction_id, "intent": intent, "placement": target.placement,
                    "object_revision": revision, "reset_epoch": epoch, "content_digest": content.loop_content_digest(),
                    "occupant": occupant_id,
                }), &transaction, &event, occurred, tuple.tuple())?;
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_tomb_basin_offerings(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,basin_uid,occupant_id,occupant_session_id,map_revision,content_digest,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15,$16,encode($17,'hex')::uuid,encode($18,'hex')::uuid,$19,$20,$21,sha256($22))")
                    .bind(&transaction).bind(&event).bind(receipt.transaction_id.as_slice()).bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                    .bind(channel.as_bytes().as_slice()).bind(&item).bind(&before).bind(&before_ground).bind(&intent)
                    .bind(&target.placement).bind(epoch).bind(revision).bind(i32::from(rule.uid)).bind(occupant_id).bind(occupant_session)
                    .bind(&ground.map_revision).bind(content.loop_content_digest().as_slice()).bind(occurred).bind(&envelope).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(&item).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(&item).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
                    .bind(&item).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS)
                    .bind(&envelope).execute(&mut **tx).await?;
                let arrival = if let (Some(occupant), Some((character, root))) = (occupant_id, recipient) {
                    let binding = sha2::Sha256::digest(&envelope);
                    super::tomb_basin_progress::record(&mut tx, catalogue, character, &root, command, rule.uid, &binding).await?;
                    let (x, y, floor) = rule.destination;
                    sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,$4,$5,$6)")
                        .bind(occupant).bind(&transaction).bind(world.as_bytes().as_slice()).bind(x).bind(y).bind(floor).execute(&mut **tx).await?;
                    Some(PendingArrival {
                        occurrence: TravelOccurrence::from_bytes(uuid_text(&transaction)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                        world_id: world,
                        destination: crate::durability::character_last_position::LastPositionCell { x, y, floor },
                    })
                } else {
                    None
                };
                Ok(Some(TombBasinCommit { receipt, sacrifice: true, arrival }))
            }.await;
            match attempt {
                Ok(Some(commit)) => {
                    tx.commit(deadline).await?;
                    Ok(Some(Ok(commit)))
                }
                Ok(None) => Ok(None),
                Err(LoopError::Database(sqlx::Error::Database(error))) if error.code().is_some_and(|code| code == "23514" || code == "23505") => Ok(Some(Err(LoopMoveRefusal::Rejected))),
                Err(error) => loop_error(error).map(|refusal| Some(Err(refusal))),
            }
        })).await
    }
}

impl DurabilityRoot {
    /// A lost acknowledgement only reoffers the exact still-pending arrival from the committed drop.
    pub async fn read_tomb_basin_arrival(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        payer: CharacterId,
        command: CommandRef,
    ) -> Result<Option<(u16, crate::foundation::GameSessionId, PendingArrival)>, DurabilityError> {
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let row = sqlx::query("SELECT r.basin_uid,r.occupant_session_id::text AS session,r.transaction_id::text AS occurrence,r.world_id::text AS world,a.tile_x,a.tile_y,a.tile_floor FROM game_tomb_basin_offerings r JOIN game_character_pending_arrivals a ON a.character_id=r.occupant_id AND a.occurrence_id=r.transaction_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric AND r.character_id=encode($3,'hex')::uuid")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(payer.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| {
                Ok::<_, DurabilityError>((
                    u16::try_from(row.try_get::<i32, _>("basin_uid")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    crate::foundation::GameSessionId::decode(&uuid_text(&row.try_get::<String, _>("session")?)?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    PendingArrival {
                        occurrence: TravelOccurrence::from_bytes(uuid_text(&row.try_get::<String, _>("occurrence")?)?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        world_id: WorldId::decode(&uuid_text(&row.try_get::<String, _>("world")?)?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        destination: crate::durability::character_last_position::LastPositionCell {
                            x: row.try_get("tile_x")?, y: row.try_get("tile_y")?, floor: row.try_get("tile_floor")?,
                        },
                    },
                ))
            }).transpose()?;
            crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}
