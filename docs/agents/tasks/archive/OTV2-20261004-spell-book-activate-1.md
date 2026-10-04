# OTV2-20261004-spell-book-activate-1

```yaml
task_id: OTV2-20261004-spell-book-activate-1
title: "OTV2-20261004-spell-book-activate-1: whole-book sweep and qualification default"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/spell-book-activate-1-20261004
pr: 1799
base_sha: 673f092e
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
review_state: awaiting control-plane Codex request
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/spell_book_sweep_tests.rs
  - apps/game-server/src/gameplay_transport/qualification.rs  # one cfg(test) module line
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/stance_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/familiar_cast_dispatch.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/native_companion_item_cast.rs  # section 1.2 refusal fix only
  - apps/game-server/src/gameplay_transport/native_world_item_cast.rs  # section 1.2 refusal fix only
  - tools/qualification/node_boot/run.sh
  - tools/qualification/spells/README.md
  - tools/content-schema/native-gameplay/README.md
  - content/abilities/SPELL-IMPORT.md
  - docs/agents/tasks/archive/OTV2-20261004-spell-book-activate-1.md
public_contracts: []
depends_on: []
```

## Outcome

Whole-book sweep (`spell_book_sweep_tests`, 3 tests) passes over the real compiled native book:
246 indices x 3 target forms (None, AttackTarget, Position) = 738 outcomes, best over the 10
vocations. No panic, no Cast without an owner effect or vitals change, no `needs_target` spell
casts without its target. No section 1.2 refusal fix was needed; no disposition changed.

Golden counts (spells per family x form; `+target` = `needs_target`):

- Conjure (48): Cast 48 with None and AttackTarget; Rejected 48 with Position.
- Effects without target (90): Cast 72 / Rejected 18 with None and AttackTarget; Cast 90 with Position.
- Effects+target (36): Rejected 36 in all three forms.
- PartyBuff (5): Rejected 5 in all three forms.
- Native (67), Classified by key gate in all three forms: familiar_cast 9, focus 2, native_combat 19 (+target 1), other_native 28 (+target 2), stance_cast 6.

Damage evidence: "Rage of the Skies" (self-centred area) and "Fire Wave" (directional wave) resolve a
positive Damage effect through the ordinary pipeline; the real owner commit path reduces the adjacent
fixture creature's health. No damage owner was built. Scope limit: geometry containment is
`ordinary_combat::lower`'s and needs the Postgres-backed room, so it is not asserted here.

Node-boot qualification defaults to `content/spells.manifest.json` under the single
`OTERYN_NATIVE_GAMEPLAY_MANIFEST` selector; the stager copies the manifest and its sources into the
node tree (staging verified for the canonical and r21 manifests). The end-to-end node boot needs
Platform and Docker and was not run in this session.

## Deviations

- Control plane D632 (option a): the sweep is DB-free. The native `prepare_from_owners` step is
  covered only by the key-gate classification; the full-fidelity seam sweep is a separate follow-up.
- The engine returns `Rejected` (not `TargetRequired`) for a `needs_target` spell without a target;
  the wire `NotAvailable` for `AttackTarget` is a DB-path dispatch rule not covered DB-free. Neither is a panic or a ghost Cast.
- `map` needs an absolute manifest path (cargo runs in the crate directory).

- The sweep module sits under `gameplay_transport::qualification` because `mod.rs` is leased to
  ATTACK-1b, so the packet filter `gameplay_transport::spell_book_sweep_tests` cannot match; the
  filter used is `spell_book_sweep_tests`.
- `cast_spell_completion` needs Postgres and Platform authority, so the sweep is DB-free over the
  pure engine `prepare_*` functions plus dispatch classification (agreed with the control plane).
- `tools/qualification/node_boot/README.md` lines 17-18 still describe the old r21 default (outside
  owned paths); `wp5_s3b/run.sh` also still defaults to r21.

## Validation

cargo fmt --all -- --check: pass
cargo clippy -p oteryn-game-server --all-targets -- -D warnings: pass
bash tools/qualification/spells/run.sh runtime spell_book_sweep_tests: pass (3 passed)
bash tools/qualification/spells/run.sh runtime: pass (321 passed)
bash tools/qualification/spells/run.sh map content/spells.manifest.json: pass (absolute path, 1 passed)
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
git diff --check: pass
