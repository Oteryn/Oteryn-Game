# PRESENT-WIRE-1

```yaml
task_id: PRESENT-WIRE-1
title: "PRESENT-WIRE-1: Presentation wire (capability 23, domains 19 and 20, cast result detail, SPELLPRES0 rows)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/present-wire-1
issue: 1622
governing_issue: 1622
pr: 2124
base_sha: ed2b1180
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/presentation.rs
  - crates/protocol-oteryn/src/presentation_tests.rs
  - crates/protocol-oteryn/src/actor_spell.rs
  - crates/protocol-oteryn/src/lib.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - docs/contracts/protocol-oteryn/v1/presentation_v1.proto
  - docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/PRESENT-WIRE-1.md
  - docs/agents/tasks/archive/PRESENT-WIRE-1.md
public_contracts: [PROTOCOL_OTERYN_V1_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json]
external_repositories: []
```

## Outcome

Registers, per SPELL-PRESENT-0 §4, §7 to §10 and §13 (accepted with conditions, #2099), capability 23
`PRESENTATION_V1` (requires 6, no command type, `offered: false`), state domain 19 `WORLD_PRESENTATION`
(delta type 1 `WorldPresentationBatchV1`, at most 131,136 bytes; snapshot type 1, always empty) and
state domain 20 `ACTOR_COOLDOWNS` (delta and snapshot type 1 `ActorCooldownsV1`, at most 65,552 bytes),
the strict codecs and the §9 overflow selection in `presentation.rs`, field 2 `detail` of
`WorldActorSpellCastResultV1` (13 refusal texts, never with CAST), and rows `SPELLPRES0-RL-01` to `-07`.
Wire only: no emitter, no runtime, nothing offered.

## Rulings and deviations (reported to the control plane)

- Numbers leased by the #1622 control plane: capability 23 (FND-02 assigns 20 to
  `CONNECTION_TRACE_V1`; 21 and 22 are leased to LOGOUT-WIRE-1 and RUNE-WIRE-1) and state domains 19
  and 20 (18 `WORLD_CLOCK` is the highest on `main`). No command type is needed, so 25 is not used.
- `SPELLPRES0-RL-04` (cooldown entries) uses the 4,096 ordinary repeated-entry ceiling because
  SPELL-RL-04 is not registered yet; the row notes say so. A smaller measured bound narrows it later.
- `SPELLPRES0-RL-06` (overflow nearness window) and `-07` (undelivered batches, 2) carry spec-style
  boundary tests: the drop and snapshot barrier need the emitting connection (SPELL-PRESENT-1).
- The §9 B3 measurement of a crowded hunt is a precondition of activation, recorded in the capability
  `offer_gate`; it needs an emitter and is taken before the capability is offered.
- The unregistered candidates `actor_spell_v2.rs`, `actor_spell_item_v2.rs` and
  `spell_presentation_candidate.rs` are left untouched; SPELL-PRESENT-1 migrates the publisher to this
  codec and deletes the candidate.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy -p oteryn-protocol-oteryn --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-protocol-oteryn`: pass (238)
- `cargo test -p oteryn-session -p oteryn-dev-client`: pass (60, 34)
- `cargo check --workspace --all-targets`: pass
- `oteryn-architecture-check workspace .`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (124)
- `python tools/repository/validate_repository_policy.py`: pass
- `git diff --check`: pass
- `cargo test -p oteryn-game-server --lib gameplay_transport::capabilities`: pass

## Review

- Codex on c71f58c, two P2s fixed in the final authoring commit: the emission ordinal takes the
  full `uint32` range (the decision bounds only event and batch bytes; the worst event is 126
  bytes), and the `ACTOR_COOLDOWNS` entry bound counts every byte of an overlong field 4 key.
- Codex on ec6f77a3, one P2 fixed: the SPELL-D1 index is no longer capped at 65,535. `spell_words`
  takes any nonzero `uint32`; a cooldown spell id is at most 2^28 - 1, the largest that keeps an
  entry within `SPELLPRES0-RL-05` (16 bytes).
- Merge Queue on f5da9198: the game-server gate table lacked capability 23. Domains 19 and 20
  are now gated by it (`capabilities.rs`), matching the registry.
- Capability renumbered from 20 to 23 on the control plane's lease (6e2744b3).
