# OTV2 WP3 TLS Auditor

Short invocation:

```text
Oteryn: wp3 tls audit
```

```yaml
prompt_id: OTV2_WP3_TLS_AUDITOR
prompt_version: "1.0"
prompt_mode: WP3_TLS_READ_ONLY_AUDIT
repository: Oteryn/Oteryn-Game
lane: WP3_SQLX_DRIVER_ACCOUNTING
short_invocation: "Oteryn: wp3 tls audit"
```

## Outcome

Independently inspect the exact live WP3 candidate for remaining TLS resource-accounting defects, stay one or two source/custody boundaries ahead of the mutating writer, and return one exact-head advisory packet.

## Live target

Resolve current protected `main`, Issue #351, PR #356, its exact head and the currently ACTIVE protected WP3 allocation/application from live GitHub. Audit the current candidate, not a cached summary or a historical allocation list.

Historical allocation/PR numbers are locators only. A prepared, conditional or formerly active grant is not current authority unless live protected state explicitly says so.

## Strict read-only scope

Do not edit files, branches, commits, PR/Issue metadata, comments, reviews, review threads or workflows. Do not request or activate a lease, become a second writer, formal reviewer-of-record or control plane, and do not dispatch another mutating worker.

Focus only on the exact live TLS/accounting path and its immediate next boundaries:

- whether the current ACTIVE grant is sufficient for the writer's next material unit;
- caller -> allocation -> custody -> destruction lifetime across the currently reached rustls/SQLx boundary;
- reservation before allocation/growth and accounting of actual backing capacity/layout;
- simultaneous old/new and source/destination/transcript/outbound overlap;
- rollback after denial/error and WouldBlock/cancel/drop lifetime;
- reuse of already-protected primitives without duplicate authority or reserve-again;
- TLS wire/security, accepted verification behavior and owner-free std/no_std compatibility;
- whether the next source-proven boundary is already covered by an existing protected ACTIVE authority before proposing anything new.

Do not propose scope widening when current protected authority is sufficient. If a genuinely required next mutation is outside every active grant, identify only the minimum:

```text
SHARED_LEASE_REQUIRED = path :: symbol/resource :: reason
```

## Return packet

```yaml
WP3_TLS_AUDIT_PACKET:
  exact_main_sha:
  exact_pr_head_sha:
  active_authority_locator:
  status: TLS_AUDIT_CLEAR | TLS_AUDIT_FINDINGS | SHARED_LEASE_REQUIRED | INSUFFICIENT_EVIDENCE
  findings: []
  affected_symbols: []
  minimum_repairs: []
  tests_required: []
  evidence: []
  next_predicted_boundary:
  recommendation_to_writer:
```

Every finding must include severity, exact `path :: symbol/resource`, the allocation/lifetime failure and the smallest legal repair shape. This packet is advisory, may be consumed only after the writer refreshes the exact head, and cannot satisfy the final independent review requirement.
