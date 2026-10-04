# Architect batch: Premium gameplay wiring and the first house, guild and party packets

- Batch: `ARCH-CORE-LOOP-PACKETS-2` part C (D486 items 4 and 5)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, order and packets below. They
  implement accepted semantics only: PREMIUM-ACTIVATION §4.1 and §4.6, PREMIUM-DELIVERY-0 §5,
  HOUSE-OWN-0, HOUSE-RUNTIME-0 §4, BED-0 §3, GUILD-0 §4.4, PARTY-PVP-0, BANK-0, MARKET-0 and CHAT-0
  §5.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D486 item 4 (a Premium gameplay wiring packet) and item 5 (the first
  HOUSE, GUILD and PARTY-PVP children that can be allocated now, with what each needs for
  acceptance).
- Amends: nothing.
- Runtime, migration, wire and production authority: NONE. Each packet needs its #162
  allocation. A packet that needs a migration leases its number from the control plane at
  allocation. This batch reserves no number; the next free one at this writing is 0073.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Findings on `main` (8a005cf6)

1. **No gameplay path reads Premium.** PREM-1a, PREM-1b and PREM-1c-harden are merged.
   `PremiumRefresher` admits and releases each account with its sessions
   (`gameplay_transport/mod.rs`, `admit_premium` and `release_premium`). But
   `PremiumConsumer::premium_current(account_id, now)` and `premium_entitlement_ended` have no
   gameplay caller.
2. **No trusted time source exists.** `TrustedNow::new(now_us, uncertainty_us)` returns `None`
   when the uncertainty exceeds `MAX_CLOCK_SKEW_US` (5 s). Outside `premium/`, nothing in the game
   server constructs a `TrustedNow`. PREMIUM-DELIVERY-0 §5 requires an NTP-synchronized node clock
   with uncertainty ≤ 5 s, and an unsynchronized node reads Premium as not current.
3. **The spell check is wired to false.** `spell/cast.rs` builds
   `CasterState { premium: false, .. }` with the note "SPELL-D5: Premium activation is not
   authorized". `spell/mod.rs` refuses `spell.premium && !caster.premium` as `PremiumRequired`,
   which is shown as `NotAvailable`. The V1 spell book (`exura`, `exura gran`, `exana pox`) has no
   Premium spell, so today the check only fails closed for spells added later.
4. **The yell gate waits on the same fact.** `chat/spam.rs` lets a character yell below level 20
   only with `premium_current`, which is false today. CHAT-1b-2a holds `chat/**` now.
5. **Promotion has no source.** PREM-2a's pure rules are merged (`domain/premium.rs`:
   `promotion_benefit_current`, `soul_maximum`, `apply_soul_gain`). No character can become
   promoted until the NPC service PREM-5 exists, so promotion state alone changes nothing a player
   sees.
6. **House roots are partly built.** HOUSE-CUSTODY-1 is merged (migration `0025`), and so are
   COND-1a (`ability/condition.rs`), DEATH-1, GOLD-FEE-1b and spell P3b-2 vitals.
7. **Two retention profiles block the economy roots.** BANK-1 needs BANK-RET-0 and GUILD-1 needs
   GUILD-RET-0. MARKET-1 also needs MARKET-RET-0. None of the three is decided, and the game event
   foundation registry has no economy or guild profile. So BANK-1 and GUILD-1 cannot start, and
   neither can everything after them: HOUSE-1, GUILD-BANK-1, GUILDHALL-1 and MARKET-1.
8. **House ownership has a long chain.** HOUSE-1 needs BANK-1 and INBOX-1. INBOX-1 needs DEPOT-1,
   which waits on DEPOT-WIRE-1 (playable-first), which in turn waits on MAP-LOAD-1 and MAP-WIRE-1.
   A house with no owner admits nobody but an operator test (HOUSE-RUNTIME-0 §5). So no house
   interior is playable before HOUSE-1.
9. **Chat relay is the party root.** PARTY-1 and GUILD-CHAT-1 need CHAT-2, the World relay
   (CHAT-0 §5). CHAT-2 writes `chat/**`, which CHAT-1b-2a and then CHAT-1b-2b hold. It needs a
   relay key per World as an environment secret.
10. **The house exit waits on ADMIT-0.** HOUSE-RUNTIME-0 §4 refuses the transfer back into a
    channel scope until ADMIT-0 §3.2's lifting conditions hold. ADMIT-0 is CANDIDATE.

## 1. Rulings

### 1.1 The trusted clock (PREMIUM-DELIVERY-0 §5)

