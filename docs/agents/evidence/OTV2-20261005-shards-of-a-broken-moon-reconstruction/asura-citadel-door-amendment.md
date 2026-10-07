# Asura Citadel door — bounded identity-key / completion-bypass amendment

Status: **PROPOSED ARCHITECTURE AMENDMENT / NO RUNTIME MUTATION**

Quest: Shards of a Broken Moon  
Door: `33899,32666,8`  
Key: `oteryn:item.tibia.i54262` (Key (Asura Citadel))

## Exact source facts

The source behavior is:

1. during the quest, the sealed door requires the Asura Citadel key;
2. Javala grants/re-grants that key through quest dialogue;
3. the exact donor Item is id `54262`;
4. donor `items.xml` has **no `keyNumber`** for this Item;
5. after Shards completion, the same door no longer requires the key.

The existing WORLD-INTERACTION-0 key-door contract cannot represent this honestly:

- §4.3 compares an instance `key_number` with a numeric gate binding;
- §4.4 requires every key capability to mint immutable `key_number in 1..65535`;
- the current ingress rule assumes RewardClaim MINT;
- Shards key is identity-qualified and NPC-granted;
- inventing a numeric key number would create source data that does not exist.

## Rejected approaches

Do not:

- synthesize a `key_number` from donor id 54262;
- alias the key to any numbered-key definition;
- copy quest completion into Item or door state;
- widen all Quest gates with arbitrary boolean expressions only for this case;
- special-case coordinate 33899,32666,8 in quest code.

## Smallest compatible amendment

Add a second typed Door/Key binding alongside numeric key doors:

```text
IdentityKeyDoor {
    required_item: ItemDefinitionRef,
    bypass: Option<QuestPredicate>,
}
```

For Shards:

```text
required_item = oteryn:item.tibia.i54262
bypass        = quest_completed(Shards of a Broken Moon)
```

The bypass is read-only and remains Quest-owned; Door runtime only consumes the accepted predicate result.

## Runtime semantics

### Before completion

When bypass is false:

- ordinary USE on locked door: `LOCKED`, no state change;
- USE-WITH `i54262`: if actor currently owns that exact Item instance/definition, unlock/open according to key-door overlay semantics;
- another Item: `KEY_MISMATCH`;
- using the key does not consume or mutate it;
- channel door overlay remains ephemeral as WORLD-INTERACTION-0 §4.2/§4.3.

No numeric key state is read or written.

### After completion

When bypass evaluates true:

- the actor is not required to present `i54262`;
- USE/pass behavior follows the accepted quest-door path for that actor;
- the completion fact is checked again on each USE/step, exactly like other Quest predicates;
- no durable “unlocked forever” bit is copied into the door.

This preserves the source statement “key during the quest; no key after completion” while keeping door state channel-local and completion durable in QuestState.

## Why this belongs to Door/Key owner

The unusual fact is not a new Quest predicate: `quest_completed` already exists.

The unusual fact is that one source key is identified by **Item definition identity**, not per-instance numeric key state.

Therefore the amendment belongs to Door/Key binding/runtime, with a read-only QuestPredicate bypass hook.

## Item ingress

Javala's key grant must use the accepted NPC quest outcome -> Item/Durability mint path once NPC-QUEST is live.

The door contract must not require RewardClaim-only provenance. Its security condition is ownership of the exact qualified Item definition at use time, not how the Item originally entered inventory.

Lost-key regrant is separately gated by quest/dialogue eligibility and remains NPC/Item-owned.

## Validation

Content/binding:

- `required_item` must resolve to one materializable, non-stackable Item definition;
- `bypass`, when present, must be an accepted read-only QuestPredicate;
- zero/ambiguous door placement holds fail closed;
- an identity-key door must not also declare numeric `key_number`.

Runtime:

- wrong item refuses;
- correct `i54262` opens before completion;
- key is not consumed or mutated;
- completed character passes without key;
- incomplete character without key cannot follow through an open door if gate semantics require per-character recheck;
- stale content/placement fails closed;
- channel restart resets overlay, not Quest completion;
- no quest revision/receipt is written by a gate read;
- no Item instance gains synthetic `key_number`.

## Scope

This amendment is intentionally narrower than adding generic `any_of` expressions to QUEST-GATE-1.

