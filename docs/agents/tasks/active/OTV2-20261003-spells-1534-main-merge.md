# OTV2-20261003-spells-1534-main-merge

```yaml
task_id: OTV2-20261003-spells-1534-main-merge
title: Bring PR 1534 (spell import r22) up to current main with one merge commit
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/spells-import-r22-20261002
pr: 1534
issue: 1622
base_sha: 8f618385
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: sole writer allocated by the control plane of coordination Issue 1622
created_at: 2026-10-03
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/ (renames 0032..0051 -> 0033..0052 and the D325 guard union, leased 0068)
  - apps/game-server/src/ability/condition.rs
  - apps/game-server/src/ability/condition_spell.rs
  - apps/game-server/src/ability/condition_tests.rs
  - apps/game-server/src/durability/
  - apps/game-server/src/foundation/
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/spell/
  - apps/game-server/tests/support/
  - docs/contracts/ (migration-number references only)
  - tools/content-schema/native-gameplay/README.md (migration-number references only)
  - apps/game-server/src/gameplay_transport/connection.rs
  - crates/protocol-oteryn/src/ (merge union and test lint allows only)
  - apps/game-server/tests/ (mount allows, D325 case, stack fix)
  - docs/agents/tasks/active/OTV2-20261003-spells-1534-main-merge.md
  - apps/game-server/src/gameplay_transport/ (merge adaptations to main)
  - apps/game-server/src/content/item_*_promotion.rs, docs/agents/evidence/OTV2-2026100*-item-*.json, tools/content-schema/item-authoring/lower_*.py (re-pin, owner decision 1a)
public_contracts: []
depends_on: [D313, D314, D319, D325, D339, D357]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PR 1534 merges current `main` through one merge commit (no rebase, force or history
rewrite) and stays semantically a union of both sides.

## Architecture and source of truth

- **PROVEN** D313 (owner 3a): `main` owns `0032_character_proficiency.sql`. The PR's
  never-merged migrations `0032_character_familiar_state.sql` .. `0051` move by +1 to
  `0033` .. `0052` with byte-identical SQL bodies; code, test-support and doc references
  to their numbers follow. Hash-bound historical r21 validation reports under
  `docs/reference/spells/r21-local-candidate/` record base `fafc51ef` paths and stay
  unchanged as evidence.
- **PROVEN** D314 (owner 4b): the runtime actor slot candidate budget is 184 bytes for the
  PR's boxed `spell_combat` on top of COND-1c's boxed player lifecycle (160).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: the merge adds no new authority, fence or recovery path; it unions
existing receipt-chain verification and renumbers unapplied migrations.

## Acceptance criteria

- [ ] Condition immunity: content immunities, SpellSkills->Attributes and
      SpellRegeneration->Recovery mapping, and Cleanse immunity all refuse (union).
- [ ] `take_due` / `take_due_non_damage` both prune expired Cleanse immunities.
- [ ] Revision chain unions familiar (0033), proficiency (0032) and quest (0056) receipts and verifiers.
- [ ] Slot size measured on the merged tree and within the D314 budget.
- [ ] fmt, clippy, game-server and protocol-oteryn tests, governance validators pass.

## Excluded scope

No new gameplay, no change to SQL bodies, no review trigger, auto-merge or Jira write.

## Implementation / findings

- Both sides added `ConditionStore::expire_non_ticking` with different semantics. `main`'s
  lifecycle owner version (Speed/Light/ManaShield, infallible) keeps its name; the PR's
  spell-owner version (every effect without a pending tick, time-guarded) becomes
  `expire_non_ticking_checked` and also prunes expired Cleanse immunities.
- Carrier patterns on the PR's `control_loss` field read `lifecycle.control_loss` after
  COND-1c's boxing.
- Creature health drain keeps both the PR's companion invisibility removal and `main`'s
  death cleanup of committed creature conditions.
- `main`'s Bestiary projection denies unknown Creature index fields; it now accepts the
  PR's `spell_imports` overlay descriptors as an ignored array.
- `CleansePlan::conflict_key` is test-only once `condition.rs` is mounted under `foundation`.
- D325 (control plane, option a): migration 0053 re-issues
  `game_character_progression_consistency_guard` as the union of the 0032 proficiency and
  0033 familiar bodies and restores its search_path pin; 0033..0052 stay byte-identical.
  `proficiency_and_familiar_writes_share_the_0053_progression_guard` interleaves both
  writers on one Character and re-verifies from a fresh authority (RED without 0053).
- Join snapshot (main ACH-NOTIFY-1): the PR's always-present empty world object overlay
  domain is kept and main's achievement notice domain follows it; main's notice
  expectation helper includes the empty overlay domain.
- D325 pre-freeze fixes: complete-reconnect future boxed off the test stack; clippy clean.
- D339: merged current main once (merge, not rebase); later predecessor merges only
  merge main.

- D357/D358 (history in git): server-seam future boxed; tile-aimed area visibility uses the
  sight origin; GuildStats extraction uses html.parser; spell samples regenerated; pre-write
  first-entry refusals retried from fresh reads (`RECONCILE_ATTEMPTS`). Open: r23 evidence-log
  whitespace (refused as evidence tampering), r25 item keys (ITEM-KEY-R25-1).
- Main merges (merge commits, union only): #1599 encounter domain from main; Wheel catalogue
  unions `spell_imports`; QUEST-STATE-1 moved the D325 guard union to leased
  `0068_character_progression_guard_union.sql` (0032 proficiency, 0033 familiar, 0056 quest
  arms, search_path pin, REVOKE), covered by
  `proficiency_familiar_and_quest_writes_share_the_0068_progression_guard`; no later main
  migration (0058, 0060) touches the guard. Content `index.json` files take main's
  `legacy_source` and keep the PR's `spell_imports`. Main's test stubs of
  `FreshAdmissionAuthority::step` take the PR's five parameters, and their join-snapshot
  expectations include the always-sent empty overlay domain. The protocol module list keeps
  `item_view` and `spell_presentation_candidate`. #1710's `ReferenceBaseVocation::None` is
  rejected (fail closed) by the equipment-claims projection: the equipment ABI admits the five
  base vocations only.

### Follow-ups (control plane)

- ITEM-KEY-R25-1 (CP 1c 2c): `item_key_references.py` reports 608 NON_CANONICAL
  `oteryn:item.source.canary.id<N>` keys in the r25 source world (emitted by
  `build_source_world.py`, required by `native_spell_world.rs` as `source-map-r3`) and the
  dangling `oteryn:item.tibia.i40450` (spell native profiles and the r25 test pack). Move
  the aliases to canonical `oteryn:item.tibia.i<N>` keys and author the i40450 Item record.
- PARTY-DECLINE (CP 1a): a Decline that removes the last invitation closes a leader-only
  party (`declining_the_last_invitation_closes_the_leader_only_inviting_party`).

## Validation

### Focused

- command/run: cargo fmt --all --check; cargo clippy -p oteryn-game-server -p
  oteryn-protocol-oteryn --all-targets -D warnings; governance validator and
  tools/agents tests; repository policy validator
- result: PASS

### Component/integration

- command/run: cargo test -p oteryn-game-server (local PostgreSQL 17.6 configured) and
  -p oteryn-protocol-oteryn
- result: PASS (see FREEZE report on PR 1534)

### E2E

- scenario: `NOT_APPLICABLE` (merge integration; CI owns E2E lanes)
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: control-plane owned
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- protected auto-merge: control-plane owned
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: single main merge plus D325 repairs validated
status: validating
branch: codex/spells-import-r22-20261002
head_sha: null
pr: 1534
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
next_action: report FREEZE to the control plane; afterwards only merge main when predecessors land
```