- **Source.** The time source is the kernel's NTP discipline state, read with `ntp_adjtime`
  (modes 0, read only) through the `libc` crate. `libc` 0.2 is already in `Cargo.lock`, so this
  adds no new crate. It is the upstream interface that `chronyd`, `ntpd` and `systemd-timesyncd`
  all keep current. Oteryn writes no time daemon and no NTP client.
- **Reading.** One call returns the realtime clock and its error bound together:
  - `now_us` is the returned `time` (microseconds, or nanoseconds divided down under `STA_NANO`);
  - `uncertainty_us` is `maxerror`;
  - the result is `TrustedNow::new(now_us, uncertainty_us)`, so the existing 5 s bound applies
    unchanged.
- **Fail closed.** These all read as `None`, which means Premium is not current:
  - the return state is `TIME_ERROR`;
  - `STA_UNSYNC` is set;
  - the call fails;
  - the target is not Linux (a `cfg` stub returns `None`).
- **Seam.** A `TrustedClock` trait has one method, `now() -> Option<TrustedNow>`. The system
  implementation is the reading above. Tests use a fixed implementation. The gameplay runtime
  takes a clock at construction, and production composition passes the system one.
- **Not cached.** The clock is read once per gameplay command that needs Premium, never stored.
  An unsynchronized node therefore stops granting Premium at its next command.

### 1.2 The Premium read seam (PREMIUM-ACTIVATION §4.1)

- The runtime exposes `premium_current(account_id) -> bool`. It calls the refresher's
  `PremiumConsumer::premium_current(account_id, clock.now())`. This is the single gameplay read;
  no consumer calls `PremiumConsumer` directly.
- The account is the admitted controller's `account_id`, which the connection already holds. It
  is passed down with the command, never looked up from the actor.
- Login always succeeds (§4.1). Admission reads no Premium.
- When no Platform entitlement source is configured, every account reads Free. This is the
  current production state, and the packet does not change it.

### 1.3 Who wires each Premium consumer

| Consumer | Rule | Owner | When |
|---|---|---|---|
| Spell cast check | PREMIUM-ACTIVATION §4.6 (PREM-4) | PREM-WIRE-1 | now (§2.1) |
| Yell below level 20 | CHAT-0 §6 | the later of CHAT-1b-2b and PREM-WIRE-1 | if CHAT-1b-2b is later, it calls the §1.2 seam; if PREM-WIRE-1 is later, it touches only the one `chat/spam.rs` call site, after CHAT-1b-2b merges and under a control plane lease |
| Promotion, soul maximum, death input | §4.2, §4.3 (PREM-2) | PREM-2b, together with PREM-5 | after NPC-TALK-1 (§3); before that nothing can be promoted (§0.5) |
| Premium areas | §4.5 (PREM-3) | PREM-3 | once the map bundle carries the area flag (§3) |
| Premium blessings and NPC services | §4.4 (PREM-5) | PREM-5 | after NPC-TALK-1 (§3) |
| Training statue | OFFLINE-0 §6 | STATUE-1 | with OFFLINE-1 (§3) |

Every consumer calls the §1.2 seam once per command and stores no result.

### 1.4 Order of the social roots

These chains run in parallel, each with one writer:

1. **Retention:** ECON-RET-0 (§2.2), then BANK-1 and GUILD-1 (§2.6).
2. **Party and PvP:** CHAT-1b-2b, then CHAT-2 (§2.5), then PARTY-1 (§2.7), then PVP-1.
3. **House entry:** SCOPE-HANDOFF-1 (§2.4), then HOUSE-RUNTIME-1 (after HOUSE-1 for owned houses).
4. **Beds:** BED-CONTENT-1 (§2.3), in the content lane.
5. **House ownership:** the map packets, then DEPOT-WIRE-1, DEPOT-1, INBOX-1, BANK-1, and then
   HOUSE-1.

Ruling (playable-first): SCOPE-HANDOFF-1 is allocated ahead of its playable caller. It is the
longest hard step in chain 3, and HOUSE-RUNTIME-0 builds it so that the later Channel change can
reuse it. It is accepted infrastructure of an accepted decision, not speculative. All the other
packets here are either on the playable path or a precondition of it.

### 1.5 Acceptance needs

Every packet needs exact-head validation, the independent review listed, and protected
integration through `game-gate` and Merge Queue. A packet that needs anything more lists it in
its `acceptance` field (§2). An item in that field that is not met keeps the packet CANDIDATE
after merge. It never lowers a review.

## 2. Packets

### 2.1 PREM-WIRE-1 (PREM-4 with the clock and the read seam)

