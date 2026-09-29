# OTV2-20260929-spell-part-b

```yaml
task_id: OTV2-20260929-spell-part-b
title: Engine runtime for the Part B spell native behaviours (S27)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1231
allocation_comment: "#162 5890192652"
base_branch: main
branch: claude/spell-part-b
base_sha: 23f9535a31e4d5544be9107b509d5a0b95b3b06a
head_sha: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T12:30:00Z
updated_at: 2026-09-29T12:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/ability/**
  - apps/game-server/tests/*spell*
  - tools/content-schema/spell-authoring/**
  - docs/agents/tasks/active/OTV2-20260929-spell-part-b.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-part-d.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Part B of `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` (owner S27), implemented where no new state, owner, wire
or mutation authority is needed. The subset was fixed before any code: only B.5 is admitted; B.3 gets its two pure
rules without admitting a key.

- **B.5 `caster_restriction`, admitted** as the target rule `targeting.allowed_targets: not_self` (the D.3 field;
  the B.5 `target_rule` values `self_only` and `self_or_summon` are already D.3's `self_only` and
  `self_or_own_summons`). `spell/target.rs` refuses the caster as the target (`TargetNotAllowed`) after the cast
  checks; nothing is spent. The reader reads `allow_on_self` false (Canary `allowOnSelf`, which it ignored before)
  as the same rule, rejects it together with `self_only` or `self_or_own_summons`, and rejects `not_self` without
  `needs_target` or with `self_target` (the heal would otherwise fall back to the caster). The schema carries the
  same rules. `forbidden_vocations` is the rune's plain `requirements.vocations` (D.3 Q8, B-QC3); the refusal text
  is the protocol owner's disposition mapping. Tests cover B.5 tests 1-4 (Nature's Embrace on self and on another
  player; both monks refused the Ultimate Healing Rune; the rune on another player, self and own summon; a
  refusal leaves the healing group ready).
- **B.3 P1/P2, spoken matching (not a key).** `SpellBook::spoken` now follows Canary `getInstantSpell` and
  `playerSaySpell`: longest case-insensitive words prefix; a spell without a parameter matches exactly; a
  parameter needs a space and one more character; quoted (unclosed runs to the end, text after the closing quote
  is chat) or one unquoted word (two words are chat); the parameter keeps its case (it was lower-cased before).
  Whitespace runs still collapse. The words of a parameter spell alone select it with an empty parameter (Canary;
  B.3 test 1 "`utevo res` alone nothing" read as nothing summoned). Tests cover B.3 tests 1 and the parsing half of 2.
- **B.3 P7, `locate_message` rule (not a key).** `spell/locate.rs` gives the band (5, 101, 251 tiles, F; QP2), the
  floor and the tangent direction in exact integers, and the F phrase. Tests cover B.3 test 3.
- Every Part B `native_behavior` key and family name stays rejected (S7/D13): `world_query`, `house`,
  `player_parameter`, `item_grant`, `caster_restriction`, `cast_restriction`, `house_access`, `locate_message`,
  `vertical_move`, `creature_appearance`, `summon_named_creature`, `random_item_grant`, `tile_item_operation`,
  `owned_field_buff`, `monster_ai_override`.

## Not admitted (design packet)

- **B.1 `world_query`**: 1a needs the chain `ranged_monsters` filter over monster type target distance, a
  reward-boss flag and a `monster_ai_override` hook in the AI owner (Part D packet); 1b needs map tile flags
  (`rope_spot`), floor-change walkability and a caster teleport (movement owner); 1c needs the Forge fiendish
  registry and Bestiary progress (no owner); 1d is Wheel-gated (S6); 1e needs `tile_item_operation` (map item
  removal, summon creation, appearance condition; Part D packet). Candidate: read-only world traits like
  `ChainWorld` once those owners exist.
- **B.2 `house`**: needs a house system with ownership (QH1: account or character; Platform owns identity),
  guest/subowner/door lists, entry positions and an editor session plus a list-edit wire. All are new persisted
  state and protocol; no accepted contract exists. Candidate: a house contract under `docs/contracts/` with the
  `access_level`, `can_edit_list` and `entry_position` read model the B.2 shape names.
- **B.3 remainder**: the V1 cast wire has no parameter field (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`
  §3 "No parameter field in V1"), so no parameter spell can be cast. P4 also needs an online-player name
  resolver with the staff-access rule and a rejection that starts the cooldown without spending mana; P5 the
  Exiva protection (QP1); P6 the appearance condition runtime; P8 map floor queries and movement; P9 summon
  ownership and creation (GAME-AI-01, Part C packet). Candidate: an additive optional parameter field on the
  cast intent (protocol owner) plus a name resolver beside the Target Resolver.
- **B.4 `item_grant`**: the only item MINT (`durability/item_mint.rs`, DUR-03) takes a creature death cause; there is
  no spell-cause mint, no inventory/container placement with drop-on-tile overflow, and QI1/QI2 are open. The draw
  itself is pure, but admitting Food would promise a grant no owner can apply. Candidate: a spell-cause MINT
  (DUR-03 amendment) and the pickup placement rule from the corpse pickup path.

## Excluded scope

The live cast wire and a talk command, name resolution, the house system, map tile queries and mutation, item
minting, summon ownership, the condition runtime, the converter and data (the converter still emits `blocked` for
the Part B spells; regenerating the readiness sample needs the pinned Canary checkout).

## Validation

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-game-server`
- `python build_formal_schema.py` (regenerated), `python verify_formal_schema.py`, `python validate_spell.py` over
  the starter bundles and a Nature's Embrace `not_self` bundle, `python tools/agents/validate_governance.py`,
  `python tools/repository/validate_repository_policy.py`
