# Architect batch: the first house, guild and party packets

- Batch: `ARCH-CORE-LOOP-PACKETS-2` part C (D486 items 4 and 5)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, order and packets below. They
  implement accepted semantics only: HOUSE-OWN-0, HOUSE-RUNTIME-0 §4, BED-0 §3, GUILD-0 §4.4, PARTY-PVP-0, BANK-0, MARKET-0 and CHAT-0
  §5.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D486 item 5 (the first HOUSE, GUILD and PARTY-PVP children that can
  be allocated now, with what each needs for acceptance). D486 item 4 (Premium gameplay wiring)
  and GUILD-1, which is Premium-gated, moved to PREMIUM-ACTIVATION-0 (#1743,
  `OTERYN_GAME_PREMIUM_ACTIVATION0_GAMEPLAY_SWITCH_OVER_DECISION_2026-10-04.md`) by control plane
  D490.
- Amends: nothing.
- Runtime, migration, wire and production authority: NONE. Each packet needs its #162
  allocation. A packet that needs a migration leases its number from the control plane at
  allocation. This batch reserves no number; the next free one at this writing is 0073.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Findings on `main` (8a005cf6)

1. **House roots are partly built.** HOUSE-CUSTODY-1 is merged (migration `0025`), and so are
   COND-1a (`ability/condition.rs`), DEATH-1, GOLD-FEE-1b and spell P3b-2 vitals.
2. **Two retention profiles block the economy roots.** BANK-1 needs BANK-RET-0 and GUILD-1 needs
   GUILD-RET-0. MARKET-1 also needs MARKET-RET-0. None of the three is decided, and the game event
   foundation registry has no economy or guild profile. So BANK-1 and GUILD-1 cannot start, and
   neither can everything after them: HOUSE-1, GUILD-BANK-1, GUILDHALL-1 and MARKET-1.
   BANK-RET-0 is packeted in #1733 §2.3, so this batch packets only GUILD-RET-0 and
   MARKET-RET-0 (§2.1).
3. **House ownership has a long chain.** HOUSE-1 needs BANK-1 and INBOX-1. INBOX-1 needs DEPOT-1,
   which waits on DEPOT-WIRE-1 (playable-first), which in turn waits on MAP-LOAD-1 and MAP-WIRE-1.
   A house with no owner admits nobody but an operator test (HOUSE-RUNTIME-0 §5). So no house
   interior is playable before HOUSE-1.
4. **Chat relay is the party root.** PARTY-1 and GUILD-CHAT-1 need CHAT-2, the World relay
   (CHAT-0 §5). CHAT-2 writes `chat/**`, which CHAT-1b-2a and then CHAT-1b-2b hold. It needs a
   relay key per World as an environment secret.
5. **The house exit waits on ADMIT-0.** HOUSE-RUNTIME-0 §4 refuses the transfer back into a
   channel scope until ADMIT-0 §3.2's lifting conditions hold. ADMIT-0 is CANDIDATE.

## 1. Rulings

### 1.1 Order of the social roots

These chains run in parallel, each with one writer:

