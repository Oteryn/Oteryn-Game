> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #674 is merged on protected main as `c7688069bc22ac3cde46e48e6b05d8051418fed1`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings-504

```yaml
task_id: OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings-504
title: CW2-B2 creature and spawn native-binding source batch
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b2-creature-spawn-bindings-504
issue: 162
pr: null
base_sha: ebc860d7cd12bb855228a48759c4cc37b828da63
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import
created_at: 2026-09-19T15:38:17+02:00
updated_at: 2026-09-19T15:43:17+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/creature_spawn_binding_catalog.py
  - tools/reference-world-corridor-census/creature_spawn_binding_catalog_self_test.py
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings-504.md
  - docs/agents/evidence/OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings.json
public_contracts: []
depends_on:
  - protected PR #671 / CW3-B1
  - protected PR #670 / CW2-B1 evidence as read-only context
blocks:
  - CW2-B3 until B2 handoff
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
```

## Outcome

Build the smallest deterministic candidate-only CW2 product that preserves exact
legacy monster definition/source identities and monster spawn occurrence/source
identities, classifies native Oteryn Content binding disposition, and retains
typed candidate observations from the existing Game-owned exporters.

The batch does not create native creature, spawn, or item identity. The protected
tree exposes no admitted explicit legacy-source to native Content binding, so the
full current source set truthfully remains `UNRESOLVED`.

## Architecture and source of truth

- **PROVEN** — protected admission Game SHA:
  `ebc860d7cd12bb855228a48759c4cc37b828da63`.
- **PROVEN** — pinned migration corpus:
  `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- **PROVEN** — primary world file blob:
  `3829f7dcd9091ace8549bfb36a00186726076898`, size `10,097,048`.
- **PROVEN** — exact protected exporter blobs are recorded and verified in the
  tracked evidence JSON.
- **PROVEN** — source bytes are read from exact Git objects, not working-tree
  line-ending transforms.
- **DERIVED** — aggregate consumed-source bytes SHA-256:
  `387640f7d581ad29dd553badf9f1cf102d3003ce6460c46c56aa8db0e6aa2a12`.
- **PROVEN** — source classification remains
  `OTERYN_LEGACY / MIGRATION_EVIDENCE`; production authority is `NONE`.
- **PROVEN** — selected closure is `CANDIDATE_ONLY`.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: source/evidence catalogue only; no production mutation, PREPARE/COMMIT, authority session, persisted recovery interpretation, or native Content promotion
```

## Acceptance criteria

- [x] Exact pinned source repository/revision and all consumed objects fail closed
  on provenance mismatch.
- [x] Existing Game-owned creature/spawn and gameplay exporters are reused rather
  than replaced by a competing parser.
- [x] Monster source definitions and spawn occurrences preserve separate source,
  Atlas/export, and native Content identity boundaries.
- [x] Native mapping disposition is explicit and partition-complete.
- [x] Display names, normalized names, Atlas entity IDs, record IDs, coordinates,
  lookType, and synthetic VSL keys cannot become native identity by inference.
- [x] Candidate presentation, stats, resistances, and spawn fields retain
  migration-only classification.
- [x] Loot rows remain deferred to CW2-B3 with no native ItemKey manufacture.
- [x] Duplicate normalized names, explicit conflicts, ambiguity, duplicate spawn
  source identity, and fake native-key promotion have focused fail-closed tests.
- [x] Input-order perturbation preserves canonical semantic output.
- [x] Two clean full runs over identical pinned bytes are byte-identical.
- [ ] Exact-head repository CI completes on the final PR head.

## Excluded scope

No writes to existing Atlas exporters, shared Content/Reference models, CW4/world
runtime, GAME-AI runtime, item B1/B3 implementation, NPC/service/dialogue/shop/
travel implementation, protocol/schema/stable-ID registries, Cargo/workspace/
lock, persistence/migrations, workflows/governance/resource limits, external
repositories, or production systems.

