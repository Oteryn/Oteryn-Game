//! Crystal AddItem seed callback joins the existing move, terrain and value-audit owners.
use super::*;
use crate::durability::terrain_tool::{MapMutation, TerrainScope};
use sha2::{Digest, Sha256};

/// The existing fenced read checks this command's actual seed source.
#[derive(Debug, Clone, Copy)]
pub struct SeedDropProbe {
    pub command: CommandRef,
    pub item: [u8; 16],
}

#[derive(Debug, Clone)]
pub struct SeedDropTarget {
    scope: TerrainScope,
    placement: SeedDropPlacement,
}

#[derive(Debug, Clone)]
enum SeedDropPlacement {
    Base {
        placement: Vec<u8>,
        expected_revision: i64,
        ordinary_plant_admitted: bool,
        ice_plant_admitted: bool,
    },
    Ground {
        item: [u8; 16],
        stack_ordinal: i64,
    },
}

impl SeedDropTarget {
    pub fn base(
        scope: TerrainScope,
        placement: Vec<u8>,
        expected_revision: i64,
        ordinary_plant_admitted: bool,
        ice_plant_admitted: bool,
    ) -> Self {
        Self {
            scope,
            placement: SeedDropPlacement::Base {
                placement,
                expected_revision,
                ordinary_plant_admitted,
                ice_plant_admitted,
            },
        }
    }

    pub fn ground(scope: TerrainScope, item: [u8; 16], stack_ordinal: i64) -> Self {
        Self {
            scope,
            placement: SeedDropPlacement::Ground { item, stack_ordinal },
        }
    }
}

struct GroundPot {
    item: [u8; 16],
    before: serde_json::Value,
    location: serde_json::Value,
    placement_context: Vec<u8>,
    first_stage: i32,
}

