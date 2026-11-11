# RewardClaim authoring

Builds the `RewardClaim` family in `content/interactions/reward_claims/` from the chest pilot
(`tools/content-schema/quest-authoring/samples/chests/`). It also keeps the family registered in
`content/project.json`, `content/manifest.json` and `content/content.lock.json`.

This is CHEST-CONTENT part 2b (#162). It is tree-first by owner decision (5912009064), like
CHARM-1 and PROF-CONTENT-1. The server does not read `content/` yet: MAP-BUNDLE-1 compiles the
tree (ADR-0021), and the Rust `RewardClaim` kind (#1334) receives the claims from it.

| File | Purpose |
|---|---|
| `reward_claim_authoring.py` | `content` writes the family and its registration; `content --check` verifies them byte for byte. Validation runs on every build. |
| `test_reward_claim_authoring.py` | No-network tests. It covers scope, readiness, source checks, every validation rule, and the rules on the committed content. `tools/content-migration/test_world_project_v2_to_tree.py` runs it, so the Content Tree Migration workflow does too. |

## Scope and rules

The original native profile holds **plain `once` claims only**. That means a per-character `once` claim whose
placements reward `items` only, with no achievement. Cooldown claims, containers, keys, written
texts, random choices and achievements are later children.

- **Reward on the placement.** Each claim lists the chests that share it, and each chest has
  its own reward (architect ruling 5905746509).
- **No PlacementKey.** A placement binds to its source instead: the project-frame position and
  the legacy unique ids of both servers. MAP-BUNDLE-1 resolves that binding to the compiled
  `placement_key` and fails compilation when it cannot (5909237761).
- **Chest appearance.** It is source evidence only, kept as `appearance_tibia_id`. Some chest
  appearances are not Items.
- **Reward keys.** Each reward is one A12 Item key (`oteryn:item.tibia.i<id>`) that resolves in
  `content/items`, with a positive count.
- **Readiness.** A claim is `ready` when every reward Item is materializable, has a known stack
  class and the count fits it. Otherwise it is `waiting_item_semantics`: the MINT fails closed
  on it (D82) until ITEM-SEM covers the Item.
- **Source checks.** A count that contradicts a known stack class is listed in the index under
  `source_checks`. StackCapable counts must fit a proven `semantics.stack.value.stack_max`,
  or D82's maximum of 100 when no smaller maximum is known. Unsupported maxima never become ready.
  Duplicate legacy unique ids are server-scoped and recorded with every claim/position;
  validation rejects missing or stale collision diagnostics. Position bindings remain authoritative.

## Poison Daggers target reward

Poison Daggers Quest uses the target's two daggers and thirty poison arrows.
The [Wiki quest](https://tibia.fandom.com/wiki/Poison_Daggers_Quest) and
[spoiler](https://tibia.fandom.com/wiki/Poison_Daggers_Quest/Spoiler), captured
2026-11-06, agree with both pinned donors' comments, while their numeric recipe
contains only one dagger. `poison_daggers.py` corrects only UID6068 at the exact
source position and recipe. It keeps the original donor in `source_variant`
and emits two quantity1 children through the existing container reward owner.
A changed source recipe, position or container fails closed.

## D277 charged source rewards

Architect ruling [5933264015](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5933264015)
closes the architecture for charged rewards: one ItemInstance, quantity1, charges
from the Item definition. The raw reward argument never creates an Item charges fact.
`source_charge_evidence.json` binds the historical two-argument chest helper calls
by source claim, position, Item and raw argument, with pinned source and ruling evidence.
Only an exact binding with known NonStackable semantics and positive
`charges.count` is normalized to physical quantity1. Source/default charge
disagreement remains an independent diagnostic. The original pilot stays unchanged;
`source_checks` preserves the raw argument and the derived canonical quantity.

A default-matching reward stays held as `NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED`:
current native MINT request/after-state have no charge initialization or persistence.
This is an implementation gap under covered architecture. Source/default disagreement
stays `SOURCE_CHARGE_MISMATCH` (CONFLICT), notably Banshee3081 raw1 versus definition5.
Unproved source arguments and unknown definition charges remain UNKNOWN without
normalization. Stackable rune charge definitions retain quantity as piece count.
Build and committed validation recompute quantity and exact source diagnostics;
forged normalization, missing diagnostics and false ready status are rejected.
An explicit source list must contain the original reward binding; an empty, removed
or moved binding is an error. Optional `None` is fixture consistency mode only.

The finite questpc4 admission covers Time Ring UID6051 (50 charges), Troll Cave
UID6039 (150), Wedding Ring UID6043 (200), and the Garlic Necklace in Triangle
Tower UID6160. Each reward is one physical Item. The existing native Item profile
must agree with the canonical physical/custody facts, exact identity and binding, default
charges and portable admission. The existing timed Item owner interprets absent
instance state as those full definition values. Partial charges, altered source
positions, incompatible native profiles and other claims retain their holds.
This qualification changes neither the source-wide Item catalogue nor the
original source dispositions. The three leaf claims become ready; Triangle
Tower gains a complete native bag recipe, preserving its two small sapphires
and dwarven ring. No quest mission or access gate is implied by this admission.

Capture the leaf source evidence and reproduce the dispatch fixtures from the
exact Crystal revision using:

```sh
python3 tools/content-schema/reward-claim-authoring/capture_container_native_source.py --crystal <source-root> --questpc4 --check
python3 tools/content-schema/quest-authoring/questpc4/build_reward_cases.py --crystal <source-root> --check
```

The fixture generator checks source UIDs, positions, appearances and every
reward against the authored claims. Its 28 quest cohorts are exercised by the
filtered dispatch library tests `source_quest_rewards`, including stale-fence
refusal, duplicate replay, new-command refusal, reconstruction of the owner,
exact custody and single-transaction audit rows. Shared choices (Troll Cave,
Pits of Inferno and Thieves Guild) approach every alternate source box with real
STEP commands and refuse it after the first claim, including refusal replay.
The Annihilator's four choices also assert the exact source achievement grant
through duplicate, alternate-choice and reconstructed-owner replay.
Numbered key cases assert the exact source key number and mint transaction;
authored book cases assert the persisted source text, initial revision and no
player writer. Quest-linked chests use the existing catalogue/session and prove
one consumed obligation and one quest receipt across reconstructed-owner replay.
Explorer UID6078 explicitly retains canonical Item12510 under PR2770 Q5=A,
with the pinned donor Item3208 difference preserved in its fixture citation.
The fixtures qualify reward paths; whole quests with NPC missions or access
puzzles need additional tests.

Desert Dungeon UID6158 uses the same finite admission for its Protection Amulet's
full 250 source charges. The complete green bag retains its Ring of Healing,
Ankh and Magic Light Wand. All six required native Item profiles already existed;
this admission changes none of them and preserves the historical SOURCE hold.

```sh
cd tools/content-schema/reward-claim-authoring
python reward_claim_authoring.py content          # rebuild after the pilot or content/items changes
python reward_claim_authoring.py content --check  # CI
python test_reward_claim_authoring.py
```

The RewardClaim Authoring workflow runs both commands on tool, chest-source, Item,
interaction and shared-registration changes, including stacked pull requests.

**A change to `content/items`,** for example ITEM-SEM adding stack facts:
- It never breaks this family. Only a claim marked `ready` that its Items no longer support is
  an error.
- A stale `waiting_item_semantics` is safe, because the MINT itself fails closed.
- Rebuild with `content` to promote newly ready claims.

The separate source variant packet preserves all105 non-plain claims and their
UID/position witnesses, exact Item references, source arguments and typed grants.
D277 ruling5933264015 marks key/text/random/cooldown formats COVERED. Container
contents wait for a content child; Achievement requests carry exact references
while runtime ownership remains deferred. Native lowering is waiting implementation.
Medusa text carrier and actual charge/default discrepancies remain CONFLICT.

```sh
python tools/content-schema/reward-claim-authoring/reward_claim_variant_migration.py --check
python tools/content-schema/reward-claim-authoring/test_reward_claim_variant_migration.py
```

## Canonical variant DATA under D277

`reward_claim_variant_authoring.py` extends the same family with strict
`authored_variant_v1` definition/shard profiles. All105 existing source variants
retain actual repeat/key/text/random/container payloads, canonical Item references,
typed Achievement requests, position/UID bindings and full source dispositions.
The null Medusa carrier stays null with a source hold. Source/default charge
conflicts remain explicit; physical quantities use only proved definition facts.

These definitions resolve Quest references but **never admit native execution**:
`native_admission` is `WAITING_IMPLEMENTATION`. Readiness is
`waiting_implementation` or `waiting_data` with separate source/Item holds.
The original231 definitions and their219/12 readiness are unchanged. The family
index counts plain and variant profiles separately and includes whole-family UID
collision diagnostics. Registration counts all336 DATA definitions.

The ordinary `reward_claim_authoring.py content` command regenerates both profiles;
then regenerate the source variant packet so its input hashes bind the new shards.
`committed_errors()` validates both profiles against current sources and Items.
`native_lowering_errors()` provides explicit unsupported-profile diagnostics before
any consumer attempts the existing plain native model; this is not a runtime implementation.

```sh
python tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py
python tools/content-schema/reward-claim-authoring/reward_claim_authoring.py content
python tools/content-schema/reward-claim-authoring/reward_claim_variant_migration.py
python tools/content-schema/reward-claim-authoring/reward_claim_authoring.py content --check
python tools/content-schema/reward-claim-authoring/reward_claim_variant_migration.py --check
```

## Authored native selectors

The runtime binder also accepts an explicitly authored `source_binding` with
`project_position` and `native_ordinal` instead of `legacy_unique_ids`. The ordinal
is zero based among top-level entries, excluding nested container contents. The
binder verifies the exact cell, ordinal and `appearance_tibia_id` against the pinned
base map. Missing entries, ambiguous palette appearances, mixed selector namespaces
and malformed ordinals remain unserved. It never falls back from a missing donor UID.

This selector uses the existing claim reservation, fenced mint and receipt replay
owners. It does not supply stage progress or qualify a quest completion graph, and
it changes no source charge or fluid rules. The source-pilot generator continues
to emit its original donor selectors; authored records must declare their native
selector explicitly in their own authoring input.
