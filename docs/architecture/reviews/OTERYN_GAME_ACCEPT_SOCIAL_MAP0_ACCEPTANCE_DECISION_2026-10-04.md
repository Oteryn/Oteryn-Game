# ACCEPT-SOCIAL-MAP-0 Acceptance of the party, guild, house and map runtime decisions

- Decision: `ACCEPT-SOCIAL-MAP0-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  reviews of §2 on the frozen head and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D551 (owner answer 1a, #162), which supersedes D549(b) and carries out
  D545. Take the five social and house decisions and ADR-0021 from CANDIDATE to ACCEPTED in one
  decision. Their implementation packets (PARTY-1, GUILD-1, HOUSE-1, MAP-OVERLAY-1 and
  MAP-CUTOVER-1) can then be written against accepted bases.
- Accepts:
  - `reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md`
    (`PARTYPVP0-PARTIES-AND-PVP-V1`);
  - `reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`
    (`GUILD0-GUILDS-AND-GUILDHALLS-V1`);
  - `reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
    (`HOUSE-OWN0-PHYSICAL-HOUSE-OWNERSHIP-V1`);
  - `reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md`
    (`HOUSE-CUSTODY0-HOUSE-ITEM-CUSTODY-V1`), with its ADR-0021 §4.4 amendment;
  - `HOUSE-RUNTIME0-HOUSE-INTERIOR-RUNTIME-V1`, in
    `reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md`;
  - `ADR-0021-world-map-runtime-loading.md` (owner decisions D188-D196). The reviewed parts are
    §4.4, §4.7 and §5, and the DUR-03 "Map items and world reset" amendment to §39.1 and §39.3.
- Out of scope: BED-0 and ECON-RET-0 stay CANDIDATE (D545).
- Runtime, migration, wire and production authority: NONE. Each child still needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

When this decision merges, the six decisions are ACCEPTED. So are the amendments they wrote as
"pending on acceptance" of themselves:

| Decision | Amendment now effective |
|---|---|
| PARTY-PVP-0 | ATTACK-0 §3 and §4 (player targets, blocks) |
| PARTY-PVP-0 | first player death decision §4.1 and §4.5 (PvP death, black skull respawn) |
| PARTY-PVP-0 | DEATH-0 §3.1 (PvP receipt fields) |
| PARTY-PVP-0 | D3 §4.4 and §5.3 (party loot window) |
| PARTY-PVP-0 | composition decision rule 1, and the multichannel scope matrix party and PvP rows |
| GUILD-0 | BANK-0 §3 (`GUILD_DEPOSIT`, `GUILD_WITHDRAW`, the disband credits, guild operations) |
| GUILD-0 | CHAT-0 §5 (guild-room line payload: `GuildId` and rank level) |
| GUILD-0 | EXP-HOUSES-01 §9, HOUSE-OWN-0 §3, §9, §10 and §12, and the house catalogue `kind` |
| HOUSE-OWN-0 | BANK-0 §3 (house ledger kinds), BANK-FEE-0 (bank-only price and rent) |
| HOUSE-OWN-0 | DUR-03 §39.3 and the gold fee decision §4.4 (house `FeeBurnCause` variants, D238) |
| HOUSE-OWN-0 | EXP-HOUSES-01 §10 and §26, HOUSE-CUSTODY-0 §4 |
| HOUSE-CUSTODY-0 | DUR-03 §5.2 (`HouseInterior` family), ADR-0021 §4.4 (owned and unowned house tiles) |
| HOUSE-RUNTIME-0 | ADR-0001 §11, HOUSE-CUSTODY-0 §3.4 and §4, CHAR-POSITION-0 §3.2 and §3.3, HOUSE-OWN-0 §11 |
| ADR-0021 | DUR-03 §39.1 and §39.3: `MapItemMaterialization` MINT and `WorldReset` DECAY_RETIRE |
| ADR-0021 | `RESOURCE_LIMITS_REGISTRY.json`: the four MAP01 budget rows |

Some amendments wait for another decision and stay pending:
- ADR-0021 §4.5 and §4.6 (palette keys, zero-destination teleports) wait for ITEM-MOVE-WIRE-0;
- the HOUSE-RUNTIME-0 amendments to ADMIT-0 and BED-0 wait for those decisions;
- MAP-WIRE-1 needs owner acceptance of its own contract candidate.

An amendment whose target is itself still CANDIDATE takes effect in that target when the target is
accepted. Those targets are ATTACK-0, CHAT-0, MARKET-0, MAIL-0, DEATH-0, D3, CHAR-POSITION-0,
BANK-FEE-0 and the first player death decision. This decision does not accept them. The amended
documents keep their "pending on acceptance" wording. This merge meets the condition, and no
contract text changes.

## 2. Independent review

Each lens was reviewed by a separate non-authoring, read-only agent against `main@c938306d`. No
lens found a P1.

**PARTY-PVP-0 (protocol, persistence, combat).**
- No P1 and no P2.
- Verified with no finding:
  - the reciprocal amendments in the five amended decisions;
  - the 32 limits `PARTYPVP0-RL-01` to `-32`, unique;
  - wire capabilities and domains left to allocation;
  - owner answers P1 and P2;
  - the staging of CHAT-2, COND-1 and GUILD-WAR-0;
  - party and PvP rows as World and consequence state that never advances CharacterRevision.

