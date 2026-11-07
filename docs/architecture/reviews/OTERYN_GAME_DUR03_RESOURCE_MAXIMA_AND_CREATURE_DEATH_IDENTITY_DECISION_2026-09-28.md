# DUR-03 resource maxima and creature-death identity decision

- Decisions: `DUR03-ONE-ITEM-RESOURCE-MAXIMA-V1` (A5) and `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` (A4)
- Status: **CANDIDATE with owner decisions D50-D52 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Source escalations: #162 comments 5866279955 (A5) and 5860789993 (A4); owner direction 5866606575; packet 5866894112
- Related: #513 (B4), VSL-COMBAT-01 stages C and D, #1031 (B2), #950, #998 (P03)
- Admission baseline: `main@57a0fc7`
- Runtime, migration, registry and production authority: **NONE**. This decision gives the values;
  the B4 and D allocations register and implement them.
- Contract text amended: DUR-03 §39.3 (restart clause) and VSL-COMBAT-01 (recovery property), for D52.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Questions

- **A5.** `OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md` (:334, :338, :409-413)
  requires an owner hard-maximum decision before any DUR-03 resource row is registered. Rows RL-07
  (audit bytes) and RL-08 (retry work) had no accepted bound; the rest had ready values.
- **A4.** VSL-COMBAT-01 defers the compact representation of `CreatureDeathOccurrenceRef` (:128),
  but requires that retry and recovery reconcile loot and XP descendants (:135) and that despawn
  never manufactures a death (:138). DUR-03 requires that restart resolve the same death and output
  cause to the same terminal mint result, and that a caller-provided UUID cannot create mint
  authority (:885-894). The #950 death ref lives only in memory, and P03 accepts only a UUIDv7
  occurrence (`character_progression.rs:31-39`).

## 2. Owner decisions (2026-09-28)

| # | Decision | Owner choice |
|---|---|---|
| D50 | RL-07: one audit event per logical transaction; normative `EventEnvelope` at most 9,216 B, payload at most 7,936 B; content keys and revisions at most 512 B, other technical fields at most 128 B; item free text excluded; unexercised fields excluded fail-closed (§3.2). RL-01, RL-02, RL-03/04/05, RL-06 and P90D take the measured values of §3.1. No private 4,096 B cap is registered. | Direction #162 5866606575; final numbers accepted: "Koperta 8 192 B", "Tak, jak w kierunku"; envelope raised after the normative-envelope recount: "9 216 B" |
| D51 | RL-08: 3 work units now, with a mandatory re-decision in stage C once real PostgreSQL reconciliation exists (§3.3). | Same |
| D52 | If the server stops after a creature death but before its loot and XP commit, those rewards are lost. They are never duplicated (§4). | "Przepadają, bez duplikatów" |

## 3. A5: DUR-03 native one-item resource maxima

### 3.1 Ready rows

| Row | Hard maximum | Failure |
|---|---|---|
| DUR03-RL-01 touched ItemInstances per transaction | 1 | reject |
| DUR03-RL-02 location/custody lines per transaction | 2 | reject |
| DUR03-RL-03 value lines | 0 (not admitted) | reject, fail-closed exclusion |
| DUR03-RL-04 transform inputs/outputs | 0 (not admitted) | reject, fail-closed exclusion |
| DUR03-RL-05 container expansion | 0 (direct root only) | reject, fail-closed exclusion |
| DUR03-RL-06 participants / effect work units | 1 participant / 3 work units | reject |
| Audit retention profile | P90D = 7,776,000 elapsed seconds | as accepted retention decision |

### 3.2 RL-07: audit events and bytes per transaction (D50)

- Events per logical transaction: **1**.
- Envelope (the normative ANL-01 `EventEnvelope`, `game-events/v1/foundation.proto`): **≤ 9,216 B**.
  Payload: **≤ 7,936 B**. ANL-01 limits still apply on top
  (`ANL01-EVENT-ENVELOPE-BYTES` is 262,144 B).
- Per-field bounds:

