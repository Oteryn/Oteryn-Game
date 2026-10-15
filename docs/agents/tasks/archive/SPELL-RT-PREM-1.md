# SPELL-RT-PREM-1

```yaml
task_id: SPELL-RT-PREM-1
title: "SPELL-RT-PREM-1: the cast reads a Premium status; the Wheel exception stays closed until W-1"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-rt-prem-1
issue: 2048
governing_issue: 2048
pr: 2119
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
depends_on: ["#2110 (PREMIUM-1)"]
owned_paths:
  - apps/game-server/src/spell/mod.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/owned_cast_facts.rs
  - apps/game-server/src/spell/cast_tests.rs
  # one-line CasterState constructions (type change only)
  - apps/game-server/src/spell/{executable_catalog,companion_lifecycle,chain_tests,part_b_tests,part_d_tests,party_tests,tests}.rs
  - apps/game-server/src/gameplay_transport/{familiar_cast,ordinary_combat,spell_book_sweep_tests}.rs
  - docs/agents/tasks/archive/SPELL-RT-PREM-1.md
```

Coordination: control plane session_01XYC239givrMonrYoBwwECV (option B, 2026-10-14).

## Outcome

- `CasterState.premium` is `CasterPremium {Current, NotCurrent, NotActivated}`, the gate's
  `PremiumStatus` without its witness (PREMIUM-ACTIVATION-0 §1.3, §2.1). The cast builds it from
  the PREM-1 view (`from_current`), so behaviour is unchanged: only `Current` casts a Premium
  spell.
- One rule, `CasterPremium::admits`, replaces the two `spell.premium && !caster.premium` checks
  (`spell/mod.rs`, `spell/cast.rs`). Familiar facts read `is_current()`.
- WHEEL0-PS-1 (a `wheel_unlock` spell skips the Premium check under `NotActivated`) is a
  fail-closed stub: `NotActivated` admits no Premium spell. It has no playable effect before
  W-1, because every `wheel_unlock` spell is role A and fails at stage 0. W-1 applies the real
  exception inside `with_premium_gate`, with the cast applied before the gate's transaction
  commits.
- No migration, registry, protocol or wire change.

## Validation

- cargo +1.95.0 fmt --all --check: pass
- cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo +1.95.0 test --locked -p oteryn-game-server --lib, after merging main (#2110 merged):
  spell:: 463, familiar 14, ordinary_combat 15, spell_book 4, premium 52 passed
- python3 tools/agents/validate_governance.py: pass; agents unittest: OK
