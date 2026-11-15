# Profile-B consumer operations

The consumer implements Platform c5780e2c revision 2, default off.

B1 failures are scoped to the account and retried with bounded backoff and a fresh nonce. A lifecycle hint ahead of a B1 answer is a benign race: the answer is ignored, the hint remains a benefit fence, and the account retries refresh.

B2 authentication failures alarm and back off on the affected node. Only a malformed wire response or non-advancing cursor durably stops the shared feed.

## Operator reset

As with enforcement operator reconciliation, first repair the producer defect, inspect the retained page and audit the reconciliation. Using the database owner/operator connection (never the runtime or read-only control identity), clear only the stop bit with a compare-and-set against the inspected cursor:

```sql
UPDATE game_profile_b_changes_cursor SET stopped = false
WHERE stopped AND stream_cursor IS NOT DISTINCT FROM :inspected_cursor;
```

Restart the affected consumer after the reset. Preserve the cursor, accepted answers, authority revisions and lifecycle/hint floors. Never rewind or delete fences to recover from a defect.