| Field | Bound |
|---|---|
| Content keys and revisions: `production_key`, `revision_ref`, map, ground content, loot, provenance content, ruleset and SIM revisions | ≤ 512 B each (`DUR04-FIRST-PROD-KEY-BYTES`, `-STRING-BYTES`) |
| Technical: `family` (closed enum), `spatial_position`, corpse ref, native room/placement context, `typed_cause` | ≤ 128 B each |
| Loot output cause (§4.2): death key (two UUIDs, u64, u32, u64), `LootTableDefinitionRef` (family ≤ 128 B, key and revision ≤ 512 B each), purpose key ≤ 512 B, draw ordinal u32 | as listed |
| `retention_profile_input` | ≤ 128 B (`ANL01-ENVELOPE-STRING-BYTES`) |
| UUID fields (item, world, channel, character, session, event, transaction) | 16 B |
| `payload_sha256` | 32 B |
| Item free text (writable text, descriptions) | excluded from the audit event |
| Inventory `typed_position` | excluded fail-closed until its owner defines it; TRANSFER stays closed |

- **Derivation.** This is the exact protobuf worst case of the final schema: the candidate payload fields, with
  the separate output and death ids and the single loot revision replaced by the full §4.2 loot
  output cause. The payload sits inside the normative `EventEnvelope` with every optional field
  present: world, channel, instance and node; session, connection generation, runtime order and
  command; operation, transaction event, correlation, causation and analytics actor; protocol
  major; and the ruleset, content and build strings at the ANL-01 128 B maximum. Every bounded
  field is at its maximum and varints are at their widest. The envelope overhead is at most
  1,038 B.

| Case | Payload | Envelope |
|---|---|---|
| MINT | 6,129 B | 7,167 B |
| TRANSFER, `typed_position` excluded | 7,433 B | 8,471 B |
| TRANSFER, `typed_position` at 128 B (future) | 7,565 B | 8,603 B |

  The payload cap is 7,936 B, so any valid envelope is at most 7,936 + 1,038 = 8,974 B, which is
  within the 9,216 B cap. Opening TRANSFER with a `typed_position` of up to 128 B needs no
  re-registration. The `OfflineCandidateEnvelope` evidence wrapper is not the registered envelope.
- Failure: `CAPACITY_EXCEEDED` or `INVALID_INPUT`, checked before allocation, never truncated,
  not visible to the client.

### 3.3 RL-08: retry and reconciliation work (D51)

- RL-08 = **3 work units** per logical transaction. All attempts reuse the same TransactionId and
  frozen candidate (DUR-03 §§23.2-23.3). An unknown outcome is held and never gets a new
  TransactionId (DUR-03 :470).
- Each pass stays within `DFR-DB-PASS-MS` (2,000 ms).
- This is a policy value, not a measurement. Stage C must **re-decide** it once real PostgreSQL
  reconciliation exists, and prove max/max+1 behaviour across a restart.

### 3.4 Registration conditions for B4

- Register with a non-candidate package and without `_fixture` field names.
- If a field type changes (for example `typed_cause` to an enum or `spatial_position` to fixed
  10 B), re-measure the goldens and bind fresh evidence.

## 4. A4: creature-death occurrence identity (D52)

### 4.1 Representation

`CreatureDeathOccurrenceRef` has the durable, restart-stable key

```text
(WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId, ActorLocalGeneration)
```

- These are the existing `ActorRef` fields (`runtime_actor_carrier.rs:132-138`). A later
  InstanceRuntime uses its own runtime-scope ref in place of `ChannelId`.
- `LethalCommittedEffectOccurrenceRef` and `BoundSemanticRevisionContext` are bound attributes.
  The same key with a different binding is an integrity `CONFLICT`.
- **Non-reuse is structural.** An ownership generation is never rolled back or reused
  (`0003_runtime_scope_assignment.sql:400-402`). A carrier serves one generation, and an actor slot
  generation only increases and is never reused after exhaustion (carrier :1312-1317). No later
  process can produce the same key, so no retention window is needed for death non-reuse.
- NodeId, wall time and pointer addresses stay out of the key (VSL :136).

