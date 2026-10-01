# WP-PROF2-1 — native selection legality
```yaml
task_id: OTV2-20261001-wp-prof2-1-selection
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
branch: codex/weapon-proficiency-native-selection-20261001
issue: 162
pr: 1478
base_sha: aad17f99d86abea01bd50b9d559e69eaa5549d88
owner: Weapon Proficiency Codex worker, packet 5936420312
owned_paths: [apps/game-server/src/domain/weapon_proficiency.rs, docs/agents/tasks/active/OTV2-20261001-wp-prof2-1-selection.md, docs/agents/tasks/archive/OTV2-20261001-wp-prof2-1-selection.md]
allocation: D281 A1, issuecomment-5936520972; accepted PROFICIENCY-0
```
Pure selection/clear validation and active projection; no SQL, wire or activation.
Also preserve cumulative XP when a compatible threshold decrease lowers Mastery.
Validation: 15 WP tests, full server package, fmt/clippy, governance+36 tests PASS.
Independent whole-diff review: no unresolved findings; lower-cap regression repaired.
Closeout: delivered in #1478; exact head in #162 FREEZE; review/MQ by control plane.
