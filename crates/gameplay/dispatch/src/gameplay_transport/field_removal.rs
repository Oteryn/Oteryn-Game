//! Retain a completed physical entry while its Ground retirement is uncertain.
use super::ComposedFreshAdmission;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction as storage;
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::durability::spell_owner_commit::SpellLanePermit;
use sha2::{Digest, Sha256};

/// Multi-tools retain machete use alongside their primary terrain action.
fn is_machete(key: &str) -> bool {
    // Crystal 201189f8 data/scripts/actions/tools/{squeezing_gear_of_girlpower,
    // whacking_driller_of_fate}.lua fall through scythe/crowbar to onUseMachete.
    matches!(
        oteryn_gameplay_world::terrain_tools::TerrainTool::from_production_key(key),
        Some(oteryn_gameplay_world::terrain_tools::TerrainTool::Machete)
    ) || is_agent_tool(key)
}

fn is_agent_tool(key: &str) -> bool {
    matches!(
        key,
        "oteryn:item.tibia.i9594" | "oteryn:item.tibia.i9596" | "oteryn:item.tibia.i9598"
    )
}

enum FieldUse {
    Declined,
    OutOfReach,
    Removed,
    Jam,
}

#[derive(Debug)]
pub(super) struct PendingFieldRemoval {
    pub(super) fence: CurrentCharacterItemFence,
    pub(super) content: [u8; 32],
    pub(super) target: SpellGroundTarget,
    pub(super) item: [u8; 16],
    pub(super) tool: Option<[u8; 16]>,
    pub(super) command: Option<u64>,
    pub(super) binding: Option<[u8; 32]>,
}

