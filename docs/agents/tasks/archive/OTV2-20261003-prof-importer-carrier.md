# PROF importer carrier
```yaml
task_id: OTV2-20261003-prof-importer-carrier
title: Carry the Weapon Proficiency data-only importer onto main
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
branch: claude/prof-importer-d418-20261003
base_branch: main
pr: 1688
issue: 1622
owner: proficiency stack integrator (D401)
created_at: 2026-10-03T20:05:00Z
owned_paths: [apps/game-server/src/content/project/v2/proficiency/import.rs, apps/game-server/src/content/project/v2/proficiency.rs, apps/game-server/src/bin/oteryn-game-import-proficiencies.rs, tools/content-schema/proficiency-authoring/README.md, docs/agents/tasks/archive/OTV2-20261003-prof-importer-carrier.md]
depends_on: [PROFICIENCY0]
```

## Outcome

D401 triage of the stale proficiency stack: #1607 was superseded by #1648; #1484 and #1606 (Snowball
identity and stats rekey) go to PROF-SNOWBALL-REKEY-1 after #1675 (D418), because the rekey cascades
into about 15 Item qualification chains. This PR carries the #1598 importer: 443 definitions,
3671 ordered perks, the 664 bindings on main and the progression ruleset. Snowball stays unbound.
Data-only: no activation, Character writes, migration, wire or effects. #1598's evidence directory
qualified a 665-binding candidate and is not carried.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib proficiency::import`: pass (3 tests; Codex round 1: point tables validated against the domain, every binding resolved against committed Item definitions)
- `cargo test --locked -p oteryn-game-server`: pass (16352 passed, 0 failed; no PG environment)
- `cargo run --locked -p oteryn-game-server --bin oteryn-game-import-proficiencies`: OK (443 definitions, 664 bindings)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

Review state: frozen-head CI and review are owned by the control plane. Merge commit/result:
squash merge of #1688.
