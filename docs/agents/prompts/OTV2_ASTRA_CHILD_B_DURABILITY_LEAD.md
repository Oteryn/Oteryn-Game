# OTV2 Child B Durability Lead

Short invocation:

```text
Oteryn: astra child-b durability lead
```

## Outcome

Finish the canonical Child B durable fresh-admission/reconnect implementation on the existing #329 / PR #335 lineage after WP3/shared-custody gates clear, preserving exact replay/nonreuse/recovery semantics and producing protected terminal WP4 evidence.

## Canonical lineage

Freshly verify #329, PR #335 and branch `agent/durable-fresh-admission-child-b-329` before mutation. Do not create a replacement worker while that lineage remains canonical.

## Start gate

Before material mutation require:

- protected terminal WP3-v2 delivery sufficient for B;
- live release/assignment of shared PostgreSQL/Cargo/Durability custody required by the current allocation;
- current WP2/GameSession-nonreuse prerequisites as required by live #364;
- exact forward migration/allocation state.

Before those gates, this alias may perform read-only preparation only.

## Production target

All DFR-governed semantic persistence must use one accepted custody path:

`producer -> bounded queue <= 8 -> active <= 2 -> DB pass -> commit/rollback/reconcile -> bounded completion -> owner acknowledgement -> release`.

No hidden executor or raw production persistence bypass may manufacture additional active work.

## Required behavior

Cover the accepted Child B families, including fresh admission, reconnect, session replacement/nonreuse, claim/guard publication, lifecycle writes, exact operation identity/replay, pending custody, original-operation reconciliation, restart/takeover and required non-rollback state.

Preserve complete semantic records; do not truncate history or silently paginate an atomic comparison merely to satisfy a budget.

## Database/locking/deadline obligations

- Use only the accepted WP3-v2 executor/resource model.
- Enumerate and validate the exact SQL corpus and lock footprint used by final B.
- Keep the absolute DB execution/reconciliation deadline across lock waits and required completion semantics.
- Do not release active custody while rollback/protocol drain/connection finality or ambiguous outcome remains owned by the operation unless ownership has explicitly transferred to a pre-funded root owner.
- Never drop correctness-required locks merely for concurrency metrics.

## Validation delta

Use configured real PostgreSQL qualification for fresh admission, reconnect, replacement/nonreuse, cross-origin races, lost response, ambiguous COMMIT/recovery, restart, queue/slot saturation, deadline/cancellation, lock contention and max/max+1 boundaries. Skipped/unconfigured DB tests are not proof.

Finish whole-diff self-review, applicable independent review and exact candidate repository checks with no unresolved P0/P1 before reporting the lane ready for protected integration.