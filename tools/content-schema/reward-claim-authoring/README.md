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

The family holds **plain `once` claims only**. That means a per-character `once` claim whose
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
