# Shards runtime dependency readback

Base readback: `main@fc3db9abfb4ae05c13b0264752cfbbca13a276e6`

This note narrows the execution blocker discovered after the initial reconstruction matrix.

## Already merged and reusable

The following Quest infrastructure is already present and must be reused rather than duplicated:

- `QUEST-STATE-1`: durable QuestState writer, receipts, admission copy and pending obligations.
- `QUEST-PRED-1` (#1723): the read-only predicate library API.
- `QUEST-XP-1` (#1724): quest XP obligation/award path.
- `QUEST-LOWER-1` (#1727): Source progress → native tracks/transitions + embedded loader.
- `QUEST-CAT-BOOT-1` / later boot work: the served content revision's `QuestStateCatalogue` is loaded and carried by gameplay transport.

The durability writer already accepts `QuestTransitionRequest` with causes including command/use/creature-death/claim-obligation. It provides exact replay, fencing, revision/hash validation, transition effects, completion and pending XP.

Therefore Shards must **not** introduce a second quest store, second transition writer or storage-number compatibility layer.

## Missing production producer seams

Fresh code search on current main found:

- no production caller of `QuestPredicate::holds()`; current calls are unit tests;
- no production implementation of `QuestPredicateFacts` outside its test fixture;
- no `commit_quest_transition(...)` call from `gameplay_transport`;
- `QuestStateCatalogue::requested_by()` is only exposed by the loader and tested there; no NPC runtime consumer found;
- `gameplay_transport` carries the boot-loaded catalogue, but does not yet dispatch ordinary NPC/map/boss events into quest transitions.

The only current production code that creates `game_character_quest_obligations` outside the QuestState module is the RewardClaim mint path. Its purpose is claim→quest retry/atomicity and it does not substitute for ordinary NPC/map/creature event producers.

This means the blocker is **not the durable quest writer**. It is the missing execution adapters that the accepted QUEST-GATE-0 architecture already assigns to:

- `QUEST-GATE-1`: door/placement gate lowering and read-time predicate checks;
- `QUEST-TRIGGER-1`: USE / ON_ENTER / ON_LEAVE interaction producers;
- `NPC-QUEST-1`: typed NPC dialogue conditions/outcomes;
- encounter/death composition for boss-driven progress.

## Consequence for Shards

The intended execution path is:

```text
NPC / placed WorldObject / tile / boss outcome
        ↓
existing typed event producer
        ↓
accepted Quest predicate/gate adapter
        ↓
QuestTransitionRequest
        ↓
existing QUEST-STATE-1 revision slot / writer
        ↓
receipt + QuestState copy update
```

Do not write direct QuestState progress from NPC handlers or WorldObject code.

### Stage routing

- s1, s2, s5, s6, s7, s9, s12, s16 → `NPC-QUEST-1`
- s3, s4, s8, s11, s13, s15 → `QUEST-TRIGGER-1` plus Item/WorldObject owners
- s10 → boss/death outcome producer → exact transition s10
- s14 → Shards-specific Encounter outcome → exact transition s14
- terminal s16 → existing exact transition `oteryn:quest-transition/authored/shards_of_a_broken_moon_quest/s16`

## Achievement access predicate

The current `QuestPredicate` closed enum contains:

- Track
- Elapsed
- QuestCompleted
- AccountCompleted
- LevelAtLeast
- HoldsItem

It has no account-Achievement fact predicate.

The Achievement owner already supplies durable account facts through `DurabilityRoot::read_account_achievements`. However the present QuestPredicate API is synchronous and has no production facts adapter. Therefore adding a DB query inside `QuestPredicate::holds()` would be the wrong composition.

For the Forbidden Gardens gate, the eventual production gate adapter should receive an Achievement-owned admission/session fact snapshot (or another owner-approved read seam) and evaluate:

```text
Shards of a Broken Moon completed
AND
account owns oteryn:achievement/forbidden_fruit@1
```

The achievement must remain Achievement-owned. Do not copy it into QuestState, and do not use an Area identity as access authority.

## Allocation implication

`SHARDS-Q1/I1/E1/R1` should be serialized behind or inside the existing executable Quest children. A standalone Shards worker can safely author exact bindings only once the control plane grants the corresponding paths/adapters.

No runtime/content mutation is required to preserve this finding.
