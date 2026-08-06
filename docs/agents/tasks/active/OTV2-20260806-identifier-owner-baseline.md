# OTV2-20260806-identifier-owner-baseline

```yaml
task_id: OTV2-20260806-identifier-owner-baseline
title: Record the owner-accepted FND-ID-01 pre-contract baseline
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-id-01-owner-baseline
pr: 56
base_sha: 26b5fa275fba19fdee0e26a6f65263489af3e500
head_sha: 6d8e15b88e5aeec750430dad675373d8fb16494d
owner: ChatGPT architecture coordinator
created_at: 2026-08-06T14:19:00+02:00
updated_at: 2026-08-06T14:24:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/agents/tasks/active/OTV2-20260806-identifier-owner-baseline.md
public_contracts:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
depends_on:
  - ADR-0001 through ADR-0011
  - FND-01 and VSL-02 destination cutover
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient
```

## Outcome

Persist the product owner's accepted four-class identifier model as a canonical architecture input without falsely claiming that the complete `FND-ID-01` gate has started or finished.

## Architecture and source of truth

- `PROVEN` — `main` at task start is `26b5fa275fba19fdee0e26a6f65263489af3e500`.
- `PROVEN` — the canonical destination workspace and client cutover are complete.
- `PROVEN` — the global register still requires the source-only `blakinio/otclient` historical marker before the full `FND-ID-01` package begins.
- `PROVEN` — the owner accepted the four-class identifier baseline on 2026-08-06.
- `DERIVED` — the accepted baseline may be recorded now as mandatory input while representation, exact type choice and the complete identifier catalogue remain unresolved.

## Acceptance criteria

- [x] Record the four identifier classes and cross-cutting invariants exactly enough to govern later analysis.
- [x] Keep UUID/ULID/database-column/wire-width and complete catalogue choices unresolved.
- [x] Preserve the historical-marker ordering gate.
- [x] Make no runtime, protocol, schema, migration or external-repository change.
- [x] Review the exact branch diff and open a documentation-only PR.
- [ ] Obtain repository validation and independent audit before merge.

## Excluded scope

- no `protocol-oteryn` schema or codec;
- no Rust identifier types;
- no PostgreSQL representation;
- no Game Session or lease token format;
- no write to `blakinio/otclient`;
- no claim that `FND-ID-01` is accepted or complete.

## Implementation / findings

The owner accepted this baseline:

1. durable cross-boundary identities are stable, immutable, non-reused and semantically opaque;
2. scoped identities are meaningful only with their owning scope;
3. runtime-local references use generation-fenced handles and never escape as durable/public identity;
4. revisions, generations and sequences are ordering/fencing values, not entity identities;
5. names, slugs and display numbers are labels or lookup aliases, not canonical identity.

The exact branch comparison against `main` contained only:

- `docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md`;
- `docs/agents/tasks/active/OTV2-20260806-identifier-owner-baseline.md`.

Draft PR #56 records the change and keeps merge blocked on repository validation and independent audit.

## Validation

### Focused

- command/run: GitHub compare `26b5fa275fba19fdee0e26a6f65263489af3e500...docs/fnd-id-01-owner-baseline`
- result: `PASS` — two new Markdown files, 393 additions, no deletions, no executable or external-repository change

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only documentation
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- head: pending after this checkpoint update
- workflow/run: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: `PASS` for the initial PR head
- unresolved review threads: pending
- related/superseded PRs: none identified
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Draft PR #56 opened with the owner-accepted four-class identifier baseline and a documentation-only changed-file review.
status: validating
branch: docs/fnd-id-01-owner-baseline
head_sha: 6d8e15b88e5aeec750430dad675373d8fb16494d
pr: 56
ci_check_generation: pending after checkpoint update
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: Full FND-ID-01 remains ordered after the source-only blakinio/otclient historical marker; this PR additionally requires exact-head validation and independent audit before merge.
next_action: Verify the exact final PR head and repository validation state.
```
