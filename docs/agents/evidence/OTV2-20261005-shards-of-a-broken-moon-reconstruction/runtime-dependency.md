# Shards runtime dependency readback

Fresh readback: `main@492f25c90ebcdb85d8cc8f2d2b273f72017dc8ef` (2026-10-07).

This note records the executable dependency boundary for Shards of a Broken Moon. It supersedes the older `fc3db9ab...` runtime readback while preserving the same rule: **do not build a quest-specific runtime**.

## Already merged and reusable

The durable Quest core is already present and must be reused:

- `QUEST-STATE-1`: QuestState writer, receipts, completion and pending obligations.
- `QUEST-PRED-1` (#1723): closed read-only QuestPredicate API.
- `QUEST-XP-1` (#1724): quest XP obligation/award path.
- `QUEST-LOWER-1` (#1727): Source progress -> native tracks/transitions + catalogue loader.
- `QUEST-CAT-BOOT-1` and later boot work: served content revision carries the QuestState catalogue.
- #1886: QUEST-TRIGGER-1 durable-cause ambiguity is resolved. A trigger child uses the root CommandRef; transition keys remain distinct under one root.
- #1807: Monster/source-mechanic reconciliation is merged, so its broad source ownership no longer blocks Shards evidence/content planning.
- #1891: Magnolia phase-2 architecture is resolved: first lethal path prevents death, sets `max_health=60000`, then performs an explicit full heal; max-health override never heals implicitly.

The durability writer already accepts `QuestTransitionRequest` and provides replay/fencing/revision/hash validation. Shards must **not** add a second Quest store, transition writer, storage-number layer or quest-local persistence.

## Planning coverage is no longer the blocker

Current Quest planning covers all **310 / 310 canonical completion-candidate owners**:

- chosen completion binding plan: 304 owners / 1,891 stages;
- Source-lowered binding plan: 6 owners / 121 existing transitions;
- total represented work: 2,012 binding units.

Native runtime bindings remain zero. The remaining bottleneck is executable owner implementation, not missing Quest authoring coverage.

## Generic runtime roots still missing

Fresh PR/main search finds no open implementation PR for:

- `QUEST-GATE-1`;
- `QUEST-TRIGGER-1`;
- `NPC-TALK-1`;
- `NPC-PLACE-1b`;
- `MAP-CUTOVER-1b`;
- `ENC-RT-1`;
- `ENC-OUTCOME-1`.

### Lane A — Quest gate / trigger

`QUEST-GATE-1` is allocation-ready. Its accepted bounded input is 198 definitions / 373 declared placements (184 quest-progress + 14 level gates); held/unresolved records stay fail-closed.

After Gate merges, `QUEST-TRIGGER-1` is the next Quest runtime root. #1886 already resolved its durable cause; do not reopen that architecture.

For Shards:

- s3/s4/s8/s11/s13/s15 consume `QUEST-TRIGGER-1` plus the existing Item/World Interaction owners;
- the lab Plinth, ritual placements and prison route already have exact qualified bindings in `binding-plan.json`.

### Lane B — served World / NPC chain

`MAP-CUTOVER-1b` is the shared serving root. Current accepted order is:

```text
MAP-CUTOVER-1b + NPC-PLACE-1a
    -> NPC-PLACE-1b
    -> NPC-TALK-1
    -> NPC-QUEST-1
```

`NPC-WIRE-1` is already merged. The NPC-PLACE-1 architecture decision is merged, but the 1a/1b implementation children are not.

For Shards:

- s1/s2/s5/s6/s7/s9/s12/s16 require `NPC-QUEST-1`;
- NPC branch evidence is already reconciled to the post-release Shards source packet in #1852;
- do not write QuestState directly from dialogue handlers.

### Lane C — Encounter outcomes

The accepted order remains:

```text
ENC-RT-1
  -> ENC-COMBAT-1 / accepted combat composition
  -> ENC-OUTCOME-1
  -> QuestTransitionRequest
```

For Shards:

- s10 Rakesh Moonfang needs the generic encounter/death outcome producer;
- s14 Magnolia uses the #1891 max-health ruling and then emits one permanent-death qualified outcome;
- do not add a quest-only creature-death bypass.

## Item / Door blockers

The three Shards items are proof-complete in #1852 but owner implementation is still required:

- `oteryn:item.tibia.i54262` — Asura Citadel Key; existing canonical identity, exact door source, no donor `keyNumber`; needs materializable admission.
- `oteryn:item.tibia.i54638` — Skewered Fish; existing canonical identity/exact Crystal binding; needs materializable admission.
- `oteryn:item.tibia.i54610` — Lit Torch (SU26); must be a distinct successor identity, never aliased to old `i34017`; then portable/useable/light admission.

Asura Citadel door remains a bounded architecture amendment:

```text
required_item = oteryn:item.tibia.i54262
bypass = quest_completed(Shards)
```

Do not synthesize a numeric key number. `ITEM-USE-1` / USE-WITH is also required before the identity-key door path can execute.

## Forbidden Gardens access

Reference semantics are qualified as:

```text
Shards of a Broken Moon completed
AND
account owns oteryn:achievement/forbidden_fruit@1
```

Current `QuestPredicate` has no Achievement fact predicate. #1852 contains the bounded amendment:

- `QuestPredicate::AccountHasAchievement(String)`;
- `QuestPredicateFacts::account_has_achievement(&str) -> bool`;
- fact supplied only from the existing Achievement-owned account surface;
- unknown/unavailable fact fails closed;
- no copied Achievement state in QuestState.

The active World placement must also be confirmed/bound through `QUEST-GATE-1` once the Summer world is actually served.

## Resolved prison route

The escape geometry is no longer an implementation-unknown:

- skeleton / Lit Torch source: `31920,31360,9`;
- icicles / Icicle Chisel source: `31914,31364,9`;
- north wall target: `31920,31359,9`;
- Rope Spot: `31923,31377,9`;
- generic rope destination: `31923,31378,8`.

Movement remains owned by generic World/rope semantics. Quest progress observes qualified interaction success.

## Execution order

The smallest non-special-case route to playable Shards is:

```text
1. QUEST-GATE-1
2. MAP-CUTOVER-1b
3. QUEST-TRIGGER-1
4. NPC-PLACE-1a/1b -> NPC-TALK-1 -> NPC-QUEST-1
5. Item-owner successor/admission for i54262/i54610/i54638
6. ITEM-USE-1 + identity-key Door/Key amendment
7. ENC-RT-1 -> ENC-OUTCOME-1
8. AccountHasAchievement predicate amendment
9. Shards exact bindings to the generic owners above
10. real-character start -> reward -> relog/restart E2E
```

These roots can be parallelized where path ownership permits; they must not be replaced by per-quest shortcuts.

## Final qualification

Shards is not complete until a real Character can:

```text
start
-> execute all native quest transitions
-> complete Rakesh
-> complete the four-rune ritual
-> complete Magnolia two-phase encounter
-> escape the prison
-> return to a valid terminal NPC branch
-> receive direct reward exactly once
-> retain Quest/reward/access state across relog and server restart
```

No runtime/content mutation is authorized by this evidence file.
