# OTV2-20261006 Quest Talk/NPC Candidate Qualification

status: implementing
base_main: f6894e793c162d9d8a43578332f3a6f77d2936a3
branch: codex/quest-talk-npc-candidates-20261006
runtime_activation: false

## Goal

Qualify source-derived Quest `talk` stages against the current canonical NPC and Dialogue families using exact existing identities only. Produce an auditable candidate inventory without selecting dialogue branches or admitting runtime dispatch.

## Owned paths

- `tools/content-schema/quest-authoring/quest_talk_npc_candidates.py`
- `tools/content-schema/quest-authoring/test_quest_talk_npc_candidates.py`
- `tools/content-schema/quest-authoring/samples/server-completion/talk-npc-candidates.json`
- `docs/agents/tasks/OTV2-20261006-quest-talk-npc-candidates.md`

No shared `run_checks.py`, canonical NPC, Dialogue, Quest, or runtime file is changed.

## Population

The current completion binding plan contains 326 `talk` stages. The existing authored lane already supplies NPC candidates for 79 of them. This task audits the remaining **247 source-derived talk stages**.

Results:

- `EXACT_NPC_WITH_DIALOGUE`: **171 stages**
  - **93 quests**
  - **129 unique canonical NPCs**
  - **129 unique canonical Dialogues**
- `EXACT_NPC_NO_DIALOGUE`: **23 stages**, across 18 quests
- `AMBIGUOUS_MULTIPLE_NPCS`: **36 stages**, across 23 quests
- `NO_EXACT_NPC`: **17 stages**, across 15 quests

Runtime bindings: **0**  
Selected dialogue branches: **0**

## Matching contract

Allowed aliases are intentionally narrow:

1. canonical NPC identity key tail, e.g. `oteryn:npc.biff_the_baker -> biff_the_baker`;
2. an existing NPC `source_bindings[].external_id` only when that binding already has `disposition=EXACT`.

Both are compared only after deterministic punctuation/underscore/case normalization.

Explicitly forbidden:

- fuzzy matching;
- display-name similarity;
- wiki/name guessing;
- selecting one NPC when multiple exact NPCs appear in the same stage;
- treating a canonical Dialogue ref as proof of the correct dialogue branch;
- runtime admission.

## Examples

### Exact candidate

`A Piece of Cake / s1`

- target: `Biff the Baker`
- NPC: `oteryn:npc.biff_the_baker`
- Dialogue: `oteryn:dialogue.npc.biff_the_baker`
- selected branch: null
- native dispatch binding: null

### Mixed NPC and non-NPC targets

`An Interest In Botany / s1`

Targets are `Rabaz` and `Botany Almanach`. Only Rabaz resolves to canonical NPC, so the stage gets one exact NPC/Dialogue candidate. The item/text target is not coerced into NPC identity.

### Ambiguous stage

`Between the Lines / s3`

Targets contain both Phillip and Wyrdin; both are exact canonical NPCs. No NPC is selected and the stage remains held.

### Exact NPC but no Dialogue

`Barbarian Test / s1`

Sven resolves exactly to `oteryn:npc.sven`, but the canonical NPC has no Dialogue ref. The stage remains held.

### No exact match

`25 Years of Tibia / s5`

`Lord Retro` has no exact canonical NPC alias under the accepted matching contract. No fuzzy substitution is attempted.

## Architecture boundary

A canonical NPC and Dialogue identity is not sufficient to bind quest progression.

The current NPC packet architecture separates:

- NPC/Dialogue content identity;
- dialogue branch selection and quest condition/action semantics;
- NPC talk runtime;
- durable QuestState transition request.

Historical NPC completion evidence also kept native progress/transition links at zero until an explicit Quest authoring/runtime owner existed. This task therefore only establishes exact identity candidates.

Each of the 171 candidates still holds:

- `DIALOGUE_BRANCH_SELECTION_PENDING`
- `NATIVE_NPC_QUEST_DISPATCH_BINDING_PENDING`

## Validation

Focused validation:

- generator: **247** source-derived talk stages
- exact NPC+Dialogue candidates: **171**
- dedicated unittest: **7/7 PASS**
- generator `--check`: PASS
- committed packet byte-for-byte drift test: PASS
- `git diff --check`: PASS

## Next action

The next NPC/Quest lane should operate only on the 171 exact identity candidates and must prove, per candidate:

1. which exact Dialogue branch corresponds to the chosen Quest stage;
2. that the branch's quest guard/action semantics match the stage occurrence;
3. that the NPC talk runtime can produce the exact Quest transition request cause;
4. that no source-context variant (for example Captain Haba Open Sea) changes the actor identity required by the binding.

The 36 multi-NPC, 23 no-Dialogue and 17 no-exact-match stages remain separate holds and must not be auto-resolved by this task.
