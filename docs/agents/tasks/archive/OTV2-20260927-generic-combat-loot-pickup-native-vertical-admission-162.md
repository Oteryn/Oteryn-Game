# OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162

```yaml
task_id: OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162
title: Generic Combat loot and pickup native vertical admission
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/generic-combat-loot-pickup-admission-162
issue: 162
pr: 1005
base_sha: 2b95309a1fe19bfbac43b7d463d668e7ddf396b4
head_sha: 59e3df4da99ce1ecef1affdc53e7f2fa46d582e4
final_head_sha: 59e3df4da99ce1ecef1affdc53e7f2fa46d582e4
final_head_frozen_at: null
owner: /root/combat_vertical_admission_author
created_at: 2026-09-27T17:45:00Z
updated_at: 2026-09-27T18:29:46Z
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
  - remaining B-F gates for whole-unit playable Combat acceptance
cross_repository_coordination_id: null
external_repositories: []
allocation_comment: 5858204757
```

## Outcome

Admission A is protected-integrated by PR #1005. B-F and playable Combat remain
open. R7 P03 is an integrated XP dependency only; no R7 path or lease was taken.

## Architecture and source of truth

- `PROVEN`: base `2b95309a1fe19bfbac43b7d463d668e7ddf396b4`, branch
  `codex/generic-combat-loot-pickup-admission-162` (#162 `5858204757`), source
  `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4`.
- `PROVEN`: #506 binds Stage-C; DUR-03 §§39.1-39.2 require separate one-item
  Ground MINT and CharacterInventory TRANSFER; item P90D remains binding.
- `PROVEN`: R7 P03 is the separate integrated XP dependency. No second XP engine,
  shared XP/loot transaction, R7 path or lease was taken.
- `REQUIRED GATE`: Character-owned progression initialization/readiness must be
  proven through its authorized route and bound to a fresh, separately allocated
  owner revision/policy before D/E admission or generic Combat XP settlement.
  No initializer design or value is selected here.
- `PROVEN`: reuse the entry room, boot pin, qualification workflow and PR #961
  Movement step; preserve start/east return. Content/Seam owns room edits.
- `PROVEN`: owner-selected Canary `47dfd51f` and CrystalServer `ff7ede593` are
  pinned read-only behavioral/ordering references. They are not numeric/product
  authority, implementation-copy permission or Reference-parity proof.
- `UNKNOWN`: native schema/profile/resource acceptance, product inputs below and
  runtime terminal evidence. A integration leaves B-F open.

Live GitHub remains lifecycle authority. #162 comment `5858509270` allocates
this local archive closeout. KAN-12 remains programme context; this worker made
no GitHub or Jira API writes.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: Documentation-only admission; no current production mutation, PREPARE/COMMIT, persisted recovery consumer, authority controller or runtime integration is implemented here. Future implementation children must complete their own applicable authority/recovery qualification before freeze.
```

Runtime authority/recovery proof belongs to future implementation children; this
documentation admission proves none.

## Acceptance criteria

- [x] The architecture text records accepted Stage-C status without reopening
  the authority model and distinguishes documentation from runtime proof.
- [x] The complete generic chain and separate item MINT/TRANSFER and XP workflow
  are explicit.
- [x] At least two deterministic definition/revision fixtures are required for
  the same generic code; Rat/cheese/Gold Coin are fixture-only identities.
- [x] Existing native room, Movement proof cells, activation pin and #961
  integration are preserved; no second room/harness is admitted.
- [x] A-F dependencies and whole-unit terminal criteria are stated, including
  Character progression readiness, product unknowns, exclusions and recovery.
- [x] Admission A is protected-integrated on exact main `dc84a5fd2510c43c052ded432c37fd52217141fb`.
- [ ] B-F and full playable native Combat acceptance remain open; no partial child
  is playable Combat evidence.

## Excluded scope

No runtime/schema/registry/protocol/Content/client/SQL/migration edits; no PR
#1004/R7 paths or lease; no second room/harness, synthetic authority, invented
identity/schema, duplicate domain resource, OTS numeric adoption, deployment,
parity or partial playable claim. Content/Seam leases any room edit.

## Implementation / findings

The independent review returned one P2 on the frozen predecessor: Character-owned
progression initialization/readiness was not explicit before D/E and XP settlement.
It was fixed before the reviewed source head by requiring an authorized route and
a fresh, separately allocated Character owner revision/policy binding. No
initializer, policy values, XP values or SQL layout was designed. D consumes only
integrated R7 P03 and one calculator; no R7 path or lease was taken.

Source head `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4` received exact-head
independent review PASS with no P0-P2. The two merge-gate runs, queue entry and
terminal protected integration are recorded below. Admission A is integrated;
B-F, real runtime qualification and all product inputs remain open/unknown.

Product inputs remain `UNKNOWN`: damage/lethality/HP; loot probability/quantity;
XP amount/formula; presentation assets; inventory capacity/stack legality; and
Movement-compatible spawn/cell layout. Owner evidence/acceptance and exact OTS
comparisons do not imply product values are admitted.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`;
  `python tools/repository/validate_repository_policy.py`; `git diff --check`;
  archive status/path, task-size, UTF-8/no-BOM, LF/final-LF and whitespace checks
- result: governance PASS (22 policy documents/9 lanes); lifecycle and archive
  size/integrity PASS (10,319 chars/210 lines); exact status is delete from
  `docs/agents/tasks/active/OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162.md`
  and add `docs/agents/tasks/archive/OTV2-20260927-generic-combat-loot-pickup-native-vertical-admission-162.md`;
  `git diff --check` PASS. Repository policy reports
  `LICENSE does not match the pinned canonical MPL-2.0 text`; `LICENSE` is
  unchanged from protected base `dc84a5fd2510c43c052ded432c37fd52217141fb`.

### Component/integration

- command/run: `NOT_APPLICABLE` for this documentation-only authoring child;
  future implementation children own component/integration qualification
- result: no runtime behavior claimed

### E2E

- scenario: required at F; real native desktop + real Game PostgreSQL against
  the existing native entry room, including restart/retry and Movement proof
- result: pending; no playable Combat claim

### Exact-head CI

- source PR head: `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4`
- source Merge gate: run `36339454077`, job `game-gate` `108676832719` — success
- post-ready Architecture gate: run `36339673355`, job `108677165031` — success
- queue request UUID: `210f3783-9f87-4346-b306-6e8d754b84ef`
- queue entry: `MQE_lQDOT8SzxM8AAAABFX_0iM4AA-xczgL8tuI`
- terminal merge group: run `36339731701`; head/merge/main
  `dc84a5fd2510c43c052ded432c37fd52217141fb`; base
  `b48d61c687b11110de6df5c60fb5e3c166f117c6`; aggregate `game-gate`
  `108678703763` and all jobs success; merged `2026-09-27T18:19:41Z`.
- result: protected integration PASS; exact blob readback follows.

## Self-review

- exact source head: `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4`
- method/reviewer: allocating author; bounded two-path working-tree review
- material findings: prior PR #1005 P2 fixed before final reviewed head
- verdict: PASS for authoring; protected integration verified at terminal group

## Independent review

- required: YES; cross-owner architecture admission
- exact head: `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4`
- method/auditor: independent exact-head review
- material findings: no P0-P2 on final source head
- verdict: PASS

## PR and closeout

- changed-file review: PR #1005 exact candidate `59e3df4da99ce1ecef1affdc53e7f2fa46d582e4`
- unresolved review threads: none reported; independent review PASS, no P0-P2
- related/superseded PRs: B-F dependency work remains separate
- Merge Queue: request UUID and entry recorded above; terminal aggregate PASS
- merge commit/result: protected `main@dc84a5fd2510c43c052ded432c37fd52217141fb`, merged `2026-09-27T18:19:41Z`
- protected blob readback at `main@dc84a5fd2510c43c052ded432c37fd52217141fb`: architecture `06582ee4455ff2ba2f85dba3ba9bd3749291ab82`; merged active task packet `8deeb9dad19ac0f4461f0a4ad2f35f2e12c7983b`
- ownership release: admission A only; B-F remain open; no R7 path/lease

## Context checkpoint

```yaml
last_progress: admission A protected-integrated via PR #1005 and exact terminal Merge Queue proof
status: completed
branch: codex/generic-combat-loot-pickup-admission-162
head_sha: 59e3df4da99ce1ecef1affdc53e7f2fa46d582e4
pr: 1005
final_head_sha: 59e3df4da99ce1ecef1affdc53e7f2fa46d582e4
final_head_frozen_at: null
ci_trigger_source: protected Merge Queue merge_group
ci_check_generation: 36339731701
ci_checks_for_current_head: 1
ci_run_ids: [36339454077, 36339673355, 36339731701]
ci_job_ids: [108676832719, 108677165031, 108678703763]
runner_assignment_state: completed
terminal_ci_wait_started_at: 2026-09-27T18:19:41Z
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none for A; B-F admission/implementation and product inputs remain
blocker: playable Combat remains open pending whole-unit B-F acceptance
next_action: continue with separately allocated B-F dependencies; do not claim playable Combat before full terminal proof
```
