//! Independent committed transfer/epoch observations used to reject old exports.
use super::{
    TestResult,
    fixture::{Harness, id},
};
use crate::native_admission_source::journal::ReadGameJournalCursorV1;
pub(super) async fn change(h: &Harness, s: &ReadGameJournalCursorV1, kind: &str) -> TestResult {
    if kind == "none" {
        return Ok(());
    }
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    if kind == "epoch" {
        sqlx::query("UPDATE game_character_account_projection_epoch SET projection_epoch=projection_epoch+1 WHERE epoch_scope=1").execute(&mut *tx).await?;
    } else {
        let mut receipt = serde_json::json!({"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","command":"TransferCharacterOwnership","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":s.character_id,"to_account_id":s.account_id,"character_revision":s.character_revision,"projection_epoch":s.projection_epoch,"from_projection_revision":s.from_projection_revision,"to_projection_revision":s.to_projection_revision,"recovery_generation":s.recovery_generation});
        if kind == "cut" {
            receipt["character_revision"] = serde_json::Value::String("2".into());
        } else {
            receipt["from_projection_revision"] = serde_json::Value::String("999".into());
        }
        let bytes = serde_json::to_vec(&receipt)?;
        sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id) VALUES('OTERYN_PLATFORM_CHARACTER_AUTHORITY',encode($1,'hex')::uuid,'TransferCharacterOwnership',$2,$3,'CHAR_CMD_OK',$4::text::numeric,1793836800,encode($5,'hex')::uuid,encode($6,'hex')::uuid)")
            .bind(id(211).as_slice()).bind(br#"{"source_revision":"1"}"#.as_slice()).bind(bytes).bind(&s.recovery_generation).bind(id(212).as_slice()).bind(id(213).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_bazaar_character_transitions(operation_id,character_id,old_account,new_account,old_lifecycle,new_lifecycle,old_revision,new_revision,issuer,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,$4::text::uuid,1,1,1,2,'OTERYN_PLATFORM_CHARACTER_AUTHORITY',level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=$2::text::uuid")
            .bind(id(211).as_slice()).bind(&s.character_id).bind(id(210).as_slice()).bind(&s.account_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

use crate::durability::journal::ProducerStore;
use crate::native_admission_source::{
    SourceError,
    journal::{
        PublishGameJournalRecordV1, ReadGameJournalCursorV1Response,
        publisher::{JournalOutbox, StoredPublication},
    },
};
pub(super) struct Losing<'a, 'f, 's> {
    pub store: ProducerStore<'a, 'f, 's>,
    pub h: &'a Harness,
}
impl JournalOutbox for Losing<'_, '_, '_> {
    async fn current(&self, s: &ReadGameJournalCursorV1) -> Result<bool, SourceError> {
        self.store.current(s).await
    }
    async fn after_cursor(
        &self,
        s: &ReadGameJournalCursorV1,
        c: &ReadGameJournalCursorV1Response,
    ) -> Result<Option<StoredPublication>, SourceError> {
        let selected = self.store.after_cursor(s, c).await?;
        assert!(selected.is_some());
        change(self.h, s, "epoch")
            .await
            .map_err(|_| SourceError::Unavailable)?;
        Ok(selected)
    }
    async fn first_for_replay(
        &self,
        s: &ReadGameJournalCursorV1,
    ) -> Result<Option<StoredPublication>, SourceError> {
        self.store.first_for_replay(s).await
    }
    async fn acknowledge(
        &self,
        s: &ReadGameJournalCursorV1,
        r: &PublishGameJournalRecordV1,
    ) -> Result<(), SourceError> {
        self.store.acknowledge(s, r).await
    }
}
