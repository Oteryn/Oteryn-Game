//! A seal funding receipt commits only with its real solo encounter and fresh session.
use super::*;
use crate::durability::item_craft::soulpit::EntryDebit;
use crate::durability::item_mint::uuid_text;
pub(super) async fn replay(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SoulpitEntryRequest,
) -> Result<Option<SoulpitEntryWritten>, SoulpitWriteError> {
    let row=sqlx::query("SELECT handoff_id::text,transaction_id::text,character_id::text,intent,species_key FROM game_soulseal_entries WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let species: String = row.try_get("species_key")?;
    if uuid_text(row.try_get("character_id")?)? != *fence.character_id.as_bytes()
        || uuid_text(row.try_get("handoff_id")?)? != request.handoff_id
        || row.try_get::<Vec<u8>, _>("intent")? != request.intent
        || !matches!(&request.funding,SoulpitFunding::Soulseals {species:expected} if *expected==species)
    {
        return Err(SoulpitWriteError::ConflictingOccurrence);
    }
    Ok(Some(SoulpitEntryWritten {
        handoff_id: request.handoff_id,
        species,
        source_transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        replayed: true,
    }))
}
pub(super) async fn debit(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SoulpitEntryRequest,
    native: &NativeGameplayState,
    species: &str,
) -> Result<EntryDebit, SoulpitWriteError> {
    let fight = crate::durability::task_board::soulseal_fight(native, species)?;
    let remaining = crate::durability::task_board::spend_soulseals_in_transaction(
        tx,
        fence,
        request.command,
        &fight,
        &request.intent,
    )
    .await?;
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO game_soulseal_entries(transaction_id,handoff_id,character_id,world_id,game_session_id,command_id,intent,species_key,stars,cost,balance_before,balance_after) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10,$11,$12)")
        .bind(&transaction).bind(request.handoff_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.intent).bind(species)
        .bind(i16::from(fight.stars)).bind(i16::try_from(fight.cost).map_err(|_|DurabilityError::InvalidStoredState)?)
        .bind(i64::try_from(remaining+u64::from(fight.cost)).map_err(|_|DurabilityError::InvalidStoredState)?).bind(i64::try_from(remaining).map_err(|_|DurabilityError::InvalidStoredState)?)
        .execute(&mut ***tx).await?;
    Ok(EntryDebit {
        transaction_id: uuid_text(&transaction)?,
        species: species.into(),
    })
}
#[derive(Debug, Clone)]
pub struct SoulsealReceipt {
    pub balance: u64,
    pub intent: Vec<u8>,
}
impl DurabilityRoot {
    /// A retained command result; deliberately not a new live authorization to enter.
    pub async fn read_soulseal_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
    ) -> Result<Option<SoulsealReceipt>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let row=sqlx::query("SELECT s.balance_after,s.intent FROM game_soulseal_entries s JOIN game_activity_handoff_groups g USING(handoff_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.game_session_id=encode($2,'hex')::uuid AND s.command_id=$3::text::numeric AND g.state=2")
                .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let value=row.map(|r|Ok::<_,DurabilityError>(SoulsealReceipt {balance:u64::try_from(r.try_get::<i64,_>("balance_after")?).map_err(|_|DurabilityError::InvalidStoredState)?,intent:r.try_get("intent")?})).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(value)
        })).await
    }
}
impl DurabilityRoot {
    /// Current Game-owned quest balance and level, without starting/resetting a weekly board.
    pub async fn read_current_soulseals(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<(u64, u32), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !matches!(crate::durability::character_progression::assert_gameplay_fence(&mut tx,&fence,&node).await,Ok(Ok(_))) {return Err(DurabilityError::Unavailable);}
            let row=sqlx::query("SELECT p.level,coalesce(t.value,0) AS seals FROM game_character_progression_state p LEFT JOIN game_character_quest_tracks t ON t.character_id=p.character_id AND t.track_key=$2 WHERE p.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(crate::durability::task_board::SOULSEALS).fetch_one(&mut *tx).await?;
            let value=(u64::try_from(row.try_get::<i64,_>("seals")?).map_err(|_|DurabilityError::InvalidStoredState)?,u32::try_from(row.try_get::<i64,_>("level")?).map_err(|_|DurabilityError::InvalidStoredState)?);
            commit_semantic_transaction(tx,deadline).await?;Ok(value)
        })).await
    }
}
