# OTV2-20260811-reference-hybrid-owner-baseline — archived

```yaml
task_id: OTV2-20260811-reference-hybrid-owner-baseline
status: completed
repository: blakinio/Oteryn-v2
owner_decision_date: 2026-08-11
source_type: USER_SOURCE
delivery_pr: 158
final_head_sha: 81a22469e90ccc4ac794d5e8c240dd91c3c3b530
merge_sha: d31b23b82d4e62ee311adc48e8861dc8182af0b7
repair_cycles: 1
implementation_status: NOT_APPLICABLE
```

## Delivered decision

The owner selected the **hybrid Reference upstream-tracking model**:

- Global Tibia behavioral evidence may be observed continuously;
- every released Oteryn Reference revision is an immutable named parity target;
- upstream changes do not live-sync into a released Reference revision;
- accepted upstream changes gain Reference product authority only through explicit promotion into a later immutable Reference revision.

The delivery explicitly supersedes only ADR-0010's prior deferment of the long-term Reference tracking-policy choice. The rest of ADR-0010 remains binding, including named/versioned parity targets, explicit parity classification, shared engine/client/`protocol-oteryn`, profile isolation and the still-open selection of the concrete Global target for each Reference revision.

Full `GAME-VISION-01` remains **NOT ACCEPTED**. No runtime/client/server/content/protocol/persistence/production implementation was authorized.

## Review and repair evidence

- Initial automatic review on older head `9f9ea9b9e299fa3faddcf1639c1b64ff4f219cc2` raised valid P2 comment `3756210156`: explicitly identify ADR-0010 §4 / Deferred Decisions scope being superseded.
- Repair added the scoped ADR-0010 authority/supersession section and preserved the exact Global baseline/date as unresolved.
- Repaired exact-head self-review: review `4904132069` — **PASS**, material findings `0`.
- P2 thread `PRRT_kwDOTuGrds6YJvvN`: replied with repaired-head evidence, resolved and outdated.
- Independent review: `NOT_REQUIRED` by current risk policy.

## Exact-head CI

Exact head `81a22469e90ccc4ac794d5e8c240dd91c3c3b530`:

- Agent Governance run `31471589712`: **success**
- Dependency Review run `31471589639`: **success**
- CodeQL run `31471589596`: **success**
- component/integration/E2E: `NOT_APPLICABLE` — architecture documentation only

## Merge and lifecycle closeout

- PR #158 squash-merged to `main` as `d31b23b82d4e62ee311adc48e8861dc8182af0b7`.
- This archive records terminal evidence and releases the active task ownership record.
- No remaining action belongs to this task.
