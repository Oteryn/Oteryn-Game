//! Fixed Rotten Blood access payment composed with the existing fee and Quest owners.
use super::*;
use super::super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts, burn_fee_in_transaction,
};
use super::super::item_mint_audit::Type2Transaction;
use oteryn_gameplay_quests::quest::rotten_blood as rules;

#[derive(Debug, Clone)]
pub struct RottenBloodAccessPayment {
    pub command: CommandRef,
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub coins: FeeChangeFacts,
    pub expected_backpack: Option<[u8; 16]>,
    pub occurred_at_unix_ms: i64,
}

impl RottenBloodAccessPayment {
    pub(super) fn request(&self) -> QuestTransitionRequest {
        QuestTransitionRequest {
            transition_key: rules::PURCHASE_ACCESS.into(),
            cause: QuestCause::Command(self.command),
        }
    }

    pub(super) fn binding(&self, character: CharacterId) -> [u8; 33] {
        let mut binding=request_binding(character,&self.request());
        let digest=Sha256::new().chain_update(binding)
            .chain_update(b"rotten-blood/access-payment/v1")
            .chain_update(self.occurrence).finalize();
        binding[1..].copy_from_slice(&digest);
        binding
    }
}

impl DurabilityRoot {
    /// Historical result read only; the NPC owner must retain the original
    /// payment identity before its first attempt and never re-authorize on resume.
    pub async fn reconcile_rotten_blood_access_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
        payment: RottenBloodAccessPayment,
    ) -> Result<Option<QuestTransitionOutcome>> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let request=payment.request();
            let binding=payment.binding(fence.character_id);
            lock_receipt_key(&mut tx,fence.character_id,&request).await?;
            let Some(row)=load_receipt(&mut tx,fence.character_id,&request).await? else {
                commit_semantic_transaction(tx,deadline).await?;
                return Ok(Ok(None));
            };
            if row.try_get::<Vec<u8>,_>("request_binding")?.as_slice()!=binding {
                return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
            }
            let receipt=decode_receipt(&row)?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(Some(QuestTransitionOutcome::AlreadyCommitted(receipt))))
        })).await?
    }
}

/// Called after the same physical transaction has checked the live gameplay
/// fence and retained Quest key. A subsequent Quest refusal rolls everything back.
pub(super) async fn consume(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    payment: &RottenBloodAccessPayment,
) -> StoredResult<bool> {
    if payment.command.game_session_id()!=fence.game_session_id {
        return Ok(false);
    }
    let stage: i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rules::ACCESS_TRACK).fetch_one(&mut ***tx).await?;
    let state=rules::RottenBloodState::from_stored(
        u8::try_from(stage).map_err(|_|DurabilityError::InvalidStoredState)?,0,0,false,false,None,
    ).map_err(|_|DurabilityError::InvalidStoredState)?;
    // This is the fixed price request, not a claim about the purse. The fee
    // owner below verifies/consumes the actual coin and bank resources.
    let Ok(change)=state.purchase_access(rules::ACCESS_PRICE) else { return Ok(false); };
    let backpack: Option<String>=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    if backpack.as_deref().map(uuid_text).transpose()?!=payment.expected_backpack { return Ok(false); }
    let request=FeeBurnRequest {
        cause:FeeBurnCause::QuestAccessPurchase { occurrence:payment.occurrence },
        fee_gold_units:change.gold_consumed,
        transaction_id:payment.transaction_id,event_id:payment.event_id,
        occurred_at_unix_ms:payment.occurred_at_unix_ms,
        server_build_id:"oteryn-game-server".into(),change:payment.coins.clone(),
    };
    match burn_fee_in_transaction(tx,&fence,&request).await {
        Ok(_) => {}
        Err(FeeBurnError::InsufficientFunds|FeeBurnError::ChangeDoesNotFit|FeeBurnError::CapacityExceeded) => return Ok(false),
        Err(FeeBurnError::Unavailable(error)) => return Err(error),
        Err(_) => return Err(DurabilityError::InvalidStoredState),
    }
    sqlx::query("INSERT INTO game_rotten_blood_access_fees(fee_occurrence,character_id,cause_id,cause_ordinal,transition_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5)")
        .bind(payment.occurrence.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(payment.command.game_session_id().as_bytes().as_slice()).bind(payment.command.command_id().get().to_string())
        .bind(rules::PURCHASE_ACCESS).execute(&mut ***tx).await?;
    Ok(true)
}

/// Keep the generic catalogue's actual successor tied to the same pure source
/// rules used by the resource callback. The generic Quest owner still owns the
/// only track write, receipt, revision and resource transaction.
pub(super) fn validate_access_changes(
    transition:&str, changes:&[QuestTrackChange], level:i64,
) -> StoredResult<bool> {
    if ![rules::START,rules::SACRIFICE,rules::PURCHASE_ACCESS].contains(&transition) {
        return Ok(true);
    }
    let access=changes.iter().find(|change|change.track==rules::ACCESS_TRACK)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let state=rules::RottenBloodState::from_stored(
        u8::try_from(access.before).map_err(|_|DurabilityError::InvalidStoredState)?,0,0,false,false,None,
    ).map_err(|_|DurabilityError::InvalidStoredState)?;
    let after=match transition {
        rules::START=>state.begin_mission(u32::try_from(level).map_err(|_|DurabilityError::InvalidStoredState)?),
        rules::SACRIFICE=>state.sacrifice(1).map(|change|change.state),
        rules::PURCHASE_ACCESS=>state.purchase_access(rules::ACCESS_PRICE).map(|change|change.state),
        _=>return Err(DurabilityError::InvalidStoredState),
    };
    let Ok(after)=after else { return Ok(false); };
    if i64::from(after.access_stage())!=access.after {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(true)
}
