# ITEM-SEM-USE packet: food and potion semantics

- Packet: `ITEM-SEM-USE-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the two packets below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane order of 2026-10-04, item 1 (the ITEM-SEM-USE content packet).
  ITEM-SEM-USE blocks ITEM-USE-1, FOOD-REGEN-1, RUNE-CONTENT-1, IMBUE-CONTENT-1 and
  FORGE-CONTENT-1.
- Builds on:
  - ITEM-USE-0 (`OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md`, accepted by
    ACCEPT-ITEMUSE-BANK-0, #1750): the ITEM-SEM-USE child row, §2, §4.1, §6.1 and §6.2;
  - ARCH-BATCH-ITEM-EQUIP-PACKETS §1.12 and §2.2a (ITEM-SEM-2b-3, #1710): the hard content-contract
    pattern, artifact v5 and the use-requirements group 17;
  - DUR-04 and `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2.md` (the v5 ceilings);
  - GAME-ABILITY-01 (the ability pipeline that ITEM-USE-0 §6.2 runs a potion through);
  - STARTER-CONTENT-1 and TIMED-CONTENT-1 (the pinned admission and facts packets applied by the
    materializer);
  - the TibiaWiki field census (`regenseconds` → `consumable.regeneration_seconds`).
- Amends: nothing. ITEM-USE-0 names one child, ITEM-SEM-USE; this packet splits it into a contract
  slice and a content slice (§1.1). ITEM-USE-1 depends on both.
- Runtime, migration, protocol and production authority: NONE. Each packet needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

ITEM-SEM-USE-1 goes first. ITEM-SEM-USE-2 starts from `main` after it merges. Neither needs a
migration, a capability or a wire change.

They do not overlap the in-flight core-loop packets: ATTACK-1b, CREATURE-*, CHAT-*, MAP-*,
DEATH-2b and BANK-1 do not own the content model or the item content tree. These open PRs share
files:

| Shared path | Open writer | Rule |
|---|---|---|
| `content/items/**` (the items-stats cascade) | PROF-SNOWBALL-REKEY-1 (#1753) | ITEM-SEM-USE-2 starts after #1753 merges or closes; it never opens on top of a stale content tree |
| `apps/game-server/src/content/item_admission.rs`, the materializer example, `content/world/**` | D3-7 (#1770), and every PR that regenerates the World Project | the PR that merges second takes `main` in with a merge commit and reruns the tools (§2.2) |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | the shared register | ITEM-SEM-USE-1 adds the v6 rows; v5 rows stay |

## 1. Rulings

### 1.1 The model lacks food and potion semantics, so ITEM-SEM-USE is two slices

`ReferenceItemSemantics` has 17 groups. None holds a food's regeneration time, a potion's effect
or its empty flask. The authoring schema has `consumable` (`edible`, `regeneration_seconds`), but
no typed group lowers it. No ability content models a potion.

The ITEM-USE-0 child row reads as a content task, but the content has nowhere to go. As in
2b-3 (§1.12 there), a new group is a durable content-contract change, so:

- **ITEM-SEM-USE-1** (hard, contract review) adds the group and artifact v6, with no content rows;
- **ITEM-SEM-USE-2** (content lane) admits the missing rows and lowers the values.

### 1.2 Group 18 `consumption`

A new group joins `ReferenceItemSemantics`, id 18, server-only (the client allowlist is unchanged).
Its known value is a closed enum:

- `Food { regeneration_seconds }`, 1-1,199 inclusive. ITEM-USE-0 §6.1 adds this to the remaining
  `FoodRegeneration` time and refuses when the total would reach 1,200 s, so a value of 1,200 or
  more could never be eaten (#1767 P2 4177976626). A source row of 1,200 s or more is reported and
  not lowered.
- `Potion { restores, empty_flask }`:
  - `restores` lists one or two entries `{ resource, min, max }`. `resource` is the closed enum
    `Health | Mana`, with at most one entry per resource, in the order `Health`, `Mana`. The bounds
    are `1 <= min <= max <= 10,000`. A spirit potion has both entries.
  - `empty_flask` is the existing `ReferenceItemField<ReferenceItemTarget>` (no new enum).
    `Known(target)` names an Item definition. `NotApplicable` means no flask, as for attribute
    potions. `Unknown` and `Conflict` are refused for a `Potion`: such a row is reported and not
    lowered. ITEM-USE-0 §4.1 reads it: a flask is a TRANSFORM, no flask is a BURN (#1767 P1
    4178009989).

The flask is not the `Use` entry of `use_transform`. That entry transforms the used item itself.
A potion leaves one flask unit from a stack of up to 100, and ITEM-USE-0 §4.1 plans that unit.

Requirements stay in group 17 (`use_requirements`, `on_use`). ITEM-USE-0 §6.2 reads them;
ITEM-SEM-USE-2 lowers the potion rows into it.

### 1.3 The potion magnitudes are carried by the Item artifact (#1767 P1 4177976617)

The activated Item artifact is the only generation-fenced carrier the runtime reads for an Item.
It holds no Ability, Effect or Formula records: `compile_with_profile` refuses every non-Item
definition, and `ReferenceFormulaDefinition` keeps no authored range. A potion that names an
Ability record could not be resolved from the artifact.

So the ranges live in group 18 itself, and the v6 Item artifact carries and bounds them. No
Ability, Effect or Formula content is written for potions, and no effect family is added to the
content model.

ITEM-USE-0 §6.2 still holds. ITEM-USE-1 builds the GAME-ABILITY-01 invocation, with origin
`ItemUse` and keyed by the CommandRef, from the group 18 entries: a `Health` entry is a health
restore and a `Mana` entry is a mana restore, each drawn from `[min, max]`. The runtime effect
representation of those two restores, including a mana restore clamped to max mana and never a
damage event, is ITEM-USE-1's (#1767 P2 4177976623: the runtime `EffectFamily` has only
`Damage`, so ITEM-USE-1 owns both restores).

Rejected:
- **Ability content records for potions.** The artifact cannot carry them. A second activated
  carrier for abilities is a larger contract with no other current consumer.
- **A `Heal` with a resource field.** It changes the meaning of every existing `Heal` record.

### 1.4 Artifact v6

Group 18 is new grammar. A v5 reader accepts group ids 1-17 only. ITEM-SEM-USE-1 therefore
allocates `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v6`, with its compiler and canonicalization profiles:

- the compiler writes only v6;
- v5 keeps explicit decoding under its own profile;
- a v5 reader refuses v6 by profile id, not by a parse error.

The v6 grammar invalidates the v5 resource profile. ITEM-SEM-USE-1 owns:

- the resource-profile tool;
- a new architecture profile, `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md`;
- its evidence;
- the registry rows.

It recomputes and registers every v6 ceiling with max and max+1 tests before the codec is
released (D448): server groups 18, client groups unchanged at 12, two `restores` entries, and
record and artifact bytes.

### 1.5 Sources and scope of ITEM-SEM-USE-2

Sources come in this order (ITEM-USE-0 child row):

1. **TibiaWiki** (the `Infobox_Item` census already used by the item lane):
   - `regenseconds` for food;
   - the potion pages for heal and mana ranges, `levelrequired` and `vocrequired`.
2. **Canary `04b83b51`**, as fallback only:
   - `foods.lua`: food value × 12 s;
   - `potions.lua`: ranges, flask ids and requirements.

Each lowered row records its source. A Canary-only value is `OTS_HYPOTHESIS_ONLY` in the evidence.

The lowering follows the TIMED-CONTENT-1 precedent. A tool lowers the sources into a pinned facts
packet, and a materializer step applies it to the World Project, which is then regenerated by the
tools. No content shard is edited by hand.

Rows:

- **Food:** every edible item with a TibiaWiki `regenseconds` (Canary as fallback) that is
  materializable after the admission below. The record lists the count lowered and the count
  reported (no source, 1,200 s or more, or not materializable).
- **Potions:** the eleven healing and mana potions:
  - health i266, strong health i236, great health i239, ultimate health i7643, supreme health
    i23375;
  - mana i268, strong mana i237, great mana i238, ultimate mana i23373;
  - great spirit i7642, ultimate spirit i23374.

  Each potion gets its group 18 `Potion` value (ranges and flask) and its group 17 requirements
  (level, vocations, `on_use`).
- **Flasks:** the empty potion flask i283, the empty strong potion flask i284 and the empty great
  potion flask i285. Each potion's `empty_flask` names one of them.

**Admission (#1767 P1 4177976619).** On `main`, ten of these rows are identity-only
(`materializable: false`, stack class `Unknown`, `stack` `Unknown`): the six potions i236, i7643,
i23375, i23373, i7642 and i23374, ham i3582, and the three flasks i283, i284 and i285. A potion or
flask that cannot be materialized cannot be used or produced. ITEM-SEM-USE-2 admits exactly these
ten rows with a pinned admission packet, schema `OTERYN_ITEM_ADMISSION/v3`, on the
STARTER-CONTENT-1 pattern:

- the shape is `materializable: true`, stack class `StackCapable`, `stack` `Known { stackable:
  true, stack_max: 100 }`, matching the five potions already admitted on `main`;
- the admission is strict: it fails closed unless the row is still identity-only with an
  `Unknown` stack;
- every row records TibiaWiki and Canary stackability as evidence. A source that says
  non-stackable stops the slice with a QUESTION to the control plane;
- no other Item is admitted. Other edible items that are not materializable are reported, not
  admitted.

Attribute potions (berserk, mastermind, bullseye and similar) are out of scope. They apply
conditions that have no definition yet.

### 1.6 What ITEM-USE-1 may assume

When both slices merge:

- the potions, flasks and ham of §1.5 are materializable and stack-capable;
- `consumption` is known for the rows of §1.5;
- `Unknown` means "not usable as food or potion", and ITEM-USE-1 returns its non-usable
  disposition;
- a potion's magnitudes are its group 18 `restores` entries, read from the activated Item
  artifact (§1.3).

ITEM-USE-1 does not read `consumable` from authoring, and it reads no Ability content for potions.

## 2. Packets

### 2.1 ITEM-SEM-USE-1

```yaml
task_id: OTV2-20261004-item-sem-use-1-consumption-group
decision: ITEM-USE-0 child row, §4.1, §6.1, §6.2; this packet §1.1-§1.4; DUR-04
worker: oteryn-hard-worker   # durable typed content contract and artifact encoding
review: independent contract review (Codex, final frozen head)
branch: claude/item-sem-use-1-20261004
base: main
migration_lease: none
depends_on: [ITEM-SEM-2b-3]
owned_paths:
  - apps/game-server/src/content/reference_playable.rs     # group 18
  - apps/game-server/src/content/reference_artifact.rs     # v6 profile; v5 decoding kept
  - apps/game-server/src/content/project/v2.rs             # authoring → typed lowering of group 18
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md  # own paragraph
  - tools/reference-item-resource-profile/**               # v6 ceilings
  - docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md  # new
  - docs/agents/evidence/OTV2-20261004-item-sem-use-1-v6-resource-evidence.{json,md}  # new
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # shared register: the v6 rows
  - docs/agents/tasks/archive/OTV2-20261004-item-sem-use-1-consumption-group.md
validation:
  - python3 tools/reference-item-resource-profile/item_resource_profile.py
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

Acceptance:

- The v6 profile is new, and the compiler writes only v6. Group 18 exists only in v6. An unknown
  group id, an unknown `consumption` variant or an unknown `resource` fails closed (tests).
- A v5 artifact still decodes under its own profile. A v5 reader refuses v6 by profile id
  (cross-profile tests).
- `Food` round-trips at 1 and 1,199 and rejects 0 and 1,200.
- `Potion` round-trips with one and two `restores` entries and with a `Known` and a
  `NotApplicable` flask. It rejects an `Unknown` or `Conflict` flask, zero or three entries, two entries for one resource, `Mana` before `Health`,
  `min` 0, `min > max` and `max` 10,001. A `Known` flask that does not resolve to an Item fails
  the load (tests).
- Every v6 ceiling is recomputed with the tool and registered with max and max+1 tests before
  the codec is released. The v5 rows stay for v5 decoding.
- No content rows; the content tree changes only where the v6 header re-digests it.
- Not in scope:
  - the restore runtime and the `FoodRegeneration` condition (ITEM-USE-1, FOOD-REGEN-1);
  - effect families in the content model or the runtime (§1.3);
  - any wire change.

### 2.2 ITEM-SEM-USE-2

```yaml
task_id: OTV2-20261004-item-sem-use-2-food-and-potion-content
decision: ITEM-USE-0 child row; this packet §1.5, §1.6
worker: oteryn-impl-worker   # content lowering and admission under an accepted contract
review: content review (Codex, final frozen head)
branch: claude/item-sem-use-2-20261004
base: main after ITEM-SEM-USE-1 merges, and after #1753 merges or closes
migration_lease: none
depends_on: [ITEM-SEM-USE-1]
owned_paths:
  - tools/content-schema/item-authoring/lower_item_consumption_packet.py       # new
  - tools/content-schema/item-authoring/test_lower_item_consumption_packet.py  # new
  - tools/content-schema/item-authoring/README.md                               # own section
  - docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json           # new, pinned
  - docs/agents/evidence/OTV2-20261004-item-sem-use-2-consumption-sources.md    # new
  - docs/agents/evidence/OTV2-20261004-consumable-item-admission.json           # new, pinned
  - apps/game-server/src/content/item_consumption_promotion.rs                  # new
  - apps/game-server/src/content/item_admission.rs        # the v3 shape; v1 unchanged
  - apps/game-server/src/content/mod.rs                   # one module line
  - apps/game-server/examples/materialize_content_world_project_v2.rs  # call lines and count prints
  - apps/game-server/tests/content_world_project_repository.rs         # admission and consumption assertions
  - content/items/** and content/world/**   # regenerated by the materializer and world_project_v2_to_tree.py only
  - docs/agents/tasks/archive/OTV2-20261004-item-sem-use-2-food-and-potion-content.md
validation:
  - python3 -m unittest tools/content-schema/item-authoring/test_lower_item_consumption_packet.py
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet
  - cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet
  - the content-tree regeneration check (the repository CI step for content/world)
```

Shared files: D3-7 (#1770) also adds an admission shape (`v2`) to `item_admission.rs` and a call
line to the materializer. The PR that merges second takes `main` in with a merge commit, keeps both
shapes and reruns the tools. Generated files are never merged by hand.

Acceptance:

- The lowering is a reproducible tool run from the recorded sources, and running it twice gives
  the same packet (test).
- Every lowered row names its source (TibiaWiki revision, or Canary `04b83b51` file and line).
- **Admission:** exactly the ten rows of §1.5 are admitted with the v3 shape; a second run fails
  closed; the v1 backpack result is unchanged (tests).
- **Food:** the record lists the lowered and reported counts by reason. Ham (i3582) is lowered,
  with its value checked against TibiaWiki.
- **Potions:** the eleven potions of §1.5 each have group 18 `Potion` with their ranges and
  flask, and their group 17 requirements. The health potion (i266) names the empty potion flask
  (i283). Both spirit potions carry two entries, `Health` then `Mana`.
- `content/abilities/**` does not change.
- No item outside the §1.5 rows changes (diff test against `main`).
- Not in scope:
  - attribute potions;
  - runes (RUNE-CONTENT-1);
  - imbuement and forge items;
  - any runtime.

## 3. Rejected options

- **A content-only ITEM-SEM-USE.** The typed model has no field to lower into. Writing values into
  authoring only would leave ITEM-USE-1 reading an untyped schema.
- **The flask as `use_transform` `Use`.** That entry is a whole-item self-transform. It cannot
  express one flask unit from a stack (§1.2).
- **Potion abilities as content records.** The Item artifact cannot carry or resolve them (§1.3).
- **Hand-written shard edits for the ten unadmitted rows.** Generated content is written only by
  the tools; the admission packet keeps the facts pinned (§1.5).
- **Food value in Canary units (× 12 at runtime).** TibiaWiki states seconds. Storing seconds keeps
  one unit from source to `FoodRegeneration`.
- **One hard slice for model and content.** That would be over the batch size, and the content
  lane can do the content part.

## 4. Decision test

- **Must decide now:** YES. ITEM-USE-1 and FOOD-REGEN-1 have no content to read, so a character
  cannot eat or drink.
- **Smallest sufficient:** one group, one artifact profile, one admission of ten rows, eleven
  potions and the edible food set. No effect family and no attribute potions.
- **Reversible:** group 18 is additive. A wrong value is a content revision.
- **Superseding evidence:** official food or potion values. A changed row is a content revision,
  not a contract change.
