# OTV2-20260929-spell-part-c

```yaml
task_id: OTV2-20260929-spell-part-c
title: Engine runtime for the Part C spell native behaviours (S27)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1224
allocation_comment: "#162 5887751302"
base_branch: main
branch: claude/spell-part-c
base_sha: 4ebb1bffaa24112fe21f950131342fcd206beba2
head_sha: c2596f3199eac7a2d4fa91d9c97ebc3cefc51ff8
final_head_sha: c2596f3199eac7a2d4fa91d9c97ebc3cefc51ff8
final_head_frozen_at: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T10:30:00Z
updated_at: 2026-09-29T11:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/ability/**
  - apps/game-server/tests/*spell*
  - tools/content-schema/spell-authoring/**
  - docs/agents/tasks/archive/OTV2-20260929-spell-part-c.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-chain-runtime.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Part C of `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` (owner S27), implemented where no new state, owner or
wire is needed:
- **C.3 party, admitted.** The reader admits `native_behavior` key `party_buff` (the C.3 authoring shape) with its
  accepted values only: an areaMatrix on the caster's floor, `requires_party` true, `min_affected` >= 1, mana
  `fixed` or `scaled` (`falloff` a whole percent, `rounding` up), one member `effect`; `costs.mana` must be 0.
  `spell/party.rs` picks the affected members (party members in the area, the caster included, by id) and computes
  `ceil(base * falloff^(X - 1) * X)` in exact integers. `resolve_party_cast` runs the cast checks first, then the
  party (no party or too few members: `NoPartyMembers`), then the computed mana; nothing is spent on a failure.
  `party_plans` gives each member its own occurrence. There is no party service, so the live adapter `SoloParty`
  makes every party cast fail (C.3 step 1). Tests cover C.3 tests 1-4, 6, 7 and the engine part of 5.
- **C.5 Cancel Magic Shield**: no native key (plain `remove_condition` magic_shield); a test covers C.5 tests 1-2
  (cast succeeds without a shield, the Canary reading; Q23 open).
- Unresolved effects now carry their Effect key, so the condition owner can apply a member condition.
- Every other key, including the other Part C keys, stays rejected (S7/D13).

## Not admitted (design packet)

- **C.1 `familiar_summon`**: needs summon creation and ownership, a 900 s lifetime that pauses offline and is
  restored on login (durable), expiry warnings, owner-follow teleport and Lever Boss refusal. No accepted owner or
  contract for summons, creature spawning by a player, or durable familiar state. Candidate location: a
  summon/creature-ownership contract beside `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`
  (runtime actor ownership) plus a DUR-02 durable field for the remaining familiar time.
- **C.2 `acquire_summon`** (Summon Creature, Animate Dead, Convince): the same summon ownership and cap owner, the
  `player_parameter` name argument (B.3) and the corpse `tile_item_operation` (D.2.3). Speed spells need no key but
  a condition runtime (haste, paralysis interplay, damage-dealt modifier), which has no owner.
- **C.4 `stance_toggle`**: the `standard` slot persists across logout and death (C.4 step 3), which is durable
  character state; SPELL-D2 keeps runtime state actor-local and non-durable, and no accepted contract holds a stance
  slot. It also needs skill and damage modifier hooks in combat. Candidate location: a SPELL-D amendment (like
  SPELL-D8 for Harmony) defining the durable `standard` slot under DUR-02, and the combat modifier seam.
- **C.5 virtues**: `stance_toggle` plus the Harmony and Serene owner (SPELL-D8, still a candidate) and a party
  service for the party bonuses.
- **Heal/Train/Protect/Enchant Party content**: the converter still emits `unresolved` for them; emitting
  `party_buff` bundles regenerates the readiness sample and needs the pinned Canary checkout. Enlighten Party stays
  blocked on Q14.

Key naming: S27 lists the Part C families (`party`, `familiar`, ...); the authored key follows the C.3 authoring
shape, `party_buff`.

## Excluded scope

The live cast wire and scheduler, the condition runtime, a party service, the converter and data.

## Validation

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-game-server`
- `python validate_spell.py` over the starter bundles and a Heal Party `party_buff` bundle,
  `python verify_formal_schema.py`, `python tools/agents/validate_governance.py`,
  `python tools/repository/validate_repository_policy.py`

## Terminal integration

- **Final head:** `c2596f3199eac7a2d4fa91d9c97ebc3cefc51ff8`.
- **Integration:** merged into main as PR #1224, merge commit
  `da1b2ac54147160ff30df46b479e3240211073f3` (2026-09-29T10:49:02Z).
- **Closeout:** the record was archived by `OTV2-20260929-spell-part-d` (issue #162, allocation comment 5888930608).
- **Owned paths:** released.
- **Binding carry-over:** the design packet above stays the reference for the Part C keys still rejected
  (`familiar_summon`, `acquire_summon`, `stance_toggle`, the virtues); Q13-Q18 stay with the owner's in-game tests.
