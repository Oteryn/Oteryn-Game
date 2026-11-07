//! Stored receipt fields cannot substitute a historical command result behind an intact hash.
use super::*;

pub(super) async fn qualify(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    change: &appearance::Change,
) -> TestResult {
    use crate::durability::character_appearance::AppearanceChangeOccurrence;
    use crate::foundation::{CommandId, CommandRef};
    let occurrence = AppearanceChangeOccurrence::for_command(CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).map_err(debug)?,
    ));
    let payload = change.encode().map_err(debug)?;
    let assignments: &[&str] = if change.prior.is_some() {
        &[
            "outfit_before=NULL,colours_before=NULL,addons_before=NULL,mount_before=NULL,content_before=NULL,selection_revision_before=NULL",
        ]
    } else {
        &[
            "outfit_after='oteryn:outfit.substituted'",
            "colours_after=ARRAY[9,9,9,9]",
            "content_revision='substituted'",
            "original_character_revision=4,committed_character_revision=5",
        ]
    };
    for assignment in assignments {
        // Privileged corruption is confined to this isolated test database and transaction.
        // This is the existing native PG fault technique; no runtime role or schema is changed.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE game_character_appearance_receipts SET {assignment} WHERE appearance_occurrence_id=encode($1,'hex')::uuid",
        ))).bind(occurrence.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let result = crate::gameplay_transport::connection::appearance_change::dispatch(
            owner, actor, command, &payload,
        )
        .await;
        // Restore before asserting, including on the deliberately red implementation.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let before = change.prior.as_ref();
        sqlx::query("UPDATE game_character_appearance_receipts SET outfit_after=$2,colours_after=$3,content_revision=$4,original_character_revision=$5::text::numeric,committed_character_revision=$6::text::numeric,outfit_before=$7,colours_before=$8,addons_before=$9,mount_before=$10,content_before=$11,selection_revision_before=$12::text::numeric WHERE appearance_occurrence_id=encode($1,'hex')::uuid")
            .bind(occurrence.as_bytes().as_slice()).bind(&change.selection.outfit_key)
            .bind(change.selection.colours.map(i32::from).to_vec()).bind(&change.content_revision)
            .bind(change.expected_root_revision.to_string())
            .bind(change.expected_root_revision.checked_add(1).ok_or("revision")?.to_string())
            .bind(before.map(|p| p.selection.outfit_key.as_str()))
            .bind(before.map(|p| p.selection.colours.map(i32::from).to_vec()))
            .bind(before.map(|p| i16::from(p.selection.addons)))
            .bind(before.and_then(|p| p.selection.mount_key.as_deref()))
            .bind(before.map(|p| p.provenance.as_str()))
            .bind(before.map(|p| p.revision.to_string()))
            .execute(&mut *tx).await?;
        tx.commit().await?;
        assert_eq!(
            result,
            Some(appearance::ChangeResult::Rejected),
            "substituted receipt: {assignment}"
        );
    }
    Ok(())
}