## Re-pin after merging origin/main 8588b1bb (owner decision 1a, 2026-10-04)

Merging main (after #1712) left only this PR's own `reference_playable.rs` drift: the file moved from `5424c687ef3b` to `fd81a9d9359e`. The current pins of the numeric17 receipt and the Mantra/Bond source qualification were moved to the new digest. Each packet was regenerated with its own generator, and the pins were cascaded on top of #1712's pins until every `--check` passed. Every changed file is identical to its pre-merge content once 64-hex digests are masked. The historical witnesses (magic-capacity4 source qualification, numeric13 source proof v2, numeric17 source qualification v1, native-loader proof) stay unchanged.

Merge adaptations: `release_terminal` (main's refactor of the abandoned release) keeps this PR's familiar save and spell-training save around the monk save. The `item_view_tests.rs` test authority (#1713) takes the PR's five-parameter `step` signature, and its expected join snapshot carries the PR's always-sent empty overlay domain.

| File | Old digest (12) | New digest (12) | Result |
|---|---|---|---|
| `apps/game-server/src/content/item_elemental_magic_modifier_promotion.rs` | `b1e786ac7c72` | `7fe2bac00cf4` | pins only |
| `apps/game-server/src/content/item_mantra_bond_modifier_promotion.rs` | `ffbde846a435` | `a7725fd37cbc` | pins only |
| `apps/game-server/src/content/item_numeric_modifier_promotion.rs` | `5ec194692eca` | `92ef6b774a53` | pins only |
| `apps/game-server/src/content/item_stack_default_successor8_promotion.rs` | `f6a81742b7ed` | `ca28a8eedbed` | pins only |
| `evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json` | `bcb8410d4020` | `ebbf84a95092` | digest-only |
| `evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json` | `64c051bbe564` | `ec0c44a1f85f` | digest-only |
| `evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json` | `c7145af1f27e` | `e4b92c87087c` | digest-only |
| `evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json` | `be0b91420416` | `d965b48f56bc` | digest-only |
| `evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json` | `1a8ffbaa48be` | `5d816ef22702` | digest-only |
| `evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json` | `aaad7f5cd2b9` | `527e864e71da` | digest-only |
| `evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json` | `5d6f08ebaa9c` | `8957686e1216` | digest-only |
| `item-authoring/lower_mantra_bond_modifier_packet.py` | `1eb88af759b9` | `a67f605b038d` | pins only |
| `item-authoring/lower_numeric_modifier17_packet.py` | `4ab1d4dc6eee` | `d895516ad8d4` | pins only |
| `item-authoring/lower_wiki_stack_default_successor8_packet.py` | `d8b7df69edeb` | `6db74d7b32e8` | pins only |
