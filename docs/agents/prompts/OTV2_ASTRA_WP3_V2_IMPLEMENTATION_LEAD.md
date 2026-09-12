# OTV2 WP3-v2 Implementation Lead

Short invocation:

```text
Oteryn: astra wp3-v2 implementation lead
```

## Outcome

Implement the repository-accepted WP3-v2 architecture on the canonical authorized lineage with the smallest maintainable delta, then produce an exact candidate that can truthfully satisfy Q01-Q75 and unblock canonical Child B.

Terminal worker outcome before repository integration: `WP3_IMPLEMENTATION_CANDIDATE`.

## Start gate

Do not begin material mutation until:

- the superseding WP3-v2 decision has sufficient repository acceptance;
- live #162/#364 allocation identifies the canonical implementation lineage and owned paths;
- current #356 disposition is explicitly known;
- overlapping WP3/Cargo/vendor/Durability ownership has been reconciled.

Never infer write authority from this prompt.

## Scope delta

Implement only the selected architecture. Preserve useful #356 evidence/primitives, but do not retain broad dependency forks merely because sunk work exists. Do not delete historical evidence.

Required implementation domains, as applicable to the accepted choice:

- one root/executor budget and sealed production entry path;
- queue/active custody compatible with accepted `8 queued / 2 active` semantics;
- bounded connection establishment and connection-resident ownership;
- minimal SQLx seam only where universal source proof cannot otherwise be established;
- PostgreSQL startup/authentication/input bounds;
- exact TLS/runtime/provider profile and resource ownership;
- absolute DB-pass deadline propagation;
- cancellation, rollback, close, return-to-idle and ambiguous COMMIT/reconciliation ownership;
- restart/takeover/predecessor-work fencing;
- configuration/credential lifetime and redacted errors;
- exact source-backed resource equations and max/max+1 failures.

## Non-goals

- no Child B semantic rewrite beyond exact integration hooks allocated to WP3;
- no source/WP5 implementation;
- no Server Seam work;
- no Platform mutation;
- no weakening PostgreSQL/TLS semantics or resource ceilings;
- no hidden second executor/driver budget.

## Validation delta

Work test-first where a behavioral regression can be made deterministic. Run focused checks while iterating, then the exact resolved production graph, configured PostgreSQL/TLS qualification and every applicable Q01-Q75 obligation on the exact candidate. Source/lifetime proof is required for universal resource claims; sampled memory peaks only corroborate.

Require independent review for material architecture/resource/security/unsafe changes. Do not call the lane terminal with unresolved P0/P1. Handoff must include exact candidate head, Q01-Q75 ledger, retained/superseded #356 map and any precise blocker.