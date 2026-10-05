# ITEM-SEM-BED packet: bed-part semantics

- Packet: `ITEM-SEM-BED-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packet below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane request of 2026-10-05 for ITEM-SEM-BED-1. BED-CONTENT-1 waits on it.
- Builds on:
  - BED-0 (`OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md`) §3: the bed fact on Item
    definitions and the `BedKey` head normalisation;
  - ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §2.2 (BED-CONTENT-1);
  - ITEM-SEM-USE-PACKET-1 (`OTERYN_GAME_ITEM_SEM_USE_FOOD_AND_POTION_SEMANTICS_PACKET_2026-10-04.md`)
    §1.4 and §2.1, which is the precedent: group 18, artifact v6 and resource profile V3 (#1790,
    `aa3749ae`);
  - DUR-04 and `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md` (the v6 ceilings);
  - the World Bundle format, OPEN-1: a placed map entry resolves to its Item key and Item compact
    id, and a WorldObject is reached only through its Item.
- Amends:
  - BED-0 §3: the fact's shape (§1.2);
  - the BED-0 child row and ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §2.2: BED-CONTENT-1's dependency
    and the source keys it reads (§1.5).
- Runtime, migration, protocol and production authority: NONE. ITEM-SEM-BED-1 needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

ITEM-SEM-BED-1 starts from `main`. BED-CONTENT-1 starts from `main` after ITEM-SEM-BED-1 merges.
Neither needs a migration, a capability or a wire change. BED-1 reads the facts and is not
changed by this packet.

| Shared path | Open writer | Rule |
|---|---|---|
| `apps/game-server/src/content/reference_artifact.rs`, `reference_playable.rs` | any open item-model PR | the PR that merges second takes `main` in with a merge commit, keeps both changes and reruns the resource tool |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | the shared register | ITEM-SEM-BED-1 adds the V4 rows and one V3 row (§1.4); the existing rows stay |

## 1. Rulings

### 1.1 The facts need a typed group and artifact v7

BED-0 §3 puts the bed fact on Item definitions. The runtime reads an Item only from the activated
Item artifact, and a placed bed part reaches the runtime as an Item compact id (World Bundle
OPEN-1). On `main`:

- `ReferenceItemSemantics` has 18 groups, and none holds a bed fact;
- the WorldObject catalogue (`content/world/objects/**`) carries an authoring `bed` group for 192
  records, with source item ids instead of typed targets. No typed model and no artifact reads it.

As for food and potions (ITEM-SEM-USE-PACKET-1 §1.1), a new group is a durable content-contract
change. So:

- **ITEM-SEM-BED-1** (hard, contract review) adds the group and artifact v7, with no content rows;
- **BED-CONTENT-1** (content lane) lowers the facts into it and builds the BED-0 §3 validator.

### 1.2 Group 19 `bed`

A new group joins `ReferenceItemSemantics` with id 19. It is server-only: the client allowlist does
not change, and the client sees beds through the map view, never through this fact. A Known value
has six required fields:

```text
ReferenceItemBed {
    part:              ReferenceBedPart,        // Head | Foot
    partner_direction: ReferenceBedDirection,   // North | East | South | West
    partner:           ReferenceItemTarget,     // the other half's Item, in the same state
    free:              ReferenceItemTarget,     // this half when the bed is free
    occupied_male:     ReferenceItemTarget,     // this half while a male character sleeps
    occupied_female:   ReferenceItemTarget,     // this half while a female character sleeps
}
```

Source mapping:

- `part`: `bedpart`, where `pillow` is Head and `blanket` is Foot.
- `partner_direction`: `partnerdirection`. On a Head it points to the Foot, and the reverse.
- `partner`: `bedpartof`.
- `free`, `occupied_male` and `occupied_female`: lowered from `maletransformto` and
  `femaletransformto` (§1.5).

Every part of one bed half carries the same triple, so the runtime needs no transform lookup. Sleep
transforms both halves to their `occupied_*` type, and wake transforms both back to `free` (BED-0
§4 and §6).

This amends BED-0 §3 in three ways:
- the fact is group 19;
- the part names are `Head` and `Foot`;
- it adds `partner`, which lets the validator check the type of the partner tile's item and not
  only the direction.

The `BedKey` rule of BED-0 §3 is unchanged.

**Record rules** (checked on encode and decode):

- the closed enums are refused on an unknown value;
- `free` differs from both occupied types, and `occupied_male` may equal `occupied_female`
  (a bed with one occupied look);
- the record's own Item is `free`, `occupied_male` or `occupied_female`;
- `partner` is not the record's own Item;
- every target resolves to an Item of the same artifact. Otherwise the encode and the load fail,
  as for the potion flask.

**Set rules** (checked over the whole artifact, at compile and at load). Call a record's role
Free, OccupiedMale, OccupiedFemale or OccupiedBoth by where its own Item appears in its triple.
Then:

- every Item named in the triple carries group 19 with the same `part`, the same
  `partner_direction` and the same triple;
- the `partner` carries group 19 with:
  - the other `part`;
  - the opposite `partner_direction`;
  - `partner` naming this Item;
  - the same role;
- the partner's own triple names the partner Item in that same role.

A violation fails the compile and the load. So a sleep or wake transform can never leave a bed
whose halves disagree. These rules are the artifact's guarantee. The map check (one Head and one
Foot per placed bed, on tiles of one house) stays BED-CONTENT-1's validator.

Rejected:

- **Keeping the facts on WorldObject records.** No artifact carries WorldObject records to the
  runtime, and the bundle resolves a placed entry to its Item (OPEN-1).
- **Storing the raw transform ids** (`maletransformto` and `femaletransformto` per part). The
  source graph is not uniform (§1.5), and every runtime reader would have to repeat its parse
  rule.
- **A `state` enum beside the triple.** It is derived from the record's own Item, and it would be
  a second source of truth.

### 1.3 Artifact v7

Group 19 is new grammar, and a v6 reader accepts group ids 1-18 only. ITEM-SEM-BED-1 therefore
allocates `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v7` on the v6 pattern:

- its own magic `OTRPA07\0`, the next typed body record version, and the compiler and
  canonicalization profiles `/v7`;
- the compiler writes only v7;
- v4, v5 and v6 keep explicit decoding under their own profiles;
- either reader refuses the other profile's bytes by manifest profile id, not by a parse error.

The Known encoding is:

- `FieldState`;
- a `u8` part (`HEAD` 1, `FOOT` 2);
- a `u8` direction (`NORTH` 1, `EAST` 2, `SOUTH` 3, `WEST` 4);
- four Item ordinals, in field order.

### 1.4 Resource profile V4 and the registry

The v7 grammar invalidates the v6 resource profile. ITEM-SEM-BED-1 owns:

- the resource tool, with `--profile v7` as the default and `--profile v6` re-checking v6 without
  drift;
- a new architecture profile, `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4.md` (CANDIDATE);
- its evidence;
- the `DUR04-REFERENCE-ITEM-PROFILE-V4-*` registry rows, each with max and max+1 tests, before
  the codec is released.

Recompute:

- server groups per record, 19;
- client groups per record, unchanged at 12;
- server record, body section, artifact and generation-pair bytes. The worst shape is the v6
  worst shape plus a Known bed with all four targets on the last definition ordinal: about 22
  bytes per record. The tool's figures are authoritative;
- client figures, unchanged.

**Cross-Item targets.** The V1 row `DUR04-REFERENCE-ITEM-PROFILE-V1-CROSS-ITEM-TARGETS` registers
12 target slots: ten UseTransform targets, Temporal decay and write-once. V3 inherited that row
unchanged, but v6 added a thirteenth slot, the potion `empty_flask`.

ITEM-SEM-BED-1 corrects this in the same push:
- a `DUR04-REFERENCE-ITEM-PROFILE-V3-CROSS-ITEM-TARGETS` row of 13 for v6 decoding;
- a V4 row of 17 for v7, which adds the four bed targets;
- each row with its max, max+1 and dangling-target tests;
- V4 §1 names the V3 correction.

No v6 bytes change.

### 1.5 Sources, the parse rule and BED-CONTENT-1

The source keys are `bedpart`, `bedpartof`, `partnerdirection`, `maletransformto` and
`femaletransformto` in `items.xml`. No bed record in either engine carries `transformonuse` or
`transformto`. This corrects the BED-0 child row and §2.2 of the batch, which named them.

Census (bed type entries, this packet's evidence run):

| | Crystal `ff7ede59` | Canary `04b83b51` |
|---|---:|---:|
| bed type ids | 396 | 377 |
| `pillow` / `blanket` | 208 / 188 | 199 / 178 |
| both sex transforms | 192 | 181 |
| male only / female only / none | 137 / 66 / 1 | 133 / 62 / 1 |
| without `bedpartof` | 6 | 6 |

The free type is not stored. The engines derive it while parsing, in ascending id order (Canary
`items.cpp`, OTS_HYPOTHESIS_ONLY):
- `maletransformto` and `femaletransformto` set the occupied type for that sex;
- a missing sex falls back to the other;
- each named occupied type takes the naming id as its free type, if it has none yet.

The graph is not uniform. Some partners are missing or not mutual, and some halves disagree on
direction or role. So BED-CONTENT-1 may lower only what passes §1.2.

BED-CONTENT-1 (ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §2.2) is amended as follows:

- `depends_on: [ITEM-SEM-2b, ITEM-SEM-BED-1]`, `base: main after ITEM-SEM-BED-1 merges`.
- The fact is group 19, lowered on the ITEM-SEM-USE-2 pattern:
  1. a tool lowers the pinned source (Crystal `ff7ede59` `items.xml`, the engine the item and
     WorldObject converters use, with Canary `04b83b51` as cross-check) into a pinned facts
     packet;
  2. a materializer step applies the packet to the World Project;
  3. the tools regenerate the tree.

  No content shard is edited by hand.
- The tool applies the parse rule above and lowers a bed set only when both halves in all of
  their states pass every §1.2 rule. The task record and the packet report every other bed type
  id by reason. A placed part whose Item has no group 19 is a validator finding, not a guess.
- A target Item that is not admitted stays a finding. BED-CONTENT-1 admits nothing.
- A Crystal/Canary disagreement on a lowered set is reported, and Crystal wins as the pinned
  engine.
- The BED-0 §3 validator, the 84-house exception list and the fixture house are unchanged.
- The control plane fixes the exact tool and module paths at allocation. They follow
  `lower_item_consumption_packet.py` and `item_consumption_promotion.rs`.

### 1.6 What BED-1 may assume

When both slices merge:

- a placed bed part's Item carries group 19, and every rule of §1.2 holds in the activated artifact;
- `Unknown` group 19 means "not a bed", and BED-1 returns its non-bed disposition;
- the head is found through `part` and `partner_direction`, and the partner's type is checked
  against `partner`;
- sleep and wake transform both halves within one triple.

## 2. Packet

### 2.1 ITEM-SEM-BED-1

```yaml
task_id: OTV2-20261005-item-sem-bed-1-bed-group
decision: BED-0 §3 as amended here; this packet §1.1-§1.4; DUR-04
worker: oteryn-hard-worker   # durable typed content contract and artifact encoding
review: independent contract review (Codex, final frozen head)
branch: claude/item-sem-bed-1-20261005
base: main
migration_lease: none
depends_on: [ITEM-SEM-USE-1]   # merged, aa3749ae
owned_paths:
  - apps/game-server/src/content/reference_playable.rs     # group 19 and its record and set rules
  - apps/game-server/src/content/reference_artifact.rs     # v7 profile; v4-v6 decoding kept
  - apps/game-server/src/content/project/v2.rs             # only if the authoring lowering needs it
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md  # own v7 paragraph
  - tools/reference-item-resource-profile/**               # v7 ceilings; v6 re-check
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4.md  # new
  - docs/agents/evidence/OTV2-20261005-item-sem-bed-1-v7-resource-evidence.{json,md}  # new
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # shared register: V4 rows, V3 target row
  - docs/agents/tasks/archive/OTV2-20261005-item-sem-bed-1-bed-group.md
validation:
  - python3 tools/reference-item-resource-profile/item_resource_profile.py
  - python3 tools/reference-item-resource-profile/item_resource_profile.py --profile v6
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - python3 tools/agents/validate_governance.py
  - python3 tools/repository/validate_repository_policy.py
```

Acceptance:

- The v7 profile is new, and the compiler writes only v7. Group 19 exists only in v7 server
  records. An unknown group id, part or direction fails closed, and so does group 19 in a client
  record or a v6 record (tests).
- v4, v5 and v6 artifacts still decode under their own profiles. A v6 reader refuses v7 by
  profile id, and the reverse (cross-profile tests).
- A Head/Foot pair round-trips in each of the four directions and in each role: Free,
  OccupiedMale, OccupiedFemale, and OccupiedBoth with `occupied_male == occupied_female`.
- A record is refused for each of these:
  - `free` equal to an occupied type;
  - its own Item outside its triple;
  - `partner` equal to itself;
  - a dangling target (tests).
- An artifact is refused, at compile and at load, for each set-rule violation. Each has its own
  test:
  - a triple member without group 19, or with another part, direction or triple;
  - a partner with the same part;
  - a partner with a non-opposite direction;
  - a partner that does not name this Item back;
  - a partner in another role.
- Every v7 ceiling is recomputed with the tool and registered with max and max+1 tests before
  the codec is released.
- The cross-Item target rows are V3 = 13 and V4 = 17 (§1.4). The v5 and v6 rows stay for
  decoding.
- No content rows. The content tree changes only where the v7 header re-digests it.
- Not in scope:
  - the facts and the validator (BED-CONTENT-1);
  - the bed runtime, sleep, wake and the sleeper table (BED-1);
  - bed modification kits and rotation (`rotateto`, `wrapableto`);
  - any wire change.

## 3. Rejected options

- **A content-only BED-CONTENT-1.** The typed model has no field to lower into, and the runtime
  cannot read WorldObject authoring (§1.1).
- **One hard slice for model and content.** It would exceed the batch size, and the content lane
  can do the lowering.
- **Lowering every source record.** Part of the source graph is broken (§1.5). Lowering a broken
  set would let the runtime create a bed whose halves disagree.
- **Set rules only in the content validator.** The artifact is the runtime's only carrier.
  Enforcing the rules at compile and load keeps the guarantee for any later content revision.

## 4. Decision test

- **Must decide now:** YES. BED-CONTENT-1 is allocated (session `session_01XBzCbT`) and cannot
  write its facts, and BED-1 then has nothing to read.
- **Smallest sufficient:**
  - one group of six fields;
  - one artifact profile;
  - one resource profile;
  - one registry correction.

  No runtime, no wire change, no content rows and no kits.
- **Reversible:** group 19 is additive, and a wrong value is a content revision.
- **Owner acceptance:** NOT REQUIRED. Artifact v7 and resource profile V4 register ceilings under
  the accepted DUR-04. They change no identity, protocol, authority, persistence or production
  trust. The V2 and V3 precedents (#1739, #1790) merged as CANDIDATE profiles after independent
  contract review on the frozen head. That review is required here too.
- **Superseding evidence:** official bed data. A changed set is a content revision, not a
  contract change.
