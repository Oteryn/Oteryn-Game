# ACCEPT-ITEMUSE-BANK-0 Acceptance of ITEM-USE-0 and BANK-0

- Decision: `ACCEPT-ITEMUSE-BANK0-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  reviews of §2 on the frozen head and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D498 (10a, #162): take ITEM-USE-0 and BANK-0 from CANDIDATE to ACCEPTED,
  so that BANK-1 and the ITEM-USE implementation packets of ARCH-BATCH-ROOT-PACKETS-V1 can be
  allocated.
- Accepts: `reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md`
  (`ITEM-USE0-FOOD-AND-POTIONS-V1`) and
  `reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md`
  (`BANK0-ACCOUNT-WORLD-BANK-BALANCE-V1`, with owner answer Q1 = b).
- Runtime, migration and production authority: NONE. Each child still needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

When this decision merges, the two decisions are ACCEPTED, and so are the amendments they wrote as
"pending on acceptance" of themselves:

| Decision | Amendment now effective |
|---|---|
| ITEM-USE-0 | USE-WIRE-V1: capability 15 `ITEM_USE_V1` (leased, ARCH-BATCH-ROOT-PACKETS-V1 §0.1), fields 4 and 5, the four dispositions (§3) |
| ITEM-USE-0 | DUR-03 §15, §39.1 and §39.3: the item use amendment and the `ItemUseCause` shapes (§4) |
| BANK-0 | DUR-03 §18 and §39.1: the bank amendment after §18 (§5, §7.1) |
| BANK-0 | the composition decision: the BANK-0 amendment paragraph before its §6 (§7.2) |
| BANK-0 | the multichannel scope matrix bank row (§7.2) |

Amendments that wait for another decision stay pending: in ITEM-USE-0 those of BAGS-0, RUNE-USE-0
and TIMED-ITEM-0; in the NPC service boundary that of BANK-FEE-0. The amended documents keep their
"pending on acceptance" wording; the condition is met by this merge, and no contract text changes.

## 2. Independent review

Each lens was reviewed by a separate non-authoring, read-only agent against `main@44f79d7f`.

**ITEM-USE-0 (protocol, persistence, combat).**
- Protocol P1: §3 left the capability number to "allocation", while the core client batch already
  depends on 15. Fixed: §3 pins capability 15, the control plane's lease (capability ids and state
  domain ids are separate registries; domain 15 `ACTOR_ANALYSER` does not collide).
- P2: the RUNE-USE-0 amendment in §4.2 cited ITEM-USE-0 itself. Fixed: it cites RUNE-USE-0 §5.
- Protocol P1 (#1750 4177200352): field 4 named `{actor_id, generation}` rather than the D85 wire
  identity. Fixed:
  - §3 pins `use_with`'s `creature` to the existing `EntityRefV1`: 16 identity bytes and a nonzero
    generation, otherwise `REJECTED`; a stale or unseen reference is `NO_TARGET`;
  - the ITEM-USE-WIRE-1 packet (ARCH-BATCH-ROOT-PACKETS-V1 §2.2) imports it and tests both
    constraints.
- Verified with no finding: fields 4 and 5 and the 529-byte bound, the `ItemUseCause` shapes, item
  only transactions with no CharacterRevision advance, idempotency by CommandRef, one use in flight,
  the GAME-ABILITY-01 pipeline, typed cooldown keys and the food regeneration condition.

**BANK-0 (persistence, economy).**
- No P1.
- The P2 notes are not changed: ARCH-BATCH-ROOT-PACKETS-V1, which records Q1 = b, is on `main`
  (#1733); the §7.2 amendment is already written as pending on acceptance of BANK-0, which this
  decision meets.
- P3 kept as declared: the withdrawal cap of three stacks stays `PARITY_PENDING`.
- Verified with no finding: lease 0071 for BANK-1, the lock order of §4.1, replay by occurrence and
  binding, Account + World scope distinct from ChannelId, conservation (only conversion and
  transfer lines, no mint or burn), the bounds of §8 and the typed refusals of §4.3.

The external review of the frozen head is the control plane's to trigger.

## 3. Unblocked

BANK-RET-0, BANK-1 and GOLD-FEE-2's bank dependency (ARCH-BATCH-ROOT-PACKETS-V1); ITEM-USE-WIRE-1,
ITEM-USE-1 and ITEM-USE-CLIENT-1. Each still needs its own #162 allocation.
