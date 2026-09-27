# OTV2-20260927-r7-p04-gold-coin

```yaml
task_id: OTV2-20260927-r7-p04-gold-coin
title: R7 P04 stable Gold Coin reference definition
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/r7-p04-gold-coin
issue: 162
pr: null
base_sha: ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root
created_at: 2026-09-27T13:11:46Z
updated_at: 2026-09-27T13:34:08Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/evidence/OTV2-20260927-r7-p04-gold-coin.json
  - docs/agents/tasks/active/OTV2-20260927-r7-p04-gold-coin.md
  - content/world/definitions/reference.json
  - content/world/manifest.json
  - content/world/content.lock.json
  - content/world/project.json
  - tools/content-migration/world_project_v2_to_tree.py
  - content/items/definitions/items-00000-00499.json
  - content/items/definitions/items-00500-00999.json
  - content/items/definitions/items-01000-01499.json
  - content/items/definitions/items-01500-01999.json
  - content/items/definitions/items-02000-02499.json
  - content/items/definitions/items-02500-02999.json
  - content/items/definitions/items-03000-03499.json
  - content/items/index.json
  - content/manifest.json
  - content/content.lock.json
  - content/project.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446
  - zimbadev/crystalserver@9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
allocation_comments:
  - 5856164543
  - 5856176777
jira_story: KAN-12
```

## Outcome

Promote exactly one Oteryn-owned Reference Content identity,
`oteryn:item.currency.gold_coin@definition-r1`, from the protected full Item
family. The record is a materializable, stack-capable carrier whose typed
semantics remain wholly unknown. It is not runtime activation or gameplay parity.

## Exact mapping

- Protected source coordinate: CrystalServer item ID `3031`.
- Protected catalogue row: index `2948`, node digest
  `9528fadc4f8fdf66e7937d15e3843b3f39b35ae47ae8a737d93ca76efe652124`.
- Current opaque allocation: sequence `2921`, key
  `oteryn:item.registry.i00002921`.
- Native target: `oteryn:item.currency.gold_coin@definition-r1`.
- Opaque key `oteryn:item.registry.i00003031` belongs to source item ID `3147`
  and must remain unchanged.

The numeric source coordinate and opaque sequence are deliberately separate.
Neither number is native identity.

## Architecture and source of truth

- `PROVEN`: protected WorldProject/v2 owns the current 38,157-record Reference
  Item family and canonical identity sorting.
- `PROVEN`: GAME-ITEM-01 permits the existing materializable/stack-class carrier;
  typed stack semantics require a known cap before `stackable=true` can be emitted.
- `PROVEN`: allocation comments `5856164543`, `5856176777` and `5856199822`
  bind the writer, exact paths, corrected source mapping and generated shard range.
- `CONFLICT`: Canary records weight `10`; CrystalServer records weight `1`; both
  current wiki revisions describe `0.01 oz`. No weight field is admitted.
- `UNKNOWN`: runtime behavior, maximum stack, economy authority and Global parity.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: P04 edits Reference Content and deterministic generators only.
It performs no production mutation, session/lease/generation decision, durable
write, PREPARE/COMMIT, controller installation or recovery interpretation.

## Acceptance

- [x] Exact evidence bytes and protected catalogue coordinates are verified.
- [x] Exactly one identity is replaced; the family remains 38,157 records.
- [x] The new record is `materializable=true` and `StackCapable`.
- [x] All typed semantics, including `semantics.stack`, remain unknown.
- [x] `max_stack=100`, weight, value and exchange rate are absent from target semantics.
- [x] Canary/CrystalServer weight conflict and exact wiki revisions are retained.
- [x] Canonical WorldProject/v2 and successor authoring tree regenerate byte-exactly.
- [x] Focused Rust, migration, repository package and governance checks pass.
- [ ] Frozen exact-head independent Luna review and hosted checks pass before MQ.

## Evidence boundaries

