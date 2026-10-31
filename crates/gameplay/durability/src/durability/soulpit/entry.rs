use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::map_position::TilePos;
use crate::domain::soulpit::{EntryParticipant, MAX_PARTICIPANTS, entry_slot, validate_entry};
use crate::durability::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use crate::durability::item_craft::soulpit::burn_entry_core;
use crate::durability::item_mint::uuid_text;

pub(super) async fn write(
    tx: &mut Type2Transaction<'_>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &SoulpitEntryRequest,
    native: &NativeGameplayState,
) -> Result<SoulpitEntryWritten, SoulpitWriteError> {
    if !is_uuid_v7(&request.handoff_id)
        || request.command.game_session_id() != fence.game_session_id
        || request.intent.is_empty()
        || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        || request.participants.is_empty() || request.participants.len() > MAX_PARTICIPANTS
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return Err(SoulpitWriteError::Rule(SoulpitError::InvalidParticipants));
    }
    lock_handoff(tx, &request.handoff_id).await?;
    // Shared CommandRef ownership makes an entry/fusion rebind impossible as well as a
    // duplicate entry. Historical success precedes live item and terminal-source checks.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:'||encode($1,'hex')||':'||$2,0))")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .execute(&mut ***tx).await?;
    let previous = sqlx::query("SELECT r.intent,uuid_send(r.character_id) AS character,uuid_send(r.source_item_instance_id) AS source,uuid_send(f.handoff_id) AS handoff,uuid_send(r.transaction_id) AS transaction_id,f.species_key FROM game_item_craft_records r LEFT JOIN game_soulpit_encounters f ON f.source_transaction_id=r.transaction_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .fetch_optional(&mut ***tx).await?;
    if let Some(row) = previous {
        if row.try_get::<Vec<u8>,_>("intent")? != request.intent
            || row.try_get::<Vec<u8>,_>("character")? != fence.character_id.as_bytes()
            || row.try_get::<Vec<u8>,_>("source")? != request.source
            || row.try_get::<Option<Vec<u8>>,_>("handoff")?.as_deref() != Some(request.handoff_id.as_slice()) {
            return Err(SoulpitWriteError::ConflictingOccurrence);
        }
        return Ok(SoulpitEntryWritten { handoff_id: request.handoff_id,
            species: row.try_get("species_key")?,
            source_transaction_id: row.try_get::<Vec<u8>,_>("transaction_id")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
            replayed: true });
    }
    match assert_current_equipment_in_transaction(tx, recovery, node, &fence, Some(request.command), native.source_digest()).await {
        Ok(_) => {}
        Err(EquipmentError::Rejected(_)) => return Err(SoulpitWriteError::AuthorityRejected),
        Err(EquipmentError::Durability(error)) => return Err(error.into()),
        Err(EquipmentError::Database(error)) => return Err(error.into()),
    }
    let group = sqlx::query("SELECT participant_count FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND activity_key=$3 AND state=1")
        .bind(request.handoff_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(SOULPIT_ACTIVITY_KEY).fetch_optional(&mut ***tx).await?
        .ok_or(SoulpitWriteError::AuthorityRejected)?;
    if usize::try_from(group.try_get::<i16,_>("participant_count")?).ok() != Some(request.participants.len()) {
        return Err(SoulpitWriteError::Rule(SoulpitError::InvalidParticipants));
    }
    let participants = sqlx::query("SELECT p.character_id::text,p.source_game_session_id::text,p.entry_x,p.entry_y,p.entry_floor,s.level FROM game_activity_handoff_participants p JOIN game_character_progression_state s USING(character_id) WHERE p.handoff_id=encode($1,'hex')::uuid ORDER BY p.character_id LIMIT 6 FOR SHARE OF s")
        .bind(request.handoff_id.as_slice()).fetch_all(&mut ***tx).await?;
    if participants.len() != request.participants.len() {
        return Err(SoulpitWriteError::Rule(SoulpitError::InvalidParticipants));
    }
    let mut entry_facts = Vec::with_capacity(participants.len());
    let mut members = Vec::with_capacity(participants.len());
    let mut actor_slot = None;
    for participant in participants {
        let character = uuid_text(participant.try_get("character_id")?)?;
        let facts: Vec<_> = request.participants.iter().filter(|fact| fact.character.as_bytes() == &character).collect();
        if facts.len() != 1 { return Err(SoulpitWriteError::Rule(SoulpitError::InvalidParticipants)); }
        let position = TilePos {
            x: u16::try_from(participant.try_get::<i32,_>("entry_x")?).map_err(|_| SoulpitError::NotOnEntryTile)?,
            y: u16::try_from(participant.try_get::<i32,_>("entry_y")?).map_err(|_| SoulpitError::NotOnEntryTile)?,
            floor: i8::try_from(participant.try_get::<i16,_>("entry_floor")?).map_err(|_| SoulpitError::NotOnEntryTile)?,
        };
        let slot = entry_slot(position).ok_or(SoulpitError::NotOnEntryTile)?;
        let level = u32::try_from(participant.try_get::<i64,_>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?;
        entry_facts.push(EntryParticipant { slot, level, game_master: facts[0].game_master });
        if character == *fence.character_id.as_bytes() {
            if uuid_text(participant.try_get("source_game_session_id")?)? != *fence.game_session_id.as_bytes() {
                return Err(SoulpitWriteError::AuthorityRejected);
            }
            actor_slot = Some(slot);
        }
        members.push((character, slot));
    }
    // The shared owner has exclusively prepared this fresh Instance. The unique handoff
    // fight row, retained shared group and source core proof close the admission barrier.
    validate_entry(actor_slot.ok_or(SoulpitError::NotOnEntryTile)?, &entry_facts, false, 1)?;
    let encounter = SoulpitEncounter::new(request.started_at_micros, &entry_facts.iter().map(|fact| fact.slot).collect::<Vec<_>>())?;
    let debit = burn_entry_core(tx, fence, request, native).await?;
    sqlx::query("INSERT INTO game_soulpit_encounters(handoff_id,species_key,source_transaction_id,started_at_micros,revision,checkpoint,terminal) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,0,$5,false)")
        .bind(request.handoff_id.as_slice()).bind(&debit.species).bind(debit.transaction_id.as_slice())
        .bind(i64::try_from(request.started_at_micros).map_err(|_| SoulpitError::InvalidClock)?)
        .bind(checkpoint(&encounter)?).execute(&mut ***tx).await?;
    for (character, slot) in members {
        sqlx::query("INSERT INTO game_soulpit_participants(handoff_id,character_id,staging_slot) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)")
            .bind(request.handoff_id.as_slice()).bind(character.as_slice()).bind(i16::from(slot)).execute(&mut ***tx).await?;
    }
    Ok(SoulpitEntryWritten { handoff_id: request.handoff_id, species: debit.species, source_transaction_id: debit.transaction_id, replayed: false })
}
