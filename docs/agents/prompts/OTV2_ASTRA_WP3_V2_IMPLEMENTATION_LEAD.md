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

Require independent review for material architecture/resource/security/unsafe changes. A terminal WP3 implementation candidate requires an exact-final-head whole-diff independent review with `PASS`, `P0=0`, `P1=0`, `P2=0`, and `BLOCKING_EVIDENCE_GAP=0`; any weaker review wording or non-PASS disposition cannot authorize terminal handoff, integration, or release. Handoff must include exact candidate head, Q01-Q75 ledger, retained/superseded #356 map and any precise blocker.

## Mandatory owner-facing successor instruction

End the final response with:

```text
CONTROL_PLANE_ACTION: <exact control-plane alias + exact action, or NONE>
NEXT_WORKER: <exact A0-A7 worker alias or NONE>
RUN_WORKER_WHEN: <exact gate>
WHY: <one concise dependency reason>
```

Routing:
- after producing a qualified candidate that still needs protected integration/readback/custody release, put that exact action in `CONTROL_PLANE_ACTION`, set `NEXT_WORKER: Oteryn: astra child-b durability lead`, and gate A5 on protected WP3 plus released Child B custody;
- only when live state already proves protected WP3 integration and Child B custody release, use `CONTROL_PLANE_ACTION: NONE` and `NEXT_WORKER: Oteryn: astra child-b durability lead`;
- if evidence is the remaining substantive task, use `NEXT_WORKER: Oteryn: sol wp3-v2 evidence auditor`;
- if no substantive worker can yet be determined truthfully, use `NEXT_WORKER: NONE` and name the exact control-plane gate.

Never place a control-plane-only alias in `NEXT_WORKER`. Do not recommend A5 for mutation before protected WP3 and custody release are true.