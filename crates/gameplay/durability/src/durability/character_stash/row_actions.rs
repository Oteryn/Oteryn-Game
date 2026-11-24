//! Resolve row versions in the stash owner before using the quick-loot settings writer.
use super::*;
use std::num::NonZeroU32;

pub struct StashRowRequest {
    pub command: CommandRef,
    pub intent: StashRowIntent,
    pub row_open: bool,
    pub locker_reachable: bool,
    pub steward: Option<(crate::foundation::ChannelId, [u8; 16])>,
}

impl DurabilityRoot {
    pub async fn stash_row_action(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: StashRowRequest,
        native: &NativeGameplayState,
    ) -> Result<Option<NonZeroU32>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let mut binding = b"oteryn.stash.row.v1".to_vec();
        binding.extend(encode_stash_row_intent(request.intent)
            .map_err(|_| DurabilityError::InvalidStoredState)?);
        let mut context = (permit, node, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,context| Box::pin(async move {
            let (permit, node, native) = *context;
            let mut tx=super::super::db::begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            let changes_list=request.intent.action.changes_loot_list();
            if changes_list {
                super::super::item_loop_move::lock_loop_command(&mut tx,request.command).await?;
            }
            let world = fence.runtime_scope.world_id();
            let character = fence.character_id.as_bytes().as_slice();
            if let Some((channel, id))=request.steward {
                permit.check_channel(world,channel)?;
                super::super::db::lock_admission_relations(&mut tx).await?;
                if !super::super::house_hireling::fence::assert_house_inventory_fence(&mut tx,node,&fence,*channel.as_bytes()).await?
                    || !super::super::house_hireling::food::service_in_transaction(&mut tx,&fence,id,Some(1003)).await? {
                    return Ok(None);
                }
            } else {
                permit.check_scope(fence.runtime_scope)?;
                assert_current_equipment_in_transaction(&mut tx,&record,node,&fence,Some(request.command),native.source_digest())
                    .await.map_err(|_| DurabilityError::Unavailable)?;
            }

            let command_id=request.command.command_id().get().to_string();
            let session=fence.game_session_id.as_bytes().as_slice();
            if changes_list {
                let previous=sqlx::query("SELECT uuid_send(character_id) AS character,request,result
                  FROM game_quick_loot_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(session).bind(&command_id).fetch_optional(&mut *tx).await?;
                if let Some(previous)=previous {
                    if previous.try_get::<Vec<u8>,_>("character")?!=character || previous.try_get::<Vec<u8>,_>("request")?!=binding {
                        return Ok(None);
                    }
                    let result=decode_stash_row_result(&previous.try_get::<Vec<u8>,_>("result")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    super::super::db::commit_semantic_transaction(tx,deadline).await?;
                    return Ok(Some(result));
                }
            }
            if !request.row_open || (!request.locker_reachable && request.steward.is_none()) {
                return Ok(None);
            }
            let row=sqlx::query("SELECT definition_key,definition_revision FROM game_character_stash_balances
              WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
                AND last_entry_id=encode($3,'hex')::uuid AND quantity>0")
                .bind(character).bind(world.as_bytes().as_slice()).bind(request.intent.row.as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row)=row else {return Ok(None);};
            let definition=definition(&row)?;
            let Some(reference)=native.item_index().and_then(|index| index.definition_ref(
                &definition.family,&definition.production_key,&definition.revision_ref,
            )) else {return Ok(None);};
            if request.intent.action == StashRowAction::Market && !stowable(native,&definition) {
                return Ok(None);
            }
            if changes_list {
                let settings=super::super::character_quick_loot::load(&mut tx,fence.character_id.as_bytes()).await?;
                let add = request.intent.action==StashRowAction::LootAdd;
                if !super::super::character_quick_loot::edit_filter_in_transaction(
                    &mut tx,fence.character_id,settings,&definition.production_key,add,
                ).await? {
                    return Ok(None);
                }
                sqlx::query("INSERT INTO game_quick_loot_receipts(game_session_id,command_id,character_id,request,result)
                  VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5)")
                    .bind(session).bind(command_id).bind(character).bind(binding)
                    .bind(encode_stash_row_result(reference)).execute(&mut *tx).await?;
            }
            super::super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(reference))
        })).await
    }
}
