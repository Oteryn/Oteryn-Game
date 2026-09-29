# VSL-COMBAT-01 §19 Combat resource rows decision

- Decision: `VSL-COMBAT-01-RESOURCE-ROWS-V1`
- Status: **CANDIDATE with owner decisions D77-D79 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Source: #162 comment 5873043638 (Combat D readiness, routing item 1)
- Owner decisions posted: #162 comment 5873104041
- Precedent: `OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md` (A5)
- Admission baseline: `main@943e17b`
- Runtime, registry, migration and production authority: **NONE**. Registration belongs to the
  Combat D allocation under the registry single-writer lease.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

VSL-COMBAT-01 §19 requires finite ceilings for twelve resource dimensions before executable
acceptance and chooses no numbers. Combat D readiness (#162 5873043638) found items 3-12
unregistered. Which ceilings apply?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D77 | A creature death may carry up to 16 loot plan entries, each materialized as its own one-item DUR-03 MINT. Bosses get a separate decision. | "16 pozycji" |
| D78 | Per-scope concurrency ceilings for deaths, corpses and in-flight loot operations are 64, matching the D57 creature envelope. | "64, jak potwory" |
| D79 | The D1 owner split exception: a test-only death key → runtime `ItemMintCause` → MINT chain with one fixture loot entry may be allocated before Character progression readiness; no XP, no activation. | "Zgoda" |

## 3. Facts

**PROVEN**

- §19 items 1-2 (combat and ability occurrences, descendant depth) are covered by the registered
  `ABILITY01-*` and `INTERACTION01-*` rows.
- D57 (#1110): up to 16 spawns per scope, 4 creatures per spawn, 64 creatures per scope.
- DUR-03 transactions are one-item (`DUR03-RL-01` = 1), with no container expansion
  (`DUR03-RL-05` = 0) and a 512 B content-key bound (`DUR03-RL-07-CONTENT-KEY-BYTES`).
- D52 (#1079): uncommitted loot and XP descendants are dropped when the generation ends, never
  duplicated.
- `MOVE-RL-11` = 1 entity per snapshot or delta.

## 4. Decision

### 4.1 Rows

| § | Row | Hard maximum | Failure |
|---|---|---|---|
| 3 | `COMBAT01-DEATH-WORKFLOWS-PER-SCOPE` | 64 active death and loot-settlement workflows per scope | structurally unreachable (creatures ≤ 64); reject if reached |
| 4 | `COMBAT01-LOOT-PLAN-ENTRIES` | 16 entries per death | content validation rejects |
| 4 | `COMBAT01-LOOT-PLAN-ITEMS` | 16 ItemInstances per death; one MINT per entry; a stack is one instance with quantity | reject before allocation |
| 4 | `COMBAT01-LOOT-PLAN-BYTES` | 30,720 B per death: 16 × 1,792 B per entry plus a 2,048 B header (§4.1.1) | reject, never truncate |
| 5 | `COMBAT01-LOOT-RNG-DRAWS` | 32 draws per death (a chance draw and a quantity draw per entry) | reject before planning |
| 6 | `COMBAT01-CORPSES-PER-SCOPE` | 64 corpse projections per scope | reject the projection; the death still commits and loot follows D52 |
| 6 | `COMBAT01-ITEMS-PER-CORPSE` | 16, direct root only | reject |
| 7 | `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE` | 64 in-flight loot MINTs per scope; the rest stay planned in their death workflow (≤ 16 each) | backpressure, fail closed; D52 on generation change |
| 8 | `COMBAT01-PENDING-PICKUPS` | 1 per player actor, 64 per scope | reject the second |
| 9 | `COMBAT01-XP-DESCENDANTS-PER-DEATH` | 1 | reject |
| 10 | `COMBAT01-REWARD-PRINCIPALS` | 1 per death (single-principal slice) | reject |
| 11 | `COMBAT01-RESULT-BYTES` | 4,096 B per combat result projection, at most 2 result entries | reject |
| 11 | `COMBAT01-STATE-ENTRIES-PER-DELTA` | 64 combat state entries per delta, within `FND02-ORDINARY-REPEATED-ENTRIES` | reject |
| 12 | `COMBAT01-DIAGNOSTIC-VARIABLE-BYTES` | 0; fixed counters only | not reachable |

#### 4.1.1 Loot plan byte derivation

The plan bound follows the complete typed identities that `durability/item_mint.rs` validates and
encodes (`validate_definition`, `push_text` with a u16 length prefix; DUR-03 RL-07 field bounds):

- A `TypedDefinitionRef` is at most 1,152 B: `family` 128 B (technical text), `production_key`
  512 B and `revision_ref` 512 B (content keys).
- Entry, 1,792 B: the item `TypedDefinitionRef` (1,152 B), the purpose key (a content key, 512 B)
  and 128 B for length prefixes and fixed fields (draw ordinal, quantity, chance and quantity
  draws).
- Header, 2,048 B: the loot table `TypedDefinitionRef` (1,152 B) and 896 B for the death key
  (World, Channel, scope generation, actor id and generation: 52 B), length prefixes and fixed
  fields.
- 16 × 1,792 B + 2,048 B = 30,720 B. A plan of 16 entries with maximum-width valid identities fits.

### 4.2 Registration conditions

- Rows register through the `RESOURCE_LIMITS_REGISTRY.json` single-writer lease, with max and max+1
  tests. Ceilings are checked before allocation and never truncate.
- A larger value needs a new owner decision.

### 4.3 D1 exception (D79)

- Scope: death key → runtime `ItemMintCause` constructor → one-item MINT with one fixture loot
  entry (draw ordinal 0). Paths: `foundation/runtime_actor_carrier.rs`, `foundation/mod.rs`,
  `durability/item_mint.rs` (the constructor only) and tests.
- No XP descendant, no activation, no protocol. D/E admission and any Combat XP still wait for
  Character progression readiness (VSL-COMBAT-01 §24.1, §24.3).

### 4.4 Flagged dependency

`MOVE-RL-11` = 1 entity per snapshot or delta lets a client see only its own player. Client
observation of creatures and corpses (Combat child E; the GAME-AI slice D53) needs the Movement or
visibility owner to re-decide that row. This decision does not change it.

## 5. Decision test

- **Must decide now:** YES. Combat D is `WAITING_DEPENDENCY` on these rows (#162 5873043638).
- **Minimum sufficient:** single-principal values of 1 where the slice allows; the per-scope and
  per-death values follow the owner's envelope choices (D57, D77, D78).
- **Superseding evidence:** boss loot above 16 entries; multi-principal rewards; nested containers;
  measured memory or throughput limits.
- **Deliberately not decided:** loot tables and probabilities, XP values, `MOVE-RL-11`, corpse
  decay times.

## 6. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5873043638 routing item 1"
owner_decisions: [D77, D78, D79]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md
resource_values_changed: true   # registered by the Combat D allocation
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # D1 (D79) may be allocated on the owner decision; D/E still wait for readiness
required_fresh_allocation: true
required_independent_review: "exact-head independent review (resource ceilings, D1 exception)"
required_revalidation:
  - "Combat D registration: max and max+1 per row; the loot plan at 16 maximum-width entries and 30,720 B accepted, 17 entries or one extra byte rejected; 64 in-flight MINTs with backpressure beyond"
  - "D1: one death key yields one MINT with the fixture entry; a replay yields the same result; no XP path exists"
remaining_unknowns:
  - loot tables, boss loot, corpse decay, MOVE-RL-11 visibility
next_action: "#162 allocates D1 and the Character progression readiness lane; the Combat D allocation registers these rows."
```
