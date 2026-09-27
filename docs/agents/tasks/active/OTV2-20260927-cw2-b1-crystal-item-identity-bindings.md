# OTV2-20260927-cw2-b1-crystal-item-identity-bindings

```yaml
task_id: OTV2-20260927-cw2-b1-crystal-item-identity-bindings
title: CW2-B1 Crystal Item identity bindings (explicit EXACT, 38,157 rows)
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: ccef9bbe8ecf75c4b1b99b07237c78ebf4642dce
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-27T00:00:00Z
updated_at: 2026-09-27T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-census/g4_item_crystal_binding_generator.py
  - tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - imports/crystalserver/bindings/items.json
  - docs/agents/tasks/active/OTV2-20260927-cw2-b1-crystal-item-identity-bindings.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md"
  - "PR #670 protected-merged (CW2-B1 item identity catalogue + allocator)"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
jira: KAN-16
```

## Outcome

The committed CW2-B1 allocator (`apps/game-server/src/content/cw2_b1_import.rs`)
already assigns every one of the 38,157 `content/items/definitions/*.json` Item
keys from Crystal `data/items/items.xml` `id` values, ascending, via the frozen
evidence catalogue. That mapping was only implicit allocator logic; per G4
doctrine (`docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`
section 3) an external identifier must become an explicit typed
`(target, source, identity_namespace, external_id, disposition)` binding, not
an inference. This task makes those existing assignments explicit as
`imports/crystalserver/bindings/items.json`: 38,157 `EXACT` bindings, Crystal
only (Canary is out of scope). No canonical identity is minted; no
`content/items/definitions/*.json` record is touched.

## Architecture and source of truth

- `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md` §3: PROVEN, gives the binding tuple shape and namespace examples (`ots/item_server_id`, `mediawiki/page_id`, ...).
- `apps/game-server/src/content/cw2_b1_import.rs`: PROVEN, `protected_cw2_b1_full_item_family_import` walks ascending `source_item_id` from `semantic_catalog.identity_records`; 64 `NATIVE_ITEM_BATCH` rows keep their semantic key, all others get `oteryn:item.registry.i%08d` via `opaque_sequence`.
- `docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`: PROVEN, 16,877,870 bytes, sha256 `7836c78c...d96a7`, matches the allocator's pinned constants.
- `imports/tibiawiki/bindings/items.json`: PROVEN, existing sibling `OTERYN_SOURCE_IDENTITY_BINDINGS/v1` format this file matches field-for-field, ordered by ascending `canonical_bytes`.
- Crystal `items.xml`'s `id` attribute is the OTServer-family server-side item type id, i.e. doctrine's `ots/item_server_id` example, distinct from `clientid`/appearance identity (which would bind Presentation, never Item).

## Acceptance criteria

- [x] `tools/content-census/g4_item_crystal_binding_generator.py` independently re-derives the allocator's mapping (parsing `NATIVE_ITEM_BATCH` from the Rust source, never hand-copied) and fails closed on: wrong evidence digest/bytes, non-ascending/duplicate `source_item_id`, wrong opaque/native counts, duplicate native key, an allocated key absent from `content/items/definitions/*.json`, a definitions key with no allocation, or the Magic Sword golden cross-check (Crystal `3288` -> `oteryn:item.registry.i00003167`, matching TibiaWiki page `5810`) disagreeing.
- [x] Generated `imports/crystalserver/bindings/items.json` has exactly 38,157 `EXACT` bindings, `identity_namespace: ots/item_server_id`, `source_key: oteryn:source.crystalserver`, `source_revision: ff7ede593c69d4c658b382c97443e8155926924a`, every `target.key` present in `content/items/definitions/*.json`.
- [x] `tools/content-census/g4_item_crystal_binding_generator_self_test.py` exercises the fail-closed paths above at small scale.
- [x] `python tools/agents/validate_governance.py`, the content-tree migration validators and ruff pass (see Validation).

## Excluded scope

- No Canary bindings (owner decision: Crystal only, this task).
- No embedding of `source_bindings` into `content/items/definitions/*.json` records: only 165/38,157 records currently carry any embedded `source_bindings` (the TibiaWiki-matched subset, mirroring `imports/tibiawiki/bindings/items.json` exactly); no validator or doctrine text requires the same for the standalone `imports/crystalserver/bindings/items.json`, and `.github/workflows/content-tree-migration.yml` triggers only on `imports/tibiawiki/bindings/**`, not `imports/crystalserver/**`. Embedding into all 38,157 records would be a much larger, unrequired change, so it was not done; flagged here for an explicit owner call if full parity is later wanted.
- No `content/items/definitions/*.json`, `content/world/**`, or `imports/tibiawiki/**` mutation.
- No gameplay-field promotion, no Presentation/Asset/runtime identity change.
- No commit/push: prepared for owner review per session instruction.

## Validation

### Focused

- command/run: `venv/bin/python tools/content-census/g4_item_crystal_binding_generator.py --check`
- result: PASS bindings=38157 bytes=10864254
- command/run: `venv/bin/python tools/content-census/g4_item_crystal_binding_generator_self_test.py`
- result: PASS
- command/run: `venv/bin/python -m ruff check tools/content-census/g4_item_crystal_binding_generator*.py`
- result: only `EXE001` (shebang on a non-executable file), a pre-existing condition shared by every committed sibling in this directory (e.g. `g4_item_binding_pilot.py`); no repo ruff config and no CI workflow runs ruff over this directory, so this is non-gating.

### Component/integration

- command/run: `venv/bin/python tools/agents/validate_governance.py`
- result: PASS ("Governance validation passed for Oteryn/Oteryn-Game.")
- command/run: `venv/bin/python tools/content-migration/test_world_project_v2_to_tree.py && venv/bin/python tools/content-migration/validate_world_project_v2_to_tree.py`
- result: both PASS, unaffected (neither reads `imports/crystalserver/**`)

### E2E

- scenario: NOT_APPLICABLE (auxiliary import-provenance data; `imports/**` classifies as `auxiliary`/`unconsumed-auxiliary-inputs` per `tools/repository/classify_pr_test_lanes.py`, confirmed by `tools/repository/test_classify_content_routing.py` fixtures already listing `imports/crystalserver/*.json`; no Rust/game-gate build is triggered by this change)

### Exact-head CI

- final head: pending (not committed; owner reviews and commits per session instruction)
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: expected `auxiliary` (no `rust`/`windows` lane)
- result: pending

## Self-review

- exact head: pending (uncommitted)
- method/reviewer: implementing agent
- material findings: the embedded-`source_bindings` question above (documented under Excluded scope) is the one doctrine-adjacent judgment call in this change; no validator requires it, and doing it would exceed the requested bounded change.
- verdict: ready for owner review and commit

## Independent review

- required: YES (owner explicitly requested review-before-commit for this identity/provenance change)
- exact head: NOT_APPLICABLE (uncommitted)
- method/auditor: pending (owner)
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending (owner)
- unresolved review threads: NOT_APPLICABLE (no PR yet)
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: generated and verified imports/crystalserver/bindings/items.json (38,157 EXACT rows) plus generator/self-test; all found validators pass; owner to review and commit
status: implementing
branch: claude/compassionate-albattani-s29syw
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
owner_action_required: "review diff and commit (or request changes) -- no git write was performed by this session per instruction"
blocker: null
next_action: owner reviews the three new files and commits
```
