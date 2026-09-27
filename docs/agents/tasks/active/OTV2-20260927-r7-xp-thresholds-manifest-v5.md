# OTV2-20260927-r7-xp-thresholds-manifest-v5

```yaml
task_id: OTV2-20260927-r7-xp-thresholds-manifest-v5
title: Register bounded R7 experience thresholds in Reference manifest revision 5
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/r7-xp-thresholds-manifest-v5
issue: 483
pr: 1019
base_sha: 1a2f28ff67156462de78effb6003f9c4b19f301a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: R7_XP_THRESHOLDS_MANIFEST_V5
created_at: 2026-09-27T19:35:40Z
updated_at: 2026-09-27T19:54:18Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/agents/evidence/OTV2-20260927-r7-xp-thresholds-source-packet.md
  - docs/agents/tasks/active/OTV2-20260927-r7-xp-thresholds-manifest-v5.md
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
depends_on:
  - "#483"
  - "#486"
  - "protected manifest revision 4"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Advance the protected Reference evidence/parity registry exactly once from manifest revision 4 to 5. Preserve schema v1 and all revision-4 cases/history, then add three bounded `DERIVED` Character progression candidates: cumulative level-7 threshold `2,600`, cumulative level-8 threshold `4,200`, and their arithmetic span `1,600`. Keep Oteryn implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`, and exact target-boundary continuity unresolved.

## Architecture and source of truth

- `PROVEN`: the protected revision-4 manifest and normative schema establish the accepted registry structure, target ID, domain vocabulary, independent target/implementation/parity axes and fail-closed classifications.
- `DERIVED`: level-7 and level-8 cumulative XP thresholds are admitted only at the evidence ceilings recorded in the public source packet.
- `DERIVED`: the level-7 span is exact arithmetic over the two bounded threshold candidates and inherits their target-continuity ceiling.
- `UNKNOWN`: exact 2026-07-28 boundary continuity, death loss, awarded kill XP, bonuses, modifiers, order of operations, delevel behavior and rounding remain outside these cases.
- `OTS_HYPOTHESIS_ONLY`: pinned Canary and CrystalServer code is retained as read-only implementation/migration evidence and is excluded from target-authority source lists.
- `NOT_STARTED`: Oteryn has no accepted exact threshold implementation or fixture bound by this evidence change.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is a public documentation/evidence registry revision. It performs no Combat, Server Seam, Movement, native-room, loot-runtime, durability, protocol, client, SQL, production, credential, authority-bearing session, persistence recovery, PREPARE/COMMIT or live-account operation.

## Acceptance criteria

- [x] Exact protected revision-4 manifest is used as the predecessor.
- [x] Only the three allocated R7 paths are changed.
- [x] Normative schema v1 is byte-identical.
- [x] `schema_version=1`, `manifest_revision=5`, target ID unchanged, and `canonical_digest=null`.
- [x] All eleven revision-4 case objects and all four revision-4 history objects are structurally unchanged.
- [x] Exactly three unique Character cases are appended and each remains `DERIVED` / `NOT_STARTED` / `PARITY_PENDING_EVIDENCE`.
- [x] Official pre/post-target observations, pinned TibiaWiki BR/Fandom revision metadata, capture digests and field dispositions are recorded in the public source packet.
- [x] Canary and CrystalServer are pinned at exact commits/blobs and retained only as `OTS_HYPOTHESIS_ONLY`.
- [x] No death-loss, awarded-kill-XP, bonus, modifier, ordering, delevel or rounding assertion is introduced.
- [x] Pinned SourceMeta metaschema/instance validation, JSON parse, semantic invariants, LF/no-CR checks and diff checks pass locally.
- [x] Exact successor head/tree and exact three-path delta are frozen by live GitHub readback.
- [ ] Fresh GPT-6 Luna whole-diff review reports zero unresolved P0/P1/P2/P3 findings.
- [ ] Exact-head hosted CI and governed Merge Queue integration complete.
- [ ] Protected `main` readback proves the exact manifest/source/task blobs.

## Excluded scope

Schema mutation/versioning; Runtime, Combat, Server Seam, Movement, native-room, loot-runtime, DUR-03, Content, protocol, client, SQL, persistence, workflow or production code; digest computation; implementation fixtures; death or kill arithmetic; bonuses and modifiers; natural loot probabilities; OTS promotion; direct merge; generic auto-merge.

## Implementation / findings

- Generated revision 5 from the exact protected revision-4 JSON and preserved its first eleven case objects plus first four history objects structurally.
- Added `character.progression.experience_threshold.level_7.v1` at `2,600` with `MEDIUM_HIGH` confidence.
- Added `character.progression.experience_threshold.level_8.v1` at `4,200` with `HIGH` confidence because an official pre-target auction, a target-near pinned community revision and an older pinned community formula agree.
- Added `character.progression.level_xp_span.level_7.v1` at `1,600` as exact subtraction with `MEDIUM_HIGH` confidence inherited from both inputs.
- Kept all three cases fail-closed at `DERIVED`; exact target-boundary primary capture remains unavailable.
- Retained the post-target official table only as a current-state observation in the source packet and excluded it from all three cases' target sources and confidence basis because continuity from the immutable target is unproven.
- Described the pinned Fandom revision as formula evidence; direct substitution yields both values, while its dynamic table module remains unpinned and unused for the numeric derivation.
- Inspected exact Canary and CrystalServer player implementations. Their matching cumulative-XP formula is useful as a future fixture hypothesis but grants no Global or Oteryn authority.
- Inspected the protected Character XP commit seam: it accepts a caller-supplied finite progression policy and does not itself pin these numeric thresholds, so `NOT_STARTED` is the truthful manifest state.
- The exact candidate containing this packet is bound by live branch/PR readback and the lifecycle freeze comment after publication; this packet intentionally does not self-assert its own commit ID.

## Validation

### Focused local qualification

- SourceMeta JSON Schema CLI `v16.3.0`, Windows x86_64 release asset SHA-256 `a1020938168f1abdf1147e7d98a42592f15140b478fcb255e19b7d90d0d2a5a6`.
- `jsonschema metaschema docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json`.
- `jsonschema validate docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`.
- `python -m json.tool docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`.
- Semantic invariants: schema/target identity, exact revision-4 preservation, unique case IDs, exactly three appended cases, exact expected values, allowed classifications, no parity confirmations, explicit artifact anchors, truthful source precision and exact owned paths.
- LF/no-CR/final-LF checks and `git diff --check` on the exact candidate.

### Component/integration

- Repository Agent Governance, Architecture Semantic Audit and Merge Gate must qualify the exact frozen PR head. Every successor generation requires a new freeze and fresh qualification.

### E2E

- `NOT_APPLICABLE`: documentation/evidence registry only; no runnable product behavior changes.

## Independent review

- required: `YES`
- method: fresh GPT-6 Luna whole-diff evidence, provenance, classification and lifecycle review on the exact frozen successor head
- status: pending exact-head publication

## Context checkpoint

```yaml
last_progress: exact successor published and externally frozen by #162 comment 5859208184; exact-head review and hosted gates are in progress
status: validating
branch: codex/r7-xp-thresholds-manifest-v5
head_sha: null
pr: 1019
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: live_github_pr_events
ci_check_generation: exact_successor_head_from_external_freeze_record
ci_checks_for_current_head: in_progress
runner_assignment_state: assigned
terminal_ci_checks_for_current_generation: in_progress
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: complete fresh exact-head independent review and hosted gates; any material repair requires an explicit return to AUTHORING and a new external freeze record
```
