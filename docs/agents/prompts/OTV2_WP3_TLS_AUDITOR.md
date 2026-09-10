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

Independently inspect the exact live WP3 candidate for remaining TLS resource-accounting defects and return one exact-head advisory packet to the WP3 writer.

## Live target

Resolve current protected `main`, Issue #351, PR #356 and its exact head from live GitHub. Audit the current candidate, not a cached summary.

## Strict read-only scope

Do not edit files, branches, commits, PR/Issue metadata, comments, reviews, review threads or workflows. Do not request a lease or become a second writer/reviewer-of-record/control plane.

Focus only on:

- #518 `emit_client_kx` allocation/custody and custom-verifier accounting;
- #501 transcript/hash ownership;
- #427 PSK-binder viability and whether #493 is genuinely necessary;
- simultaneous-live source/destination/transcript/outbound overlap;
- reservation-before-allocation and actual destruction-bound release;
- WouldBlock/error/cancel/drop lifetimes;
- TLS wire/security and owner-free std/no_std compatibility.

Do not propose scope widening when current protected authority is sufficient. If a real boundary exists, identify the minimum:

```text
SHARED_LEASE_REQUIRED = path :: symbol :: reason
```

## Return packet

```yaml
WP3_TLS_AUDIT_PACKET:
  exact_main_sha:
  exact_pr_head_sha:
  status: TLS_AUDIT_CLEAR | TLS_AUDIT_FINDINGS | SHARED_LEASE_REQUIRED | INSUFFICIENT_EVIDENCE
  findings: []
  affected_symbols: []
  minimum_repairs: []
  tests_required: []
  evidence: []
  recommendation_to_writer:
```

Every finding must include severity, exact `path :: symbol`, the allocation/lifetime failure and the smallest legal repair shape. This packet is advisory and cannot satisfy the final independent review requirement.
