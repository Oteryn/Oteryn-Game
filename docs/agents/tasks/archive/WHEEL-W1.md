# WHEEL-W1

```yaml
task_id: WHEEL-W1
title: "WHEEL-W1 Wheel of Destiny allocation storage, writer, admission reset, load and stages"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: wheel
base_branch: main
branch: claude/wheel-w1
pr: PENDING
base_sha: bac15d8
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-0184G9SKwaLcDGfZ6JnXvERc (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md child W-1 (§4, §8 rows, §13); OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md §3.2-§3.4; owner decision D483; control plane decision D484 (W-R runtime carry)"
leases: migration 0070 (control plane lease)
owned_paths:
  - apps/game-server/migrations/0070_character_wheel_allocation.sql
  - apps/game-server/src/durability/character_wheel.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/wheel_gem_data.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/character_wheel_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/WHEEL-W1.md
  - docs/agents/tasks/archive/WHEEL-W1.md
depends_on:
  - "W-R Wheel data on main (#1697), data-only; its runtime revision piece is carried here (D484)"
  - "CHAR-REV-SEQ-1 merged (#1663)"
public_contracts: []
external_repositories: []
```

## Rows

WHEEL-0 §8 registered in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` before implementation:
`WHEEL0-RL-01`..`06`, plus the two unnumbered rows as `WHEEL0-RL-07-CHANGE` (allocation change:
0 items, 0 value lines, 1 CharacterRevision, 1 receipt) and `WHEEL0-RL-08-RESET` (Wheel reset:
the same, once per Character per reset revision).

## Outcome

WHEEL-0 §4 on the CHAR-REV-SEQ-1 sequencer. No wire (W-2), no combat effect (W-FX-1).

- **W-R runtime carry (control plane D484).** W-R on `main` (#1697) is data-only
  (`runtime_admitted:false`) and had no runtime revision or revision kinds. Migration 0070 adds
  `game_wheel_ruleset_revisions` (ordinal, kind `INITIAL`/`VALUE_ONLY`/`RESET`) and
  `game_wheel_ruleset_slot_capacities`, seeded with `wheel-authoring-candidate-r1` (`INITIAL`,
  the topology's 36 capacities). Both are immutable; a revision needs its 36 capacities. A later
  revision is inserted by its own W-R migration. `wheel.json` is unchanged and stays
  `runtime_admitted:false`; no effect is activated.
- **Migration 0070.** `game_character_wheel_state`, `game_character_wheel_slots` (non-zero slots
  only) and `game_character_wheel_receipts` (common envelope, request binding, both Wheel
  revisions, both vectors, both ruleset revisions; unique on occurrence, original, committed and
  after Wheel revision). CHECKs: vector shape; an `ALLOCATION` is a real change under one
  revision with a UUIDv7 occurrence; a `RULESET_RESET` moves to another revision with an all-zero
  after vector and a version 8 derived occurrence. The shared consistency guard (full 0056 body)
  gains the tenth receipt kind in the chain, revision one and the progression transition, and
  calls the Wheel arm `game_character_wheel_consistency`: full chain, ruleset chain (a reset
  needs a `RESET` revision after the source up to the destination), capacities under the
  receipt's revision, row tip (state row and slot rows equal the latest receipt; rows without
  receipts fail). State and slot row changes run the Wheel arm too. Grants follow 0056.
- **Writer** (`durability/character_wheel.rs`). `commit_character_wheel`: occurrence replay first
  (binding = occurrence, Character, expected `wheel_revision`, slots), then the gameplay fence and
  `character_root` FOR UPDATE, progression state, Wheel rows. Refusals before any write, in order:
  `RULESET_NOT_CURRENT`, `STALE_REVISION`, `NO_CHANGE`, `NOT_ELIGIBLE` (vocation, level above 50,
  promotion; Premium not applied, W1 a), `OVER_CAPACITY`, `NOT_ADJACENT` (minimum points and a
  path of full open slots from a centre slot), `OVER_POINTS` (raises only, WHEEL0-PT-1),
  `REMOVAL_NOT_AT_TEMPLE`. Promotion and the temple position are caller facts
  (`WheelChangeFacts`): no durable promotion exists yet (PREM-2), and position is the W-2
  caller's (RL-05).
- **Reset** (`reset_character_wheel`, WHEEL0-RST-1): a `RESET` revision after the stored one up to
  the active one writes one `RULESET_RESET` receipt, deletes the slots and moves the pin; a
  value-only path keeps the allocation; a newer stored revision or an unregistered active one is
  unresolved and writes nothing (the allocation then fails closed).
- **Sequencer.** `RevisionSlot::commit_wheel` and `reset_wheel`, retry once on a mismatch; the
  structural gate lists both writers.
- **Admission and load.** `admit_character_wheel` runs the reset in the revision slot, then
  `read_character_wheel`; `ComposedFreshAdmission` does it at fresh admission and resume next to
  the quest copy and keeps the allocation per session (`None` fails the Wheel closed, never the
  login). The refresh after a committed change belongs to the W-2 caller.
- **Stages.** `WheelStages::derive` from the cached allocation and the eligibility read at the
  use (WHEEL0-EL-1): revelation stage per perk (250/500/1000), augment stage per conviction key
  and per targeted spell (full slots, at most 2); all 0 when not current or not eligible.
  `ComposedFreshAdmission::wheel_stages` is the read SPELL-WHEEL-GATE-1 (not on `main`) passes to
  the cast snapshot.
- **Ruleset reader.** `WheelRuleset::from_catalogue` (closed shape: 36 numbered slots, known
  domains and capacities, adjacency within 1..36, nine slots per domain, five vocations with 36
  slots and four revelations), exposed by `WheelGemData::wheel_ruleset`.

## Tests

- Unit (`durability::character_wheel::tests`): the embedded revision equals the 0070 seed;
  malformed catalogues refused; thresholds 249/250/500/999/1000; each vocation's perk; augments
  from full slots only; ineligible or not current derives 0; every validation rule including the
  level-loss decrease and a self-supporting cycle away from the centre; bounds; the derived reset
  occurrence.
- PostgreSQL 17.6 (`character_authority_postgres`, `character_wheel_postgres_cases`): commit,
  replay, conflict, each refusal writing nothing, a stale fence, removal at a temple, reconcile;
  the slot's one retry after a bypass; value-only kept, reset fail-closed before and once at
  admission, replay writes nothing, unresolved revisions; every 0070 guard branch by SQL.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server` with PostgreSQL 17.6: see the PR body.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests).

## Review

Independent persistence review on the final frozen head; the control plane requests it. The W-R
runtime carry (D484) is stated in the PR body for review.
