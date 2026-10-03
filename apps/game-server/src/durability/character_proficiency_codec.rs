//! Inert retained-value reads. No content admission, effects or controller authority.
use super::{
    DurableProficiencyState, ProficiencyCause, ProficiencyLineCandidate, ProficiencyOccurrence,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_authority::{
    ReconciledCharacterAuthority, assert_recovery_fence,
    verify_character_proficiency_history_with_definitions,
};
use crate::durability::character_progression::{CharacterProgressionError, numeric_u64, uuid_text};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::{DurabilityError, DurabilityRoot};
use sqlx::Row;
use std::collections::BTreeMap;
type StoredResult<T> = std::result::Result<T, DurabilityError>;

/// Retained values and the track's own last committed Character revision (PROFWIRE0).
/// This is not current content or gameplay admission evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProficiencyTrack {
    pub state: DurableProficiencyState,
    pub committed_character_revision: CharacterRevision,
    pub last_proficiency_occurrence_id: [u8; 16],
}
/// Immutable committed evidence only; a recovery read never restores a controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedProficiencyChange {
    pub character_id: CharacterId,
    pub occurrence_id: [u8; 16],
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub cause: ProficiencyCause,
    pub command_binding: Vec<u8>,
    pub policy_digest: [u8; 32],
    pub lines: Vec<ProficiencyLineCandidate>,
}
fn decode_values(
    item: &str,
    definition: &str,
    revision: &str,
    progress: i64,
    selections: Vec<Option<i16>>,
    canonical_array: bool,
) -> StoredResult<DurableProficiencyState> {
    if !canonical_array {
        return Err(DurabilityError::InvalidStoredState);
    }
    let selections = selections
        .into_iter()
        .map(|choice| choice.map(u8::try_from).transpose())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    DurableProficiencyState::new(
        item,
        definition,
        revision,
        u64::try_from(progress).map_err(|_| DurabilityError::InvalidStoredState)?,
        selections,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)
}
pub(in crate::durability) fn decode_state(
    row: &sqlx::postgres::PgRow,
    suffix: &str,
) -> StoredResult<DurableProficiencyState> {
    decode_values(
        row.try_get("item_key")?,
        row.try_get(format!("definition_key{suffix}").as_str())?,
        row.try_get(format!("definition_revision{suffix}").as_str())?,
        row.try_get(format!("progress{suffix}").as_str())?,
        row.try_get(format!("selections{suffix}").as_str())?,
        row.try_get(format!("canonical{suffix}").as_str())?,
    )
}
fn cause(value: &str) -> StoredResult<ProficiencyCause> {
    ProficiencyCause::from_key(value).ok_or(DurabilityError::InvalidStoredState)
}
fn revision(row: &sqlx::postgres::PgRow, key: &str) -> StoredResult<CharacterRevision> {
    CharacterRevision::new(numeric_u64(row, key)?).map_err(|_| DurabilityError::InvalidStoredState)
}
fn occurrence(row: &sqlx::postgres::PgRow, key: &str) -> StoredResult<[u8; 16]> {
    let bytes = uuid_text(row.try_get(key)?)?;
    CharacterId::from_bytes(bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(bytes)
}
pub(in crate::durability) fn decode_line(
    row: &sqlx::postgres::PgRow,
) -> StoredResult<ProficiencyLineCandidate> {
    ProficiencyLineCandidate::new(
        cause(row.try_get("cause")?)?,
        decode_state(row, "_before")?,
        decode_state(row, "_after")?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)
}
fn validate_transition(
    previous: Option<&DurableProficiencyState>,
    line: &ProficiencyLineCandidate,
) -> StoredResult<()> {
    if previous.map_or_else(
        || line.before().progress() != 0 || line.before().selections().iter().any(Option::is_some),
        |state| state != line.before(),
    ) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
const LINE_SELECT: &str = "SELECT l.*, h.content_revision, h.policy_digest, h.command_binding, h.original_character_revision::text AS original_revision, l.proficiency_occurrence_id::text AS occurrence, l.committed_character_revision::text AS revision, \
 coalesce(array_ndims(selections_before)=1 AND array_lower(selections_before,1)=1,false) AS canonical_before, \
 coalesce(array_ndims(selections_after)=1 AND array_lower(selections_after,1)=1,false) AS canonical_after \
 FROM game_character_proficiency_receipt_lines l JOIN game_character_proficiency_receipts h USING (proficiency_occurrence_id) WHERE l.character_id=encode($1,'hex')::uuid \
 ORDER BY l.item_key, l.committed_character_revision";
async fn load_lines(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> StoredResult<Vec<sqlx::postgres::PgRow>> {
    Ok(sqlx::query(LINE_SELECT)
        .bind(character.as_bytes().as_slice())
        .fetch_all(&mut **tx)
        .await?)
}
pub(in crate::durability) async fn verify_track_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    definitions: Option<&dyn super::ProficiencyDefinitions>,
) -> StoredResult<Vec<StoredProficiencyTrack>> {
    let mut latest = BTreeMap::<String, StoredProficiencyTrack>::new();
    let mut intents = BTreeMap::<
        [u8; 16],
        (
            ProficiencyCause,
            CharacterRevision,
            [u8; 32],
            Vec<u8>,
            Option<CharacterRevision>,
            Vec<ProficiencyLineCandidate>,
        ),
    >::new();
    for row in load_lines(tx, character).await? {
        let line = decode_line(&row)?;
        let previous = latest.get(line.before().item_key());
        validate_transition(previous.map(|track| &track.state), &line)?;
        let digest: [u8; 32] = row
            .try_get::<Vec<u8>, _>("policy_digest")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if line.requires_mapping_verification() {
            super::writer::verify_migration(
                definitions,
                row.try_get("content_revision")?,
                &digest,
                &line,
            )?;
        }
        // A selection binds its same-track historical predecessor, which may precede
        // the original global revision. This intent value never grants current authority.
        let expected_track = if line.cause() == ProficiencyCause::PerkSelection {
            Some(
                previous
                    .ok_or(DurabilityError::InvalidStoredState)?
                    .committed_character_revision,
            )
        } else {
            None
        };
        let entry = intents.entry(occurrence(&row, "occurrence")?).or_insert((
            line.cause(),
            revision(&row, "original_revision")?,
            digest,
            row.try_get("command_binding")?,
            expected_track,
            Vec::new(),
        ));
        entry.5.push(line.clone());
        let committed_character_revision = revision(&row, "revision")?;
        if previous
            .is_some_and(|track| track.committed_character_revision >= committed_character_revision)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        latest.insert(
            line.after().item_key().to_owned(),
            StoredProficiencyTrack {
                state: line.after().clone(),
                committed_character_revision,
                last_proficiency_occurrence_id: occurrence(&row, "occurrence")?,
            },
        );
    }
    for (id, (cause, original, digest, binding, expected_track, lines)) in intents {
        // A perk_modification binding covers its modification line; that verifier checks it.
        if cause == ProficiencyCause::PerkModification {
            continue;
        }
        let occurrence = ProficiencyOccurrence::from_bytes(id)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let request =
            super::ProficiencyChangeRequest::new(occurrence, cause, lines, expected_track, digest)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        if request.command_binding(character, original).as_slice() != binding {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let rows = sqlx::query("SELECT *, committed_character_revision::text AS revision, last_proficiency_occurrence_id::text AS occurrence, \
        coalesce(array_ndims(selections)=1 AND array_lower(selections,1)=1,false) AS canonical \
        FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid ORDER BY item_key")
        .bind(character.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    let mut tracks = Vec::with_capacity(rows.len());
    for row in rows {
        let track = StoredProficiencyTrack {
            state: decode_state(&row, "")?,
            committed_character_revision: revision(&row, "revision")?,
            last_proficiency_occurrence_id: occurrence(&row, "occurrence")?,
        };
        if latest.remove(track.state.item_key()).as_ref() != Some(&track) {
            return Err(DurabilityError::InvalidStoredState);
        }
        tracks.push(track);
    }
    if !latest.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(tracks)
}
impl DurabilityRoot {
    /// Explicit recovery-fenced inert load. Empty means no rows: the future content owner
    /// supplies the zero/unfilled seed. No level, Mastery or effects are inferred here.
    pub async fn read_character_proficiency_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> std::result::Result<Vec<StoredProficiencyTrack>, CharacterProgressionError> {
        self.load_retained_proficiency(authority, character, None, None)
            .await
            .map(|(tracks, _)| tracks)
    }
    /// Read retained evidence for this Character and occurrence; no authority is granted.
    /// This is not timeout reconciliation. Without an independent semantic source,
    /// migration history fails Unavailable; use the source-aware read for retained maps.
    pub async fn read_character_proficiency_occurrence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        occurrence_id: [u8; 16],
    ) -> std::result::Result<Option<CommittedProficiencyChange>, CharacterProgressionError> {
        CharacterId::from_bytes(occurrence_id)
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        self.load_retained_proficiency(authority, character, Some(occurrence_id), None)
            .await
            .map(|(_, receipt)| receipt)
    }
    /// Recovery-fenced retained read using independent historical migration declarations.
    pub async fn read_character_proficiency_with_definitions(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        occurrence: Option<ProficiencyOccurrence>,
        definitions: std::sync::Arc<dyn super::ProficiencyDefinitions>,
    ) -> std::result::Result<
        (
            Vec<StoredProficiencyTrack>,
            Option<CommittedProficiencyChange>,
        ),
        CharacterProgressionError,
    > {
        self.load_retained_proficiency(
            authority,
            character,
            occurrence.map(|id| *id.as_bytes()),
            Some(definitions),
        )
        .await
    }
    async fn load_retained_proficiency(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        requested: Option<[u8; 16]>,
        definitions: Option<std::sync::Arc<dyn super::ProficiencyDefinitions>>,
    ) -> std::result::Result<
        (
            Vec<StoredProficiencyTrack>,
            Option<CommittedProficiencyChange>,
        ),
        CharacterProgressionError,
    > {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| { Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            // Serialize with every Character writer before reading headers, lines and rows.
            let root = sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if root.is_none() { return Err(DurabilityError::InvalidStoredState); }
            let tracks = verify_character_proficiency_history_with_definitions(&mut tx, character, definitions.as_deref()).await?;
            let receipt = if let Some(occurrence_id) = requested {
                let row = sqlx::query("SELECT *, character_id::text AS character, proficiency_occurrence_id::text AS occurrence, \
                    original_character_revision::text AS original_revision, committed_character_revision::text AS revision \
                    FROM game_character_proficiency_receipts WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
                    .bind(occurrence_id.as_slice()).fetch_optional(&mut *tx).await?;
                if let Some(row) = row {
                    if uuid_text(row.try_get("character")?)? != *character.as_bytes() { return Err(DurabilityError::InvalidStoredState); }
                    let mut lines = Vec::new();
                    for line in load_lines(&mut tx, character).await? {
                        if occurrence(&line, "occurrence")? == occurrence_id { lines.push(decode_line(&line)?); }
                    }
                    Some(CommittedProficiencyChange { character_id: character, occurrence_id,
                        original_character_revision: revision(&row,"original_revision")?, committed_character_revision: revision(&row,"revision")?,
                        cause: cause(row.try_get("cause")?)?, command_binding: row.try_get("command_binding")?,
                        policy_digest: row.try_get::<Vec<u8>,_>("policy_digest")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?, lines })
                } else { None }
            } else { None };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok((tracks, receipt)))
        }) }).await?
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::character_proficiency::ProficiencyCause;
    const ITEM: &str = "oteryn:item.tibia.i3295";
    const DEF: &str = "oteryn:proficiency.tibia.p1";
    fn state(progress: u64, selections: Vec<Option<u8>>) -> DurableProficiencyState {
        DurableProficiencyState::new(ITEM, DEF, "r1", progress, selections).expect("state")
    }
    #[test]
    fn proficiency_read_rejects_noncanonical_arrays_and_signed_choice_alias() {
        let _ = std::mem::size_of::<super::super::CommittedProficiencyChange>();
        let _ = DurabilityRoot::read_character_proficiency_state;
        let _ = DurabilityRoot::read_character_proficiency_occurrence;
        assert!(decode_values(ITEM, DEF, "r1", 1, vec![None], false).is_err());
        assert!(decode_values(ITEM, DEF, "r1", 1, vec![Some(256)], true).is_err());
        assert!(decode_values(ITEM, DEF, "r1", 1, vec![Some(-256)], true).is_err());
        assert_eq!(
            decode_values(ITEM, DEF, "r1", i64::MAX, vec![None, Some(2)], true)
                .expect("retained")
                .selections(),
            &[None, Some(2)]
        );
    }
    #[test]
    fn proficiency_read_history_rejects_one_wrong_predecessor_value() {
        let before = state(3, vec![None]);
        let after = state(4, vec![None]);
        let line = ProficiencyLineCandidate::new(ProficiencyCause::Training, before.clone(), after)
            .expect("line");
        assert!(validate_transition(Some(&before), &line).is_ok());
        assert!(validate_transition(Some(&state(2, vec![None])), &line).is_err());
    }
    #[test]
    fn proficiency_read_history_rejects_nonzero_or_filled_seed_independently() {
        for before in [state(1, vec![None]), state(0, vec![Some(0)])] {
            let mut after = before.clone();
            after.progress += 1;
            let line = ProficiencyLineCandidate::new(ProficiencyCause::Training, before, after)
                .expect("line");
            assert!(validate_transition(None, &line).is_err());
        }
    }
    // Corruption fixture deliberately omits SQL guards; transaction-local tables only.
    // This qualifies the inert decoder/history consumer, not 0032's guard/full admission gate.
    #[test]
    fn proficiency_read_postgres_corruption_and_raw_array_metadata() {
        tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("test runtime").block_on(async {
        use sqlx::Connection;
        let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            return;
        };
        let mut connection = sqlx::PgConnection::connect(&url).await.expect("local PG");
            let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&mut connection).await.expect("server version");
            assert!(version.starts_with("17.6"), "qualification requires PostgreSQL 17.6");
        let mut tx = connection.begin().await.expect("isolated transaction");
        sqlx::raw_sql("CREATE TEMP TABLE game_character_roots AS SELECT '01020304-0506-7008-800a-0b0c0d0e0f01'::uuid character_id, 2::numeric character_revision;
            CREATE TEMP TABLE game_character_proficiency_receipts AS SELECT character_id, 1::numeric original_character_revision, 2::numeric committed_character_revision,
             '02020304-0506-7008-800a-0b0c0d0e0f02'::uuid proficiency_occurrence_id, 'training'::text cause, decode('01','hex') command_binding,
             decode(repeat('00',32),'hex') policy_digest, 1::bigint level_before, 1::bigint level_after, 0::bigint experience_before, 0::bigint experience_after,
             'r1'::text profile_revision,'r1'::text ruleset_revision,'r1'::text content_revision,'r1'::text simulation_revision,
             'r1'::text evidence_revision,'r1'::text declaration_revision,'r1'::text policy_revision,'r1'::text reward_revision,0::bigint committed_at FROM game_character_roots;
            CREATE TEMP TABLE game_character_proficiency_receipt_lines AS SELECT character_id,committed_character_revision,proficiency_occurrence_id,cause,
             'oteryn:item.tibia.i3295'::text item_key, 'oteryn:proficiency.tibia.p1'::text definition_key_before,'oteryn:proficiency.tibia.p1'::text definition_key_after,
             'r1'::text definition_revision_before,'r1'::text definition_revision_after,0::bigint progress_before,1::bigint progress_after,
             ARRAY[NULL,2]::smallint[] selections_after,ARRAY[NULL,NULL]::smallint[] selections_before FROM game_character_proficiency_receipts;
            UPDATE game_character_proficiency_receipt_lines SET selections_after=selections_before;
            CREATE TEMP TABLE game_character_proficiency AS SELECT character_id,item_key,committed_character_revision,proficiency_occurrence_id last_proficiency_occurrence_id,
             definition_key_after definition_key,definition_revision_after definition_revision,progress_after progress,selections_after selections FROM game_character_proficiency_receipt_lines;
        ").execute(&mut *tx).await.expect("corruption fixture");
        let character =
            CharacterId::from_bytes([1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 1])
                .expect("character");
        let request = super::super::ProficiencyChangeRequest::new(
            ProficiencyOccurrence::from_bytes([2, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 2]).expect("occurrence"),
            ProficiencyCause::Training,
            vec![ProficiencyLineCandidate::new(ProficiencyCause::Training, state(0, vec![None, None]), state(1, vec![None, None])).expect("line")],
            None,
            [0; 32],
        ).expect("fixture intent");
        let binding = request.command_binding(character, CharacterRevision::new(1).expect("original revision"));
        sqlx::query("UPDATE game_character_proficiency_receipts SET command_binding=$1")
            .bind(binding.as_slice()).execute(&mut *tx).await.expect("valid v1 fixture binding");
        assert_eq!(
            verify_character_proficiency_history_with_definitions(&mut tx, character, None)
                .await
                .expect("valid history")[0]
                .committed_character_revision
                .get(),
            2
        );
        for mutation in [
            "UPDATE game_character_proficiency_receipt_lines SET character_id='03020304-0506-7008-800a-0b0c0d0e0f03'",
            "UPDATE game_character_proficiency_receipt_lines SET committed_character_revision=3",
            "UPDATE game_character_proficiency_receipt_lines SET cause='perk_selection'",
            "UPDATE game_character_proficiency_receipts SET command_binding='\\x'",
            "UPDATE game_character_proficiency_receipts SET policy_digest='\\x00'",
            "UPDATE game_character_proficiency SET progress=2",
            "UPDATE game_character_proficiency SET committed_character_revision=3",
            "UPDATE game_character_proficiency SET last_proficiency_occurrence_id='03020304-0506-7008-800a-0b0c0d0e0f03'",
            "UPDATE game_character_proficiency_receipt_lines SET selections_after='[0:1]={NULL,NULL}'",
            "UPDATE game_character_proficiency_receipt_lines SET selections_after='{{NULL,NULL}}'",
            "DELETE FROM game_character_proficiency_receipt_lines",
            "DELETE FROM game_character_proficiency",
        ] {
            sqlx::query("SAVEPOINT one_invariant")
                .execute(&mut *tx)
                .await
                .expect("savepoint");
            sqlx::query(mutation)
                .execute(&mut *tx)
                .await
                .expect("one mutation");
            assert!(
                verify_character_proficiency_history_with_definitions(&mut tx, character, None)
                    .await
                    .is_err(),
                "{mutation}"
            );
            sqlx::query("ROLLBACK TO one_invariant")
                .execute(&mut *tx)
                .await
                .expect("restore fixture");
        }
        // Actual PG wire arrays retain NULL entries; canonical metadata is decoded separately.
        for (array, valid) in [
            ("ARRAY[NULL,2]::smallint[]", true),
            ("'[0:1]={NULL,2}'::smallint[]", false),
            ("'{{NULL,2}}'::smallint[]", false),
        ] {
            let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT 'oteryn:item.tibia.i3295'::text item_key,'oteryn:proficiency.tibia.p1'::text definition_key,
                'r1'::text definition_revision,9223372036854775807::bigint progress,{array} selections,
                array_ndims({array})=1 AND array_lower({array},1)=1 canonical"))).fetch_one(&mut *tx).await.expect("raw array");
            let decoded = decode_state(&row, "");
            assert_eq!(decoded.is_ok(), valid);
            if valid {
                assert_eq!(decoded.expect("canonical").selections(), &[None, Some(2)]);
            }
        }
        tx.rollback().await.expect("no retained fixture");
        });
    }
    #[test]
    fn proficiency_read_migration_refuses_without_retained_mapping() {
        let before = state(0, vec![None]);
        let after = DurableProficiencyState::new(ITEM, DEF, "r2", 0, vec![None]).expect("after");
        let line = ProficiencyLineCandidate::new(ProficiencyCause::Migration, before, after)
            .expect("candidate");
        assert!(matches!(
            super::super::writer::verify_migration(None, "r1", &[0; 32], &line),
            Err(DurabilityError::Unavailable)
        ));
    }
}