async fn lock_ground_pot(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    ground: &LoopGround,
    item: [u8; 16],
    stack_ordinal: i64,
) -> LoopResult<GroundPot> {
    let (world, channel) = scope_of(fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
    // Lock the same item row pickup locks before reading custody in a new statement.
    let Some((definition, 1, _)) = lock_live_item(tx, item, *world.as_bytes()).await? else {
        return refuse(LoopMoveRefusal::Stale);
    };
    if definition.family != "Item" || definition.production_key != "oteryn:item.tibia.i306"
        || definition.revision_ref != "definition-r1" {
        return refuse(LoopMoveRefusal::Stale);
    }
    let locked = sqlx::query(
        "SELECT to_jsonb(i) AS before, to_jsonb(g) AS location, g.native_room_placement_context,
                coalesce((SELECT max(source_stage) FROM game_spell_field_temporal_schedules
                          WHERE item_instance_id=i.item_instance_id),0)+1 AS first_stage
           FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
          WHERE i.item_instance_id=encode($1,'hex')::uuid
            AND g.scope_kind=1 AND g.channel_id=encode($2,'hex')::uuid
            AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5
            AND g.runtime_scope_ownership_generation=$6::text::numeric AND g.stack_ordinal=$7
            AND ((g.native_room_placement_context=$8 AND game_item_loop_drop_ref(g.corpse_ref))
                 OR game_item_loop_map_mint_source(i.item_instance_id)
                 OR game_item_action_ground_source(i.item_instance_id))
            AND NOT EXISTS (
                SELECT 1 FROM game_spell_field_temporal_schedules p
                 WHERE p.item_instance_id=i.item_instance_id
                   AND NOT EXISTS (SELECT 1 FROM game_spell_field_temporal_receipts d
                                    WHERE (d.item_instance_id,d.source_stage)=(p.item_instance_id,p.source_stage)))",
    )
    .bind(item.as_slice())
    .bind(channel.as_bytes().as_slice())
    .bind(&ground.spatial_position)
    .bind(&ground.map_revision)
    .bind(&ground.content_revision)
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(stack_ordinal)
    .bind(&ground.placement_context)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(locked) = locked else {
        return refuse(LoopMoveRefusal::Stale);
    };
    let placement_context: Vec<u8> = locked.try_get("native_room_placement_context")?;
    if placement_context != ground.placement_context
        && (placement_context.len() != 32
            || crate::durability::map_item_mint_audit::map_revision_of(&placement_context)
                != ground.map_revision)
    {
        return refuse(LoopMoveRefusal::Stale);
    }
    Ok(GroundPot {
        item,
        before: locked.try_get("before")?,
        location: locked.try_get("location")?,
        placement_context,
        first_stage: locked.try_get("first_stage")?,
    })
}

impl GroundPot {
    async fn transform(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        transaction: &str,
        content: [u8; 32],
        after: u16,
        decay: &[(u32, i64)],
        occurred: i64,
    ) -> LoopResult<()> {
        let key = |id: u32| format!("oteryn:item.tibia.i{id}");
        let mut source = key(u32::from(after));
        let updated = sqlx::query(
            "UPDATE game_item_instances i SET definition_production_key=$2,last_transaction_id=$3::uuid
              WHERE i.item_instance_id=encode($1,'hex')::uuid AND to_jsonb(i)=$4
                AND EXISTS (SELECT 1 FROM game_item_ground_locations g
                             WHERE g.item_instance_id=i.item_instance_id AND to_jsonb(g)=$5)",
        )
        .bind(self.item.as_slice())
        .bind(&source)
        .bind(transaction)
        .bind(&self.before)
        .bind(&self.location)
        .execute(&mut **tx)
        .await?;
        if updated.rows_affected() != 1 {
            return refuse(LoopMoveRefusal::Stale);
        }
        let mut deadline = occurred;
        let mut stage = self.first_stage;
        for &(target, duration) in decay {
            let target = key(target);
            deadline = deadline.checked_add(duration).ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query(
                "INSERT INTO game_spell_field_temporal_schedules(
                    item_instance_id,source_stage,seed_drop_source_transaction_id,source_ordinal,
                    duration_millis,expires_at_unix_ms,definition_family,definition_production_key,
                    definition_revision,target_family,target_production_key,target_revision,
                    content_digest,blocks_movement,blocks_projectile,immovable_block_solid)
                 VALUES(encode($1,'hex')::uuid,$2,$3::uuid,1,$4,$5,'Item',$6,
                        'definition-r1','Item',$7,'definition-r1',$8,false,false,false)",
            )
            .bind(self.item.as_slice()).bind(stage).bind(transaction).bind(duration).bind(deadline)
            .bind(&source).bind(&target).bind(content.as_slice()).execute(&mut **tx).await?;
            source = target;
            stage = stage.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
        }
        Ok(())
    }
}