### 4.2 Derived causes

- **Loot MINT cause:** the typed tuple
  `(death key, LootTableDefinitionRef, LootEntryOrPurposeKey, DeterministicDrawOrdinal)`, stored in
  full as the MINT's unique source cause. There is no hash-only equality (DUR-01 :361).
- **XP occurrence:** the Combat owner mints one server UUIDv7 `ExperienceRewardOccurrence` per
  (death, eligible character) when it commits the death, and keeps it in its death record. Every
  retry in that generation reuses it, and the P03 receipt deduplicates. P03 is unchanged.
- Neither id comes from a caller. The authority is the physical owner's committed death, whose
  constructor accepts no caller bytes (carrier :373-375).

### 4.3 Live-generation rule (D52)

- A descendant (loot MINT, XP award) commits only while the death's ownership generation is the
  current assignment, held by the committing node's current incarnation. This is the same fence P03
  and #1033 use.
- When the generation ends (restart, crash, scope move), uncommitted descendants are dropped. No
  later generation retries them, and none can produce the same key.
- Retry and reconciliation inside the generation resolve to the same result. After the generation
  ends, whatever committed stays committed; an uncommitted descendant is terminally not minted or
  not awarded. This PR writes that terminal rule into DUR-03 §39.3 (the restart clause) and
  VSL-COMBAT-01 (the recovery property), so stage D does not receive conflicting recovery rules.
  No durable death row is needed.
- The owner lane never waits on the database (VSL :151, :518). Descendants run off the lane.
- A despawn or scope retirement creates no death key (VSL :138).

### 4.4 Rejected options

- **Durable death record with resumable rewards.** It needs a new table, a migration, a recovery
  scanner, and XP delivery to offline characters, whose P03 fence needs a live session. The owner
  chose D52 instead. It stays the upgrade path if resumable rewards are ever required.
- **Hash-derived child ids.** P03 rejects non-v7 UUIDs, and forcing the version nibble breaks
  DUR-01 :109 and :361.

## 5. Decision test

- **Must decide now:** YES. B4 cannot register rows and D cannot mutate without these values.
- **Minimum sufficient:** A5 uses measured counts and derived byte bounds; A4 reuses
  existing actor identity and fences, with no new table.
- **Superseding evidence:** an audit field that exceeds its bound in accepted content; the stage C
  RL-08 re-decision; an owner requirement to resume rewards after a crash.
- **Deliberately not decided:** loot tables and probabilities; XP values; inventory capacity;
  TRANSFER admission; party XP split; the physical MINT schema.

## 6. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalations: ["#162 5866279955 (A5)", "#162 5860789993 (A4)"]
owner_decisions: [D50, D51, D52]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md
resource_values_changed: true   # values selected here; registration is B4's
production_authority_changed: false
cross_repository_authority_changed: false
implementation_lanes:
  - "B4 (#513): register DUR03-RL-01..08, the P90D profile entry and the event type with these maxima"
  - "C: prove RL-08 with max/max+1 tests across restart"
  - "D: build the death key and descendant fences per §4"
implementation_may_resume: true   # B4 and D may be allocated now on this owner decision; this text still needs protected integration
required_fresh_allocation: true
required_independent_review: "exact-head independent review (DUR/ANL resource values, death identity)"
required_revalidation:
  - "B4: max and max+1 per row; the worst-case MINT event of the final schema in the normative EventEnvelope (7,167 B) is accepted; a 513 B content key or a 129 B technical field is rejected, not truncated"
  - "C: re-decide RL-08 from real PostgreSQL reconciliation; prove max/max+1 across restart"
  - "D: a replayed lethal effect makes no second death; a retry in the same generation returns the same MINT and XP result; after a generation change a pending descendant is refused and nothing is minted or awarded; the same key with a different binding conflicts; a stale actor handle cannot kill a recycled actor; a despawn creates no death key"
remaining_unknowns:
  - loot tables, XP values, inventory capacity, TRANSFER admission
next_action: "#162 allocates B4 and D against these values; this text integrates through the governed Merge Queue."
```
