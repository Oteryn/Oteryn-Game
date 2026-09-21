# CONTENT-QUEST-01 r10 — Annihilator source QuestLog / journal witness

Status: **OTS_HYPOTHESIS_ONLY / SOURCE_PRESENTATION_FOOTPRINT / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709
Selected source: `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`, `data-global`.

## Purpose

Close the remaining source-side question of whether the selected Crystal Annihilator is represented in the central QuestLog/mission catalogue or whether its observable source footprint is execution/reward/access/achievement driven without a central QuestLog definition.

This is source-footprint evidence only. Absence from Crystal QuestLog does not prove absence from Global Tibia at the 2026-07-28 target cut.

## Exact source inputs

| Path | Git blob | Role |
|---|---|---|
| `data-global/lib/core/quests.lua` | `1b3e292826d90e9b92b75f091566d2fe57e682b3` | central Quest/mission catalogue |
| `data/libs/functions/quests.lua` | `7c2d39d0aa59cd598461c240ed6112cab552343d` | QuestLog/QuestLine/tracking projection logic |
| `data-global/lib/core/storages.lua` | `af3e5b1317ef73e2eb6e870835a785f2cf10212a` | legacy storage declarations |

## Full central-catalogue readback

`data-global/lib/core/quests.lua` was read in full for the selected revision. Exact occurrence census:

```text
Annihilator                                  -> 0
TheAnnihilator                               -> 0
Storage.Quest.U7_24.TheAnnihilator           -> 0
numeric token 10102                          -> 1
```

The one `10102` occurrence is **not Annihilator**. It is:

```text
Paw and Fur: Wyverns
storageId = 65016
missionId = 10102
```

This is a concrete identifier-kind collision already predicted by r2/F03. Numeric equality between a legacy storage/action value and an unrelated mission ID is not semantic identity.

## QuestLog projection mechanics

`data/libs/functions/quests.lua` explicitly loads the central catalogue with:

```text
dofile(DATA_DIRECTORY .. "/lib/core/quests.lua")
```

`Player.sendQuestLog()` enumerates `questId = 1..#Quests` and emits only entries for which the corresponding central Quest definition is started. `Player.sendQuestLine()` likewise resolves the selected central Quest and its `missions`.

`Player.getQuestDataByMissionId()` scans `Quests` and their missions for a matching `mission.missionId`. Therefore the unrelated `missionId = 10102` belongs to its own Quest catalogue context and cannot be rebound to `Storage.Quest.U7_24.TheAnnihilator.Reward` merely because the number matches.

## Dynamic-addition census

A repository-wide search for `Game.addQuest(` at the selected revision returns only the function definition in `data/libs/functions/quests.lua`. No dynamic Annihilator Quest definition was found outside the central catalogue.

Within this bounded textual source footprint, the selected `data-global` Annihilator therefore has:

- lever/admission source logic;
- static-map/startup bindings;
- reward selectors and reward-consumption storage;
- achievement side effect;
- door/access use of the legacy scalar;
- Avar Tar cosmetic/outfit follow-up;
- **no central QuestLog/QuestLine definition found**.

## Importer consequence

Crystal source completeness and Oteryn journal authoring are separate dimensions.

The importer must not synthesize a native Quest journal entry merely because:

- a `Storage.Quest.*` symbol exists;
- a reward chest writes that storage;
- an achievement has the same human-readable name;
- an unrelated `missionId` happens to equal the numeric storage value;
- a community wiki presents the activity as a quest.

An Oteryn `QuestDefinition` may still exist as the canonical native progression/orchestration model even where the source had no central QuestLog entry. Its client/journal projection is an explicit authored/evidence-bound projection under Q18, not a mandatory mirror of Crystal's central `Quests` table.

For Reference content, exact target QuestLog visibility/text/state remains `EVIDENCE_REQUIRED`. Source absence is not target absence.

## Native modeling consequence

The selected source is a useful counterexample to treating legacy presentation metadata as the canonical quest domain:

```text
source gameplay footprint != source QuestLog catalogue entry
native QuestDefinition != client journal projection
```

Therefore:

- Quest durable progression can be canonical even when client journal projection is absent or evidence-gated;
- client journal visibility cannot become the trigger for gameplay progress/completion;
- achievement/reward/access/cosmetic outputs remain separate owned channels;
- source importer provenance must retain `source_journal_entry = NOT_FOUND_IN_SELECTED_TEXTUAL_CATALOGUE` rather than fabricating text/state;
- later official evidence may author a Reference journal projection without rewriting the source footprint record.

## New required test

- **T80 — execution-only source quest / no synthetic journal:** a source footprint with valid gameplay/reward/access evidence but no central QuestLog definition imports as an explicit journal-unknown/absent-source-presentation candidate; the importer must not synthesize journal text/state, must not bind an unrelated same-number mission ID, and must keep native gameplay authority independent from client journal projection.

Required CONTENT-QUEST-01 corpus is now **80 cases**.

## Evidence classification

```yaml
annihilator_source_questlog:
  source_classification: OTS_HYPOTHESIS_ONLY
  central_quest_catalogue_read: FULL_FOR_SELECTED_FILE
  central_annihilator_entry: NOT_FOUND
  dynamic_addquest_annihilator_entry: NOT_FOUND_IN_REPO_WIDE_TEXT_SEARCH
  numeric_10102_collision:
    annihilator_legacy_storage: OBSERVED_ELSEWHERE
    central_mission_id: Paw_and_Fur_Wyverns
    semantic_identity: REJECTED
  native_quest_model: INDEPENDENT_OF_SOURCE_JOURNAL_PRESENCE
  reference_target_questlog: EVIDENCE_REQUIRED
```

This r10 witness composes with r1-r9 and does not promote source absence to Global target truth.
