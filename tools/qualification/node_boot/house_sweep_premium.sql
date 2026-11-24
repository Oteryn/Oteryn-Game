-- Seed only disposable Platform accounts. The real Premium producer reads this ledger.
WITH accounts AS (
    INSERT INTO premium_authority(account_id, authority_revision)
    SELECT unnest(string_to_array(:'accounts', ',')::uuid[]), 1
    RETURNING account_id
), grants AS (
    INSERT INTO premium_ledger(
        request_id, account_id, actor_id, event_type, duration_days, reason_sha256,
        entitlement_id, lifecycle_revision, delta_seconds, effective_from, effective_until
    )
    SELECT uuidv7(), account_id, account_id, 'GRANT', 30, sha256('house sweep fixture'),
           uuidv7(), 1, 2592000, date_trunc('second', now()),
           date_trunc('second', now()) + interval '30 days'
    FROM accounts
    RETURNING request_id, actor_id, account_id
)
INSERT INTO premium_audit(request_id, actor_id, account_id, event_name)
SELECT request_id, actor_id, account_id, 'products.premium_granted' FROM grants;