pub(super) fn park(removal: PendingFieldRemoval) -> Box<dyn std::any::Any + Send> {
    Box::new(
        super::actor_spell::native_combat_cast::UnresolvedSpellCommit::FieldRemoval(Box::new(
            removal,
        )),
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `None` means a different tool owner; `Some(None)` retains an uncertain
    /// outcome without sequencing a rejection over a possible committed removal.
    pub(super) async fn use_machete_impl(
        &self,
        actor: crate::foundation::ExactActorRef,
        command: super::connection::UseCommand,
        intent: &oteryn_protocol_oteryn::world_object::UseIntent,
        source: Option<super::item_view::ItemKey>,
        target: Option<super::item_view::ItemKey>,
    ) -> Option<Option<super::connection::UseOutcome>> {
        use super::item_view::ItemKey;
        use oteryn_protocol_oteryn::world_object::{UseDisposition, UseTarget};
        if intent.use_with_position.is_some()
            || intent.use_with.is_some()
            || intent.use_with_map_item.is_none()
        {
            return None;
        }
        let hotkey = match (&intent.target, source) {
            (UseTarget::ItemByDefinition(definition), _) => Some(*definition),
            (UseTarget::Item(_), Some(ItemKey::Instance(_))) => None,
            _ => return None,
        };
        let fence = command.item_fence?;
        // Tool classification reads the durable inventory too. Serialize it with
        // the pick owner so a competing probe cannot consume its semantic pass.
        let Some(mut permit) = self.spell_lane_permit().await else {
            return Some(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let tool = if let Some(definition) = hotkey {
            use crate::durability::item_mint::TypedDefinitionRef;
            let runtime = self.runtime.lock().await;
            let index = super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
            let (key, revision) = index.definition_at(definition)?;
            if !is_agent_tool(key) {
                return None;
            }
            inventory
                .first_unreserved(
                    &TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.into(),
                        revision_ref: revision.into(),
                    },
                    &[],
                )
                .ok()?
        } else {
            let Some(ItemKey::Instance(tool)) = source else {
                return None;
            };
            tool
        };
        let bag = inventory.backpack.as_ref()?;
        let carried = bag
            .entries
            .iter()
            .chain(inventory.nested.iter().map(|(_, entry)| entry));
        let tool_item = carried
            .into_iter()
            .find(|entry| entry.item.item_instance_id == tool)?;
        if !is_machete(&tool_item.item.definition.production_key) {
            return None;
        }
        let encoded = oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).ok()?;
        let binding: [u8; 32] = Sha256::digest(&encoded).into();
        let refused = || {
            Some(Some(super::connection::UseOutcome {
                disposition: UseDisposition::TargetIllegal,
                ..super::connection::UseOutcome::rejected()
            }))
        };
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return refused();
        };
        // The pinned Item index owns the admitted tool identity. A machete
        // has no spell field recipe; the target still needs that qualified
        // recipe and the writer independently proves current tool custody.
        if native
            .item_index()
            .and_then(|index| {
                index.definition_ref(
                    &tool_item.item.definition.family,
                    &tool_item.item.definition.production_key,
                    &tool_item.item.definition.revision_ref,
                )
            })
            .is_none()
        {
            return refused();
        }
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let field_ready = !states.is_dead(actor)
            && states.spell_idle(&runtime, actor, command.game_session_id)
            && runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none());
        let room = self.rooms().gate_field_step_ingress();
        let Ok(position) = runtime.read_actor_position(actor) else {
            return refused();
        };
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return Some(None);
        };
        let mut context = (
            self,
            &runtime,
            &mut permit,
            false,
            tool_item.item.definition.production_key.as_str(),
        );
        let result = pass.run_with_context(&mut context,|holder,deadline,context|Box::pin(async move {
            let (owner,runtime,permit,committing,key)=context;
            let mut tx=storage::begin_spell_owner_transaction(holder,deadline).await?;
            let replay:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_field_removals WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid AND tool_item_instance_id=encode($4,'hex')::uuid AND intent_binding=$5)").bind(command.game_session_id.as_bytes().as_slice()).bind(command.command_id.to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(tool.as_slice()).bind(binding.as_slice()).fetch_one(&mut *tx).await?;
            if replay {tx.rollback().await?;return Ok(FieldUse::Removed)}
            let field_definition = |key: &str, revision: &str| {
                matches!(key, "oteryn:item.tibia.i2130" | "oteryn:item.tibia.i3635" | "oteryn:item.tibia.i30224")
                    && native.item_policy(key, revision).is_some()
            };
            let (wanted, field) = if let Some(ItemKey::Instance(item))=target {
                let ground:Option<(Vec<u8>, String, String)>=sqlx::query_as("SELECT g.spatial_position,i.definition_production_key,i.definition_revision_ref FROM game_item_ground_locations g JOIN game_item_instances i USING (item_instance_id,world_id) WHERE g.item_instance_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid")
                    .bind(item.as_slice()).bind(runtime.binding().world_id().as_bytes().as_slice()).bind(runtime.binding().channel_id().ok_or(crate::durability::DurabilityError::Unavailable)?.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let (bytes, key, revision) = ground.ok_or(crate::durability::DurabilityError::Unavailable)?;
                let decoded=crate::durability::spell_items_abi::decode_ground_cell(&bytes)
                    .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                (crate::foundation::MovementLocalPosition{x:decoded.x,y:decoded.y,floor:i16::try_from(decoded.z).map_err(|_|crate::durability::DurabilityError::Unavailable)?}, field_definition(&key, &revision))
            } else { return Ok(FieldUse::Declined) };
            let from=position.position();
            if !field {
                return Ok(FieldUse::Declined);
            }
            // Room readiness belongs to field removal. A landed corpse or
            // another Ground item must still reach its own interaction owner.
            let room = room.filter(|_| field_ready).ok_or(crate::durability::DurabilityError::Unavailable)?;
            if from.floor != wanted.floor || from.x.abs_diff(wanted.x)>1 || from.y.abs_diff(wanted.y)>1 {
                return Ok(FieldUse::OutOfReach);
            }
            let address=SpellGroundTarget::for_native_tile_read(room,runtime,wanted).map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let read=storage::read_standing_player_tile_in_transaction(&mut tx,permit,owner.root,owner.character,owner.holder,&fence,native.source_digest(),&address).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let item=if let Some(ItemKey::Instance(item))=target {
                read.items().iter().find(|row|row.item_instance_id==item)
            } else { read.items().first() };
            let Some(item)=item.filter(|row| field_definition(&row.definition.production_key, &row.definition.revision_ref)) else { return Ok(FieldUse::Declined) };
            if super::item_craft::rope::agent_tool_jams(command, key, tool) {
                tx.rollback().await?;
                return Ok(FieldUse::Jam);
            }
            let item=item.item_instance_id;
            crate::durability::field_removal::remove_field_in_transaction(&mut tx,owner.root,&read,item,Some(tool),Some(command.command_id),Some(binding)).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let pending=PendingFieldRemoval{fence,content:native.source_digest(),target:address,item,tool:Some(tool),command:Some(command.command_id),binding:Some(binding)};
            let mut window=permit.open_commit_window(pending,park);
            *committing=true;
            crate::durability::pvp_live::commit_live_pvp_transaction(tx,deadline,&mut window).await?;
            window.install();
            owner.states.loop_ground.invalidate();
            Ok(FieldUse::Removed)
        })).await;
        match result {
            Ok(FieldUse::Jam) => {
                drop((states, runtime));
                Some(
                    self.commit_agent_jam(&permit, command, tool, encoded)
                        .await
                        .ok(),
                )
            }
            Ok(FieldUse::Removed) => Some(Some(super::connection::UseOutcome {
                disposition: UseDisposition::Committed,
                ..super::connection::UseOutcome::rejected()
            })),
            // Non-field targets retain their skinning, harvest or cask owner.
            Ok(FieldUse::Declined) if is_agent_tool(&tool_item.item.definition.production_key) => {
                None
            }
            Ok(FieldUse::Declined | FieldUse::OutOfReach) => refused(),
            Err(_) => {
                if context.3 {
                    Some(None)
                } else {
                    refused()
                }
            }
        }
    }

    pub(super) async fn resolve_field_removal(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingFieldRemoval,
    ) {
        let mut window = permit.open_commit_window(pending, park);
        window.mark_already_committed();
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return;
        };
        let mut context = (self, &mut window);
        let result = pass.run_with_context(&mut context, |holder, deadline, context| Box::pin(async move {
            let (owner, window) = context;
            let mut tx = storage::begin_spell_owner_transaction(holder, deadline).await?;
            let pending = window.attempt();
            // The Channel lock serializes against the original unknown COMMIT.
            let read = storage::read_standing_player_tile_in_transaction(&mut tx, window.permit(), owner.root, owner.character, owner.holder, &pending.fence, pending.content, &pending.target).await
                .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let done: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_field_removals WHERE item_instance_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND tool_item_instance_id IS NOT DISTINCT FROM CASE WHEN $3::bytea IS NULL THEN NULL ELSE encode($3,'hex')::uuid END AND command_id IS NOT DISTINCT FROM $4::text::numeric AND intent_binding IS NOT DISTINCT FROM $5)")
                .bind(pending.item.as_slice()).bind(pending.fence.game_session_id.as_bytes().as_slice()).bind(pending.tool.as_ref().map(|id|id.as_slice())).bind(pending.command.map(|id|id.to_string())).bind(pending.binding.as_ref().map(|value|value.as_slice())).fetch_one(&mut *tx).await?;
            if !done {
                crate::durability::field_removal::remove_field_in_transaction(&mut tx, owner.root, &read, pending.item, pending.tool, pending.command, pending.binding).await
                    .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            }
            crate::durability::pvp_live::commit_live_pvp_transaction(tx, deadline, window).await
        })).await;
        if result.is_ok() {
            window.install();
            self.states.loop_ground.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_machete;

    #[test]
    fn global_machetes_and_secret_agent_tools_share_field_removal() {
        for id in [3308, 3330, 9594, 9596, 9598] {
            assert!(is_machete(&format!("oteryn:item.tibia.i{id}")), "{id}");
        }
        assert!(!is_machete("oteryn:item.tibia.i5908"));
        assert!(!is_machete("oteryn:item.tibia.i3457"));
    }

    #[test]
    fn multi_tools_keep_their_target_specific_terrain_actions() -> Result<(), &'static str> {
        use oteryn_gameplay_world::terrain_tools::{crystal_core_bindings, dispatch};

        let catalogue = crystal_core_bindings();
        for (id, target, action, transformed, yield_item) in [
            (9596, 3653, None, 3651, Some(3605)),
            (9596, 5464, None, 5463, Some(5466)),
            (9598, 4848, Some(40041), 3134, None),
        ] {
            let key = format!("oteryn:item.tibia.i{id}");
            assert!(is_machete(&key));
            let plan = dispatch(&catalogue, &key, target, action, 0).ok_or("terrain action")?;
            assert_eq!(plan.binding.transformed, transformed);
            assert_eq!(plan.binding.yield_item, yield_item);
            for field in [2130, 3635, 30224] {
                assert!(dispatch(&catalogue, &key, field, None, 0).is_none());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) mod faults {
    use std::cell::RefCell;
    use tokio::sync::oneshot;
    pub(in crate::gameplay_transport) const REMOVAL: u8 = 0;
    pub(in crate::gameplay_transport) const PLAYER: u8 = 1;
    thread_local! {static ARMED:RefCell<Option<(u8,bool,oneshot::Sender<()>)>>=const{RefCell::new(None)};}
    pub(in crate::gameplay_transport) fn arm(family: u8, after: bool) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        ARMED.with_borrow_mut(|slot| {
            assert!(slot.is_none());
            *slot = Some((family, after, sender));
        });
        receiver
    }
    pub(in crate::gameplay_transport) fn take(family: u8, after: bool) -> bool {
        ARMED.with_borrow_mut(|slot| {
            if let Some((_, _, sender)) =
                slot.take_if(|(kind, point, _)| *kind == family && *point == after)
            {
                let _ = sender.send(());
                true
            } else {
                false
            }
        })
    }
}
