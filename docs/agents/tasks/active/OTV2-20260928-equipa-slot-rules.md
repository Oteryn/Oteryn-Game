# OTV2-20260928-equipa-slot-rules

```yaml
task_id: OTV2-20260928-equipa-slot-rules
title: EQUIP-a - pure Reference equipment-slot rules
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1176
allocation_comment: "#162 5879250283 (notice), owner standing mode 12A in 5879169470"
base_branch: main
branch: claude/equipa-slot-rules
base_sha: 9b3f84846e7057098ee6497e2a1858b07b738288  # stacked on PR 1172 -> 1171
head_sha: null
owner: "Oteryn: impl domains" (Claude Code)
created_at: 2026-09-28T22:00:00Z
updated_at: 2026-09-28T22:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/equipment.rs
  - apps/game-server/src/domain/mod.rs  # one line: pub mod equipment;
  - docs/agents/tasks/active/OTV2-20260928-equipa-slot-rules.md
public_contracts: []
depends_on: [OTV2-20260928-appa-appearance-validation]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`apps/game-server/src/domain/equipment.rs` holds the pure Reference slot rules (tibia.com manual
§3.4.1, PROVEN in #162 5879250283 row 4; owner answer 4 "as Global"). It has no caller yet.

- `EquipCategory::slot`: head, armor, legs, feet, necklace, ring, container, weapon (right hand),
  shield/spellbook/quiver (left hand), extra (the `Ammo` slot).
- `check_equip`: the slot must be free; a two-handed weapon and a shield-slot item exclude each
  other in either order, except a two-handed distance weapon with a quiver.
  Review fix (Codex P2): `EquipCategory::pattern` exposes each item's complete `EquipPattern` claim
  (both hands for two-handed; right hand plus a non-quiver left-hand group for distance weapons), and
  `check_equip` derives the hands conflict from claim overlap.

## Excluded scope

Vocation and level requirements (typed item requirements from content), item categories in content,
swaps and the DUR-03 move, weapon or armor stats.

## Validation

- `cargo fmt --all --check`
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
- `cargo test -p oteryn-game-server --lib domain::equipment`
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`
