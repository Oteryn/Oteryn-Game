//! Atomic Newhaven kits extend STARTER-1 and the existing equipment/custody owners.
use super::{
    CurrentCharacterItemFence, DurabilityError, DurabilityRoot, ItemLoopContent,
    NodeIncarnationProof, ReconciledCharacterAuthority, grant_error, new_uuid,
};
use crate::domain::item_definition::TypedDefinitionRef;
use crate::durability::character_equipment::{
    assert_current_equipment_in_transaction, read_equipment_in_transaction,
};
use crate::durability::item_loop_move::carried_weight;
use crate::durability::item_transfer::{
    load_entries, load_slot, read_derived_transfer_capacity_centi_oz, scope_of,
};
use oteryn_gameplay_items::starter_grant::{
    StarterDestination, StarterItem, newhaven_starter_items,
};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewhavenStarterOutcome {
    Granted,
    AlreadyGranted,
    NoRoom,
    NotAdmitted,
}
fn definition(id: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    }
}
struct Line {
    item: [u8; 16],
    source: StarterItem,
    parent: Option<[u8; 16]>,
    ordinal: Option<u64>,
    destination: i16,
}

impl DurabilityRoot {
    pub async fn grant_newhaven_starter_items<C: ItemLoopContent>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content: &C,
        vocation: &str,
    ) -> std::result::Result<NewhavenStarterOutcome, DurabilityError> {
        let Some(kit) = newhaven_starter_items(vocation) else {
            return Ok(NewhavenStarterOutcome::NotAdmitted);
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let vocation = vocation.to_owned();
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::Unavailable)?;
        let digest = content.loop_content_digest();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,content| {
            let kit=&kit;let record=&record;let node=&node;let vocation=&vocation;
            Box::pin(async move {
                use NewhavenStarterOutcome as O;
                let content=*content;
                let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
                let authority=assert_current_equipment_in_transaction(&mut tx,record,node,&fence,None,digest).await.map_err(grant_error)?;
                let character=fence.character_id;
                // The same Character root lock serializes all inventory writers and retry callers.
                let granted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_starter_kit_receipts WHERE character_id=encode($1,'hex')::uuid)").bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if granted { return Ok(O::AlreadyGranted); }
                let eligible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid AND vocation=$2)").bind(character.as_bytes().as_slice()).bind(vocation).fetch_one(&mut *tx).await?;
                if !eligible { return Ok(O::NotAdmitted); }
                let equipment=read_equipment_in_transaction(&mut tx,&authority).await.map_err(grant_error)?;
                // An occupied requested slot is retryable: never displace, partially mint or drop.
                if kit.iter().any(|row| matches!(row.destination,StarterDestination::Equipment(slot) if equipment.items.iter().any(|i|i.slot==slot))) { return Ok(O::NoRoom); }
                let slot=load_slot(&mut tx,character).await?;
                let backpack=if let Some(slot)=&slot { slot.item_instance_id } else { new_uuid(&mut tx).await? };
                let backpack_definition=slot.as_ref().map_or_else(||definition(2854),|s|s.definition.clone());
                let Some(backpack_facts)=content.loop_facts(&backpack_definition) else {return Ok(O::NotAdmitted);};
                let Some(capacity)=backpack_facts.container_capacity else {return Ok(O::NotAdmitted);};
                let entries=if slot.is_some(){load_entries(&mut tx,backpack).await?}else{Vec::new()};
                let backpack_rows=kit.iter().filter(|r|r.destination==StarterDestination::Backpack).count();
                if entries.len().checked_add(backpack_rows).is_none_or(|n|n>usize::try_from(capacity.min(20)).unwrap_or(0)) {return Ok(O::NoRoom);}
                let tree_size:i64=sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)").bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if tree_size + i64::try_from(backpack_rows).map_err(|_|DurabilityError::InvalidStoredState)? > 500 { return Ok(O::NoRoom); }
                let mut incoming=0_u64;
                for row in kit.iter().copied().chain(slot.is_none().then_some(StarterItem{tibia_id:2854,quantity:1,destination:StarterDestination::Backpack})) {
                    let def=definition(row.tibia_id);
                    let Some(facts)=content.loop_facts(&def) else{return Ok(O::NotAdmitted);};
                    let maximum=crate::durability::item_transfer::stack_maximum(facts.stack).map_err(|_|DurabilityError::InvalidStoredState)?.0;
                    if row.quantity>maximum {return Ok(O::NotAdmitted);}
                    if row.tibia_id==35562 && (facts.container_capacity.is_none_or(|n|n<2)||!content.is_quiver(&def)){return Ok(O::NotAdmitted);}
                    let Ok(weight)=content.weight_of(&def) else{return Ok(O::NotAdmitted);};
                    if facts.weight!=Some(weight){return Ok(O::NotAdmitted);}
                    incoming=incoming.checked_add(u64::from(weight)*u64::from(row.quantity)).ok_or(DurabilityError::InvalidStoredState)?;
                }
                let carried=carried_weight(&mut tx,character,*world.as_bytes(),content).await.map_err(|_|DurabilityError::Unavailable)?;
                let allowed=read_derived_transfer_capacity_centi_oz(&mut tx,character).await?;
                if carried.checked_add(incoming).is_none_or(|n|n>allowed){return Ok(O::NoRoom);}
                let quiver=if kit.iter().any(|r|r.tibia_id==35562){Some(new_uuid(&mut tx).await?)}else{None};
                let mut ordinal=entries.iter().map(|e|e.placement_ordinal).max().unwrap_or(0);
                let mut quiver_ordinal=0;
                let mut lines=Vec::new();
                for source in kit {
                    let item=if source.tibia_id==35562{quiver.ok_or(DurabilityError::InvalidStoredState)?}else{new_uuid(&mut tx).await?};
                    let (parent,next,destination)=match source.destination {
                        StarterDestination::Equipment(_)=>(None,None,1),
                        StarterDestination::Backpack=>{ordinal=ordinal.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;(Some(backpack),Some(ordinal),2)},
                        StarterDestination::Quiver=>{quiver_ordinal+=1;(quiver,Some(quiver_ordinal),3)},
                    };
                    lines.push(Line{item,source:*source,parent,ordinal:next,destination});
                }
                if slot.is_none(){lines.push(Line{item:backpack,source:StarterItem{tibia_id:2854,quantity:1,destination:StarterDestination::Backpack},parent:None,ordinal:None,destination:4});}
                let transaction=new_uuid(&mut tx).await?;let event=new_uuid(&mut tx).await?;
                let envelope=serde_json::to_vec(&json!({"schema":"OTERYN_NEWHAVEN_STARTER/v1","vocation":vocation,"items":lines.iter().map(|l|json!({"item":l.item,"definition":l.source.tibia_id,"quantity":l.source.quantity,"destination":l.destination,"parent":l.parent,"ordinal":l.ordinal})).collect::<Vec<_>>()})).map_err(|_|DurabilityError::InvalidStoredState)?;
                let after=equipment.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                // Synthetic command identity is server-generated once inside the atomic kit;
                // the Character/template receipt, not this command, is the idempotency key.
                let command=u64::from_be_bytes(transaction[8..16].try_into().map_err(|_|DurabilityError::InvalidStoredState)?).max(1);
                sqlx::query("INSERT INTO game_character_equipment_receipts(transaction_id,event_id,character_id,game_session_id,command_id,connection_generation,lease_generation,world_id,channel_id,scope_generation,content_digest,revision_before,revision_after,operation,binding,intent,mode_before,mode_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,$12::text::numeric,$13::text::numeric,3,sha256($14),$14,$15,$15)")
                    .bind(transaction.as_slice()).bind(event.as_slice()).bind(character.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).bind(digest.as_slice()).bind(equipment.revision.to_string()).bind(after.to_string()).bind(envelope.as_slice()).bind(equipment.combat_mode.map(|m|match m{crate::durability::character_equipment::CombatMode::Attack=>1_i16,crate::durability::character_equipment::CombatMode::Balanced=>2,crate::durability::character_equipment::CombatMode::Defense=>3})).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_starter_kit_receipts(character_id,template_key,transaction_id,content_digest,line_count) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5)").bind(character.as_bytes().as_slice()).bind(vocation).bind(transaction.as_slice()).bind(digest.as_slice()).bind(i16::try_from(lines.len()).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut *tx).await?;
                for line in &lines {
                    let slot=match line.source.destination{StarterDestination::Equipment(s)=>Some(i16::from(s)),_=>None};
                    sqlx::query("INSERT INTO game_character_starter_kit_lines(item_instance_id,character_id,transaction_id,definition_production_key,quantity,destination,slot,parent_item_instance_id,placement_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,CASE WHEN $8::bytea IS NULL THEN NULL ELSE encode($8,'hex')::uuid END,$9::text::numeric)")
                        .bind(line.item.as_slice()).bind(character.as_bytes().as_slice()).bind(transaction.as_slice()).bind(format!("oteryn:item.tibia.i{}",line.source.tibia_id)).bind(i64::from(line.source.quantity)).bind(line.destination).bind(slot).bind(line.parent.map(|p|p.to_vec())).bind(line.ordinal.map(|n|n.to_string())).execute(&mut *tx).await?;
                }
                for line in &lines {
                    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
                        .bind(line.item.as_slice()).bind(world.as_bytes().as_slice()).bind(format!("oteryn:item.tibia.i{}",line.source.tibia_id)).bind(i64::from(line.source.quantity)).bind(transaction.as_slice()).execute(&mut *tx).await?;
                }
                for line in &lines {
                    match line.destination {
                        1=>{let StarterDestination::Equipment(slot)=line.source.destination else{return Err(DurabilityError::InvalidStoredState);};sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid)").bind(character.as_bytes().as_slice()).bind(i16::from(slot)).bind(line.item.as_slice()).bind(world.as_bytes().as_slice()).bind(transaction.as_slice()).execute(&mut *tx).await?;},
                        2=>{sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)").bind(character.as_bytes().as_slice()).bind(line.parent.map(|p|p.to_vec())).bind(line.item.as_slice()).bind(world.as_bytes().as_slice()).bind(line.ordinal.map(|n|n.to_string())).bind(transaction.as_slice()).execute(&mut *tx).await?;},
                        3=>{sqlx::query("INSERT INTO game_item_nested_entries(parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid)").bind(line.parent.map(|p|p.to_vec())).bind(line.item.as_slice()).bind(world.as_bytes().as_slice()).bind(line.ordinal.map(|n|n.to_string())).bind(transaction.as_slice()).execute(&mut *tx).await?;},
                        4=>{sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(character.as_bytes().as_slice()).bind(line.item.as_slice()).bind(world.as_bytes().as_slice()).bind(transaction.as_slice()).execute(&mut *tx).await?;},
                        _=>return Err(DurabilityError::InvalidStoredState),
                    }
                }
                sqlx::query("UPDATE game_character_equipment_state SET revision=$1::text::numeric,last_transaction_id=encode($2,'hex')::uuid WHERE character_id=encode($3,'hex')::uuid").bind(after.to_string()).bind(transaction.as_slice()).bind(character.as_bytes().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_audit_outbox(event_id,transaction_id,envelope) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)").bind(event.as_slice()).bind(transaction.as_slice()).bind(envelope.as_slice()).execute(&mut *tx).await?;
                sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut *tx).await?;
                crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
                Ok(O::Granted)
            })
        }).await
    }
}
