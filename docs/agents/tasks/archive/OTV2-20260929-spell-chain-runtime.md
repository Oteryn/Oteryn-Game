# OTV2-20260929-spell-chain-runtime

```yaml
task_id: OTV2-20260929-spell-chain-runtime
title: Engine chain runtime for player spells with Ability.chain (D12, S23)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1219
allocation_comment: "#162 5886481110"
base_branch: main
branch: claude/spell-chain-runtime
base_sha: 1869ade0e657856c2125ff8c5fdbe21cc61f4e1a
head_sha: 46889a4641ddc499f068a71b9cd0d1e61f0f2c0e
final_head_sha: 46889a4641ddc499f068a71b9cd0d1e61f0f2c0e
final_head_frozen_at: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T08:50:00Z
updated_at: 2026-09-29T10:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/ability/**
  - apps/game-server/tests/*spell*
  - tools/content-schema/spell-authoring/**
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-chain-runtime.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

The spell core admits an Ability with `chain` and resolves it per `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md`
§3-§6 for Chained Penance, Forked Glacier, Forked Thorns and Lightning (`chain-behaviours.json`):
- `spell/chain.rs`: first creature (cast target, else the attacked creature, else the nearest), square same-floor
  ranges, validity (may hit, sight, not the caster, not hit yet), sequential and fork shapes, `1 + max_targets`,
  50 ms steps, effect tiles on the walking path; ties go to the lowest creature id (Q1 proposal);
- `resolve_chain_cast`: cast checks first, no first creature fails the cast with nothing spent, one draw per
  creature scaled by its step; `chain_plans`: one Ability occurrence per creature;
- a direction cast of Lightning does not chain; backtracking and `target_filter` stay rejected (support chains).
- D12 wording (B1 fork import minus 1; B2 integer step rounding, half away from zero, Canary-equal for Chained
  Penance).

World facts come through the `ChainWorld` trait; no party service exists, so the caster is solo. No protocol,
persisted state or runtime composition changes.

## Excluded scope

Spiritual Outburst, Executioner's Throw, Divine Dazzle, Chivalrous Challenge, Wheel augments, the live cast
wire and scheduler, and the chain doc itself.

## Validation

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-game-server`
- `python validate_spell.py` over the starter bundles, `python tools/agents/validate_governance.py`

## Terminal integration

- **Final head:** `46889a4641ddc499f068a71b9cd0d1e61f0f2c0e`.
- **Integration:** merged into main as PR #1219, merge commit
  `291f5eb79d6720c627335a0ad2aa84fb0603f074` (2026-09-29T09:46:57Z).
- **Closeout:** the record was archived by `OTV2-20260929-spell-part-c` (issue #162, allocation comment 5887751302).
- **Owned paths:** released.
- **Binding carry-over:** Q1-Q2 of the chain candidate remain for the owner's in-game tests; the support chains
  (chain §4.2) and the four excluded spells stay blocked by their own behaviours.
