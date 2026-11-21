-- This fixture represents a retained tag-3 bootstrap in a disposable Game database.
-- Run after normal empty-store admission and before the node acquires its recovery seal.
BEGIN;
DO $$
BEGIN
    IF current_database() <> 'oteryn_node_boot'
        OR session_user <> 'oteryn_node_boot_admin'
        OR (SELECT count(*) FROM game_character_recovery_admissions) <> 1
        OR EXISTS (SELECT 1 FROM game_character_roots)
        OR EXISTS (SELECT 1 FROM game_character_operation_receipts)
        OR EXISTS (SELECT 1 FROM game_character_bootstrap_intent_floors)
        OR EXISTS (SELECT 1 FROM game_node_registrations)
    THEN
        RAISE EXCEPTION 'D150 requires an empty admitted node-boot fixture database before launch';
    END IF;
END $$;

INSERT INTO game_character_account_guards (account_id) VALUES (:'account'::uuid);
INSERT INTO game_character_roots (
    character_id, account_id, world_id, lifecycle, character_revision,
    profile_revision, ruleset_revision, content_revision, starter_template_revision, name, sex
) VALUES (
    game_character_uuid_v7(), :'account'::uuid, :'world'::uuid, 1, 1,
    :'profile', :'ruleset', :'content', 'nb-starter-1', 'Legacy Starter', 2
);

WITH ids AS (
    SELECT game_character_uuid_v7() AS operation_id, game_character_uuid_v7() AS issuer_decision_id,
           game_character_uuid_v7() AS event_id, game_character_uuid_v7() AS transaction_id
), receipt AS (
    INSERT INTO game_character_operation_receipts (
        operation_id, command_binding, account_id, character_id, world_id, character_revision,
        event_id, occurred_at, server_build_id, transaction_id, issuer_decision_id,
        intent_source_revision, issued_at_source, expires_at_source
    )
    SELECT operation_id, decode('03', 'hex') || uuid_send(issuer_decision_id) || int8send(1)
           || uuid_send(operation_id) || uuid_send(account_id) || uuid_send(world_id)
           || int8send(1) || int8send(120)
           || int2send(octet_length(profile_revision)::int2) || convert_to(profile_revision, 'UTF8')
           || int2send(octet_length(ruleset_revision)::int2) || convert_to(ruleset_revision, 'UTF8')
           || int2send(octet_length(content_revision)::int2) || convert_to(content_revision, 'UTF8')
           || int2send(octet_length(starter_template_revision)::int2) || convert_to(starter_template_revision, 'UTF8')
           || int2send(octet_length(name)::int2) || convert_to(name, 'UTF8') || decode('02', 'hex'),
           account_id, character_id, world_id, 1, event_id, 1, 'node-boot-d150/1', transaction_id,
           issuer_decision_id, 1, 1, 120
      FROM game_character_roots CROSS JOIN ids
    RETURNING issuer_decision_id, command_binding
)
INSERT INTO game_character_bootstrap_intent_floors (issuer_scope, source_revision, issuer_decision_id, intent_binding)
SELECT 1, 1, issuer_decision_id, command_binding FROM receipt;
COMMIT;
