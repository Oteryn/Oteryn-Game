# OTV2-20261006 Quest Completion All-373 R2

status: implementing
base_main: 52712f633786721c0d255888a92c81165cd2d5c3
branch: codex/quest-completion-all373-20261006-r2
runtime_activation: false
last_progress: completion candidate regenerated on current main with 303 canonical quest owners and 8 exact kill-to-Encounter binding candidates qualified fail-closed
next_action: implement the first native event-dispatch lane for exact kill outcomes without promoting unrelated quest stages

## Goal

Carry the full 373-title Quest completion backlog on top of current `main`, without reverting newer NPC / Make Believe / binding work and without treating chosen approximation as donor equivalence.

## Current completion state

Freshly regenerated from current main:

- wiki titles: **373**
- canonical Quest definitions: **352**
- mapped wiki titles: **373 / 373**
- completion candidate owners: **303**
- typed tracks: **2445**
- typed transitions: **4378**
- completion transitions: **211**
- existing source-lowered owners: **96**
- authored chosen typed-progress owners: **68**
- chosen-source typed-progress owners added by this lane: **139**
- chosen-source recipes held fail-closed: **7**
- binding-plan owners: **207**
- binding-plan stages: **1115**
- native event dispatch bindings: **0**
- native NPC dialogue bindings: **0**
- native reward delivery bindings: **0**
- runtime activation: **false**

The seven held chosen-source recipes are:

- Bear Room Quest
- Behemoth Quest
- Demon Helmet Quest
- Dragon Tower Quest
- Edron Goblin Quest
- Opticording Sphere Quest
- Rift Warrior Outfits Quest

They have terminal `complete` stages with count greater than one. They remain held because a generic automatic completion reducer would not provide the required 1:1 request cause.

## All-373 backlog after typed-progress expansion

The generated matrix now separates source fidelity from implementation work.

Implementation state:

- `NATIVE_BINDINGS_PENDING`: **214**
- `NATIVE_LOWERING_PENDING`: **117**
- `DEFINITION_READY_RUNTIME_UNKNOWN`: **41**
- `MAPPING_REVIEW`: **1**

Typed-progress state:

- `CHOSEN_SOURCE_TYPED_PROGRESS_ONLY`: **139**
- `CHOSEN_TYPED_PROGRESS_ONLY`: **68**
- `LOWERED`: **7**
- `NOT_LOWERED_MULTI_TRACK`: **72**
- `NOT_LOWERED_NO_MISSIONS`: **38**
- `NO_CANDIDATE`: **48**
- `MULTIPLE`: **1**

Source fidelity is tracked independently:

- source holds present: **222** wiki titles
- source holds clear: **151** wiki titles

A source hold does not erase a consciously chosen Oteryn recipe or typed-progress candidate.

## Chosen-source event / reward qualification

The source139 event/reward packet is regenerated from current definitions against the exact historical qualified World snapshot:

- archive SHA256: `617c34e33099feefc5ab675ed7f067cfb33bfeb1c2bbe71e5477a880684dc420`
- qualified World declarations SHA256: `e20b2e849ce03e00acab7d38196610ab1ba05db18f057d073450b83fcb51fe4b`
- regenerated packet SHA256: `930b0ef70b152d395d9a0cc3194a195daa57b48705d53753961f981908012162`

Packet counts:

- quests: **139**
- stages: **699**
- non-dialogue stages: **616**
- exact stage target refs: **283**
- reward intents: **300**
- exact reward refs: **232**
- explicit non-identity reward intents: **3**
- exact existing Encounter outcome seams: **8**

## Exact kill-to-Encounter candidates

These are qualification candidates only. Every row remains `execution_verified=false`, `native_dispatch_binding=null`, `runtime_admitted=false`.

1. Dark Trails `s7`: Death Priest Shargon -> `oteryn:encounter.death_priest_shargon`, party credit.
2. Ferumbras Ascension `s4`: Plagirath -> `oteryn:encounter.plagirath`, damage-contributors credit.
3. Ferumbras Ascension `s6`: Mazoran -> `oteryn:encounter.mazoran`, damage-contributors credit.
4. Ferumbras Ascension `s7`: Razzagorn -> `oteryn:encounter.razzagorn`, damage-contributors credit.
5. Ferumbras Ascension `s8`: Ragiaz -> `oteryn:encounter.ragiaz`, damage-contributors credit.
6. Ferumbras Ascension `s9`: Tarbaz -> `oteryn:encounter.tarbaz`, damage-contributors credit.
7. Ferumbras Ascension `s10`: Shulgrax -> `oteryn:encounter.shulgrax`, damage-contributors credit.
8. Ferumbras Ascension `s11`: Ferumbras Mortal Shell -> `oteryn:encounter.ferumbras_mortal_shell`, damage-contributors credit.

The server already has `QuestCause::CreatureDeath` and durable `commit_quest_transition`, but no production death-pipeline quest consumer has been proven for these rows. Do not promote these seams merely from identity association.

## Validation on current main base

Focused qualification after final LF / SHA repin:

- `quest_completion_import.py --check`: PASS
- `quest_completion_matrix.py --check`: PASS
- chosen-source progress / binding / completion / matrix tests: **18 PASS**
- chosen-source event/reward packet tests: **6 PASS**
- `git diff --check`: PASS
- Quest schema: **269 / 269 PASS**
- RewardClaim authoring: **13 PASS**
- RewardClaim stack serialization: **6 PASS**
- RewardClaim content check: PASS
- RewardClaim variant migration check: PASS

A targeted regression set covering the prior failures ran **83 tests** on this branch and failed with exactly **7 errors**. The same exact 7 errors reproduce on a clean worktree at base main `52712f633786721c0d255888a92c81165cd2d5c3`:

- 5 requester/source-occurrence tests expecting `write.requested_by`;
- Crystal `The Ultimate Challenges` primary-selection fixture;
- Windows archive member path in chosen-journal replay.

They are baseline failures and are not introduced by this branch.

A broader pre-cleanup run also showed missing local `g++` for the map-node test. The attempted `ots_questlog.py` Windows path normalization was removed from this branch because `ots_questlog.py` is SHA-fenced by `source_fix_guard` and requires a separate reviewed source-repair receipt.

## Safety boundaries

- no production runtime activation;
- no donor-equivalence claim for chosen-source recipes;
- no fuzzy identity joins;
- no automatic terminal reducer for the seven held quests;
- no mutation of production `content/quests/missions/quest-state.json`;
- Make Believe post-release event-count correction overlay from current main is preserved;
- current-main NPC/binding changes are preserved rather than overwritten by the older completion branch.

## Next implementation lane

Start with the **8 exact kill-to-Encounter seams** and prove the actual creature-death consumer boundary. A binding may be admitted only when the real death/outcome occurrence can produce the exact `QuestTransitionRequest` cause and owner required by QuestState. If the encounter owner does not expose that boundary, escalate the owner/architecture seam rather than introducing a quest-specific bypass.
