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
  - BED-0 §3 and §8: the fact's shape and the free look (§1.2);
  - the BED-0 child row and ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §2.2: BED-CONTENT-1's dependency,
    the source keys it reads and its lowering rule (§1.5).
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
has four required fields:

```text
ReferenceItemBed {
    part:              ReferenceBedPart,        // Head | Foot
    partner_direction: ReferenceBedDirection,   // North | East | South | West
    occupied_male:     ReferenceItemTarget,     // this part's look while a male character sleeps
    occupied_female:   ReferenceItemTarget,     // this part's look while a female character sleeps
}
```

**The free look is the placed Item.** BED-0 §5 keeps occupancy in the sleeper record, and BED-0
§8 renders an occupied bed as a presentation change over the map items. The map item is never
rewritten. So:

- a free bed shows the Item placed on the map;
- an occupied bed shows each part's `occupied_<variant>`;
- freeing the bed drops the presentation.

No `free` type is stored, and none is needed. This is also the engine's model: Canary and Crystal
keep the sleeper on the bed item, not in its type (`BedItem`, OTS_HYPOTHESIS_ONLY).

**Pairing is by direction.** The other half is the bed part on the tile in `partner_direction`,
with the other `part` and the opposite direction. This matches BED-0 §3 and the engines'
`getNextBedItem`. The source `bedpartof` is not read, so no `partner` field is stored. Two halves
of different styles still form a valid bed, as in the engines.

An `occupied_<variant>` equal to the record's own Item means "no change for that variant". The
engine does the same for a missing or non-bed transform target (§1.5).

This amends BED-0 in three ways:

- §3: the fact is group 19, and `free_type` is removed;
- §3: the part names are `Head` and `Foot`;
- §8: "a free bed shows `free_type`" reads "a free bed shows its placed Items".

The `BedKey` rule and the validator of BED-0 §3 are unchanged.

**Record rules** (checked on encode and decode):

- the closed enums are refused on an unknown value;
- both targets resolve to an Item of the same artifact. Otherwise the encode and the load fail, as
  for the potion flask.

**Set rule** (checked over the whole artifact, at compile and at load): every `occupied_<variant>`
target other than the record's own Item carries group 19 with the same `part` and the same
`partner_direction`. A violation fails the compile and the load. So an occupied look can never
turn a head into a foot or point the pair the wrong way.

The map check stays BED-CONTENT-1's validator: one Head and one Foot per placed bed, on tiles of
one house.

Rejected:

- **Keeping the facts on WorldObject records.** No artifact carries WorldObject records to the
  runtime, and the bundle resolves a placed entry to its Item (OPEN-1).
- **A stored `free` type, or a `wake` transition.** In the source, the free and occupied types
  name each other (§1.5), so a stored free type is a guess. And BED-0 never rewrites the map item,
  so the free look is already known: it is the placed Item.
- **A `partner` field from `bedpartof`.** The engines pair by direction. 34 Canary bed types have a
  missing or non-bed `bedpartof`, and storing it would refuse beds the engines accept.

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
- two Item ordinals, in field order.

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
  worst shape plus a Known bed with both targets on the last definition ordinal: about 14 bytes
  per record. The tool's figures are authoritative;
- client figures, unchanged.

**Cross-Item targets.** The V1 row `DUR04-REFERENCE-ITEM-PROFILE-V1-CROSS-ITEM-TARGETS` registers
12 target slots: ten UseTransform targets, Temporal decay and write-once. V3 inherited that row
unchanged, but v6 added a thirteenth slot, the potion `empty_flask`.

ITEM-SEM-BED-1 corrects this in the same push:

- a `DUR04-REFERENCE-ITEM-PROFILE-V3-CROSS-ITEM-TARGETS` row of 13 for v6 decoding;
- a V4 row of 15 for v7, which adds the two bed targets;
- each row with its max, max+1 and dangling-target tests;
- V4 §1 names the V3 correction.

No v6 bytes change.

### 1.5 Sources, the lowering rule and BED-CONTENT-1

**Source.** The pinned source is Canary `04b83b51` `items.xml` (`imports/canary/items-xml`, the
D384 pin, OTS_HYPOTHESIS_ONLY), which BED-0 and BED-CONTENT-1 already use.

**Keys.** The keys read are `bedpart`, `partnerdirection`, `maletransformto` and
`femaletransformto`. No bed record carries `transformonuse` or `transformto`. This corrects the
BED-0 child row and §2.2 of the batch, which named them.

**Lowering rule.** The rule lowers the observable engine behaviour, which has two steps
(OTS_HYPOTHESIS_ONLY):

- **Parse.** Crystal `ItemParse::parseBeds` (`imports/ots-source-evidence/item-fx-audio295`;
  Canary is the same) stores every `<sex>transformto` value in `transformToOnUse`, with the
  other-sex fallback, and sets the target's reverse `transformToFree`. It does not check the
  target's type.
- **Apply.** Canary `BedItem::updateAppearance` (`src/items/bed.cpp` at `04b83b51`, lines
  283-298; Crystal is the same) transforms a bed part only when the target's type is
  `ITEM_TYPE_BED`. For any other target it does nothing.

So a non-bed target is parsed but never shown: in Canary, item 743's female target is 727, a
lava trashholder, and a sleeping woman leaves 743 unchanged. Group 19 stores the look the engine
shows, not the parsed value. This is the engines' behaviour, not a divergence.

For each `type="bed"` Item:

