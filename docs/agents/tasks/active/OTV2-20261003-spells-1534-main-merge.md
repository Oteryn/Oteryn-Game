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
- Pre-existing PR-head defects fixed before freeze (D325): the complete-reconnect
  scenario future is boxed off the 2 MiB test stack; `clippy -D warnings` is clean for
  game-server and protocol-oteryn (mechanical fixes, reasoned allows matching main's
  conventions, test-module allows). Hand-written delta about 630 added / 290 removed lines.
- D339: merged current main once (merge, not rebase); later predecessor merges only
  merge main.

- D357 candidate (P1 round 1 of 2): server-seam scenario future boxed (stack); Codex P1
  tile-aimed area visibility uses the populated sight origin; CodeQL HIGH GuildStats script
  extraction uses html.parser; spell readiness and starter bundle samples regenerated with
  convert_spells.py from the pinned sources.
- D358 merge_group NODE_BOOT_FAIL (use_wire: Accepted then ProtocolError 1004): the PR's
  first entry adds Character data reads (cast facts, build, stance, current root) through
  the node's single ready-only holder (`try_acquire_ready`, max 1 connection); a concurrent
  pass (the other session's grace release or tick) makes one read refuse, so the fresh actor
  stayed unpositioned (admission-only). Fix: the pre-write first-entry refusals
  (Unavailable, StaleAuthority) are repeated from fresh reads, bounded by
  `RECONCILE_ATTEMPTS` with `RECONCILE_BACKOFF`; those paths wrote no runtime state and the
  durable inits are idempotent. Not reproduced locally (PR head passed node boot locally).
- D357 open: the r23 evidence-log whitespace repair was refused by the session's permission
  policy as evidence tampering; r25 source-world non-canonical Canary item keys and two
  dangling Item keys need a content decision. world-metadata (35600), Item+Mount
  TAXONOMY_SOURCE_COVERAGE and the 14 retired taxonomy keys fail on main itself.
- Main merges after D357 (no rebase): encounter domain taken from `main` (#1599 carrier);
  `forge_dust` modules and the populated Wheel catalogue unioned with the PR's
  `spell_imports` overlay on `rulesets/progression/wheel-of-destiny/index.json`.

- Main 602ebe4b merge (QUEST-STATE-1 #1684, union only): main's 0056 re-issues
  `game_character_progression_consistency_guard` without the familiar arms. CP 1a moves
  the D325 union from 0053 to leased `0068_character_progression_guard_union.sql`: the
  0032 proficiency, 0033 familiar and 0056 quest arms (revision-one EXISTS, chain SELECT,
  transition receipts) plus the search_path pin and REVOKE; no main migration in
  0057..0067 re-issues the guard. The integrity chain CTE unions familiar and quest
  receipts; the transport keeps main's quest retry loop joined with the PR's source owner
  cycles and both method sets; `character_authority_postgres` keeps the stance, familiar
  and quest case modules.
  `proficiency_familiar_and_quest_writes_share_the_0068_progression_guard` interleaves
  familiar, proficiency, quest and familiar writes on one Character (revision 5) and
  re-verifies from a fresh authority.

### Follow-ups (control plane)

- ITEM-KEY-R25-1 (CP 1c 2c): `item_key_references.py` reports 608 NON_CANONICAL
  `oteryn:item.source.canary.id<N>` keys in the r25 source world (emitted by
  `build_source_world.py`, required by `native_spell_world.rs` as `source-map-r3`) and the
  dangling `oteryn:item.tibia.i40450` (spell native profiles and the r25 test pack). Move
  the aliases to canonical `oteryn:item.tibia.i<N>` keys and author the i40450 Item record.
- PARTY-DECLINE (CP 1a, supersedes the D245 deferral; Codex P2 on `world_party.rs`
  Decline): Decline locks the requested party row, and a decline that removed an
  invitation closes that party when it is leader-only with no invitations left.
  `declining_the_last_invitation_closes_the_leader_only_inviting_party` covers the sole,
  shared and repeated declines.

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
