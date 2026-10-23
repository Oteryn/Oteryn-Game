//! Verify creation receipts, including both historical tag 2 and sex-bearing tag 3.
use super::*;

pub(super) async fn verify(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_creation_receipts c USING (character_id) \
           LEFT JOIN game_character_operation_receipts o USING (character_id) \
           LEFT JOIN game_character_audit_outbox a ON a.event_id = c.event_id \
          WHERE c.character_id IS NULL OR c.account_id <> r.account_id OR c.world_id <> r.world_id \
             OR o.character_revision <> 1 \
             OR o.command_binding <> (CASE WHEN r.sex IS NULL THEN decode('02','hex') ELSE decode('03','hex') END) || uuid_send(o.issuer_decision_id) \
                || int8send(o.intent_source_revision) || uuid_send(o.operation_id) \
                || uuid_send(r.account_id) || uuid_send(r.world_id) \
                || int8send(o.issued_at_source) || int8send(o.expires_at_source) \
                || int2send(octet_length(r.profile_revision)::int2) || convert_to(r.profile_revision, 'UTF8') \
                || int2send(octet_length(r.ruleset_revision)::int2) || convert_to(r.ruleset_revision, 'UTF8') \
                || int2send(octet_length(r.content_revision)::int2) || convert_to(r.content_revision, 'UTF8') \
                || int2send(octet_length(r.starter_template_revision)::int2) || convert_to(r.starter_template_revision, 'UTF8') \
                || int2send(octet_length(r.name)::int2) || convert_to(r.name, 'UTF8') \
                || (CASE WHEN r.sex IS NULL THEN ''::bytea ELSE substring(int2send(r.sex) from 2 for 1) END) \
             OR (a.event_id IS NOT NULL AND (a.character_id <> r.character_id \
                 OR a.transaction_id <> c.transaction_id OR a.server_build_id <> c.server_build_id \
                 OR a.occurred_at <> c.occurred_at \
                 OR a.payload_sha256 <> sha256(a.payload))) \
         UNION ALL \
         SELECT 1 FROM game_character_audit_outbox a \
           LEFT JOIN game_character_creation_receipts c ON c.event_id = a.event_id \
          WHERE c.event_id IS NULL OR c.character_id <> a.character_id \
         UNION ALL \
         SELECT 1 FROM game_character_creation_receipts c \
          GROUP BY c.character_id HAVING count(*) > 1 \
         UNION ALL \
         SELECT 1 FROM game_character_command_receipts k \
           JOIN game_character_operation_receipts o USING (operation_id) \
         UNION ALL \
         SELECT 1 FROM game_character_operation_receipts o \
           LEFT JOIN game_character_roots r USING (character_id) \
          WHERE r.character_id IS NULL \
         UNION ALL \
         SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_name_reservations n \
             ON n.name_key = r.name_key AND n.character_id = r.character_id \
          WHERE n.name_key IS NULL \
         UNION ALL \
         SELECT 1 FROM game_character_audit_legal_holds h \
           LEFT JOIN game_character_audit_outbox a USING (event_id) \
          WHERE h.released_at IS NULL AND a.event_id IS NULL \
         UNION ALL \
         SELECT 1 WHERE (SELECT count(*) FROM game_character_bootstrap_intent_floors) \
                     <> (SELECT count(*) FROM (SELECT 1 FROM game_character_operation_receipts LIMIT 1) x) \
         UNION ALL \
         SELECT 1 FROM game_character_bootstrap_intent_floors f \
          WHERE f.source_revision <> (SELECT max(intent_source_revision) FROM game_character_operation_receipts) \
             OR NOT EXISTS (SELECT 1 FROM game_character_operation_receipts o \
                             WHERE o.intent_source_revision = f.source_revision \
                               AND o.issuer_decision_id = f.issuer_decision_id \
                               AND o.command_binding = f.intent_binding) \
         LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .map_or(Ok(()), |_| Err(DurabilityError::Unavailable))?;
    Ok(())
}