Canary and CrystalServer are pinned read-only implementation evidence. TibiaWiki
BR and Fandom are pinned descriptive evidence. Oteryn selects the native key.
The sources do not authorize a stack maximum, economy semantics, runtime use,
DUR-03 MINT/TRANSFER, Global parity, licensing disposition or production claims.
The server source ID is not a MediaWiki page ID, so this slice adds no false
TibiaWiki identity binding.

## R7 boundary after P04

P04 only makes one stable reference carrier available. Durable XP application,
production DUR-03 closure, Gold Coin MINT/TRANSFER and a playable loop remain
separate R7 gates. The aggregate Jira Story stays active until those gates and
the later breadth/qualification packets are complete.

## Excluded scope

Runtime activation, Item instances, inventory custody, MINT/TRANSFER, DUR-03
transactions, persistence/SQL, protocol/client behavior, economy rules, stack
maximum, typed weight, licensing disposition and Global parity.

## Implementation / findings

The protected importer applies one exact evidence-bound overlay after the existing
69-field semantic promotion. It updates the record, candidate binding, reimport
identity and allocation digest together, preserves source `3147` at unrelated
opaque key `i00003031`, and re-sorts canonical identities. The generator advances
the package revisions and regenerates the six affected successor shards.

Pre-write reconstruction corrected the proposed source mapping from opaque
sequence `3031` to sequence `2921`. That correction was recorded on #162 before
code or generated content changed.

## Validation

### Focused

- WSL LF `cargo test -p oteryn-game-server --test content_world_cw2_b1_import --locked`:
  PASS, 16 tests.
- WSL LF `cargo test -p oteryn-game-server --test content_world_project_repository --locked`:
  PASS, 3 tests.
- Strict Clippy for the two tests and materializer example with `--no-deps -- -D warnings`:
  PASS; the pre-existing vendored Tokio documentation warning remains outside `--no-deps`.
- `cargo fmt --all -- --check`: PASS in the LF qualification checkout.

### Component/integration

- Canonical materializer: PASS, 11 documents, 38,157 Items, tree digest
  `b50e5980a9c5b4e08b45f7275880c7c3441172f6c0ea45faba214b831c4e1a4a`.
- WorldProject/v2 regeneration: byte-equal across all 11 package locators.
- Successor tree regeneration: byte-equal across 137 tracked content/import files.
- Migration validator: PASS, 38,157 Items, 252 Mounts, 165 Item bindings,
  164 authoring entries, 221 relations and 1,022 provenance facts.
- Successor inventory test: PASS, 89 managed files and 77 Item shards.
- Governance validator: PASS, 22 policy documents and 9 project lanes.
- Repository policy validator: PASS, 23 files and 43 workflows.

### E2E

`NOT_APPLICABLE`: P04 introduces no runtime consumer or playable transaction.

### Exact-head CI

- final head: pending remote freeze
- trigger source: pending PR
- workflow/run/job: pending
- runner assignment: pending
- classification: server-consumed evidence and game-server code; hosted routing authoritative
- result: pending

## Self-review

- exact head: pending remote freeze
- method/reviewer: `/root`, complete allocated-delta and provenance-boundary review
- material findings: corrected source-ID versus opaque-sequence mapping before code mutation; no open material finding in the complete allocated delta
- verdict: PASS for the local authoring candidate; exact remote SHA review remains pending freeze

## Independent review

- required: YES; native identity/content compiler change
- exact head: pending remote freeze
- method/auditor: independent GPT-6 Luna subagent
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending final staged candidate
- unresolved review threads: pending PR
- related/superseded PRs: draft PR #952 is not adopted and has no path overlap
- protected auto-merge: pending native Merge Queue
- merge commit/result: pending
- ownership release: pending protected-main readback

## Context checkpoint

```yaml
last_progress: local implementation, deterministic LF qualification and governance checks complete
status: implementing
branch: codex/r7-p04-gold-coin
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
next_action: stage the exact allocated delta, commit, publish and freeze one remote candidate
```