- `part`: `pillow` is Head and `blanket` is Foot.
- `partner_direction`: `partnerdirection`.
- `occupied_male`: `maletransformto`, or `femaletransformto` when the male key is absent.
- `occupied_female`: `femaletransformto`, or `maletransformto` when the female key is absent.
- A target that is absent, `0` or not a `type="bed"` Item lowers to the record's own Item ("no
  change"), as `updateAppearance` does. BED-CONTENT-1 lists each such target, with the source
  id and its source type, in the facts packet, and tests that 743 lowers to "no change" for
  `occupied_female`.

The engines also derive a "free" type from whichever id names a type first. §1.2 does not use it.

**What is held.** BED-CONTENT-1 holds an Item, gives it no group 19, and reports it by reason, when:

- its `bedpart` or `partnerdirection` is missing or unknown;
- a target has another part or direction (the set rule of §1.2);
- it, or a target, has no content definition.

**Expected result.** Against the 39 holds of `OTV2-20261005-bed-facts-v1.json`:

- the 34 `bedpartof` holds (`partner_not_a_bed_item` 28, `bedpartof_missing` 6) leave the hold
  list, because `bedpartof` is not read;
- the 7 non-bed transform targets lower as "no change" and are listed;
- this packet's run lowers 370 of 377 Canary bed types and holds 7 for a part or direction change;
- all 58 bed types placed on the World Project map lower, covering all 5,679 bed tiles on house
  tiles.

The tool's figures are authoritative.

**Census** (bed type entries; Crystal shown for comparison only):

| | Canary `04b83b51` | Crystal `ff7ede59` |
|---|---:|---:|
| bed type ids | 377 | 396 |
| `pillow` / `blanket` | 199 / 178 | 208 / 188 |
| both sex transforms | 181 | 192 |
| male only / female only / none | 133 / 62 / 1 | 137 / 66 / 1 |
| without `bedpartof` | 6 | 6 |

BED-CONTENT-1 (ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §2.2) is amended as follows:

- `depends_on: [ITEM-SEM-2b, ITEM-SEM-BED-1]`, `base: main after ITEM-SEM-BED-1 merges`.
- The fact is group 19, lowered on the ITEM-SEM-USE-2 pattern:
  1. a tool lowers the pinned source with the rule above into a pinned facts packet;
  2. a materializer step applies the packet to the World Project;
  3. the tools regenerate the tree.

  No content shard is edited by hand.
- A placed part whose Item has no group 19 is a validator finding, not a guess.
- BED-CONTENT-1 admits no new Item definitions.
- The BED-0 §3 validator, the 84-house exception list and the fixture house are unchanged.
- The control plane fixes the exact tool and module paths at allocation. They follow
  `lower_item_consumption_packet.py` and `item_consumption_promotion.rs`.

### 1.6 What BED-1 may assume

When both slices merge:

- a placed bed part's Item carries group 19, and every rule of §1.2 holds in the activated artifact;
- `Unknown` group 19 means "not a bed", and BED-1 returns its non-bed disposition;
- the head is found through `part` and `partner_direction`. The other tile must hold a group-19
  Item with the other part and the opposite direction;
- an occupied bed shows each part's `occupied_<variant>`, and a free bed shows its placed Items.
  No map item is rewritten.

## 2. Packet

### 2.1 ITEM-SEM-BED-1

```yaml
task_id: OTV2-20261005-item-sem-bed-1-bed-group
decision: BED-0 §3 and §8 as amended here; this packet §1.1-§1.4; DUR-04
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
- Round-trip tests cover:
  - a Head and a Foot in each of the four directions;
  - distinct occupied looks, `occupied_male == occupied_female`, and "no change" (a target equal
    to the record's own Item).
- A record with a dangling target is refused (test).
- An artifact is refused, at compile and at load, when an occupied target has no group 19, has
  another part, or has another direction. Each case has its own test.
- Every v7 ceiling is recomputed with the tool and registered with max and max+1 tests before
  the codec is released.
- The cross-Item target rows are V3 = 13 and V4 = 15 (§1.4). The v5 and v6 rows stay for
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
- **Deriving the free type from the placed map.** The map places some occupied-look types: 2507
  and 2508 are placed 13 times each, and in Canary their sex transforms point back to 2503 and 2504.
  The free look is the placed Item anyway (§1.2), so a derived free type adds nothing.
- **Omitting the occupied types.** BED-0 §8 needs the occupied look. Leaving it out would push a
  guess into BED-1.
- **Set rules only in the content validator.** The artifact is the runtime's only carrier.
  Enforcing the rule at compile and load keeps the guarantee for any later content revision.

## 4. Decision test

- **Must decide now:** YES. BED-CONTENT-1 is allocated (session `session_01XBzCbT`) and cannot
  write its facts, and BED-1 then has nothing to read.
- **Smallest sufficient:**
  - one group of four fields;
  - one artifact profile;
  - one resource profile;
  - one registry correction.

  No runtime, no wire change, no content rows and no kits.
- **Reversible:** group 19 is additive, and a wrong value is a content revision.
- **Owner acceptance:** NOT REQUIRED. Artifact v7 and resource profile V4 register ceilings under
  the accepted DUR-04. They change no identity, protocol, authority, persistence or production
  trust. The V2 and V3 precedents (#1739, #1790) merged as CANDIDATE profiles after independent
  contract review on the frozen head. That review is required here too.
- **Superseding evidence:** official bed data. A changed look is a content revision, not a
  contract change.