1. **Retention:** BANK-RET-0 (#1733 §2.3), then BANK-1. In parallel, ECON-RET-0 (§2.1), then
   GUILD-1 (#1743 §2.2, which also needs PREM-WIRE-1) and later MARKET-1.
2. **Party and PvP:** CHAT-1b-2b, then CHAT-2 (§2.4), then PARTY-1 (§2.5), then PVP-1.
3. **House entry:** SCOPE-HANDOFF-1 (§2.3), then HOUSE-RUNTIME-1, tested on HOUSE-RUNTIME-0
   §10's operator-owned test house. It does not wait on HOUSE-1 (#1738 P1 4177048585).
4. **Beds:** BED-CONTENT-1 (§2.2), in the content lane.
5. **House ownership:** the map packets, then DEPOT-WIRE-1, DEPOT-1, INBOX-1, BANK-1, and then
   HOUSE-1, which also waits on HOUSE-RUNTIME-1 from chain 3 (HOUSE-OWN-0 brief; HOUSE-CUSTODY-0
   §4). Owned-house admission, through HOUSE-1's owner rows and HOUSE-ACL-1, is tested when those
   children land, not before.

Ruling (playable-first): SCOPE-HANDOFF-1 is allocated ahead of its playable caller. It is the
longest hard step in chain 3, and HOUSE-RUNTIME-0 builds it so that the later Channel change can
reuse it. It is accepted infrastructure of an accepted decision, not speculative. All the other
packets here are either on the playable path or a precondition of it.

### 1.2 Acceptance needs

Every packet needs exact-head validation, the independent review listed, and protected
integration through `game-gate` and Merge Queue. A packet that needs anything more lists it in
its `acceptance` field (§2). An item in that field that is not met keeps the packet CANDIDATE
after merge. It never lowers a review.

## 2. Packets

### 2.1 ECON-RET-0 (GUILD-RET-0 and MARKET-RET-0)

```yaml
task_id: OTV2-20261004-econ-ret-0
decision: GUILD-0 §4.4 (GUILD-RET-0), MARKET-0 (MARKET-RET-0); BANK-RET-0 is #1733 §2.3, not this packet
mode: CONTRACT
worker: architect or oteryn-impl-worker under the control plane's routing (these children are "control plane routes")
review: privacy review (Codex, final frozen head)
branch: claude/econ-ret-0-20261004
base: main
depends_on: []
migration_lease: none
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ECON_RET0_GUILD_AND_MARKET_EVENT_RETENTION_DECISION_2026-10-04.md
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (the new retention_profiles entries only)
  - docs/agents/tasks/archive/OTV2-20261004-econ-ret-0.md
validation:
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - python -m unittest discover -s tools/agents/tests
  - git diff --check
acceptance: owner answer for any duration above the 90-day ordinary ceiling
```

Builds two immutable profiles with every `required_profile_fields` entry of the registry:

- the market event, purpose `ECONOMY_LEDGER`;
- the guild event, with a ceiling of at least the 30-day member log (`GUILD0-RL-10`).

The profiles follow the Character and DUR-03 retention decisions:

- a finite ceiling;
- the legal hold as an explicit exception;
- no in-place change after first admission.

A duration above 90 days, or a purpose beyond proof, reconciliation and bounded support and
security investigation, is an owner question returned as `QUESTION`. The packet writes no event
type, code or schema. GUILD-1 and MARKET-1 bind these ids when they register their events. The
bank profile is BANK-RET-0 (#1733 §2.3), and this packet does not touch it. If both land in the
registry together, the second rebases its `retention_profiles` entries onto the first.

### 2.2 BED-CONTENT-1

```yaml
task_id: OTV2-20261004-bed-content-1
decision: BED-0 §3
worker: oteryn-impl-worker (content lane)
review: content review (Codex, final frozen head)
branch: claude/bed-content-1-20261004
base: main
depends_on: [ITEM-SEM-2b]
migration_lease: none
owned_paths: the Item-definition bed facts, their converter, the BED-0 §3 bed validator with
  its exception list, and docs/agents/tasks/archive/OTV2-20261004-bed-content-1.md; the control
  plane fixes the exact content and tool paths at allocation against the map lane
validation: the content lane's item and bundle checks and the governance checks of §2.1
acceptance: none beyond §1.2
```

Builds BED-0 §3:

- the `bed` fact on Item definitions: the part, the partner direction, and the free and occupied
  item types per sex. They come from Canary `items.xml` (`partnerdirection`, `transformonuse`,
  `transformto`), with TibiaWiki for names;
- the validator: one head and one foot per placed bed, each naming the other, on tiles of one
  house, and each house's valid pairs compared with the catalogue's `beds` count;
- the exception list of the 84 known-discrepancy houses (`samples/otbm-tile-check.json`).

Any other mismatch fails the bundle. If the active bundle places no beds yet, the validator's
tests use a fixture house, and the packet does not wait on the map lane. No runtime reads the
facts before BED-1.

### 2.3 SCOPE-HANDOFF-1

```yaml
task_id: OTV2-20261004-scope-handoff-1
decision: HOUSE-RUNTIME-0 §4 (with its ADMIT-0 amendment); HOUSE-CUSTODY-0 §3.4; ADR-0001 §10
worker: oteryn-hard-worker
review: hard, security and durability review, and protocol review for the exit (Codex, final frozen head)
branch: claude/scope-handoff-1-20261004
base: main
depends_on: [HOUSE-CUSTODY-1]
migration_lease: one number from the control plane at allocation
owned_paths: the scope-assignment and session schema of the house scope kind, the handoff
  module and its tests, the leased migration, and
  docs/agents/tasks/archive/OTV2-20261004-scope-handoff-1.md; the control plane fixes the
  exact paths at allocation against the live session and admission lanes
acceptance:
  - the exit into a channel scope stays refused until ADMIT-0 is accepted and its §3.2
    lifting conditions are met under this packet's protocol and security review
```

Builds, from HOUSE-RUNTIME-0 §4:

- the house scope kind in scope assignments and sessions;
- the origin routing metadata;
- the recoverable prepare/commit/abort transition;
- its typed refusals (`NO_ACCESS`, `IN_COMBAT`, `BUSY`, `HOUSE_CLOSED`, `NO_ROOM`).

The admission commit re-reads the property row under FOR SHARE. **Amendment
(`OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md` §1.2).** It reads it through
`game_house_access(world_id, house_key, character_id)`, which this packet creates as a stub
returning no row; the owner children replace its body. The packet admits nobody into a
real house: with no HOUSE-1 there is no owner, so HOUSE-RUNTIME-0 §10's operator test is the only entry, and
tests use it.

Acceptance tests, each with a crash point:

- a crash after prepare and before commit recovers the character in its source session;
- a crash after commit admits it once, with a fresh `GameSessionId`;
- a concurrent revocation either refuses the admission or finds the character inside.

### 2.4 CHAT-2 (World relay)

```yaml
task_id: OTV2-20261004-chat-2
decision: CHAT-0 §5, §6 (durable mute row)
worker: oteryn-hard-worker
review: hard, security and privacy review (Codex, final frozen head)
branch: claude/chat-2-20261004
base: main after CHAT-1b-2b merges (chat/** is held until then)
depends_on: [CHAT-1b-2b]
migration_lease: one number from the control plane at allocation (the durable mute row)
owned_paths: apps/game-server/src/chat/** and the relay module, the leased migration, and
  docs/agents/tasks/archive/OTV2-20261004-chat-2.md
acceptance:
  - the relay key per World is an environment secret. Tests generate a key; no key material is
    in the repository. Provisioning a key for any protected or production environment is
    separate authority and is not part of this packet.
  - `log_parameter_max_length = 0` on the chat connection is asserted at listener start, and a
    node that cannot assert it is not ready for chat
```

Builds everything in CHAT-0 §5 and the durable mute row of §6:

- the per-World `LISTEN`/`NOTIFY` channel;
- AES-256-GCM sealing with `key_id`, and key rotation over `CHAT0-RL-12`;
- the replay window `CHAT0-RL-13` with its de-duplication set;
- chat readiness;
- private messages with the `NOT_ONLINE` rules;
- the four World rooms;
- the fixed payload with its bounds.

Acceptance tests:

- a tampered, replayed and stale line is each dropped and counted;
- a recipient in grace reads `NOT_ONLINE`;
- a listener that is down refuses with `CHAT_UNAVAILABLE`;
- the maximum payload seals to 1,216 bytes.

### 2.5 PARTY-1

```yaml
task_id: OTV2-20261004-party-1
decision: PARTY-PVP-0 §3, §4 (owner answers P1, P2b)
worker: oteryn-hard-worker
review: hard, persistence, security and privacy review (Codex, final frozen head)
branch: claude/party-1-20261004
base: main after CHAT-2 merges
depends_on: [CHAT-2]
migration_lease: one number from the control plane at allocation
owned_paths: the party module, its migration and tests, the relay change hint, and
  docs/agents/tasks/archive/OTV2-20261004-party-1.md
acceptance: none beyond §1.2
```

Builds PARTY-PVP-0 §3 and §4:

- the party, member, invitation and consent tables;
- every party transaction it lists;
- the node `PartyView` cache, the revisioned presence record and the cleanup job.

After it merges, PVP-1 (skulls in the death transaction, `pvp_type` `OPTIONAL` for the first
World) is allocatable, since DEATH-1 is merged.

## 3. Held children and what releases them

| Child | Waits on | Released by |
|---|---|---|
| BANK-1 | BANK-RET-0 | #1733 §2.3 |
| INBOX-1 | DEPOT-1, which waits on DEPOT-WIRE-1, MAP-LOAD-1 and MAP-WIRE-1 | the map packets |
| HOUSE-1, HOUSE-ACL-1, HOUSE-WIRE-1 | BANK-1; INBOX-1; HOUSE-RUNTIME-1 | chains 1, 3 and 5 of §1.1 |
| HOUSE-RUNTIME-1 | SCOPE-HANDOFF-1 (HOUSE-CUSTODY-1 is merged); not HOUSE-1, since it is tested on HOUSE-RUNTIME-0 §10's operator-owned test house | §2.3 |
| HOUSE-VIEW-1, HOUSE-ITEM-WIRE-1 | HOUSE-RUNTIME-1; the MAP-WIRE-1 children; for the last, HOUSE-VIEW-1, ITEM-MOVE-WIRE-1 and its own wire decision | §2.3 and the map packets |
| BED-1, BED-REGEN-1 | OFFLINE-1, STATUE-1, HOUSE-RUNTIME-1, HOUSE-VIEW-1, WORLDINT-WIRE-1; DUR-02 | their roots |
| GUILD-BANK-1, GUILDHALL-1, GUILD-WIRE-1 | BANK-1; HOUSE-1, HOUSE-ACL-1; HOUSE-WIRE-1 | chains 1 and 5 |
| GUILD-CHAT-1, PARTY-CHAT-1 | CHAT-2 (and GUILD-1 or PARTY-1) | §2.4 |
| PVP-1 | PARTY-1 | §2.5 |
| PARTY-XP-1 | PARTY-1; the D118 XP lane; D3-4 | §2.5 and the XP lane |
| PVP-RT-1, PVP-BLOCK-1, PVP-DEATH-1, PVP-WIRE-1 | ATTACK-1, COND-1, PVP-1; SPEED-1; DEATH-3; VIS-2, ATTACK-WIRE-1 | their roots |

## 4. Rejected options

- **One retention decision per child.** Three profiles of one shape go through one privacy review
  instead of three (AGENTS.md batching).
- **HOUSE-RUNTIME-1 waiting on HOUSE-1.** HOUSE-1 already waits on the runtime child (HOUSE-OWN-0
  brief), so the two would hold each other forever (#1738 P1 4177048585).
- **Starting HOUSE-1 against a stub bank or inbox.** That would build durable value on a
  placeholder custody path.

## 5. Decision queue for the control plane

1. **Route BANK-RET-0 (#1733 §2.3) and ECON-RET-0 first.** BANK-RET-0 releases BANK-1.
   ECON-RET-0 releases GUILD-1 (with PREM-WIRE-1, #1743) and MARKET-1. Through them every house, guildhall and market child
   is released.
2. **ADMIT-0 acceptance.** SCOPE-HANDOFF-1's exit, and every later scope transfer, wait on it.
   Recommendation: send ADMIT-0 to its protocol and security review with the FND-04 owner.
3. **The relay key.** Before CHAT-2 can be qualified on a protected environment, provisioning a
   per-World relay secret needs separate authority from the owner.

## 6. Decision test

The mandatory answers (ARCHITECTURE_DECISION_DISCIPLINE, #1738 P2 4176947456):

1. **Must decide now?** YES. ECON-RET-0, BED-CONTENT-1, SCOPE-HANDOFF-1, CHAT-2 and PARTY-1 are
   the next allocatable work in their lanes (D486 item 5).
2. **What is blocked?** Without these packets nothing is blocked unsafely, but nothing in these
   lanes can start. The guild, party and house chains (§1.1, §3) all wait on them.
3. **What becomes harder later?** The guild and market retention profiles are immutable after
   first admission (§2.1).
4. **What would supersede it?** Any of these would reopen this batch:
   - an owner answer above the 90-day retention ceiling;
   - ADMIT-0 rejecting the scope handoff shape.
5. **What is not decided?** The following stay with their owners:
   - Premium gameplay wiring, GUILD-1 and the Premium held children (PREMIUM-ACTIVATION-0, #1743);
   - the relay key's provisioning;
   - house ownership, guildhalls and the market;
   - BANK-RET-0 (#1733);
   - every held child in §3.

| Question | Answer |
|---|---|
| Does a Free account's play change? | No |
| Does any packet here touch `chat/**` while CHAT-1b-2 holds it? | No (§2.4) |
| Does any packet need a number not leased by the control plane? | No |
| Is any packet infrastructure no player reaches? | Only SCOPE-HANDOFF-1, by ruling §1.1 |
| Is any production or secret mutation authorized? | No |
