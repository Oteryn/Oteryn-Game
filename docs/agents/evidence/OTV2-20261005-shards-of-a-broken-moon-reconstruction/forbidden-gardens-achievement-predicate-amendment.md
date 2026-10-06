# Forbidden Gardens achievement predicate amendment

Status: PROPOSED ARCHITECTURE AMENDMENT / NO RUNTIME MUTATION

## Gap

Forbidden Gardens requires:

- Shards of a Broken Moon completed;
- account achievement `oteryn:achievement/forbidden_fruit@1`.

Current `QuestPredicate` supports Track, Elapsed, QuestCompleted, AccountCompleted, LevelAtLeast and HoldsItem. It has no Achievement predicate.

Achievement durability already exposes a read-only account fact surface through `DurabilityRoot::read_account_achievements`.

## Smallest amendment

Add:

```rust
QuestPredicate::AccountHasAchievement(String)
```

and:

```rust
fn account_has_achievement(&self, achievement: &str) -> bool;
```

Evaluation remains read-only:

```rust
Self::AccountHasAchievement(key) => facts.account_has_achievement(key)
```

The production `QuestPredicateFacts` adapter must obtain the value from the Achievement-owned fact surface. QuestState must not copy or persist that fact.

## Gate composition

No OR or expression-tree widening is needed. The existing conjunction is sufficient:

```text
quest_completed(oteryn:quest.authored.shards_of_a_broken_moon_quest)
AND
account_has_achievement(oteryn:achievement/forbidden_fruit@1)
```

## Fail closed

Return false when the Achievement key is unknown, the fact is unavailable, or content generation is stale.

## Tests

- owned known achievement -> true;
- unowned known achievement -> false;
- unknown achievement key -> false;
- conjunction passes only when both Shards completion and Achievement fact are true;
- predicate read causes no Quest or Achievement write;
- relog/restart reloads the same durable Achievement fact.

## Ownership

Achievement remains Achievement-owned. Quest/Gate consumes only the read-only fact. No new persistence is required.


## Production adapter shape

Fresh current-main readback closes the I/O shape.

`QuestPredicate::holds()` is intentionally synchronous. Do not put a database call inside the predicate trait.

The gameplay transport that will own QUEST-GATE-1 already has:

```text
DurabilityRoot
ReconciledCharacterAuthority
AchievementCatalogue
```

and already uses the accepted async durability read:

```text
DurabilityRoot::read_account_achievements(...)
```

for account-achievement display/query handling.

Therefore the bounded runtime shape is:

```text
gate check / authoritative command occurrence
  -> async prefetch of the account's Achievement facts through the existing owner read
  -> build immutable QuestPredicateFacts view for this evaluation
  -> synchronous QuestPredicate::holds(...)
  -> discard the temporary view
```

No session-wide achievement cache is required by this amendment.

On durability/read failure the gate fails closed; it must not treat an unavailable Achievement read as an unowned-but-valid fact and then persist any substitute state.

The existing Achievement catalogue remains the key-validation authority. The adapter should reject an unknown achievement key before or while constructing the facts view.

This keeps the change limited to:

1. one new closed predicate variant;
2. one read-only trait method;
3. the production Gate facts adapter/prefetch;
4. focused tests.

It does not add:

- new tables;
- new outbox/event rows;
- QuestState copies of Achievement facts;
- a new session state domain;
- a background cache invalidation protocol.
