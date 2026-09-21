> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #677 merged as `f108cf6cd14c8e2af4edef62bee04fe6dd6996ad`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260919-wp3-v2-retained-config-limits

```yaml
task_id: OTV2-20260919-wp3-v2-retained-config-limits
title: Freeze WP3-v2 retained PostgreSQL configuration limits
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp3-v2-retained-config-limits-351
pr: 677
base_sha: c7688069bc22ac3cde46e48e6b05d8051418fed1
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: Oteryn: astra wp3-v2 architecture lead
created_at: 2026-09-19
updated_at: 2026-09-19
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_V2_SUPERSEDING_ARCHITECTURE_DECISION_2026-09-12.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260919-wp3-v2-retained-config-limits.md
public_contracts:
  - WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1
  - DUR-FRESH-RESOURCE-ENVELOPE-V1
depends_on:
  - issue:162
  - issue_comment:5743113461
  - pr:673
blocks:
  - WP3-A PR #673 P1-2 repair
  - WP3-A four-P1 exact-head requalification
cross_repository_coordination_id: null
external_repositories:
  - Oteryn/Oteryn-Platform@623435ec1b907d6d9770b767806c90300252a71c
```

## Outcome

Close only WP3-A PR #673 finding P1-2 by adding owning finite authority for the five retained production PostgreSQL configuration fields and their checked aggregate. Runtime repair remains with the existing WP3-A implementation writer after protected integration/readback of this amendment.

## Architecture and source of truth

- **PROVEN:** #162 comment `5743113461` grants this worker exact write custody for only the three paths listed above.
- **PROVEN:** protected Revision 3 requires explicit bounded retained configuration, pre-copy same-root reservation and lifetime charging but previously supplied no numeric field capacities.
- **PROVEN:** transition disposition N30 leaves retained-config capacities `UNKNOWN` pending exact source proof / owning acceptance.
- **PROVEN:** protected DFR keeps `DFR-TOTAL-RESIDENT-BYTES = 12,582,912` and the WP3-v2 equation `I + max(R,T) + Q + A <= 12 MiB`.
- **PROVEN:** PostgreSQL 17 documents the default identifier maximum as 63 bytes; the selected first-slice server profile is PostgreSQL 17.6.
- **PROVEN:** RFC 1035 limits DNS labels to 63 octets and a wire name to 255 octets, yielding a 253-byte maximum uncompressed presentation name without a trailing root dot.
- **PROVEN:** protected `NSRC-TLS-CERTS` already establishes an Oteryn conservative PKI envelope of four configured trust roots, 4,096 DER bytes each / 16,384 aggregate DER bytes.
- **PROVEN:** RFC 7468 generated textual encodings use 64-character base64 lines. With allowed LF/CRLF canonical text, 4,096 DER bytes require at most 5,692 PEM bytes per certificate, so four require 22,768 bytes.
- **PROVEN:** `Oteryn/Oteryn-Platform@623435ec1b907d6d9770b767806c90300252a71c:app/Http/Requests/Identity/LoginIdentityRequest.php` applies `max:1024` to password ingress. This is evidence for a conservative Oteryn secret ceiling, not Game authority or byte-unit inheritance.
- **DERIVED/OWNER DECISION:** the Game first-slice DB password ceiling is 1,024 UTF-8 bytes. PostgreSQL 17 SCRAM does not publish a smaller password-length requirement; larger service secrets are outside this first-slice profile and require a later owning amendment.

## Retained configuration decision

| field | hard maximum |
| --- | ---: |
| `tls_server_name` | 253 ASCII bytes |
| `database` | 63 UTF-8 bytes |
| `username` | 63 UTF-8 bytes |
| `password` | 1,024 UTF-8 bytes |
| `root_ca_pem` | 22,768 ASCII bytes |

Checked variable-input aggregate:

```text
253 + 63 + 63 + 1,024 + 22,768 = 24,171 bytes
```

All six registry entries are fixed first-slice maxima and use `UNAVAILABLE`, `client_visible=false`. Input length/grammar and checked aggregate arithmetic must run on borrowed/already-bounded source before retained allocation/copy/PEM parse. Actual owned capacities and fixed backing remain charged to the existing same-root `I` ledger; the 24,171-byte aggregate is not a new memory allowance.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/resource-contract amendment only; no production mutation, PREPARE/COMMIT, controller/session replacement, persisted recovery interpretation, credential use, database operation, or runtime source change is authorized.
```

## Acceptance criteria

- [x] Exact finite unit-explicit maximum exists for each of the five retained fields.
- [x] Checked aggregate retained-config input maximum is exact and overflow-safe by contract.
- [x] Over-limit data is rejected before retained copy/parse/allocation and maps to bounded internal `UNAVAILABLE`.
- [x] Same-root lifetime charging is preserved; no independent config allowance is introduced.
- [x] Registry boundary obligations cover each max/max+1 and aggregate max/max+1.
- [x] Existing 12 MiB DFR total and WP3-v2 R/T non-overlap model are unchanged.
- [ ] Exact-head repository/governance checks complete.
- [ ] Fresh independent HIGH whole-diff architecture/resource/security review complete with zero blocking material findings.
- [ ] Protected integration/readback complete before PR #673 implementation resumes.

## Excluded scope

No runtime Rust, PR #673 mutation, `fresh_admission.rs`, WP4/WP5/Server-Seam, Tokio/rustls/SQLx/vendor/Cargo/lock, migrations, workflows/rulesets, Platform/Atlas writes, production/deployment mutation, direct merge or generic auto-merge.

## Implementation / findings

P1-2 is treated as an owning resource-contract gap. This task does not repair P1-1/P1-3/P1-4 and does not claim PR #673 ready. The amendment makes no statement that PostgreSQL universally has these product limits; they are the accepted finite first-slice Oteryn profile.

The pre-copy rule is intentionally stronger than the current PR #673 constructor shape: receiving arbitrary already-owned unbounded `String`/`Vec<u8>` values and checking them afterward cannot satisfy P1-2 because allocation has already occurred.

## Validation

### Focused

- registry JSON parse and six-entry exact-value/assertion check: required on candidate exact head
- exact changed-path set must equal the three allocated paths
- arithmetic proof: `4 * (27 + 2 + 5464 + 86*2 + 25 + 2) = 22,768`; aggregate `24,171`
- governance validator: required on candidate exact head

### Component/integration

`NOT_APPLICABLE`: docs/contract-only amendment; no runtime component is changed.

### E2E

`NOT_APPLICABLE`: docs/contract-only amendment; configured PostgreSQL execution belongs to the resumed PR #673 repair after protected readback.

### Exact-head CI

- final head: external PR/check evidence
- trigger source: normal PR checks
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: external PR evidence
- method/reviewer: implementing architecture lead
- material findings: pending final whole-diff readback
- verdict: pending

## Independent review

- required: YES — owning resource/security limits unblock a HIGH-risk durability implementation
- exact head: external PR evidence
- method/auditor: fresh independent HIGH whole-diff architecture/resource/security review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending exact-head readback
- unresolved review threads: pending
- related/superseded PRs: PR #673 remains parked; no supersession
- protected auto-merge: forbidden to this worker; coordinator/governed Merge Queue only after acceptance
- merge commit/result: pending
- ownership release: only after protected integration/readback

## Context checkpoint

```yaml
last_progress: P1-2 finite retained-config owner decision authored within exact three-path custody
status: validating
branch: agent/wp3-v2-retained-config-limits-351
head_sha: external_pr_evidence
pr: 677
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: normal_pr
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
```
