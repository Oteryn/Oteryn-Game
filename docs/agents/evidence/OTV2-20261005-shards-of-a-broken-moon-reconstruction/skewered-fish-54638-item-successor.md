# Skewered Fish i54638 — quest reward Item successor

Status: **EVIDENCE / ALLOCATION-READY, NO CONTENT MUTATION**

## Current canonical state

Fresh protected-main readback:

```text
canonical Item:
  oteryn:item.tibia.i54638@definition-r1

shard:
  content/items/definitions/items-29000-29499.json

current:
  materializable: false
  client_projection: ClientSafe
  stack_class: Unknown
  physical:
    movable: true
    pickupable: true
    weight: 200
  stack:
    stackable: false
```

A12 alias table already has:

```text
oteryn:item.registry.i00038475
  state: ALIAS
  source_item_id: 54638
  target: oteryn:item.tibia.i54638
```

Therefore no identity mint/re-key is required.

## Qualified source facts

Pinned Crystal Summer:

```text
zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f
donor item 54638
name: skewered fish
family profile: tool
```

Current post-release TibiaWiki corroborates:

- Skewered Fish is a Shards of a Broken Moon reward;
- weight 2.00 oz;
- it is a taming item;
- it is used on Jaracal for the Jaracal Mount.

These facts agree with the existing canonical Item record. No source conflict requiring a native-core hold was found.

## Existing accepted promotion path

Do **not** create a new Rust Item admission framework.

The established quest Item pipeline already exists:

```text
tools/content-migration/quest_reward_item_semantics.py
docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json
docs/agents/evidence/OTV2-20261001-quest-interaction-item-admission.json
```

The migrator's existing rule is:

```python
if not facts["holds"]:
    definition["materializable"] = True
```

and it already:

- validates Item identity/revision;
- derives NonStackable vs StackCapable;
- preserves/qualifies physical facts;
- refuses conflicting existing facts;
- only promotes proof-backed scoped Items.

The historical 2026-10-01 packets are immutable evidence and should remain byte-exact.

## Successor shape

Add one bounded successor input, conceptually:

```json
{
  "schema": "OTERYN_QUEST_REWARD_ITEM_CORE_ADMISSION_SUCCESSOR/v1",
  "predecessor": "<sha256 of frozen reward admission packet>",
  "admissions": [
    {
      "item_id": 54638,
      "item_key": "oteryn:item.tibia.i54638",
      "client_name": "skewered fish",
      "facts": {
        "stackable": false,
        "capacity": null,
        "charges": null,
        "holds": []
      }
    }
  ]
}
```

The successor must carry its own exact source witnesses/digests; it must not append the row into the frozen historical packet.

## Smallest implementation lease

Expected Item-owner lease:

```text
tools/content-migration/quest_reward_item_semantics.py
tools/content-migration/test_quest_reward_item_semantics.py
docs/agents/evidence/<new-skewered-fish-successor>.json
imports/tibiawiki/quest-reward-items/<new pinned source capture, if source policy requires it>
content/items/definitions/items-29000-29499.json
```

Generated manifest/index/lock/provenance surfaces only as required by the normal repository validator/materializer.

No need to touch:

- `content/items/aliases.json`;
- historical B1b donor crosswalks;
- Item identity rules;
- Starter `item_admission.rs`;
- the immutable 2026-10-01 reward packet.

## Expected postcondition

```text
oteryn:item.tibia.i54638
  same identity/revision
  materializable = true
  stack_class = NonStackable
  movable = true
  pickupable = true
  weight = 200
```

No new Item identity and no retired-key use.

Once this promotion is merged, Shards s16 RewardClaim may mint one `i54638`, and the later Jaracal taming interaction may consume/use that same canonical Item under its own runtime owner.