impl DurabilityRoot {
    /// Read the actual fenced source before reserving the source AddItem cue. Ordinary
    /// drops onto a flower pot must not depend on an unrelated presentation reservation.
    /// The writer below repeats this check together with the full custody/CAS proof.
    pub async fn item_loop_is_seed<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        request: SeedDropProbe,
    ) -> Result<bool, DurabilityError> {
        let SeedDropProbe { command, item } = request;
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, content| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_type2_transaction(holder, deadline).await?;
                    let attempt: LoopResult<bool> = async {
                        assert_recovery_fence(&mut tx, &record).await?;
                        assert_current_equipment_in_transaction(
                            &mut tx,
                            &record,
                            &node,
                            &fence,
                            Some(command),
                            content.loop_content_digest(),
                        )
                        .await?;
                        let (world, _) = scope_of(&fence)
                            .map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                        Ok(lock_live_item(&mut tx, item, *world.as_bytes())
                            .await?
                            .is_some_and(|(definition, _, _)| {
                                matches!(
                                    definition.production_key.as_str(),
                                    "oteryn:item.tibia.i647" | "oteryn:item.tibia.i13844"
                                )
                            }))
                    }
                    .await;
                    match attempt {
                        Ok(seed) => {
                            tx.commit(deadline).await?;
                            Ok(seed)
                        }
                        Err(error) => loop_error(error).map(|_| false),
                    }
                })
            })
            .await
    }

    /// None means the locked source is not a seed; the caller uses the ordinary move.
    /// An exact existing move receipt is returned before current map/custody is inspected.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_item_loop_seed_drop<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: SeedDropTarget,
    ) -> Result<Option<Result<LoopMoveReceipt, LoopMoveRefusal>>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,context|Box::pin(async move{
            let(permit,content)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            let attempt:LoopResult<Option<Result<LoopMoveReceipt,LoopMoveRefusal>>>=async{
                assert_recovery_fence(&mut tx,&record).await?;
                let intent=intent_bytes(fence.character_id,&request)?;
                lock_loop_command(&mut tx,command).await?;
                if let Some(replay)=committed_loop_move(&mut tx,command,fence.character_id,&intent).await?{
                    return Ok(Some(replay));
                }
                assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(command),content.loop_content_digest()).await?;
                let(world,channel)=scope_of(&fence).map_err(|_|LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let Some((definition,_,_))=lock_live_item(&mut tx,request.item,*world.as_bytes()).await?else{return refuse(LoopMoveRefusal::Stale);};
                let after = match definition.production_key.as_str() {
                    "oteryn:item.tibia.i647" => 324_u16,
                    "oteryn:item.tibia.i13844" => 14031,
                    _ => return Ok(None),
                };
                let LoopDestination::Ground(ground)=&request.destination else{return refuse(LoopMoveRefusal::Rejected);};
                if target.scope.content_generation!=content.loop_content_digest()
                    || target.scope.map_revision!=ground.map_revision || target.scope.reset_epoch<0 {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                let decay = oteryn_gameplay_items::event_planting::planting_decays(after);
                if let SeedDropPlacement::Base {
                    placement, expected_revision, ordinary_plant_admitted, ice_plant_admitted,
                } = &target.placement {
                    if placement.len()!=40 || *expected_revision<0 {
                        return refuse(LoopMoveRefusal::Rejected);
                    }
                    if (after==324 && !ordinary_plant_admitted) || (after==14031 && !ice_plant_admitted) {
                        return refuse(LoopMoveRefusal::NotSupported);
                    }
                } else if !std::iter::once(u32::from(after)).chain(decay.iter().map(|&(id, _)| id)).all(|id|
                    content.loop_facts(&TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: format!("oteryn:item.tibia.i{id}"),
                        revision_ref: "definition-r1".into(),
                    }).is_some()
                ) {
                    return refuse(LoopMoveRefusal::NotSupported);
                }
                // Pickup locks the pot before acquiring the Ground owner lock. Use that
                // order here too, before the seed move inserts a Ground entry.
                let pot = match &target.placement {
                    SeedDropPlacement::Ground { item, stack_ordinal } => {
                        Some(lock_ground_pot(&mut tx, &fence, ground, *item, *stack_ordinal).await?)
                    }
                    SeedDropPlacement::Base { .. } => None,
                };
                // A map-minted pot uses its bundle digest as its placement context.
                // Admit the seed against the pot's proven physical Ground context so
                // the same-transaction census counts that root. Replay still compares
                // the original wire-resolved move intent.
                let admitted_request = pot.as_ref()
                    .filter(|pot| pot.placement_context != ground.placement_context)
                    .map(|pot| LoopMoveRequest {
                        destination: LoopDestination::Ground(LoopGround {
                            placement_context: pot.placement_context.clone(),
                            ..ground.clone()
                        }),
                        ..request.clone()
                    });
                // Existing owner proves the exact drop/split, including nested/Ground source
                // custody and every original deferred move constraint, before this consequence.
                let mut receipt=apply_loop_move(&mut tx,permit,&record,&node,&fence,command,content,
                    admitted_request.as_ref().unwrap_or(&request),&intent,None,None,None,None).await?;
                let planted=sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,uuid_send(i.item_instance_id) AS item,i.quantity FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.spatial_position=$4 FOR UPDATE OF i")
                    .bind(receipt.transaction_id.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position)
                    .fetch_one(&mut **tx).await?;
                let before:serde_json::Value=planted.try_get("before_item")?;
                let before_ground:serde_json::Value=planted.try_get("before_ground")?;
                let item:Vec<u8>=planted.try_get("item")?;
                let quantity:i64=planted.try_get("quantity")?;
                let map_intent:[u8;32]=Sha256::digest(&intent).into();
                // The preceding move already ran its full consistency checks.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                let (placement, expected_revision) = match &target.placement {
                    SeedDropPlacement::Base { placement, expected_revision, .. } => {
                        let mutation = MapMutation {
                            placement: placement.clone(), expected_revision: *expected_revision,
                            before: 306, after: u32::from(after), decay: decay.clone(),
                        };
                        if !crate::durability::terrain_tool::apply_map_in_transaction(
                            &mut tx,fence,command,&map_intent,&target.scope,&mutation,
                        ).await? {
                            return refuse(LoopMoveRefusal::Stale);
                        }
                        (Some(placement.as_slice()), *expected_revision)
                    }
                    SeedDropPlacement::Ground { .. } => (None, 0),
                };
                let(transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let tuple=tx.tuple();
                let mut proof = json!({
                    "recipe":"oteryn:map-seed-drop.crystal","before":before,"ground":before_ground,
                    "move_transaction":receipt.transaction_id,"intent":intent,"map_intent":map_intent,
                    "placement":placement,"reset_epoch":target.scope.reset_epoch,"before_item_id":306,"after_item_id":after,
                    "quantity_after":quantity-1,"content_digest":content.loop_content_digest(),
                });
                if let Some(pot) = &pot {
                    proof["pot"] = json!({"item":pot.before,"ground":pot.location,"first_stage":pot.first_stage,"decay":decay});
                }
                let envelope=crate::durability::item_craft::audit::encode_map_harvest(
                    fence,command,proof,&transaction,&event,occurred,tuple.tuple(),
                )?;
                sqlx::query("INSERT INTO game_map_seed_drop_records(transaction_id,event_id,move_transaction_id,game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,before_item,before_ground,intent,placement,reset_epoch,object_revision,after_item_id,map_revision,occurred_at,envelope_sha256,pot_item_instance_id,pot_before,pot_ground,pot_first_stage,pot_content_digest) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15,$16,$17,$18,sha256($19),encode($20,'hex')::uuid,$21,$22,$23,$24)")
                    .bind(&transaction).bind(&event).bind(receipt.transaction_id.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&item).bind(&before).bind(&before_ground)
                    .bind(&intent).bind(placement).bind(target.scope.reset_epoch).bind(expected_revision).bind(i64::from(after)).bind(&target.scope.map_revision)
                    .bind(occurred).bind(&envelope)
                    .bind(pot.as_ref().map(|p| p.item.as_slice()))
                    .bind(pot.as_ref().map(|p| &p.before)).bind(pot.as_ref().map(|p| &p.location))
                    .bind(pot.as_ref().map(|p| p.first_stage))
                    .bind(pot.as_ref().map(|_| target.scope.content_generation.as_slice()))
                    .execute(&mut **tx).await?;
                if let Some(pot) = &pot {
                    pot.transform(&mut tx, &transaction, target.scope.content_generation, after, &decay, occurred).await?;
                    receipt.floor_effect = Some(oteryn_gameplay_items::event_planting::SEED_POFF_EFFECT_ID);
                }
                sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(&item).bind(&transaction).execute(&mut **tx).await?;
                if quantity==1{sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(&item).execute(&mut **tx).await?;}
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(&item).bind(occurred)
                    .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
                Ok(Some(Ok(receipt)))
            }.await;
            match attempt{
                Ok(Some(Ok(receipt)))=>{tx.commit(deadline).await?;Ok(Some(Ok(receipt)))},
                Ok(other)=>Ok(other),Err(error)=>loop_error(error).map(|refusal|Some(Err(refusal))),
            }
        })).await
    }
}