```yaml
task_id: OTV2-20261004-prem-wire-1
decision: this batch §1.1-§1.3; PREMIUM-ACTIVATION §4.1, §4.6; PREMIUM-DELIVERY-0 §5
worker: oteryn-hard-worker
review: security review (time and entitlement) and spell review (Codex, final frozen head)
branch: claude/prem-wire-1-20261004
base: main
depends_on: [PREM-1c-harden]
migration_lease: none
owned_paths:
  - apps/game-server/src/premium/clock.rs
  - apps/game-server/src/premium/mod.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/archive/OTV2-20261004-prem-wire-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
acceptance: none beyond §1.5
```

Builds:

- `TrustedClock` with the system and fixed implementations (§1.1).
- The runtime seam `premium_current(account_id)` (§1.2).
- `cast_spell` takes the admitted account. `CasterState.premium` is the seam's value, read once
  at cast time, and the SPELL-D5 note is removed.

Touch rules:

- The connection and test fakes change only by the added account argument.
- The packet touches no `chat/**`, protocol, registry, migration or content file.
- If `gameplay_transport/mod.rs` is leased to another lane at allocation, the control plane
  serializes the two.

Acceptance tests:

- A Premium fixture spell is refused for a Free account (`NotAvailable`).
- The same spell is cast when the consumer reads current under a fixed synced clock.
- It is refused when the clock reads `None`.
- An uncertainty of 5,000,000 µs is current; 5,000,001 µs is not.
- `STA_UNSYNC` and `TIME_ERROR` map to `None`; this is a unit test of the decoding function over
  a `timex` value, so no kernel state is needed.
- The non-Linux stub returns `None`.
- The V1 book's three spells cast for a Free account exactly as before.

### 2.2 ECON-RET-0 (BANK-RET-0, GUILD-RET-0 and MARKET-RET-0)

```yaml
task_id: OTV2-20261004-econ-ret-0
decision: BANK-0 (BANK-RET-0), GUILD-0 §4.4 (GUILD-RET-0), MARKET-0 (MARKET-RET-0)
mode: CONTRACT
worker: architect or oteryn-impl-worker under the control plane's routing (these children are "control plane routes")
review: privacy review (Codex, final frozen head)
branch: claude/econ-ret-0-20261004
base: main
depends_on: []
migration_lease: none
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ECON_RET0_ECONOMY_AND_GUILD_EVENT_RETENTION_DECISION_2026-10-04.md
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (the new retention_profiles entries only)
  - docs/agents/tasks/archive/OTV2-20261004-econ-ret-0.md
validation:
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - python -m unittest discover -s tools/agents/tests
  - git diff --check
acceptance: owner answer for any duration above the 90-day ordinary ceiling
```

Builds three immutable profiles with every `required_profile_fields` entry of the registry:

- the bank event, purpose `ECONOMY_LEDGER`;
- the market event, purpose `ECONOMY_LEDGER`;
- the guild event, with a ceiling of at least the 30-day member log (`GUILD0-RL-10`).

The profiles follow the Character and DUR-03 retention decisions:

- a finite ceiling;
- the legal hold as an explicit exception;
- no in-place change after first admission.

A duration above 90 days, or a purpose beyond proof, reconciliation and bounded support and
security investigation, is an owner question returned as `QUESTION`. The packet writes no event
type, code or schema. BANK-1, GUILD-1 and MARKET-1 bind these ids when they register their
events.

### 2.3 BED-CONTENT-1

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
validation: the content lane's item and bundle checks and the governance checks of §2.2
acceptance: none beyond §1.5
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

### 2.4 SCOPE-HANDOFF-1

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

The admission commit re-reads the property row under FOR SHARE. The packet admits nobody into a
real house: with no HOUSE-1 there is no owner, so §5's operator test is the only entry, and
tests use it.

Acceptance tests, each with a crash point:

- a crash after prepare and before commit recovers the character in its source session;
- a crash after commit admits it once, with a fresh `GameSessionId`;
- a concurrent revocation either refuses the admission or finds the character inside.

### 2.5 CHAT-2 (World relay)

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

### 2.6 GUILD-1

```yaml
task_id: OTV2-20261004-guild-1
decision: GUILD-0 §3, §4 (owner answers G1a, G2a)
worker: oteryn-hard-worker
review: hard, persistence and security review (Codex, final frozen head)
branch: claude/guild-1-20261004
base: main after ECON-RET-0 merges
depends_on: [ECON-RET-0]
migration_lease: one number from the control plane at allocation
owned_paths: the guild module, its migration and tests, the guild event registration, and
  docs/agents/tasks/archive/OTV2-20261004-guild-1.md
acceptance: none beyond §1.5
```

