# OTV2-20260929-spell-cast-wire-w1

```yaml
task_id: OTV2-20260929-spell-cast-wire-w1
title: Spell cast wire and own-actor vitals - registries and codecs (§9 step 1, SPELL-D8 H-3)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1254
allocation_comment: "#162 5894588548"
base_branch: main
branch: claude/spell-cast-wire-w1
base_sha: 37a06b5cd45c33cab961652882b074b23a09dd1f
head_sha: 6b10381a04ea44ed9e63d85e24b87dd1bf3ebeae
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T16:58:00Z
updated_at: 2026-09-29T18:10:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - crates/protocol-oteryn/**
  - docs/agents/tasks/active/OTV2-20260929-spell-cast-wire-w1.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-harmony-runtime.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: null  # no spell Story is mapped (KAN-16 is the content census); Jira sync pending
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

§9 step 1 of `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` (§3; SPELL-D1, D2, D6, D7, and
SPELL-D8 H-3), in the first-control-wire M1 pattern. Nothing is composed.

- **Protocol registry.** Command type 3 `WORLD_ACTOR_SPELL_CAST_INTENT` (intent at most 32 bytes, result at most
  4) and state domain 3 `ACTOR_VITALS` with delta type 1 and snapshot type 1 (both `ActorVitalsV1`, at most 32
  bytes). All four IDs were free on base `37a06b5c`.
- **Resource registry.** `SPELL-RL-01` (1 cast input per actor per Channel owner work cycle), `SPELL-RL-02` (2
  effects per cast Effect Plan, the same bound as `ABILITY01-EFFECT-PLAN-ENTRIES`) and `SPELL-RL-03` (1 actor per
  `ACTOR_VITALS` snapshot or delta), as accepted in SPELL-D6. `SPELL-RL-04` needs a measured value at registration
  and is left to composition.
- **Schema.** `docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto`, as §3 states it, with SPELL-D8
  `harmony = 6` and `serene = 7`.
- **Codecs.** `crates/protocol-oteryn/src/actor_spell.rs` encodes and decodes the intent, the result and the
  vitals (one pair for the delta and the snapshot). Decoding fails closed on zero or unknown enum values, unknown or
  repeated fields, over-bound payloads, spell index 0 (SPELL-D1 is 1-based), a floor outside int16, a
  `target_position` without `POSITION` (or `POSITION` without it), a bool other than 0 or 1, a vital above its
  SPELL-D8 bound and Harmony above 5. The encoder refuses a vital above its bound as a server fault; the Rust types
  cannot express spell 0 or a position with another intent. Health is not checked against max health: the
  contract leaves a lowered maximum to composition (SPELL-D5).
- **Fixtures.** Hand-computed canonical bytes for each payload, including the 28-byte intent and 27-byte vitals
  worst cases. They were cross-checked with the reference protobuf runtime (protoc-generated Python, protobuf
  7.36.2) outside the repository.
- **Registry binding test.** Ties the new command, domain, schema anchors and `SPELL-RL-*` rows to the code
  constants.

## Excluded scope

Composition (§9 step 2: Channel owner vitals and cooldowns, `CasterState`, cast commit and publication), the native
client (§9 step 3), `.github/**` and any `docs/architecture/**` text.

## Validation

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-protocol-oteryn`
- `python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`

## Independent review

Required (§8.1: protocol and security, independent exact-byte fixtures). The control plane triggers it on the
frozen head.

## Closeout

- PR #1254 merged via Merge Queue: final head `6b10381a04ea44ed9e63d85e24b87dd1bf3ebeae`, merge commit `c91055f66dd2c5724f17e226c7b7da2430a1a914`.
- Protected-main readback: all 7 files the merge changed are byte-identical between the final head and the merge commit.
- Ownership released on merge. The record stayed in `tasks/active/` and turned `Agent governance` on `main` red; archived by the Work coordinator in a P0 archive batch.
