# OTV2-20260930-spell-present0

```yaml
task_id: OTV2-20260930-spell-present0
title: "SPELL-PRESENT-0 spell and combat presentation"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-spell-present-0
pr: "the PR named in the #162 FREEZE_SHA entry"
base_sha: a6a054e6
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SPELL_PRESENT0_SPELL_AND_COMBAT_PRESENTATION_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-spell-present0.md
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
public_contracts:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

SPELL-PRESENT-0 decides how spells and combat are shown (owner direction, 2026-09-30: build now,
full Global parity).

- **Stream:** capability `PRESENTATION_V1` and domain `WORLD_PRESENTATION`: one-shot, bounded,
  non-durable events (magic effects, projectiles, spell words by index, damage, heal, mana and
  experience numbers, sound cues) in the sync unit of the committed change.
- **Words and refusals:** words only after a successful cast, in CHAT-0's `say` range; the refusal
  smoke to spectators with Canary's exceptions; the cast result gains `detail` for the refusal texts.
- **Cooldowns:** own-actor state domain `ACTOR_COOLDOWNS` with snapshots, so the bar survives a
  reconnect.
- **Content:** a binding table from asset keys and sound cues to 15.30 client ids; creature `race`.
- Pointer amendments to SPELL-D1 §10, CHAT-0 §3 and ATTACK-0 §3, pending on acceptance.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: SPELL-D1 to SPELL-D8 and `actor_spell.rs`; S9 and S18; content presentation fields
  (`content/project/v2/creature.rs`); VIS-2; CHAT-0; ATTACK-0; CONDITIONS-0; FND-02; the 15.30
  client asset manifest (no sound files).
- `DERIVED`: the Tibia manual (`magic.md`, `interface.md`, `combat.md`); Canary `04b83b51`
  (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. PRESENT-WIRE-1 needs protocol review; COMBAT-PRESENT-1 combat and
determinism review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol, combat, determinism).
- [ ] Protected Merge Queue integration.

## Owner questions

- P1 (§14): the 15.30 client sound files and their redistribution rights; recommendation a.

## Excluded scope

- Code; actor names on the wire; parameter spell words; rune use; blood splashes; analysers;
  durable cooldowns; item-use exhaustion.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-spell-present-0
owner_action_required: "P1 (sound files)"
blocker: null
next_action: null
```
