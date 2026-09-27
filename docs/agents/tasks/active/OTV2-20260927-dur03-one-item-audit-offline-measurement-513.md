# OTV2-20260927-dur03-one-item-audit-offline-measurement-513

```yaml
task_id: OTV2-20260927-dur03-one-item-audit-offline-measurement-513
title: Offline closed one-item candidate audit schema and measurement
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-one-item-audit-offline-measurement-513
pr: null
base_sha: a822326c9cf4607100e58bbc3673748f3fa299bb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/dur03_offline_writer
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
classification: NONPRODUCTION_OFFLINE_CANDIDATE_SCHEMA_MEASUREMENT
issue: 513
jira_story: KAN-12
allocation: https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5855106063
writer_count: 1
owned_paths:
  - docs/contracts/game-events/v1/item_transaction.proto
  - apps/game-server/examples/dur03_reference_one_item_resource_prototype.rs
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json
  - docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-offline-measurement-513.md
public_contracts: []
depends_on:
  - DUR-03 section 39.1 protected by PR 971 at the exact base
  - ANL-01 foundation envelope and shared limits at the exact base
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

`PROVEN`: this authoring delta adds an explicitly unregistered candidate IDL for
the protected closed MINT/TRANSFER aggregate and private matching prost structs
only in the existing standalone example. All five writable paths are allocated
exclusively by the live comment above. The source worktree is the managed
`dur03-offline-measurement` checkout at the stated exact base.

Bound META organization policy and API publication contract resolve at
`Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb` (policy 3.1.0).
No local commit has been selected for publication. The allocated preferred
publication primitive is one `createCommitOnBranch(expectedHeadOid=base)`
candidate containing the full bounded five-path delta. After publication the
returned head must equal live branch readback, with sole parent=base and exactly
the owned-path tree/blob delta; that SHA is then frozen outside this record.
No material repair or second write is permitted by this worker after freeze.

Jira mapping KAN-12 / W toku / readiness-active is the control plane's allocation
observation. This worker does not claim a fresh Jira mutation or Story completion.

## Acceptance and measured results

- [x] Closed MINT absence-to-live Ground, TRANSFER same-live-item to direct-root
  CharacterInventory; no arbitrary metadata or unsupported operation meaning.
- [x] Private prost carriers match candidate/foundation field tags; four literal
  hand-derived independent payload/envelope goldens and field/wire/length table.
- [x] Upstream prost 0.14.4, sha2 and serde_json used without dependency changes.
- [x] Checked finite shape before preallocation/encoded_len/encode; borrowed
  schema-specific preflight enforces ANL envelope/payload/string/depth jointly.
- [x] Reject malformed/truncated/wire/varint/group/size/revision/variant inputs;
  safe additive numeric unknown fields and reordered envelope fields tested.
- [x] Exact EventId, membership, semantic envelope, payload/hash and retained wire
  fixed before ambiguity; duplicates, abort/retry, corruption, identity-content
  conflict, conservation, privacy and read-only replay cases covered.
- [x] Actual retained payload/envelope/aggregate/carrier/vector sizes measured;
  prototype and synthetic measurements remain separate; production audit nulls
  retain EVIDENCE_GAP.
- [x] Repeated normalized output and --reverse-fixtures are byte equal.
- [ ] Remote candidate freeze and hosted qualification (control plane).

Candidate MINT/TRANSFER payload=155/223 bytes, full envelope and single-event
aggregate=346/462 bytes. Frozen inline carrier=576 bytes each; dynamic retained
vectors/strings=643/859 bytes. Final model inline=1968, dynamic=1502, total=3470.
Synthetic count1/16-byte budget probe is not an event measurement. Production
mandatory event count/bytes remain unknown/null. Quantity1 and private IDs are
fixtures, not accepted production maxima or Content mappings.

Normalized JSON: 11865 bytes including final LF;
SHA-256 `d1ea0f574542fc677d6d5f60170743b4e758594ad5e1fc69e6ba8aad412f3bdd`.
The paired evidence Markdown records method, exact sources, field/wire oracle,
host limitations, reproduction commands and complete excluded claims.

## Validation and self-review

`PROVEN`: authoring-delta Linux WSL checks from the exact base plus allocated files
in `/home/mole/dur03-offline-a822-513`, rustc/cargo 1.94.0,
host `x86_64-unknown-linux-gnu`: 30 focused tests, locked focused build, strict
example Clippy, full-workspace rustfmt, governance, repository policy,
architecture workspace boundaries and deterministic output equality PASS.
The executable has no build-time compiler attestation, so its toolchain identity
remains explicitly UNKNOWN; the observed tools are recorded separately.

Native Windows governance passes. Native focused server compilation is unavailable
because inherited Linux-only server modules cause 67 errors before the example
compiles. Native repository policy sees the CRLF LICENSE/pinned-MPL mismatch;
the exact LF Linux route passes. Initial CRLF scratch formatting failure was
resolved by recreating the base archive with core.autocrlf=false; unallocated
source files remain untouched. Existing Tokio missing_docs warning is preserved.

Self-review: the implementing worker reviewed the complete bounded delta,
checked authority boundaries, IDL/prost parity, every negative family, wire
arithmetic, independent oracle, immutable retries and retained-copy accounting.
Authoring findings fixed: reject partial transfer before publication; validate
frozen carrier before reconciliation as well as retry; measure the retained
candidate rather than regenerate it; distinguish inline from dynamic retention;
reject changed numeric fixture cause instead of silently mapping it to the original;
reject uninterpreted unknown byte fields while keeping safe numeric additivity.
No known material authoring finding remains. This is not independent approval.

AuthorityInvariant × ConsumerBoundary × MutationOperator: production applicability
`NOT_APPLICABLE`; no production mutation/controller/fence or persisted recovery
consumer changes. Independent fixture facts nevertheless cover missing, nil,
stale and mismatched world/channel/corpse/Character/session/definition/generation
at the offline validation boundary. Replay takes only an immutable Model borrow.
Time expiry/future authority, production concurrency and PostgreSQL reload are
`NOT_APPLICABLE` to this offline child, not proven runtime behavior.

Independent exact-head review selection, hosted PR/CI and merge eligibility remain
the control plane's responsibility. Worker funding/review trigger authority=NONE.
No PR exists at authoring time; no ready/validating status is claimed here.

## Excluded scope

Registries, foundation.proto, accepted DUR-03/ANL contracts, Cargo/build files,
runtime modules, foundation APIs, SQL and migrations are read-only. No registered
event type ID, item purpose/retention/access/export/expiry profile, numeric resource
acceptance, production schema admission, runtime activation, authority change,
PostgreSQL audit atomicity/restart, connected Combat/pickup, real Content mapping,
natural Rat loot probability or Reference parity is established.

No source filler fabricates a production maximum. No generic serialization
platform or dependency fork is added. No PR/comment/review trigger, ready state,
queue, merge, production, credential, protection or external-repository mutation
is authorized. Candidate measurements cannot close aggregate Jira acceptance.

## Context checkpoint

```yaml
last_progress: bounded five-path authoring delta and focused offline evidence prepared
status: implementing
branch: codex/dur03-one-item-audit-offline-measurement-513
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
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
owner_action_required: null
blocker: null
next_action: publish one expected-head API-native candidate then freeze and verify exact live head externally
```

This record is intentionally truthful before its containing candidate exists.
It cannot contain its own SHA. The final exact publication/freeze evidence must
be returned to the control plane without a follow-up checkpoint commit.
