# OTV2-20261123-w11-w17-source-recovery

```yaml
task_id: OTV2-20261123-w11-w17-source-recovery
title: Recover and reconcile W11 W15b W16 W17 source facts
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: staging
branch: chatgpt/w11-w17-source-recovery-20261123
issue: 1622
pr: 3459
base_sha: 4f13511ca9ec3fb152f11299bb5cf7c71de16c41
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT source-recovery lead
created_at: 2026-11-23T17:01:21Z
updated_at: 2026-11-23T19:17:08Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/w11-w17-source-recovery-20261123/**
  - docs/agents/tasks/active/OTV2-20261123-w11-w17-source-recovery.md
  - tools/content-migration/mimar_crystal_profile.py
  - tools/content-migration/test_mimar_crystal_profile.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/protected_loot_yields.py
  - tools/content-schema/native-gameplay/mimar-crystal-profile.json
  - crates/gameplay/durability/src/durability/item_loop_move.rs
  - crates/gameplay/durability/src/durability/item_loop_move/first_move_decay.rs
  - crates/gameplay/durability/src/durability/item_transfer.rs
  - crates/gameplay/durability/src/durability/spell_item_temporal.rs
  - crates/gameplay/durability/src/durability/mimar_crystal_decay_tests.rs
  - crates/gameplay/durability/src/durability/mimar_crystal_decay_tests/**
  - crates/gameplay/durability/src/durability/mail.rs
  - crates/gameplay/durability/src/durability/house_scope_handoff.rs
  - crates/gameplay/durability/src/durability/ground_cell_limit.rs
  - apps/game-server/migrations/3077_mimar_crystal_first_move_decay.sql
  - crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission/wheel_capacity.rs
  - crates/gameplay/durability/src/durability/boss_encounter/activity_assignment_tests/native_corpse.rs
  - crates/gameplay/durability/src/durability/boss_encounter/activity_assignment_tests/native_corpse/activity_entry.rs
  - crates/gameplay/durability/src/durability/boss_encounter/activity_assignment_tests/native_corpse/mimar_crystal.rs
  - tools/content-migration/tibiadrome_source_profile.py
  - tools/content-migration/test_tibiadrome_source_profile.py
  - tools/content-schema/native-gameplay/tibiadrome-source-profile.json
  - content/items/definitions/spell-native-profiles.json
  - content/items/definitions/srx1-item-updates.json
  - content/items/index.json
  - content/spells.manifest.json
  - content/manifest.json
  - content/content.lock.json
  - content/creatures/prey-slot-values.json
  - tools/content-schema/native-gameplay/loot-tables.json
  - tools/content-schema/native-gameplay/prey-model-results.json
  - tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json
  - content/quests/grizzly-boss-rooms.json
  - docs/balance/PREY-SLOT-VALUE-2026-11-01.md
  - docs/balance/PREY-TIME-SIM-2026-11-01.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

The owner's 2026-11-23 request authorizes source research, implementation and
continuation of draft PR #3459 into staging. The owner additionally requests
periodic publication of verified work with subagents. The lead is the sole
publisher; workers have allocated source/review/runtime paths and no Git-write,
review-trigger or merge authority. No deployment or protected-branch integration
is authorized by this candidate.

Bound META policy: `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`,
version 3.1.0. Existing native Item, Creature, Map, Quest and boss/activity owners
remain authoritative. The source registry is
`docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`.

## Checkpoint scope

The first continuation checkpoint publishes the independently checked W17
source importer, its retained source packet and eight tests, plus W11/W15b and
W17 evidence updates. It closes five look mappings and the Thais battle mask;
`runtime_admission` remains false. Goblin milk is already implemented as subtype
9; Phantom slot eight remains unresolved. See [the evidence packet](../../evidence/w11-w17-source-recovery-20261123/README.md).

The inherited Moonsilver Crystals implementation is preserved in an isolated
working checkout and remains AUTHORING. Continuation tests reproduced missing
expiry inside Ground containers and escapes through mail/House custody. Those
fixes and their independent review are in progress. Earlier four-case runtime
results are not represented as validation of this new runtime candidate.

## Publication route

CLI push authentication was unavailable. The lead selects the bound policy's
connector-compatible new API candidate route: freshly inspect the predecessor,
create one complete successor with that sole parent, perform one non-force
expected-head update, immediately read back, and validate the returned head.
No sequential Contents commits, force updates or protected-ref mutations.
Each checkpoint is separately bounded; the live PR records exact SHA and checks.

## Acceptance criteria

- [x] Inspect current native owners and distinguish facts, derived observations,
  accepted Oteryn defaults and unknowns.
- [x] Verify existing Goblin milk without a duplicate override.
- [x] Qualify the W17 source importer and retained witnesses.
- [x] Publish the initial documentation packet in draft PR #3459.
- [ ] Publish and verify this source checkpoint on the returned remote head.
- [ ] Publish the bounded crystal-lifetime implementation after fresh real-owner
  PostgreSQL checks, formatting/compilation and independent review.
- [ ] Recover remaining field-level facts before claiming complete encounters.

## Runtime recovery requirements

First successful movement must bind the immutable source policy and a
same-transaction receipt. Failure/replay cannot start or reset the timer.
Expiry must prove current Content, exact custody, current scope/generation and
real session authority, then remove the Item once. Ground ancestry, House/mail
refusal and the House PREPARE-to-COMMIT pickup race require negative tests.
The donor first-move/30-second policy is `OTS_HYPOTHESIS_ONLY` for Global and
must not be confused with the boss vulnerability duration.

## Validation

Observed before this checkpoint: eight W17 tests and actual retained-source
check PASS; 23 Goblin reward tests PASS; governance validation (22 documents,
9 lanes) and 135 governance tests PASS. Bounded checks are repeated after
publication on the actual returned SHA. No complete encounter E2E is claimed.
Exact-head CI/review state belongs to the live PR. At most the second permitted
Codex review is requested after the final material implementation candidate.

## Context checkpoint

```yaml
status: implementing
branch: chatgpt/w11-w17-source-recovery-20261123
head_sha: null
pr: 3459
last_progress: Verified source checkpoint ready; independent workers validate remaining Item runtime fixes
blocker: Encounter source gaps remain explicit and public admission disabled
owner_action_required: null
next_action: Publish source checkpoint, finish runtime tests and review, then publish the runtime successor
```