Builds GUILD-0 §3 and §4:

- the guild, rank, member, invitation and account-leadership tables;
- the found, invite, join, leave, exclude, rank, resign and disband transactions;
- the formation and vice World jobs;
- the guild event, bound to ECON-RET-0's guild profile.

The packet has no wire. Its playable entry is GUILD-WIRE-1, which waits on GUILD-BANK-1 (§3).
GUILD-1 can still start now, because GUILD-0 says it does not wait for houses.

### 2.7 PARTY-1

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
acceptance: none beyond §1.5
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
| PREM-2b (promotion state, soul maximum, death input) | a promotion source | PREM-5, after NPC-TALK-1; packeted together with it |
| PREM-3 (Premium areas) | the area flag in the map bundle; Movement entry refusal | the map packets (another agent prepares maps) |
| PREM-5 (blessings, NPC services) | NPC-TALK-1 | NPC lane |
| BANK-1 | ECON-RET-0 | §2.2 |
| INBOX-1 | DEPOT-1, which waits on DEPOT-WIRE-1, MAP-LOAD-1 and MAP-WIRE-1 | the map packets |
| HOUSE-1, HOUSE-ACL-1, HOUSE-WIRE-1 | BANK-1; INBOX-1; SCOPE-HANDOFF-1 for the runtime child | chains 1, 3 and 5 of §1.4 |
| HOUSE-RUNTIME-1, HOUSE-VIEW-1, HOUSE-ITEM-WIRE-1 | SCOPE-HANDOFF-1; HOUSE-1; the MAP-WIRE-1 children; a wire decision for the last | §2.4, then HOUSE-1 |
| BED-1, BED-REGEN-1 | OFFLINE-1, STATUE-1, HOUSE-RUNTIME-1, HOUSE-VIEW-1, WORLDINT-WIRE-1; DUR-02 | their roots |
| GUILD-BANK-1, GUILDHALL-1, GUILD-WIRE-1 | BANK-1; HOUSE-1, HOUSE-ACL-1; HOUSE-WIRE-1 | chains 1 and 5 |
| GUILD-CHAT-1, PARTY-CHAT-1 | CHAT-2 (and GUILD-1 or PARTY-1) | §2.5 |
| PVP-1 | PARTY-1 | §2.7 |
| PARTY-XP-1 | PARTY-1; the D118 XP lane; D3-4 | §2.7 and the XP lane |
| PVP-RT-1, PVP-BLOCK-1, PVP-DEATH-1, PVP-WIRE-1 | ATTACK-1, COND-1, PVP-1; SPEED-1; DEATH-3; VIS-2, ATTACK-WIRE-1 | their roots |

## 4. Rejected options

- **Reading `SystemTime` with no error bound.** This fails PREMIUM-DELIVERY-0 §5: an
  unsynchronized node would grant Premium on a wrong clock.
- **An in-process NTP client or querying chrony over its socket.** Both add a protocol and a
  dependency when the kernel already exposes the disciplined error bound. A fork of a time crate
  is rejected for the same reason.
- **Caching `premium_current` in the session.** A cache would outlive an entitlement end or a
  loss of sync. The refresher already holds the entitlement, so reading per command is cheap.
- **Building PREM-2b now.** Promotion state with no way to promote is infrastructure no player
  reaches (§0.5).
- **One retention decision per child.** Three profiles of one shape go through one privacy review
  instead of three (AGENTS.md batching).
- **Starting HOUSE-1 against a stub bank or inbox.** That would build durable value on a
  placeholder custody path.

## 5. Decision queue for the control plane

1. **Route ECON-RET-0 first.** It releases BANK-1 and GUILD-1, and through them every house,
   guildhall and market child.
2. **ADMIT-0 acceptance.** SCOPE-HANDOFF-1's exit, and every later scope transfer, wait on it.
   Recommendation: send ADMIT-0 to its protocol and security review with the FND-04 owner.
3. **PREM-5 together with PREM-2b.** Packet them after NPC-TALK-1 is allocated.
4. **The relay key.** Before CHAT-2 can be qualified on a protected environment, provisioning a
   per-World relay secret needs separate authority from the owner.

## 6. Decision test

| Question | Answer |
|---|---|
| Does a Free account's play change? | No |
| Can an unsynchronized node grant Premium? | No (§1.1) |
| Does any packet here touch `chat/**` while CHAT-1b-2 holds it? | No (§1.3, §2.5) |
| Does any packet need a number not leased by the control plane? | No |
| Is any packet infrastructure no player reaches? | Only SCOPE-HANDOFF-1, by ruling §1.4 |
| Is any production or secret mutation authorized? | No |
