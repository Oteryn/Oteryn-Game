//! M49: retire the exact arriving corpse and persist its summon intent in the drop transaction.
use super::*;
use oteryn_gameplay_quests::quest::elemental_arrival::SummonPlan;

/// The dispatch owner retains its owned map reservation with the spell lane until
/// this writer's outcome is known, including any parked unresolved attempt.
#[derive(Debug, Clone)]
pub struct ElementalArrivalTarget {
    plan: SummonPlan,
}
impl ElementalArrivalTarget {
    pub fn new(plan: SummonPlan) -> Self {
        Self { plan }
    }
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the physical successor shares the ordinary move's complete authority and content fence"
    )]
    pub async fn commit_item_loop_elemental_arrival<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: ElementalArrivalTarget,
    ) -> Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content) = *context;
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            let attempt: LoopResult<LoopMoveReceipt> = async {
                assert_recovery_fence(&mut tx, &record).await?;
                let intent = intent_bytes(fence.character_id, &request)?;
                lock_loop_command(&mut tx, command).await?;
                if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                    return replay.map_err(LoopError::Refused);
                }
                let plan = &target.plan;
                let scope = plan.fence();
                let corpse = plan.corpse();
                let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let LoopDestination::Ground(ground) = &request.destination else { return refuse(LoopMoveRefusal::Rejected); };
                let reset: i64 = scope.reset_epoch.try_into().map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let mut position = i32::from(corpse.position.x).to_be_bytes().to_vec();
                position.extend(i32::from(corpse.position.y).to_be_bytes());
                position.extend(i16::from(corpse.position.floor).to_be_bytes());
                if scope.world != *world.as_bytes() || scope.channel != *channel.as_bytes()
                    || scope.scope_generation != fence.scope_ownership_generation.get()
                    || scope.content != content.loop_content_digest() || corpse.instance != request.item
                    || corpse.quantity != 1 || ground.spatial_position != position
                    || request.count.is_some_and(|count| count != 1) {
                    return refuse(LoopMoveRefusal::Rejected);
                }
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let current: bool = sqlx::query_scalar("SELECT $1='sha256:'||encode($2,'hex') AND game_world_reset_map_epoch_current(encode($3,'hex')::uuid,$4::text::numeric)")
                    .bind(&ground.map_revision).bind(scope.map.as_slice()).bind(world.as_bytes().as_slice()).bind(reset.to_string()).fetch_one(&mut **tx).await?;
                if !current { return refuse(LoopMoveRefusal::Stale); }
                let (definition, quantity, revision) = lock_live_item(&mut tx, request.item, *world.as_bytes()).await?
                    .ok_or(LoopError::Refused(LoopMoveRefusal::Stale))?;
                if definition.family != "Item" || definition.production_key != format!("oteryn:item.tibia.i{}", corpse.item)
                    || definition.revision_ref != "definition-r1" || quantity != 1
                    || revision.checked_add(1) != Some(corpse.state_revision) {
                    return refuse(LoopMoveRefusal::Stale);
                }
                let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_elemental_boss_leases WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND reset_epoch=$3 AND altar_id=$4)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(reset).bind(i32::from(plan.recipe().altar)).fetch_one(&mut **tx).await?;
                let moved = apply_loop_move(&mut tx, permit, &record, &node, &fence, command, content, &request, &intent, None, None, None, None).await?;
                if busy { return Ok(moved); } // Source onAddItem leaves a busy boss's arriving corpse alone.
                let (transaction,event,occurred): (String,String,i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
                let readset: Vec<_> = plan.readset().iter().map(|cell| json!({"key":cell.key,"revision":cell.revision,"position":[cell.position.x,cell.position.y,cell.position.floor],"appearance":cell.appearance})).collect();
                let mut placement = scope.map.to_vec(); placement.extend(plan.target().key.to_be_bytes());
                let tuple = tx.tuple();
                let envelope = crate::durability::item_craft::audit::encode_map_harvest(fence, command,
                    json!({"quest":"oteryn:quest.elemental_spheres_quest","altar":plan.recipe().altar,"corpse":corpse.instance,
                        "state_revision":corpse.state_revision,"quantity_before":1,"quantity_after":0,"content_digest":scope.content,"scope_generation":scope.scope_generation,"readset":readset,"placement":placement,"reset_epoch":reset,
                        "move_transaction":moved.transaction_id,"creature":plan.recipe().creature}),
                    &transaction,&event,occurred,tuple.tuple())?;
                sqlx::query("SELECT game_elemental_arrival_consume(encode($1,'hex')::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9,$10,$11,$12,$13)")
                    .bind(moved.transaction_id.as_slice()).bind(&transaction).bind(&event).bind(corpse.instance.as_slice())
                    .bind(corpse.state_revision.to_string()).bind(i32::from(plan.recipe().altar)).bind(&placement).bind(json!(readset))
                    .bind(reset).bind(occurred).bind(&envelope).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
                    .execute(&mut **tx).await?;
                Ok(moved)
            }.await;
            match attempt {
                Ok(receipt) => { tx.commit(deadline).await?; Ok(Ok(receipt)) },
                Err(LoopError::Database(sqlx::Error::Database(error))) if error.code().is_some_and(|code| code=="23514" || code=="23505") => Ok(Err(LoopMoveRefusal::Rejected)),
                Err(error) => loop_error(error).map(Err),
            }
        })).await
    }
}
