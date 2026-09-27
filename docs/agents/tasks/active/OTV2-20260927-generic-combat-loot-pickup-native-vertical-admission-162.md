# OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162

```yaml
task_id: OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162
title: Generic Combat loot and pickup native vertical admission
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/generic-combat-loot-pickup-admission-162
issue: 162
pr: null
base_sha: 2b95309a1fe19bfbac43b7d463d668e7ddf396b4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/combat_vertical_admission_author
created_at: 2026-09-27T17:45:00Z
updated_at: 2026-09-27T17:45:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/active/OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162.md
public_contracts:
  - docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md
depends_on:
  - "#506 Stage-C owner acceptance and combat death workflow/resource gate"
  - "#513 DUR-03 one-item transaction resource and audit admission"
  - "KAN-12 programme Story context as routed by #162; no Jira mutation"
blocks:
  - whole-unit playable Combat terminal acceptance until children A-F pass
cross_repository_coordination_id: null
external_repositories: []
allocation_comment: 5858204757
```

## Outcome

Admit one generic, real-boundary Game delivery chain from native player intent
through legal GAME-ABILITY commit, one creature-death generation, deterministic
legal loot, separate one-item DUR-03 MINT, pickup, separate DUR-03 TRANSFER into
native CharacterInventory, authoritative native-client observation, and
restart/retry proof. XP settles separately through integrated R7 P03 Character
APIs. This packet and the architecture update are not runtime implementation or
playability evidence.

## Architecture and source of truth

- `PROVEN`: exact protected base is
  `main@2b95309a1fe19bfbac43b7d463d668e7ddf396b4`; branch allocated as
  `codex/generic-combat-loot-pickup-admission-162` under #162 comment
  `5858204757`.
- `PROVEN`: live #506 says Stage-C VSL-COMBAT-01 acceptance is binding and
  supplies the death -> separate loot/XP descendants -> acknowledged loot ->
  retry-safe pickup authority chain. #506 does not itself grant implementation,
  registry or production authority.
- `PROVEN`: accepted DUR-03 §§39.1-39.2 define separate one-item MINT and
  TRANSFER semantics and native audit-binding prerequisites. MINT creates one
  fresh item in typed Ground custody with applicable corpse association;
  TRANSFER moves that same item to direct-root CharacterInventory. Missing
  native schema/profile/security/resource acceptance keeps physical admission
  closed. Existing item-specific P90D remains binding.
- `PROVEN`: current GAME-ITEM CharacterInventory family and integrated R7 P03
  API are the item destination and separate persistent XP owner. No second XP
  engine or shared XP/loot transaction is permitted.
- `PROVEN`: existing protected native entry room, boot activation pin,
  `content_native_entry_room.rs`, native-entry qualification workflow and
  integrated PR #961 Movement step supply the current preproduction Movement /
  Server Seam environment. Preserve start/east step-and-return proof cells.
  Content/Seam owner owns room/source changes.
- `PROVEN`: owner-selected Canary `47dfd51f` and CrystalServer `ff7ede593` are
  pinned read-only behavioral/ordering references. They are not numeric/product
  authority, implementation-copy permission or Reference-parity proof.
- `UNKNOWN`: native typed schema/event/profile and accepted security purpose;
  registered resource limits and measured physical transaction/audit/outbox
  evidence; exact product inputs enumerated below; and terminal real-PostgreSQL /
  native-desktop evidence.
- `DERIVED`: staged dependency-safe PRs can reduce implementation risk, but A
  remains open until the whole F acceptance matrix and protected integration
  evidence are complete.

