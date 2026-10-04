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
  - apps/game-server/migrations/ (renames 0032..0051 -> 0033..0052, D325 guard union 0068, D495 0037 removal and 0071 guard union)
  - tools/qualification/wp5_s3a/, tools/qualification/wp5_s3b/README.md (D495 revert to main only)
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
  - apps/game-server/src/movement.rs, apps/game-server/src/movement/ (SPEED-1 reconciliation, CP option a)
  - apps/game-server/src/content/item_*_promotion.rs, docs/agents/evidence/OTV2-2026100*-item-*.json, tools/content-schema/item-authoring/lower_*.py (re-pin, owner decision 1a)
public_contracts: []
depends_on: [D313, D314, D319, D325, D339, D357, D495]
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

- D357/D358: repairs in git history. Open: r23 evidence-log whitespace (refused as evidence
  tampering), r25 item keys (ITEM-KEY-R25-1).
- Main merges (merge commits, union only): #1599 encounter domain from main; Wheel catalogue
  unions `spell_imports`; QUEST-STATE-1 moved the D325 guard union to leased
  `0068_character_progression_guard_union.sql` (0032 proficiency, 0033 familiar, 0056 quest
  arms, search_path pin, REVOKE), covered by
  `proficiency_familiar_and_quest_writes_share_the_0068_progression_guard`; no later main
  migration (0058, 0060) touches the guard. Content `index.json` files take main's
  `legacy_source` and keep the PR's `spell_imports`. Main's test stubs' join-snapshot
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

## Re-pin after merging origin/main 8588b1bb (owner decision 1a)

Only this PR's `reference_playable.rs` drifted (`5424c687ef3b` -> `fd81a9d9359e`). The
numeric17 receipt and Mantra/Bond source-qualification pins moved to it; each packet was
regenerated with its generator and pins cascaded until every `--check` passed. The 14 changed
files equal their pre-merge content with 64-hex digests masked.
`release_terminal` (main) keeps this PR's familiar and spell-training saves around the monk save.

## SPEED-1 reconciliation (CP option a, merge of main c739aa6f)

Main's SPEED-1 is the only pacing: `movement/speed.rs` with `step_speed_v1.json`,
`movement/pacing.rs` and the connection's `StepPacer` under capability 13 are unchanged. The PR's
inline table, `StepPacing`/`BufferedStep` and the actor-held pacing state are removed, and
`FreshAdmissionAuthority::step` keeps main's signature. The PR's owner step (`paced_step`:
spell commit fence, field ingress, source floor change, door) returns the step and its duration
onto the destination, computed before the commit; a step without a duration is refused.
Effective speed: no spell state -> main's level 1 base plus the runtime `SPEED` delta; spell
state -> its base speed plus spell and runtime `SPEED` deltas plus equipment (required on
qualified source ground, 0 elsewhere). Ground: `SourceStepProof` and `QualifiedCellGroundSpeed`
implement `GroundSpeedSource` (qualified ground speed, 0 -> 150, unknown -> 0, refused);
engineering cells use `EngineeringGroundSpeed`. Companion haste validates its resulting
duration on the caster's qualified tile. Dropped: the PR's floor-change x2 step cost (not in
CONDITIONS-0 §4.2; follow-up if accepted).

Later main merges (04cd6fa2, eaa40100, 83c4e94a, e14275e8, 41d4b38b) are union-only: content
manifest/lock regenerated with `regenerate_content.py --resolve`; test expectations include the
always-sent empty overlay domain.

## WHEEL-W1 reconciliation (D495 option a, merge of main 5c239e17)

Main's `character_wheel.rs` and migration 0070 are the only Wheel owner and stay unchanged. This
PR's `0037_character_wheel.sql`, its Wheel owner and its first-entry Wheel initialization are
removed. The spell side reads no Wheel owner: main keeps runtime Wheel effects unadmitted until
SPELL-WHEEL-GATE-1, so the cast facts carry no Wheel stage projection (Wheel-gated spells are
refused) and the magnitude baseline's flat Wheel bonus resolves to 0. No owner API was added.
0070 re-issued the shared progression guard from the 0056 body and dropped the 0068 familiar
arms; `0071_character_progression_guard_wheel_union.sql` re-issues 0070's body with every 0068
familiar clause, the search_path pin and REVOKE
(`proficiency_familiar_and_quest_writes_share_the_0071_progression_guard`, RED without 0071).
D495 S3-A: `wp5_s3a/compose.yml`, `platform-fpm.Dockerfile` and the S3-B README equal main.
G4 `item_key_references` stays deferred to ITEM-KEY-R25-1.
