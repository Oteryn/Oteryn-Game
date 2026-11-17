use super::*;
use crate::durability::item_transfer::{ItemStackClass, load_slot};
use oteryn_gameplay_items::starter_grant::{
    NEWHAVEN_STARTER_TEMPLATE_REVISION, NewhavenStarterDestination, newhaven_starter_items,
};

impl DurabilityRoot {
    /// Grant the donor Newhaven kit atomically, under the same current item fence as equipment.
    /// Definitions come from the active native index and are requalified inside this pass.
    pub async fn grant_newhaven_starter_items<C: ItemLoopContent>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content: &C,
        vocation: &str,
        definitions: Vec<crate::domain::item_definition::TypedDefinitionRef>,
    ) -> Result<NewhavenStarterOutcome, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let vocation = vocation.to_owned();
        let Some(kit) = newhaven_starter_items(&vocation) else {
            return Ok(NewhavenStarterOutcome::NotApplicable);
        };
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, content| {
            Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, None, content.loop_content_digest()).await.map_err(grant_error)?;
                let character = fence.character_id;
                let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::Unavailable)?;
                let (current, creation_revision): (String, String) = sqlx::query_as("SELECT coalesce((SELECT vocation FROM game_character_build_state WHERE character_id=r.character_id),'none'),r.starter_template_revision FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if creation_revision != NEWHAVEN_STARTER_TEMPLATE_REVISION { return Ok(NewhavenStarterOutcome::NotApplicable); }
                if current != vocation { return Err(DurabilityError::Unavailable); }
                let granted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_starter_grants WHERE character_id=encode($1,'hex')::uuid AND template_key LIKE 'oteryn:starter.newhaven.%')")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if granted { super::super::db::commit_semantic_transaction(tx,deadline).await?; return Ok(NewhavenStarterOutcome::AlreadyGranted); }
                let Some(backpack) = load_slot(&mut tx, character).await? else { return Ok(NewhavenStarterOutcome::NotAdmitted); };
                let capacity = content.loop_facts(&backpack.definition).and_then(|f| f.container_capacity).map(|n| n.min(20));
                let Some(capacity) = capacity else { return Ok(NewhavenStarterOutcome::NotAdmitted); };
                if kit.len()!=definitions.len() { return Ok(NewhavenStarterOutcome::NotAdmitted); }
                for (item, definition) in kit.iter().zip(&definitions) {
                    let Some(facts) = content.loop_facts(definition) else { return Ok(NewhavenStarterOutcome::NotAdmitted); };
                    let maximum = match facts.stack {
                        ItemStackClass::NonStackable => 1,
                        ItemStackClass::Stackable { proven_maximum } => proven_maximum.unwrap_or(100).min(100),
                        ItemStackClass::Unknown => 0,
                    };
                    if definition.family!="Item" || definition.production_key!=format!("oteryn:item.tibia.i{}",item.tibia_id)
                        || facts.definition!=*definition || facts.weight.is_none() || item.quantity>maximum
                        || (item.tibia_id==35562 && (!content.is_quiver(definition) || facts.container_capacity.is_none_or(|n|n<2))) {
                        return Ok(NewhavenStarterOutcome::NotAdmitted);
                    }
                }
                let occupied: Vec<i16> = sqlx::query_scalar("SELECT slot FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid ORDER BY slot")
                    .bind(character.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
                let (count, last): (i64,String) = sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
                    .bind(backpack.item_instance_id.as_slice()).fetch_one(&mut *tx).await?;
                let mut ordinal: u64 = last.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                let mut placements = Vec::with_capacity(kit.len());
                let mut quiver = None;
                let mut quiver_ordinal = 0_u64;
                let mut needed = 0_u32;
                let mut equipped = false;
                for (item, definition) in kit.into_iter().zip(definitions) {
                    let instance = new_uuid(&mut tx).await?;
                    let transaction = new_uuid(&mut tx).await?;
                    let event = new_uuid(&mut tx).await?;
                    let (destination,slot,parent,entry) = match item.destination {
                        NewhavenStarterDestination::Equipment(slot) if !occupied.contains(&i16::from(slot)) => {
                            equipped=true; (1_i16,Some(i16::from(slot)),None,None)
                        }
                        NewhavenStarterDestination::Quiver => {
                            quiver_ordinal += 1;
                            (3,None,Some(quiver.ok_or(DurabilityError::InvalidStoredState)?),Some(quiver_ordinal))
                        },
                        _ => {
                            needed+=1; ordinal=ordinal.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                            (2,None,Some(backpack.item_instance_id),Some(ordinal))
                        }
                    };
                    if item.tibia_id==35562 { quiver=Some(instance); }
                    placements.push((item,definition,instance,transaction,event,destination,slot,parent,entry));
                }
                if count+ i64::from(needed)>i64::from(capacity) { return Ok(NewhavenStarterOutcome::NotAdmitted); }
                let tree: i64 = sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)").bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if tree+i64::try_from(placements.len()).map_err(|_|DurabilityError::InvalidStoredState)?>500 { return Ok(NewhavenStarterOutcome::NotAdmitted); }
                let revision: String = sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let revision: u64 = revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
                if equipped && revision==u64::MAX { return Err(DurabilityError::InvalidStoredState); }
                let first = placements.first().ok_or(DurabilityError::InvalidStoredState)?.3;
                for (index,(item,definition,instance,transaction,event,destination,slot,parent,entry)) in placements.iter().enumerate() {
                    let envelope=serde_json::to_vec(&json!({"source":"newhaven","vocation":vocation,"character":character.as_bytes(),"world":world.as_bytes(),"template":format!("oteryn:starter.newhaven.{index:02}"),"template_revision":"newhaven-9f5a72c6-r1","starter_template_revision":creation_revision,"definition":[definition.family,definition.production_key,definition.revision_ref],"quantity":item.quantity,"content_digest":content.loop_content_digest(),"item":instance,"destination":destination,"slot":slot,"parent":parent,"ordinal":entry})).map_err(|_|DurabilityError::InvalidStoredState)?;
                    sqlx::query("INSERT INTO game_character_starter_grants(character_id,template_key,template_revision,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,outcome,item_instance_id,transaction_id,event_id,grant_envelope,grant_envelope_sha256,kit_destination,kit_slot,kit_parent,kit_ordinal,kit_equipment_revision_before,kit_creation_revision) VALUES(encode($1,'hex')::uuid,$2,'newhaven-9f5a72c6-r1',encode($3,'hex')::uuid,$4,$5,$6,$7,1,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11,sha256($11),$12,$13,CASE WHEN $14::bytea IS NULL THEN NULL ELSE encode($14,'hex')::uuid END,$15::text::numeric,$16::text::numeric,'newhaven-starter-r1')")
                        .bind(character.as_bytes().as_slice()).bind(format!("oteryn:starter.newhaven.{index:02}"))
                        .bind(world.as_bytes().as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(item.quantity))
                        .bind(instance.as_slice()).bind(transaction.as_slice()).bind(event.as_slice()).bind(envelope).bind(destination).bind(slot)
                        .bind(parent.as_ref().map(|p|p.as_slice())).bind(entry.map(|n|n.to_string())).bind((equipped && index==0).then(||revision.to_string())).execute(&mut *tx).await?;
                    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid)")
                        .bind(instance.as_slice()).bind(world.as_bytes().as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(item.quantity)).bind(transaction.as_slice()).execute(&mut *tx).await?;
                    match destination {
                        1 => { sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid)")
                            .bind(character.as_bytes().as_slice()).bind(slot).bind(instance.as_slice()).bind(world.as_bytes().as_slice()).bind(transaction.as_slice()).execute(&mut *tx).await?; }
                        2 => { sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                            .bind(character.as_bytes().as_slice()).bind(parent.ok_or(DurabilityError::InvalidStoredState)?.as_slice()).bind(instance.as_slice()).bind(world.as_bytes().as_slice()).bind(entry.map(|n|n.to_string())).bind(transaction.as_slice()).execute(&mut *tx).await?; }
                        _ => { sqlx::query("INSERT INTO game_item_nested_entries(parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid)")
                            .bind(parent.ok_or(DurabilityError::InvalidStoredState)?.as_slice()).bind(instance.as_slice()).bind(world.as_bytes().as_slice()).bind(entry.map(|n|n.to_string())).bind(transaction.as_slice()).execute(&mut *tx).await?; }
                    }
                }
                if equipped { sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=encode($2,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character.as_bytes().as_slice()).bind(first.as_slice()).execute(&mut *tx).await?; }
                sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut *tx).await?;
                super::super::db::commit_semantic_transaction(tx,deadline).await?;
                Ok(NewhavenStarterOutcome::Granted)
            })
        }).await
    }
}
