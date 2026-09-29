# OTV2-20260929-spell-part-d

```yaml
task_id: OTV2-20260929-spell-part-d
title: Engine runtime for the Part D spell native behaviours (S27)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1228
allocation_comment: "#162 5888930608"
base_branch: main
branch: claude/spell-part-d
base_sha: da1b2ac54147160ff30df46b479e3240211073f3
head_sha: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T11:30:00Z
updated_at: 2026-09-29T11:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/ability/**
  - apps/game-server/tests/*spell*
  - tools/content-schema/spell-authoring/**
  - docs/agents/tasks/active/OTV2-20260929-spell-part-d.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-part-c.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Part D of `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` (owner S27), implemented where no new state, owner, wire
or path outside this task is needed. None of the Part D shapes that are admitted is a `native_behavior` key; they
are spell fields and a formula input.

- **D.3 `target_default`, admitted** as `targeting.allowed_targets` (`any`, `self_only`, `self_or_own_summons`;
  absent = `any`). `spell/target.rs` checks a `CastTarget` (caster, creature, master). `resolve_targeted_cast` runs
  the cast checks first, then refuses a target that is not allowed (`TargetNotAllowed`); nothing is spent.
  `resolve_cast` refuses a targeted cast of such a spell without the target facts (`TargetFactsRequired`).
  The accepted `CastResolution` keeps the checked target (opaque `CheckedTarget`); `effect_plan` refuses any
  other target (`TargetMismatch`, Codex P2 4132906709). The
  reader rejects `allowed_targets` together with a chain or a party buff. No summon owner exists, so every live
  creature has no master and only the caster qualifies (as `SoloParty` in Part C). Q7 uses the doc proposal
  (`self_or_own_summons`, F); Q8 is plain rune vocations. Tests cover D.3.4 tests 1-5.
- **D.4 `equipment_dependent`, admitted** as the spell fields `needs_weapon` (read now; it was ignored before) and
  `needs_shield` (optional, default false), and the formula input `shield_defense`. `CasterState` carries
  `melee_weapon` and `shield_defense` from the equipment owner. `needs_weapon` is checked after the vocation and
  before premium (Canary/Crystal `playerSpellCheck`); `needs_shield` after the engine checks (script check). The
  reader and `validate_spell.py` reject a formula that reads `shield_defense` in a spell without `needs_shield`.
  Tests cover D.4.4 tests 1 and 2 (99..121, average 110).
- The Part D `native_behavior` keys (`owned_field_buff`, `delayed_strike`, `tile_item_operation`,
  `monster_ai_override`) and the family names stay rejected (S7/D13); `cast_at_position` stays rejected (SPELL-D7
  "until delivered").

## Not admitted (design packet)

- **D.1 `delayed_or_repeated`**: the Avatars, Divine Empowerment, Divine Grenade and the Spiritual Outburst repeat
  are Wheel spells (`wheel_unlock`, S6) and need the condition runtime (damage-taken and critical modifiers), an
  owned field item with a damage hook, or Harmony (SPELL-D8, candidate). Death Echo is not Wheel-gated but is a
  `cast_at_position` spell, and its echo needs a scheduled strike that re-reads the area at expiry and drops when
  the caster is offline: a scheduler owner and the area resolver, neither of which exists in the core. Candidate
  location: the SPELL-D7 delivery child (position intent) plus a `delayed_strike` occurrence in the Ability
  pipeline (like chain hits, `delay_micros`) with a world re-read at expiry. Q1-Q3, Q5, Q19, Q20 stay open.
- **D.2 `target_position`**: the chosen-position spells wait for SPELL-D7 step 1 (protocol lane). The three runes
  (`tile_item_operation`) need a use-with-on-tile command, summon creation and ownership (GAME-AI-01), map item
  removal with the Desintegrate rules, and an appearance condition (Chameleon). No owner exists for any of them.
- **D.3 remainder**: the "caster or top creature" rule on a used tile is `Effect.affects.top_creature_only`
  without `kind`, which is the monster authoring schema (`tools/content-schema/monster-authoring/`), outside this
  task's paths. The current rune target is the creature the Target Resolver names.
- **D.4 remainder**: the shield debuff is a new condition type `next_auto_attack_reduction` (monster schema
  `condition`, outside this task's paths) and needs the condition runtime with an auto-attack hook; D.4.4 tests 4
  and 5 wait for it, so Shield Bash and Shield Slam cannot be authored complete yet. Shield Slam's 3x3 area (test 3)
  and the weapon pick of test 6 (left hand, then right; `attack_*` from the weapon) are the area resolver and a
  world combat rule outside the core. Flurry of Blows, Sweeping Takedown and Spiritual Outburst stay behind
  Harmony (S26); Q9 and Q10 are open.
- **D.5 `extra_presentation_only`**: `presentation.caster_effect_asset_binding` on `Effect` is a monster schema
  change (outside this task's paths). Heal Friend also needs the player-name parameter (B.3, no V1 wire field) and
  `allow_on_self`, which the reader still ignores; Paralyze Rune needs the paralysis condition runtime.
- **D.6**: Magic Wall and Wild Growth need map item creation with a duration and tile checks (no owner) and Q12/Q13;
  Inflict Wound needs the bleeding condition and Q14 (values unknown, kept rejected); Challenge and Balanced Brawl
  need a `monster_ai_override` hook in the AI owner (outside this task's paths); Mass Spirit Mend waits for Q16.

The converter still emits `blocked` for the Part D spells; emitting the new fields regenerates the readiness
sample, which needs the pinned Canary checkout.

## Excluded scope

The live cast wire and scheduler, the condition runtime, summon ownership, map item mutation, the monster
authoring schema, the converter and data. The disposition mapping of the new rejections (`WeaponRequired`,
`ShieldRequired`, `TargetNotAllowed`) is for the protocol owner (proposal: `TARGET_ILLEGAL` for the target rule,
`REJECTED` otherwise).

## Validation

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --quiet -- -D warnings`,
  `cargo test --quiet -p oteryn-game-server`
- `python build_formal_schema.py` (regenerated), `python verify_formal_schema.py`, `python validate_spell.py` over
  the starter bundles and an Ultimate Healing Rune and a Shield Bash bundle,
  `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`
