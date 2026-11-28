-- A durable input for one disposable Platform account. Game must pull the
-- producer's ordinary snapshot on both admissions.
BEGIN;
INSERT INTO premium_authority(account_id) VALUES (:'account'::uuid);
INSERT INTO premium_ledger (
    request_id, account_id, actor_id, event_type, duration_days, reason_sha256,
    entitlement_id, lifecycle_revision, delta_seconds, effective_from, effective_until
)
SELECT account_id, account_id, account_id, 'GRANT', 1,
       sha256(convert_to('node-boot q_p10', 'UTF8')), account_id, 1, 86400,
       issued - interval '1 minute', issued - interval '1 minute' + interval '1 day'
FROM (SELECT :'account'::uuid AS account_id, date_trunc('second', now()) AS issued) fixture;
INSERT INTO premium_audit (request_id, actor_id, account_id, event_name)
VALUES (:'account'::uuid, :'account'::uuid, :'account'::uuid, 'products.premium_granted');
COMMIT;