No canonical creature/spawn key decision is made. No Crystal/display/name/hash/
coordinate identity is promoted to an `oteryn:*` Content key. Synthetic
`oteryn:vsl.*` fixture identities remain forbidden as import binding authority.

## Implementation / findings

The mapper verifies the exact source Git revision and consumes exactly:

- 5 `*-monster.xml` files under the pinned world root;
- 1,802 `*.lua` files under the pinned monster root.

It materializes exact Git blob bytes into temporary isolated parser inputs and
then invokes the protected exporters. This avoids checkout line-ending transforms
while preserving the existing parser semantics.

Observed deterministic product:

- 1,800 monster definition source identities;
- 87,565 monster spawn occurrences;
- native creature mapping: 0 RESOLVED / 1,800 UNRESOLVED / 0 AMBIGUOUS / 0 CONFLICT;
- native spawn mapping: 0 RESOLVED / 87,565 UNRESOLVED / 0 AMBIGUOUS / 0 CONFLICT;
- 17,086 observed loot rows, all retained as B3-deferred/non-native;
- 0 normalized-name collision groups in the pinned corpus;
- 2 Lua helper files explicitly retained as unparsed source files:
  `rottenEssences.lua` and `grand_master_oberon_functions.lua`.

Tracked evidence size is 43,288,817 bytes. Product digest:
`f69a941a4953c9f3b2532e90ab6589c7a89126b1e8123b0ff2e8d88777f7853f`.

Two clean full generations produced the same 43,288,817-byte file and identical
file SHA-256:
`cad1e3755b7154efde08501cee874c24a416a475b38fd4574db91f49a9e89a62`.

## Validation

### Focused

- `python -B tools/reference-world-corridor-census/creature_spawn_binding_catalog_self_test.py`
  — PASS.
- Negative coverage includes duplicate normalized names, ambiguous candidate
  bindings, conflicting asserted bindings, duplicate spawn source identity,
  candidate-only non-promotion, invalid Atlas key promotion, and synthetic VSL
  target rejection.

### Component/integration

- `python -B tools/game-atlas-creatures/self_test.py` — PASS.
- `python -B tools/game-atlas-creature-gameplay/self_test.py` — PASS.
- `python -B tools/reference-world-corridor-census/content_source_batch_self_test.py`
  — PASS.
- Two clean full B2 generations — PASS / byte-identical.
- `python -B tools/agents/validate_governance.py` — PASS.
- `git diff --cached --check` — PASS.
- Exact changed-path readback — PASS, exactly four allocated custody paths.

### E2E

- scenario: NOT_APPLICABLE — candidate-only offline source/evidence batch; no
  executable Reference promotion or runtime behavior is authorized.
- result: NOT_APPLICABLE.

### Exact-head CI

- final head: pending
- trigger source: pending PR
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending; final SHA recorded externally after commit
- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: 0
- verdict: `PASS_ZERO_MATERIAL_FINDINGS`
- evidence checks: source/native identity partitions, B3 loot deferral, product-digest recomputation, mapper/evidence hash binding, forbidden VSL/native-key promotion paths

## Independent review

- required: NO
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE
- reason: bounded candidate-only importer/evidence tooling with no shared model,
  protocol, production authority, or native identity promotion; allocation
  requires whole-diff self-review plus normal exact-head CI.

## PR and closeout

- changed-file review: PASS — exactly four allocated custody paths
- unresolved review threads: pending
- related/superseded PRs: CW2-B1 #670, CW3-B1 #671
- protected auto-merge: forbidden for worker
- merge commit/result: coordinator-owned
- ownership release: pending handoff

## Context checkpoint

```yaml
last_progress: local candidate qualified; deterministic evidence and adversarial whole-diff review PASS
status: ready
branch: agent/content-world-cw2-b2-creature-spawn-bindings-504
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
next_action: commit and non-force push exact candidate, open PR, then require exact-head CI
```
