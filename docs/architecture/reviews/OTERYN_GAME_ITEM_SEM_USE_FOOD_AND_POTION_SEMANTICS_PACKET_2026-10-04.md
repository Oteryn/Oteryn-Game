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
  - GAME-ABILITY-01 (the Ability → Effect → Formula chain, effect families `Damage` and `Heal`);
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
DEATH-2b and BANK-1 do not own the content model or the item and ability content trees. Three
open PRs share files:

| Shared path | Open writer | Rule |
|---|---|---|
| `content/items/**` (the items-stats cascade) | PROF-SNOWBALL-REKEY-1 (#1753) | ITEM-SEM-USE-2 starts after #1753 merges or closes; it never opens on top of a stale content tree |
| `content/abilities/**` | the spell import PRs (#1534, #1755) | ITEM-SEM-USE-2 adds only new `oteryn:ability.item.*`, effect and formula records in new shards; it does not regenerate existing shards. If a spell import merges first, ITEM-SEM-USE-2 rebases its new shards onto `main` by merge, not by regeneration |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | the shared register | ITEM-SEM-USE-1 adds the v6 rows; v5 rows stay |

## 1. Rulings

### 1.1 The model lacks food and potion semantics, so ITEM-SEM-USE is two slices

`ReferenceItemSemantics` has 17 groups. None holds a food's regeneration time, a potion's effect
or its empty flask. The authoring schema has `consumable` (`edible`, `regeneration_seconds`), but
no typed group lowers it. No ability content models a potion.

The ITEM-USE-0 child row reads as a content task, but the content has nowhere to go. As in
2b-3 (§1.12 there), a new group is a durable content-contract change, so:

- **ITEM-SEM-USE-1** (hard, contract review) adds the group, the effect family and artifact v6,
  with no content rows;
- **ITEM-SEM-USE-2** (content lane) lowers the rows.

### 1.2 Group 18 `consumption`

A new group joins `ReferenceItemSemantics`, id 18, server-only (the client allowlist is unchanged).
Its known value is a closed enum:

- `Food { regeneration_seconds }`, 1-1,200 inclusive. ITEM-USE-0 §6.1 adds this to the remaining
  `FoodRegeneration` time and refuses at 1,200 s, so a larger value could never be eaten. A source
  row above 1,200 is reported and not lowered.
- `Potion { ability, empty_flask }`:
  - `ability` is a `TypedDefinitionRef` of family Ability, resolved at load like other
    references. An unresolved reference fails the load closed.
  - `empty_flask` is `Known(target)` (an Item definition) or `KnownNone` (no flask, as attribute
    potions). ITEM-USE-0 §4.1 reads it: a flask is a TRANSFORM, no flask is a BURN.

The flask is not the `Use` entry of `use_transform`. That entry transforms the used item itself.
A potion leaves one flask unit from a stack of up to 100, and ITEM-USE-0 §4.1 plans that unit.

Requirements stay in group 17 (`use_requirements`, `on_use`). ITEM-USE-0 §6.2 reads them;
ITEM-SEM-USE-2 lowers the potion rows into it.

### 1.3 Effect family `ManaRestore`

GAME-ABILITY-01 content has two effect families, `Damage` and `Heal`. All 670 `Heal` effects
restore health. A mana potion needs a mana restore. Canary uses a separate call for it
(`doTargetCombatMana`, `potions.lua`, OTS_HYPOTHESIS_ONLY).

A third family, `ManaRestore`, joins `ReferenceEffectFamily`, `EffectFamilyDocument` and the model
`EffectFamily`. Its formula is a `Range`. It is closed, like the other two. The runtime meaning (a
mana gain clamped to max mana, never a damage event) is ITEM-USE-1's (ITEM-USE-0 §6.2). A spirit
potion is one ability with two effects, `Heal` then `ManaRestore`.

Rejected: a `Heal` with a resource field. It changes the meaning of every existing `Heal` record.

### 1.4 Artifact v6

Group 18 and the third family are new grammar. A v5 reader accepts group ids 1-17 only.
ITEM-SEM-USE-1 therefore allocates `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v6`, with its compiler and
canonicalization profiles:

- the compiler writes only v6;
- v5 keeps explicit decoding under its own profile;
- a v5 reader refuses v6 by profile id, not by a parse error.

The v6 grammar invalidates the v5 resource profile. ITEM-SEM-USE-1 owns:

- the resource-profile tool;
- a new architecture profile, `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md`;
- its evidence;
- the registry rows.

It recomputes and registers every v6 ceiling with max and max+1 tests before the codec is
released (D448): server groups 18, client groups unchanged at 12, and record and artifact bytes.

If the effect family does not reach the artifact, the profile tool records that and the family adds
no ceiling.

### 1.5 Sources and scope of ITEM-SEM-USE-2

Sources come in this order (ITEM-USE-0 child row):

1. **TibiaWiki** (the `Infobox_Item` census already used by the item lane):
   - `regenseconds` for food;
   - the potion pages for heal and mana ranges, `levelrequired` and `vocrequired`.
2. **Canary `04b83b51`**, as fallback only:
   - `foods.lua`: food value × 12 s;
   - `potions.lua`: ranges, flask ids and requirements.

Each lowered row records its source. A Canary-only value is `OTS_HYPOTHESIS_ONLY` in the evidence.

Rows:

- **Food:** every edible item with a TibiaWiki `regenseconds` (Canary as fallback) and a
  materializable definition. The record lists the count lowered and the count reported (no
  source, above 1,200 s, or not materializable).
- **Potions:** the eleven healing and mana potions:
  - health, strong health, great health, ultimate health and supreme health;
  - mana, strong mana, great mana and ultimate mana;
  - great spirit and ultimate spirit.

  Each potion gets:
  - one `oteryn:ability.item.<slug>` Ability with `Heal` and/or `ManaRestore` effects and
    `Range` formulas;
  - its group 18 `Potion` value;
  - its group 17 requirements (level, vocations, `on_use`).
- **Flasks:** the empty flask definitions that the potions name (`empty potion flask` i283 and
  its strong and great variants). Each potion's `empty_flask` names one of them.

Attribute potions (berserk, mastermind, bullseye and similar) are out of scope. They apply
conditions that have no definition yet.

### 1.6 What ITEM-USE-1 may assume

When both slices merge:

- `consumption` is known for the rows of §1.5;
- `Unknown` means "not usable as food or potion", and ITEM-USE-1 returns its non-usable
  disposition;
- a potion's ability resolves;
- its effects are `Heal` and `ManaRestore` only.

ITEM-USE-1 does not read `consumable` from authoring.

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
  - apps/game-server/src/content/reference_playable.rs     # group 18, ReferenceEffectFamily::ManaRestore
  - apps/game-server/src/content/reference_artifact.rs     # v6 profile; v5 decoding kept
  - apps/game-server/src/content/project/v2.rs             # authoring → typed lowering of group 18
  - apps/game-server/src/content/project.rs                # EffectFamilyDocument::ManaRestore
  - apps/game-server/src/content/model.rs                  # EffectFamily::ManaRestore
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
  group id, an unknown `consumption` variant or a fourth effect family fails closed (tests).
- A v5 artifact still decodes under its own profile. A v5 reader refuses v6 by profile id
  (cross-profile tests).
- `Food` round-trips at 1 and 1,200 and rejects 0 and 1,201.
- `Potion` round-trips with `Known` and `KnownNone` flasks. A potion whose ability does not
  resolve, or resolves to a non-Ability family, fails the load (test).
- `ManaRestore` lowers from authoring and round-trips. Every existing `Damage` and `Heal` record
  is byte-identical in the regenerated artifact (test or digest evidence).
- Every v6 ceiling is recomputed with the tool and registered with max and max+1 tests before
  the codec is released. The v5 rows stay for v5 decoding.
- No content rows; the content tree changes only where the v6 header re-digests it.
- Not in scope:
  - the `ManaRestore` runtime and the `FoodRegeneration` condition (ITEM-USE-1, FOOD-REGEN-1);
  - any wire change.

### 2.2 ITEM-SEM-USE-2

```yaml
task_id: OTV2-20261004-item-sem-use-2-food-and-potion-content
decision: ITEM-USE-0 child row; this packet §1.5, §1.6
worker: oteryn-impl-worker   # content lowering under an accepted contract
review: content review (Codex, final frozen head)
branch: claude/item-sem-use-2-20261004
base: main after ITEM-SEM-USE-1 merges, and after #1753 merges or closes
migration_lease: none
depends_on: [ITEM-SEM-USE-1]
owned_paths:
  - tools/content-schema/item-authoring/lower_item_consumption_packet.py       # new
  - tools/content-schema/item-authoring/test_lower_item_consumption_packet.py  # new
  - tools/content-schema/item-authoring/README.md                               # own section
  - content/items/definitions/**        # group 17 and 18 values of the §1.5 rows only
  - content/abilities/definitions/abilities-item-*.json   # new shards only
  - content/abilities/effects/effects-item-*.json         # new shards only
  - content/abilities/formulas/formulas-item-*.json       # new shards only
  - content/abilities/{definitions,effects,formulas}/index.json  # record counts
  - docs/agents/evidence/OTV2-20261004-item-sem-use-2-consumption-sources.{json,md}  # new
  - docs/agents/tasks/archive/OTV2-20261004-item-sem-use-2-food-and-potion-content.md
validation:
  - python3 -m unittest tools/content-schema/item-authoring/test_lower_item_consumption_packet.py
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet
```

Acceptance:

- The lowering is a reproducible tool run from the recorded sources, and running it twice gives
  the same tree (test).
- Every lowered row names its source (TibiaWiki revision, or Canary `04b83b51` file and line).
- **Food:** the record lists the lowered and reported counts by reason. Ham (i3582) is lowered,
  with its value checked against TibiaWiki.
- **Potions:** the eleven potions of §1.5 each have:
  - group 18 `Potion`, with a resolving ability;
  - the ranges as `Range` formulas;
  - their flask;
  - their group 17 requirements.

  The health potion (i266) names the empty potion flask (i283). Both spirit potions carry two
  effects in the order `Heal`, `ManaRestore`.
- Existing shards of `content/abilities` are byte-identical, apart from `index.json` counts.
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
- **Potion effects as `Heal` with a resource field.** It changes the meaning of 670 existing
  records (§1.3).
- **Food value in Canary units (× 12 at runtime).** TibiaWiki states seconds. Storing seconds keeps
  one unit from source to `FoodRegeneration`.
- **One hard slice for model and content.** That would be over the batch size, and the content
  lane can do the content part.

## 4. Decision test

- **Must decide now:** YES. ITEM-USE-1 and FOOD-REGEN-1 have no content to read, so a character
  cannot eat or drink.
- **Smallest sufficient:** one group, one effect family, one artifact profile, eleven potions and
  the edible food set. No attribute potions.
- **Reversible:** group 18 and `ManaRestore` are additive. A wrong value is a content revision.
- **Superseding evidence:** official food or potion values. A changed row is a content revision,
  not a contract change.
