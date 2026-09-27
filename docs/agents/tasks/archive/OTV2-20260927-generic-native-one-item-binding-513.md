# OTV2-20260927-generic-native-one-item-binding-513

```yaml
task_id: OTV2-20260927-generic-native-one-item-binding-513
title: Generic native one-item binding specialization
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/generic-native-one-item-binding-513
issue: 162
pr: 1013
base_sha: 3c9aef87a4dff22b9a8308d82a55f5bfe8469bf6
head_sha: 678d38253a3b834702ebe9fcbea11c1043a14966
final_head_sha: 678d38253a3b834702ebe9fcbea11c1043a14966
final_head_frozen_at: 2026-09-27T19:12:14Z
owner: allocated documentation author
created_at: 2026-09-27T21:01:40+02:00
updated_at: 2026-09-27T21:39:41+02:00
execution_policy: continuous_progress
owned_paths:
  - MODIFY docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - ADD docs/agents/tasks/active/OTV2-20260927-generic-native-one-item-binding-513.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on:
  - #162 allocation comment 5858771487
  - DUR-03 §§39.1-39.2
  - GAME-ITEM-01, ANL-01, Content typed identity, VSL-COMBAT-01
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

B1 closes the generic native one-item representation/admission contract only.
PR #1013 source head `678d38253a3b834702ebe9fcbea11c1043a14966` is protected-integrated
as main `708b3ef8f6c6a7328f886238c1de5842923a77f9`. This does not close
gate B, physical MINT/TRANSFER, Character readiness, or playable Combat.

## Architecture and source of truth

- **PROVEN** — B1 added the generic native item definition/state, committed
  deterministic loot cause, actual Ground context, direct-root
  CharacterInventory binding, expected-binding/current-authority separation,
  MINT-first staging, and explicit migration-0009 Character composition gate.
- **PROVEN** — protected source PR #1013 is merged at `2026-09-27T19:29:52Z`;
  its exact source head is `678d38253a3b834702ebe9fcbea11c1043a14966` and protected
  merge/main head is `708b3ef8f6c6a7328f886238c1de5842923a77f9`.
- **PROVEN** — separate exact-head Sol review comment `5858965938` records PASS
  for frozen head `678d38253a3b834702ebe9fcbea11c1043a14966`, with no P0-P3
  findings. This is independent task-review evidence, not a formal GitHub
  approving review or integration authority.
- **PROVEN** — source PR merge gate run `36343444074` / `game-gate` job
  `108688102754` and post-ready Architecture run `36343828557` / job
  `108688942340` succeeded for source head `678d38253a3b834702ebe9fcbea11c1043a14966`.
- **PROVEN** — native Merge Queue receipt UUID
  `4a58049c-31bd-4745-8c9a-abbb9a552c76` returned exact target and same-receipt
  GET reported enqueued; queue entry was
  `MQE_lQDOT8SzxM8AAAABFYZyHc4AA-xczgL8zaU`.
- **PROVEN** — actual merge-group run `36343892565` used head/main
  `708b3ef8f6c6a7328f886238c1de5842923a77f9`, sole parent
  `e2e2038b0f1020751f87b75df174cdfadc9b430c`, one commit and exactly the two
  allocated documentation paths with source blobs preserved. Every job
  succeeded; aggregate `game-gate` job `108690979356` succeeded.
- **PROVEN** — protected contract blob was
  `67e0de468d63dc74fc982a8e9a1e80cf90952414`, 63,476 LF bytes, SHA-256
  `97219675e6854608c79c8520ee037a444ff59137b70a17ff66e146ab0e75c958`.
- **PROVEN** — protected active task blob was
  `ddbb4c09b7815926a6f30d51ed3e73e1d3e56a04`, 8,019 LF bytes, SHA-256
  `a99fe81e0438e135b18cebaa6a580d36db79625d6e5e53dd6801a153213bf9fc`.
- **PROVEN** — `python tools/repository/validate_repository_policy.py` failed
  only because protected-base `LICENSE` does not match pinned canonical MPL-2.0.
  This is unchanged protected-base evidence; LICENSE was neither waived nor
  modified and is outside B1's candidate paths.
- **DERIVED** — retaining the archive packet preserves lifecycle evidence while
  leaving dependent admission/runtime gates with their owners.
- **CONFLICT** — migration 0009 does not define how a non-XP CharacterInventory
  transfer composes with its global CharacterRevision/XP-receipt chain.
- **UNKNOWN** — source/receipt grammar, destination legality/capacity, Character
  revision composition, native schema/profile/resource limits and physical
  MINT/TRANSFER recovery evidence remain unresolved owner gates.

## Acceptance criteria

- [x] Generic native definition/state binding reuses the existing Content typed-definition/revision grammar and GAME-ITEM legality.
- [x] MINT source is a committed deterministic death/output occurrence; caller UUIDs do not confer authority; non-reuse survives restart/audit expiry.
- [x] Ground binds actual WorldId/ChannelId and accepted map/content/native-room/typed-position context.
- [x] CharacterInventory is direct-root CharacterId plus owning typed position; no InventoryRootId; TRANSFER remains gated on legal destination admission.
- [x] Expected immutable bindings are separate from independently current Content, session/lease, source and runtime authority.
- [x] Character initialization/readiness and migration-0009 composition conflict are routed as separate dependencies.
- [x] Product values, IDs, registry ceilings, SQL and runtime permission remain undecided.
- [x] Source task and evidence were archived without claiming playable Combat or the lifecycle candidate's future SHA.

## Excluded scope and open gates

This task closes B1 representation/admission contract only. The following remain
OPEN: B2 native schema/codec measurement; B3 Content/Item legality and product
admission; B4 registry/profile/resources; separately allocated Character
initialization/revision readiness; C physical MINT/TRANSFER; D generic
death/loot/reward orchestration; E room/Seam/protocol/native-client composition;
F terminal qualification; and playable Combat.

R7 P03 is only a later integrated XP dependency. No R7 path or lease was taken.
Canary/Crystal remain read-only. Product unknowns remain: quantity, probability,
stack maximum, capacity, formulas, HP/damage/XP values, event/protocol IDs and
registry ceilings. The source task changed no runtime, SQL, migration, registry,
schema, protocol, room, Movement, Server Seam, client, Cargo, workflow, or foreign
path.

## Lifecycle closeout allocation

The separate lifecycle allocation is #162 comment `5859106277`, on branch
`codex/generic-native-one-item-binding-closeout-513` from protected main
`708b3ef8f6c6a7328f886238c1de5842923a77f9`. Its sole owned paths are deletion of
the active task packet and addition of this archive copy. Root remains the sole
publisher/control plane. Lifecycle-candidate publication, exact readback and
freeze remain pending outside this archived B1 task record; no future lifecycle
head SHA is asserted here.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: PASS — repository governance validator accepted the task lifecycle packet.
- command/run: `python tools/repository/validate_repository_policy.py`
- result: FAIL — unchanged protected-base LICENSE mismatch noted above; no waiver or LICENSE change.
- command/run: `git diff --check`
- result: PASS — lifecycle archive candidate has no whitespace errors.
- command/run: `python -m pytest -q tools/agents/tests/test_governance_lifecycle_discovery.py`
- result: PASS — 2 passed.

### Component/integration

- command/run: NOT_APPLICABLE — B1 is a documentation-only representation/admission contract.
- result: NOT_APPLICABLE.

### E2E

- scenario: NOT_APPLICABLE — B1 does not claim physical MINT/TRANSFER or playable Combat.
- result: NOT_APPLICABLE.

### Exact-head CI

- source head: `678d38253a3b834702ebe9fcbea11c1043a14966`
- source PR run: `36343444074`; `game-gate` job `108688102754`; SUCCESS.
- post-ready Architecture run: `36343828557`; job `108688942340`; SUCCESS.
- merge-group head/main: `708b3ef8f6c6a7328f886238c1de5842923a77f9`; sole parent `e2e2038b0f1020751f87b75df174cdfadc9b430c`.
- merge-group run: `36343892565`; aggregate `game-gate` job `108690979356`; all jobs SUCCESS.
- merge time: `2026-09-27T19:29:52Z`.

## Self-review

- exact lifecycle archive delta: active packet removal plus same-name archive addition on base `708b3ef8f6c6a7328f886238c1de5842923a77f9`
- method/reviewer: root coordinating agent performed mandatory whole archive/delta review.
- material findings: no P0-P2.
- verdict: PASS for the lifecycle archive delta; separate Sol source review is recorded below.

## Independent review

- required: YES — B1 records item identity, value and authority boundaries.
- exact head: `678d38253a3b834702ebe9fcbea11c1043a14966`
- method/auditor: separate Sol FINAL_CANDIDATE_REVIEW, PR comment `5858965938`.
- material findings: no P0-P3 findings.
- verdict: PASS; advisory task-review evidence, not a formal GitHub approving review or merge authority.

## PR and closeout

- changed-file review: PASS — one source commit, exactly the two allocated documentation paths, +291/-0.
- unresolved review threads: no blocker reported in the terminal allocation evidence.
- related/superseded PRs: none recorded here.
- protected auto-merge: native exact-target `merge-async` receipt and enqueued readback as recorded above.
- merge commit/result: PR #1013 merged to protected main at `708b3ef8f6c6a7328f886238c1de5842923a77f9`; merge-group aggregate passed.
- ownership release: B1 source task complete; separate lifecycle archive candidate publication/readback remains pending outside this file.

## Context checkpoint

```yaml
last_progress: B1 source PR #1013 integrated at protected main 708b3ef8f6c6a7328f886238c1de5842923a77f9; this record archived under separate lifecycle allocation
status: completed
branch: codex/generic-native-one-item-binding-513
head_sha: 678d38253a3b834702ebe9fcbea11c1043a14966
pr: 1013
final_head_sha: 678d38253a3b834702ebe9fcbea11c1043a14966
final_head_frozen_at: 2026-09-27T19:12:14Z
ci_trigger_source: PR #1013 plus actual merge_group run 36343892565
ci_check_generation: source head and integrated merge-group head recorded above
ci_checks_for_current_head: 19
ci_run_ids: [36343444074, 36343828557, 36343892565]
ci_job_ids: [108688102754, 108688942340, 108690979356]
runner_assignment_state: terminal_success
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 9
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: root completes the separately allocated lifecycle candidate and fresh protected archive readback; this file asserts no future candidate SHA
```