Live GitHub remains lifecycle authority. The control plane owns allocation
reconciliation, publication, review dispatch, Merge Queue and closeout. Jira
KAN-12 is programme context only; mapping and any synchronization stay with the
coordinator. This worker makes no GitHub or Jira API writes.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: Documentation-only admission; no current production mutation, PREPARE/COMMIT, persisted recovery consumer, authority controller or runtime integration is implemented here. Future implementation children must complete their own applicable authority/recovery qualification before freeze.
```

Future whole-unit recovery acceptance must independently exercise current
session, Character lease, runtime/scope, actor-local generation and content/revision
fences; exact retry identity; lost acknowledgement and ambiguous commit; source
non-reuse; one-winner pickup concurrency; and restart/replay on real Game
PostgreSQL with native-client observation. Persisted expected bindings never
replace fresh authority evidence.

## Acceptance criteria

- [x] The architecture text records accepted Stage-C status without reopening
  the authority model and distinguishes documentation from runtime proof.
- [x] The complete generic chain and separate item MINT/TRANSFER and XP workflow
  are explicit.
- [x] At least two deterministic definition/revision fixtures are required for
  the same generic code; Rat/cheese/Gold Coin are fixture-only identities.
- [x] Existing native room, Movement proof cells, activation pin and #961
  integration are preserved; no second room/harness is admitted.
- [x] A-F dependency owners, leases/serialization boundaries, whole-unit terminal
  DoD, product unknowns, authority exclusions and recovery boundaries are stated.
- [ ] Whole-unit terminal acceptance: every A-F matrix row has exact evidence,
  then exact-head review/CI/MQ and protected-main readback prove the complete
  playable native Combat path. Until then A remains open and no partial child may
  claim playable Combat.

## Excluded scope

No runtime/code/schema/registry/protocol/Content/client/SQL/DDL/resource or
migration changes; no PR #1004 or R7 P03 owned paths absent a new shared lease;
no second room or harness; no synthetic fixture IDs/bytes promoted as native
production data; no invented InventoryRootId, root/source grammar, event/field
IDs or security purposes; no duplicate Interaction/DUR/Character resources; no
schema/API layout selection; no Canary/Crystal numeric adoption; no production,
deployment, parity, or partial playable-Combat claim. Do not alter
`native_entry_room.json`; any necessary revision belongs to a separately leased
Content/Seam owner.

## Implementation / findings

The executable dependency sequence is A admission -> B native owner bindings,
schema/profile and measured resources -> C separate physical DUR-03 MINT and
TRANSFER with receipt/audit/outbox/recovery proof -> D generic death/loot plus
separate XP orchestration through R7 P03 -> E existing-room Server Seam/protocol/
native-client composition -> F real native desktop + real Game PostgreSQL
restart/anti-dup terminal qualification. Registry files, `durability/mod.rs`,
migration numbering, `foundation/mod.rs`, `runtime_actor_carrier.rs`, protocol
and client composition require serialized exclusive leases. PR #1004 and R7 P03
paths are excluded unless the live control plane grants a new shared lease.

No evidence in a child substitutes for F: concurrent pickup must have one winner;
duplicate command/death/source replay cannot duplicate; exact candidate bytes and
semantic envelope must be retained; lost ACK and ambiguous commit must reconcile;
restart before/after each transaction must retain one item; stale authority and
revision fences must reject before mutation; one authoritative location and no
corpse ghost must remain; the source cannot be reused; audit/outbox pressure must
fail closed; and accepted resource maxima, max+1 and overflow must be checked
before allocation/mutation. Owner-lane DB waits are forbidden. Protocol fixtures
must be independent and include malformed/gap/resync cases. Exact-head required
review, CI, Merge Queue aggregate `game-gate` and protected-main readback complete
the repository terminal gate.

Product values deliberately remain `UNKNOWN`: entry-room damage/lethality and
HP math; loot probability and quantity; XP amount/formula; client presentation
assets; inventory capacity/stack legality; and spawn/cell arrangement consistent
with Movement proof. Future owners must supply exact evidence and owner acceptance
before selecting values. Pinned OTS references only allow comparison/presentation.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`; `git diff --check`;
  UTF-8/no-BOM, LF-only/final-LF and trailing-whitespace checks; exact two-path
  `git status --short` readback
- result: PASS — governance validated 22 policy documents and 9 lanes;
  diff/encoding/whitespace passed. Exact status: `M
  docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md`;
  `?? docs/agents/tasks/active/OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162.md`.
  Architecture: 32,499 bytes; SHA-256
  `e6076145df6f0bd694e9e44e64217589f54a164ea71ff841429ee60d110faf17`; Git
  blob `0d4a5d4ac765c87ea6cc85453f23595286a8ad31`. Task manifest returned
  externally to avoid self-reference.

### Component/integration

- command/run: `NOT_APPLICABLE` for this documentation-only authoring child;
  future implementation children own component/integration qualification
- result: no runtime behavior claimed

### E2E

- scenario: required at F; real native desktop + real Game PostgreSQL against
  the existing native entry room, including restart/retry and Movement proof
- result: pending; no playable Combat claim

### Exact-head CI

- final head: pending root publication/freeze
- trigger source: root control plane
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending root publication/freeze
- method/reviewer: `/root/combat_vertical_admission_author`; complete working-tree
  two-path diff review, including authority, acceptance and exact path scope
- material findings: no P0-P2 findings
- verdict: PASS for authoring; exact remote head still pending root publication/freeze

## Independent review

- required: YES; material accepted-architecture delivery admission with cross-owner
  durability, gameplay, client and protocol boundaries
- exact head: pending root publication/freeze
- method/auditor: independent reviewer selected by control plane
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending exact candidate
- unresolved review threads: pending PR
- related/superseded PRs: dependency work is not whole-unit acceptance
- protected auto-merge: pending exact-head Merge Queue
- merge commit/result: pending `merge_group` `game-gate` and protected-main readback
- ownership release: root control plane only

## Context checkpoint

```yaml
last_progress: focused author validation and complete working-tree two-path self-review passed; no P0-P2 findings
status: implementing
branch: codex/generic-combat-loot-pickup-admission-162
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
next_action: root whole-diff self-review, then atomic expectedHeadOid publication/freeze
```