It solves the exact source shape while preserving:

- existing numeric key doors unchanged;
- QuestState ownership unchanged;
- Item durability unchanged;
- no second persistence model;
- no Shards-specific coordinate branch in runtime.


## Existing-composition alternatives checked

Fresh accepted-contract readback rules out the two apparent shortcuts.

### Generic `tool_target` is not an arbitrary Item-definition matcher

WORLD-INTERACTION-0 §6.3 defines `tool_target` only by closed `tool_kind` values such as:

```text
rope
shovel
pick
machete
```

It is not an `ItemDefinitionRef` matcher. Reclassifying the Asura Citadel key as a generic tool would falsify Item semantics.

### QUEST-TRIGGER-1 does not own USE-WITH

QUEST-GATE-0 §4 defines trigger edges as:

```text
USE
ON_ENTER
ON_LEAVE
```

It does not define `USE-WITH carried Item -> map item` as a quest-trigger root.

Therefore the key use cannot honestly be represented as:

```text
QUEST-TRIGGER-1(item 54262 on door) -> overlay open
```

without broadening the Quest trigger contract merely to avoid the Door/Key amendment.

### WORLDINT key path is numeric-only today

WORLD-INTERACTION-0 §4.3/§4.4 explicitly requires:

```text
key.key_number == door.key_binding
key_number in 1..65535
```

and the key number is immutable ItemInstance state.

Exact donor `54262` has no such source field.

## Result

The narrow identity-key door amendment remains the smallest owner-correct representation:

```text
Door/Key owner:
  required_item = oteryn:item.tibia.i54262
  optional bypass = quest_completed(Shards)
```

This avoids all three invalid alternatives:

- inventing a numeric key number;
- pretending the key is a rope/shovel/pick/machete-like tool;
- widening QUEST-TRIGGER-1 to USE-WITH only for this door.


## Additional runtime dependency: ITEM-USE-1

Fresh current-main readback adds one transport dependency that the Door/Key amendment must not bypass.

`apps/game-server/src/gameplay_transport/connection.rs` currently documents and enforces:

```text
ITEM-USE-WIRE-1 decodes use_with only under capability 15.
Until ITEM-USE-1, no item is usable on a creature or by its definition.
A UseIntent with use_with=Some(...) is rejected fail-closed.
```

Therefore the Asura Citadel door implementation sequence is:

```text
ITEM-USE-1 accepted USE-WITH occurrence
  -> identity-key Door/Key binding for i54262
  -> optional read-only QuestCompleted bypass
  -> existing WorldInteraction door overlay/open semantics
```

The Door/Key amendment must not add a private parser/dispatcher for USE-WITH and must not special-case the raw wire command.

This is an execution dependency only. It does not change the source model: the key remains exact Item identity `oteryn:item.tibia.i54262`, non-consumed on use, with no synthetic numeric key number.


## Exact placement/object readback

Pinned Summer world:
`zimbadev/crystalserver@00ce02a5:data-global/world/world.otbm`

Exact tile:

```text
33899,32666,8
  ground/base: item 53254
  door:        item 53380 "closed door"
               type=door
               blockProjectile=1
```

Therefore the Door/Key binding is not a free-text/coordinate-only target. It can bind the exact placed door object:

```text
placement = (33899,32666,8)
door_source_item = 53380
required_item = oteryn:item.tibia.i54262
```

## Key admission readback

Current canonical `oteryn:item.tibia.i54262@definition-r1` already exists but is still:

```text
materializable = false
stack_class = Unknown
physical.movable = true
physical.pickupable = true
physical.weight = 100
```

Exact 15.30 client facts:

```text
flags.usable = true
flags.multiuse = true
flags.take = true
flags.cyclopediaitem = true
cyclopediaitem.cyclopedia_type = 54262
```

Exact Crystal Summer `items.xml`:

```text
id=54262
name=key
article=a
primarytype=others
weight=100
keyNumber absent
```

Running this row through the existing `quest_reward_item_semantics.py::item_facts`
logic yields:

```text
stackable=false
capacity=null
charges=null
holds=[]
```

So key identity is resolved and admission is proof-complete, but runtime cannot mint/regrant it until the Item successor promotion makes the existing canonical record materializable.

This is an Item-admission dependency only; do not fold it into the Door/Key amendment itself.
