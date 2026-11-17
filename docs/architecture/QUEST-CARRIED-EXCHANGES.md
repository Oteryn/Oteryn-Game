# Carried quest exchanges

`quest::reward_callbacks::RewardCallback.carried_exchange` is an explicit,
default-false extension to existing shared reward callbacks. Use it for a
source-backed ordinary carried USE that consumes the selected instance and
mints an item reward through the existing quest transaction.

The callback must declare:

- `carried_item`: the exact admitted native Item key;
- `carried_exchange: true`;
- `take`: 1–8 unique Item keys, each with count 1–100, including exactly one
  unit of `carried_item`;
- `give`: the existing `ItemReward` shape;
- the ordinary source-owned quest storage guards and updates.

The selected definition consumes only the instance selected by ingress. Other
costs come from the same locked carried tree. Input consumption, quest state,
completion, reward MINT and replay proofs share the existing fenced transaction.
Missing ingredients or an uncommittable reward leave the input and quest state
unchanged. The existing fee and actor-condition restrictions on carried USE
remain in force. NPC dialogue cannot execute a callback with `carried_item`.

Leave the flag absent for existing outfit/addon tools. Those retain the exact
one-input, no-item-delivery contract and omit the new flag from serialized
definition bytes. This mode does not imply repeatability: the normal quest
transition and once-claim identity still govern the exchange.

Explicit authored stage bindings can use
`authored_bindings::Trigger::CarriedUse { callback }` and
`Bindings::parse(...).attach(&catalogue)`. The immutable catalogue hashes the
binding document, including its exact native identities and recipe digest.
Retained reward callback key collisions fail during attachment. This API does
not infer NPCs, items or selectors from prose and does not auto-promote recipes.

The production-path regression is
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/authored_quest_bindings.rs`.
This API does not promote any whole-quest completion status.

Reuse: questpc3 commit `9936a44dd27374dfcd9c5430888c972b0d35e5ef`, selected
definition filtering and bounded exchange input proofs. QENGINE2 migration1552
adds the explicit mode guard. Collector integration must reconcile peer1226
with1552 and retain one compatible installed proof implementation.
