# OTV2 WP3-v2 Evidence Auditor

Short invocation:

```text
Oteryn: sol wp3-v2 evidence auditor
```

## Outcome

Perform a strict read-only exact-source audit that supplies the evidence needed for the WP3-v2 architecture decision and later implementation qualification.

Terminal output is an evidence package where every material claim is classified `PROVEN`, `DERIVED`, `UNKNOWN` or `BLOCKING_EVIDENCE_GAP`.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- READ_ONLY: no tracked-file mutation, branch creation, PR mutation, issue mutation, vendor changes, workflow changes or external-repository writes.
- Live locators: #351/#356, #329/#335, #588, #162, #364.
- Read current protected main, exact relevant PR heads, Q01-Q75, accepted DFR resource authority and exact resolved production dependency graph before making reachability claims.

## Evidence work

### SQL corpus

Enumerate every reachable production Child B/WP3 SQL statement family and record source path, operation family, parameters/max variable bytes, result cardinality/max bytes, `fetch_all`/stream behavior, same-snapshot bounds, returned backing lifetime, error lifetime, lock prerequisites, deadline coverage, statement/type/cache effects and max/max+1 behavior. No reachable production query may remain silently unclassified.

### Lock footprint

For each operation family enumerate typed logical keys, advisory locks, relation locks, FK/unique/index waits, deterministic ordering, parent/sibling participation and exact maxima. Distinguish correctness-required serialization from accidental implementation locking.

### Connection lifecycle

Trace exact reachable lifecycle from configuration -> DNS/socket -> TLS -> PostgreSQL Startup/authentication -> ReadyForQuery -> checkout/use -> COMMIT/rollback -> return/ping -> idle/maintenance/replacement -> close/drop/reactor release. Identify automatic retry/backoff/background paths and whether they can overlap.

### Resource model

Derive source-backed terms where possible:

- `I` executor/runtime resident;
- `R` per-connection resident;
- `T` connect transient peak;
- `Q` actual queued charge;
- `A` actual active charge.

If two connections may establish concurrently, test the applicability of `I + max(2R, R+T, 2T) + Q + A <= 12 MiB`. Narrow only when exact serialization/unreachability is proven.

### Configuration/credential/TLS

Inventory reachable explicit URL/struct, PG environment/passfile/OS fallback, CA/root/client material, DNS, application/options fields and secret/error retention. Audit exact resolved Cargo features and provider/runtime reachability rather than one dependency edge.

## Handoff

Return concise tables usable by A1 and A4. Do not propose broad implementation unless the evidence establishes the need. Do not call sampled memory, a skipped test, local PASS or one dependency edge a universal proof.

## Mandatory next-agent instruction

End the final response with:

```text
NEXT_AGENT: <exact alias>
RUN_WHEN: <exact gate>
WHY: <one concise dependency reason>
```

Default routing for this lane:
- when the requested evidence package is complete and A1 is still deciding architecture, `NEXT_AGENT: Oteryn: astra wp3-v2 architecture lead`;
- when architecture is already accepted and A4 needs implementation proof support, `NEXT_AGENT: Oteryn: astra wp3-v2 implementation lead`;
- when evidence exposes a programme-level blocker or ownership conflict, `NEXT_AGENT: Oteryn: astra wp3-v2 programme coordinator`.

Never recommend a mutating worker before its live start gate is true.