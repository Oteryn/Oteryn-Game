# OTV2-20261006 Quest terminal-count semantics

status: implementing
base_pr: 1876
base_head: debe301202e9b56f95e53e88d637875d652c2f6c
branch: codex/quest-terminal-count-audit-20261006
runtime_activation: false

## Scope

Classify the nine chosen-source Quest recipes that remain fail-closed because the terminal `complete` stage has `count > 1`.

This task is evidence/authoring qualification only. It does not change QuestState runtime, RewardClaim runtime or canonical quest recipes.

## Closed hold set

The exact nine owners are:

- Barbarian Arena Quest
- Bear Room Quest
- Behemoth Quest
- Demon Helmet Quest
- Dragon Tower Quest
- Edron Goblin Quest
- Opticording Sphere Quest
- Rift Warrior Outfits Quest
- The Ancient Tombs Quest

The generated packet is:

`tools/content-schema/quest-authoring/samples/server-completion/terminal-count-semantics/audit.json`

Generator:

`tools/content-schema/quest-authoring/samples/server-completion/terminal-count-semantics/builder.py`

## Classification

### EXACT_CLAIM_SET_CARDINALITY_CANDIDATE — 4

The canonical Quest's retained exact RewardClaim set has the same cardinality as the chosen terminal stage count:

- Behemoth Quest: terminal count 4 / exact RewardClaims 4
- Demon Helmet Quest: 3 / 3
- Dragon Tower Quest: 2 / 2
- Edron Goblin Quest: 2 / 2

All retained RewardClaim refs resolve back to exact source chest records.

This is only a bounded candidate signal. It does not prove that every claim is the terminal occurrence and does not provide the runtime event selector required to complete on the Nth occurrence.

### CLAIM_SET_CARDINALITY_MISMATCH — 2

- Bear Room Quest: terminal count 3 / exact RewardClaims 4.
  The source set contains `bear_chest1`, `bear_chest2`, `bear_chest3` plus the separate key claim `quest/key/id4601`. Selecting the three bear boxes is plausible from names/rewards but is not promoted by cardinality alone.
- Barbarian Arena Quest: terminal count 3 / exact RewardClaims 9.
  The retained source has three reward claims per arena difficulty; the chosen terminal objective refers to three difficulty trophies. No three-claim subset is selected here.

Both require exact source/event review before any subset can become a terminal binding.

### NO_EXACT_REWARD_CLAIM_SET — 3

- Opticording Sphere Quest: terminal count 2, no exact RewardClaim set.
- Rift Warrior Outfits Quest: terminal count 100, no exact RewardClaim set; the chosen objective describes handing Cledwyn the second set of 100 Silver Tokens, so the numeric value may be resource quantity rather than 100 terminal dialogue occurrences.
- The Ancient Tombs Quest: terminal count 7, no exact RewardClaim set; the chosen objective describes combining seven Helmet of the Ancients pieces, so the numeric value may be component quantity rather than seven terminal completion occurrences.

The last two observations are semantic warnings, not automatic count corrections.

## Runtime constraint

Current QuestState completes through a specific requested transition. A terminal counted stage cannot be represented safely by setting one repeated transition to `completes=true`: that would complete on the first accepted occurrence.

A generic automatic reducer is not introduced here. Correct runtime admission needs an owning rule that resolves, for a counted terminal event set, when progress-only vs final-completing transition is requested. For unordered claim sets this must not impose an arbitrary order.

## Existing RewardClaim seam

CHEST-QUEST-BIND-1 is already production-wired:

RewardClaim placement -> RewardClaimMintRequest.quest_transition -> pending ClaimObligation -> gameplay quest refresh -> QuestState.

However the accepted source lowering binds a chest only when its source marker exactly matches an existing Source QuestState track. Current audit:

- Canary chest progress writes with explicit markers: 189
- exact Source-track bindings: 2
- unbound with invalid/null value: 0
- unmatched by exact marker/track rule: 187

The two existing bindings are:

- `quest/u7_8/the_shattered_isles/dragahs_spellbook`
- `quest/u8_4/the_hidden_city_of_beregar/firewalker_boots`

This task does not infer new tracks from RewardClaim markers or chosen recipes.

## Validation

Focused local validation:

- terminal-count builder generation: PASS
- terminal-count audit drift check: PASS
- `test_terminal_count_semantics.py`: 4/4 PASS
- `git diff --check`: PASS

The drift check is registered in `tools/content-schema/quest-authoring/run_checks.py`.

## Next action

1. Obtain an owner/architecture ruling for counted terminal completion semantics before runtime mutation.
2. Review Bear Room and Barbarian Arena exact source occurrences to decide whether a closed terminal claim subset exists.
3. Review Rift Warrior and Ancient Tombs recipe semantics to distinguish quantity from occurrence count; correct the chosen recipe only with pinned evidence and a reviewed refinement path.
4. Keep Opticording Sphere held until an owning terminal occurrence is identified.
