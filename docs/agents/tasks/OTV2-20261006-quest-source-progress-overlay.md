# OTV2-20261006 Quest Source Progress Overlay

status: implementing
repository: Oteryn/Oteryn-Game
base_pr: 1875
base_sha: 4229fa24da103219ff4583b77383300f5214bbdb
branch: codex/quest-source-overlay-20261006
runtime_activation: false
next_action: keep the 9 non-unit terminal recipes held and move the remaining 322 title rows to native binding work

## Purpose

Add chosen Oteryn typed-progress as an additive overlay to existing Source-lowered Quest owners whose Source completion is explicitly not lowered. Preserve every Source track and transition unchanged; do not claim donor equivalence or activate runtime.

## Source recipe partition

The 242 `chosen_source_completion_v1` canonical recipes partition exactly as:

- 139 new owners already introduced by PR #1875;
- 88 existing Source owners eligible for additive chosen-progress overlay;
- 6 existing Source owners already `LOWERED` and deliberately left untouched;
- 9 recipes held because the terminal `complete` stage has count > 1.

The six Source-complete owners left untouched are Hot Cuisine, Oramond, Sam's Old Backpack, Spirithunters, The Ape City and The Outlaw Camp.

The nine held recipes are Bear Room, Behemoth, Demon Helmet, Dragon Tower, Edron Goblin, Barbarian Arena, Opticording Sphere, Rift Warrior Outfits and The Ancient Tombs.

## Candidate result

The candidate still has **303 canonical Quest owners**. The overlay changes only typed progress depth:

- tracks: **3152**
- transitions: **5085**
- completion transitions: **299**
- Source + chosen overlay owners: **88**
- new chosen-source owners: **139**
- authored chosen owners: **68**
- Source-complete owners: **6**
- remaining Source NOT_LOWERED owners: **2**
- held chosen-source recipes: **9**
- runtime activation: **false**

Every one of the 88 overlay records retains the original Source tracks and Source transitions as an exact prefix. New tracks and transitions use only the `oteryn:quest-progress/chosen-source/**` and `oteryn:quest-transition/chosen-source/**` namespaces. Key collisions are rejected.

## Binding evidence result

The regenerated chosen-source event/reward packet now covers all **227** projected chosen-source recipes:

- stages: **1406**
- non-dialogue stages: **1159**
- exact stage target refs: **599**
- reward intents: **485**
- exact reward refs: **353**
- explicit non-identity reward intents: **15**
- exact existing Encounter outcome seams: **25**
- packet SHA256: `3bd94c3673cda49bfadb3e14b24e26210fbdf0cb11c2387156619458d8105fde`

It is regenerated against the same qualified World declaration SHA256
`e20b2e849ce03e00acab7d38196610ab1ba05db18f057d073450b83fcb51fe4b`.

The completion binding plan grows to:

- **295 quest owners**
- **1822 stages**
- native event dispatch bindings: 0
- native NPC dialogue bindings: 0
- native reward delivery bindings: 0
- runtime enabled: false

## All-373 backlog after overlay

- `NATIVE_BINDINGS_PENDING`: **322**
- `NATIVE_LOWERING_PENDING`: **9**
- `DEFINITION_READY_RUNTIME_UNKNOWN`: **41**
- `MAPPING_REVIEW`: **1**

Typed-progress title states:

- `CHOSEN_SOURCE_TYPED_PROGRESS_ONLY`: 139
- `SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY`: 108
- `CHOSEN_TYPED_PROGRESS_ONLY`: 68
- `LOWERED`: 7
- `NOT_LOWERED_MULTI_TRACK`: 1
- `NOT_LOWERED_NO_MISSIONS`: 1
- `NO_CANDIDATE`: 48
- `MULTIPLE`: 1

Source fidelity remains a separate axis and is unchanged: 222 title rows retain source holds and 151 are clear.

## Why the 9 recipes remain held

QuestState marks a quest complete from the committed transition's plain `completes` boolean. A terminal stage with count > 1 cannot safely use one repeated transition with `completes=true`: the first occurrence would mark the quest complete. Those recipes need an explicit progress-vs-final transition design and owning event-selection semantics, not a relaxed counter check.

## Encounter dependency

The 25 exact Encounter outcome seams remain qualification evidence only. ENCOUNTER-RT-0 assigns real outcome delivery to ENC-OUTCOME-1, which depends on ENC-RT-1 and currently has no allocated production runtime. Do not introduce a quest-only death bypass.

## Validation

Exact stacked worktree validation:

- `quest_completion_import.py --check`: PASS
- `quest_completion_matrix.py --check`: PASS
- chosen-source progress / binding / completion / matrix tests: **19 PASS**
- chosen-source event/reward packet tests: **6 PASS**
- `git diff --check`: PASS
- Source-prefix preservation proved for all 88 overlay owners
- no runtime promotion and no native binding admitted