**GUILD-0 (persistence, economy, protocol).**
- No P1.
- P2 "BANK-0 §3 and CHAT-0 §5 lack the amendment text" does not reproduce. Both blocks are written
  as pending on acceptance of GUILD-0: BANK-0 after the house kinds, and CHAT-0 §5 with the 17-byte
  destination. The `GUILDHALL_*` kinds belong to `game_guild_bank_entries` (GUILD-0 §5.1), not to
  the Account ledger, so BANK-0 rightly lists only `GUILD_DEPOSIT`, `GUILD_WITHDRAW` and the disband
  credits. No change.
- P3 kept: GUILD-0 builds on MARKET-0 and CHAT-0. A material change to either reopens this lens.
- Verified with no finding:
  - owner answers G1 a and G2 a;
  - `GUILD0-RL-01` to `-20` and the lock order `GUILD0-LO-01`;
  - the guildhall reserve headroom under `HOUSEOWN0-RL-13`.

**HOUSE-OWN-0 (persistence, economy, security).**
- No finding.
- Verified:
  - `HOUSEOWN0-RL-01` to `-14`, with RL-13 equal to `MARKET0-RL-10`;
  - the bid, price and rent ledger kinds;
  - owner answers H1 and H2a;
  - the cross-references into GUILD-0, MAIL-0, BED-0, HOUSE-RUNTIME-0, BANK-0 and DUR-03 §39.3;
  - the interior runtime gate of HOUSE-CUSTODY-0 §4 before auctions open.

**HOUSE-CUSTODY-0 (persistence).**
- No P1 and no P2.
- P3: "DUR-03 §39.3 should say WorldReset never touches HouseInterior". It already does: the
  amendment retires Ground roots and entries only and never `HouseInterior`. No change.
- P3: the lock scope of the reset step-4 recheck (table or row, same-house moves) belongs to the
  MAP-OVERLAY-1 packet, which must fix it.
- Verified with no finding:
  - the single-location invariant;
  - `HouseInterior` as a World-scoped typed location family;
  - the §3.5 closure and the deferral of container contents;
  - the reset preflight and recheck;
  - migration 0025 reserved for HOUSE-CUSTODY-1.

**HOUSE-RUNTIME-0 (security, persistence, protocol).**
- No P1 and no P2.
- `HOUSERT0-RL-03` stays `PARITY_PENDING`.
- Verified with no finding:
  - the two-phase entry handoff, with the actor frozen and a combat lock check before commit;
  - revalidation under FOR SHARE;
  - the session fence;
  - the durable handoff row as commit point, and the exit position writes;
  - house items as durable entities, never `MAP_TILES`;
  - HOUSE-ITEM-WIRE-1 declared as a gap.

**ADR-0021 §4.4, §4.7, §5 and the DUR-03 amendment (persistence).**
- No P1, no P2 and no P3.
- Verified:
  - a per-channel generation fences in-flight commits, and crash recovery resumes idempotently;
  - the `MapItemMaterialization` cause binds world, channel, base bundle digest, placement key and
    reset epoch;
  - `WorldReset` and `CorpseDecay` share one per-item retirement uniqueness, with a cause
    discriminator CHECK;
  - conservation: one MINT per eligible entry and one retirement per transaction;
  - only Ground is retired, with the `HouseInterior` preflight and in-transaction recheck;
  - durable Ground items are never refused for overlay budget;
  - no new fence kind.
- §4.4 and §4.7 do not depend on the pending ITEM-MOVE-WIRE-0 amendments.

The external review of the frozen head is the control plane's to trigger.

## 3. Unblocked

Each child still needs its own #162 allocation, and any other accepted base that its packet
names:
- PARTY-PVP-0: PARTY-1, PARTY-XP-1, PARTY-CHAT-1, PVP-1, PVP-RT-1, PVP-DEATH-1, PVP-BLOCK-1 and
  PVP-WIRE-1. The PARTY-1 server packet follows CHAT-2.
- GUILD-0: GUILD-1, GUILD-RET-0, GUILD-BANK-1, GUILDHALL-1, GUILD-WIRE-1 and GUILD-CHAT-1. GUILD-1
  follows BANK-1 and INBOX-1.
- HOUSE-OWN-0: HOUSE-1, which follows BANK-1 and INBOX-1; HOUSE-ACL-1 and HOUSE-WIRE-1.
- HOUSE-CUSTODY-0: HOUSE-CUSTODY-1.
- HOUSE-RUNTIME-0: SCOPE-HANDOFF-1, HOUSE-RUNTIME-1 and HOUSE-VIEW-1, then HOUSE-ITEM-WIRE-1 after
  its own wire decision.
- ADR-0021: MAP-OVERLAY-1, then MAP-CUTOVER-1, written as one packet, MAP-OVERLAY-CUTOVER-PACKET-1.
  MAP-LOAD-1 is allocated after #1759 merges.
