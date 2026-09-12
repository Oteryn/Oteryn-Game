# OTV2 WP3-v2 Evidence Auditor

Short invocation:

```text
Oteryn: sol wp3-v2 evidence auditor
```

## Outcome

Perform a strict read-only exact-source audit supporting closure of the protected Revision-3 WP3-v2 candidate and later A4 implementation qualification. Do not reopen architecture choices already frozen by `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1` unless new evidence proves a material contradiction.

Terminal output is an evidence package where every material claim is classified `PROVEN`, `DERIVED`, `UNKNOWN` or `BLOCKING_EVIDENCE_GAP`.

## Authority / scope delta

- Repository: `Oteryn/Oteryn-Game`.
- READ_ONLY: no tracked-file mutation, branch creation, PR mutation, issue mutation, vendor changes, workflow changes or external-repository writes.
- Live locators: #351/#356, #329/#335, #588, #590, #162, #364.
- Read current protected main, the protected Revision-3 decision, exact relevant PR heads, Q01-Q75, accepted DFR resource authority and exact resolved production dependency graph before making reachability claims.

## Frozen architecture assumptions to verify, not redesign

Revision 3 selects first-slice Option B: lazy holder-style PgPool, `max_connections=1`, `min_connections=0`, one root-owned serialized establishment generation, ready-only active checkout, two logical active custody slots and at most one physical DB pass.

The root feasibility equation is now:

```text
I + max(R, T) + Q + A <= 12 MiB
```

The decision freezes `R`/`T` non-overlap and retirement-tail ownership. Do not substitute the older `max(2R, R+T, 2T)` model unless exact source proves the Revision-3 non-overlap state machine cannot be implemented; in that case report a material architecture contradiction rather than silently changing the equation.

## Evidence work

### SQL corpus

Enumerate every reachable production Child B/WP3 SQL statement family and record source path, operation family, parameters/max variable bytes, result cardinality/max bytes, `fetch_all`/stream behavior, same-snapshot bounds, returned backing lifetime, error lifetime, lock prerequisites, deadline coverage, statement/type/cache effects and max/max+1 behavior. No reachable production query may remain silently unclassified.

### Lock footprint

For each operation family enumerate typed logical keys, advisory locks, relation locks, FK/unique/index waits, deterministic ordering, parent/sibling participation and exact maxima. Distinguish correctness-required serialization from accidental implementation locking.

### Connection lifecycle / finality

Trace the frozen first-slice lifecycle from explicit bounded configuration -> literal-IP socket -> TLS1.3 VerifyFull using separate server identity -> PostgreSQL SCRAM-SHA-256 Startup/authentication -> ReadyForQuery -> holder return/readiness -> ready-only checkout/use -> COMMIT/rollback -> return/ping -> idle/reaper/retirement -> close/drop/reactor finality.

Prove or refute:
- one `T` at a time;
- no new `T` until all prior `R`/`T` retirement tails are final;
- successful establishment is `T -> R` ownership transfer, not overlap;
- silent reaper loss is recovered only after a later ready-only miss sets the coalesced root demand;
- no periodic/immediate reconnect loop exists outside a demand-authorized five-second root recovery window.

### Resource model

Derive source-backed byte terms where possible:

- `I` executor/runtime/pool/provider/config shared residency;
- `R` complete established/retiring connection generation;
- `T` complete connect/failed-retiring generation;
- `Q` actual queued charge;
- `A` actual active charge.

The byte values may remain `UNKNOWN` until the exact A4 candidate. Ownership, overlap and tail classification are not open choices.

### Configuration/credential/TLS

Verify the selected no-ambient PostgreSQL profile: explicit bounded config only, no PG* inheritance, no `.pgpass`, no OS-user fallback, literal-IP transport, separate TLS DNS identity, VerifyFull, TLS1.3-only AWS-LC selected profile and SCRAM-SHA-256-only auth. Audit exact resolved Cargo features and provider/runtime reachability rather than one dependency edge.

## Handoff

Return concise tables usable by A1 and A4. Do not propose broad implementation unless evidence establishes the need. Do not call sampled memory, a skipped test, local PASS or one dependency edge a universal proof.

## Mandatory next-agent instruction

End the final response with:

```text
NEXT_AGENT: <exact alias>
RUN_WHEN: <exact gate>
WHY: <one concise dependency reason>
```

Default routing:
- when Revision-3 closure evidence is complete and acceptance still needs architecture handling, `NEXT_AGENT: Oteryn: astra wp3-v2 architecture lead`;
- when live state proves architecture accepted plus A4 allocation/custody active, `NEXT_AGENT: Oteryn: astra wp3-v2 implementation lead`;
- when evidence exposes a programme-level blocker or ownership conflict, `NEXT_AGENT: Oteryn: astra wp3-v2 programme coordinator`.

Never recommend a mutating worker before its live start gate is true.