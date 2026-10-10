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


## Superseding checkpoint — terminal-count normalization

**Current branch:** `codex/quest-terminal-normalization-20261006`
**Base main:** `ee71e79eccd1d498d6c39ea25ac01ee74ccd118c`
**Runtime activation:** `false`

This checkpoint supersedes the older held-terminal counts above. The immutable canonical chosen recipes remain unchanged; the correction is applied only in the typed candidate projection through a SHA-fenced terminal-normalization packet.

### Result

- canonical Quest definitions: **352**
- wiki titles mapped: **373 / 373**
- completion candidate owners: **310**
- typed tracks: **3219**
- typed transitions: **5154**
- completion transitions: **308**
- chosen-source projected owners: **236**
  - new owners: **146**
  - additive overlays over existing Source owners: **90**
  - Source-complete owners intentionally skipped: **6**
- chosen-source terminal-count holds: **0**
- binding-plan owners: **304**
- binding-plan stages: **1891**
- native event dispatch bindings: **0**
- native NPC dialogue bindings: **0**
- native reward delivery bindings: **0**
- runtime activation: **false**

### All-373 implementation state

- `NATIVE_BINDINGS_PENDING`: **331**
- `DEFINITION_READY_RUNTIME_UNKNOWN`: **42**
- `NATIVE_LOWERING_PENDING`: **0**

Typed-progress state:

- `CHOSEN_SOURCE_TYPED_PROGRESS_ONLY`: **146**
- `SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY`: **110** wiki-title rows
- `CHOSEN_TYPED_PROGRESS_ONLY`: **68**
- `LOWERED`: **7**
- `NO_CANDIDATE`: **42**

Source fidelity remains a separate axis:

- `SOURCE_HOLDS_PRESENT`: **221**
- `SOURCE_HOLDS_CLEAR`: **152**

### Nine terminal-count normalizations

The previous holds were caused by chosen recipes using terminal `complete count > 1` to describe work that must happen before completion. The candidate projection now normalizes exactly these nine records, while leaving canonical recipe payloads and SOURCE evidence untouched:

- Barbarian Arena Quest: collect 3 trophies, then complete once.
- Bear Room Quest: use/open 3 reward boxes, then complete once.
- Behemoth Quest: use/open 4 reward chests, then complete once.
- Demon Helmet Quest: use/open 3 reward chests, then complete once.
- Dragon Tower Quest: use/open 2 supply boxes, then complete once.
- Edron Goblin Quest: use/open 2 throne-room chests, then complete once.
- Opticording Sphere Quest: use/open 2 final ornate chests, then complete once.
- Rift Warrior Outfits Quest: the two 100-token inventory stages remain quantity gates; each Cledwyn handoff is one dialogue occurrence, then complete once.
- The Ancient Tombs Quest: one combination action after the seven pharaoh trials, then complete once.

Normalization packet:

- schema: `OTERYN_QUEST_TERMINAL_NORMALIZATION/v1`
- packet SHA256: `234fa874bf5652339147e5b224c3bb0da2fb992afefebb834b0c3e28d69d82d0`
- runtime enabled: `false`
- source holds preserved: `true`

### Source event/reward projection after normalization

- schema: `OTERYN_CHOSEN_SOURCE_EVENT_REWARD_ASSOCIATIONS/v3`
- packet SHA256: `1219badd0163147d357fda0bd4fe0c624f0d0c7924af1629a92856440a2f1c14`
- quests: **236**
- stages: **1475**
- non-dialogue stages: **1225**
- exact stage target refs: **617**
- reward intents: **508**
- exact reward refs: **372**
- exact existing Encounter outcome seams: **25**

Every stage and reward remains non-executable: `runtime_admitted=false`; all native binding fields remain null.

### Validation

Focused final validation on this branch:

- terminal-stage refinement tests: PASS
- chosen-source progress tests: PASS
- chosen-source binding-plan tests: PASS
- completion importer tests: PASS
- all-373 matrix tests: PASS
- chosen-source event/reward packet tests: PASS
- combined focused suite: **26 PASS**
- event/reward packet suite: **6 PASS**
- `quest_completion_import.py --check`: PASS
- `quest_completion_matrix.py --check`: PASS
- `git diff --check`: PASS

### Next lane

The lowering backlog is closed. Work should now move only to native binding/runtime owner seams. The Encounter kill lane is still blocked on the accepted Encounter runtime architecture (`ENC-RT-1` / `ENC-OUTCOME-1`); do not introduce a quest-specific death bypass.
