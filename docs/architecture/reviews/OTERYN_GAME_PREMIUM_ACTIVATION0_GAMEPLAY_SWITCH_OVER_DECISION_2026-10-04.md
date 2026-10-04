# Premium activation for gameplay: the trusted clock, the read seam, PREM-WIRE-1 and GUILD-1

- Decision: `PREMIUM-ACTIVATION-0` (split out of `ARCH-CORE-LOOP-PACKETS-2` part C, #1738, by
  control plane D490)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and packets below. They
  implement accepted semantics only: PREMIUM-ACTIVATION §4.1 and §4.6, PREMIUM-DELIVERY-0 §5 and
  §10.3, PROD-ENTITLEMENTS-01 §7 and §16, GUILD-0 §3 and WHEEL-0 §6.2.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D486 item 4 (a Premium gameplay wiring packet) and the Premium-gated
  part of item 5 (GUILD-1). The rest of item 5 stays in #1738
  (`OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md`).
- Amends: nothing.
- Runtime, migration, wire and production authority: NONE. Each packet needs its #162
  allocation. A packet that needs a migration leases its number from the control plane at
  allocation. This decision reserves no number.
- Setting a production activation: NONE. It needs PREM-1's activation record and separate owner
  authority (§1.2).
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

- The runtime exposes one gate, never a bare status read (#1743 P1 4177076440):
  `with_premium_gate(account_id, |status, tx| …)`. It yields `PremiumStatus<'g>`, a closed enum of
  `NotActivated(PreDelivery<'g>)`, `Current` and `NotCurrent` (#1738 P1 4176947451), and the
  consumer's closure runs inside the gate. This is the single gameplay read, and no consumer calls
  `PremiumConsumer` directly. `premium_current(account_id)` is a gate whose closure returns
  `status == Current`; it never acts on `NotActivated`.
  - While no latch row is held, the gate opens a transaction, takes the shared lock, reads the
    table and decides the status. If the status is `NotActivated`, it runs the closure with that
    transaction as `tx` and commits only after the closure returns. For any other status it ends
    that transaction before the closure, and `tx` is `None`. While a row is held, it decides from
    memory and `tx` is `None`.
  - The gate decides the status before it runs the closure, and runs the closure once. When the
    decision needs a latch write (step 6 below), the gate first ends its shared transaction. No
    closure has run and nothing was written, so it rolls back. Only then does it latch. After the
    latch the row is held, so the closure runs with `tx = None`. A connection that holds the
    shared lock therefore never waits for the exclusive one (#1743 P1 4177096277).
  - `PreDelivery<'g>` is neither `Copy` nor `Clone` and borrows the gate, so the type system
    keeps `NotActivated` from leaving the closure. A bypass takes `&PreDelivery` as an argument:
    the Wheel exception's `cast_spell` path and GUILD-1's founding and rank writes cannot apply
    a bypass without it, and so only while the lock is held.
  - `NotActivated`: Premium has not started, so a consumer whose rule has a pre-delivery
    behaviour (GUILD-0 §3.2 and §3.3, owner answer G1 a; WHEEL-0 §6.2) applies it inside the
    closure.
  - `Current` and `NotCurrent`: Premium has started, and the rule is enforced.
- The account is the admitted controller's `account_id`, which the connection already holds. It
  is passed down with the command, never looked up from the actor.
- Login always succeeds (§4.1). Admission reads no Premium.
- When no Platform entitlement source is configured, every account reads Free. This is the
  current production state, and the packet does not change it.
- **Activation gate (PREMIUM-DELIVERY-0 §10.3, #1738 P1 4176929764).** A configured snapshot
  source alone grants nothing. The runtime takes an `Option<PremiumActivation>` at construction,
  holding the activation record's id and its switch-over instant `S`.
- **Conservative, irreversible switch-over (#1738 P1 4176973984, PROD-ENTITLEMENTS-01 §7,
  §16).** `TrustedNow` gives a window: `lower = now_us - uncertainty_us` and
  `upper = now_us + uncertainty_us`. A durable latch records that the switch-over may have
  happened. It is a new single-row table (#1743 P1 4177047049): `id` SMALLINT primary key with
  `CHECK (id = 1)`, `activation_id` TEXT, `switch_over_us` BIGINT and `latched_at_us` BIGINT,
  all NOT NULL. It is written only by `INSERT … (id = 1) ON CONFLICT (id) DO NOTHING RETURNING`
  and never updated or deleted. The row means Premium has been delivered on this deployment,
  whatever the current configuration says. Because the key is the constant 1, not the
  activation, two nodes with different configured activations cannot both latch: exactly one
  insert wins. A node whose insert returns no row re-reads the table and holds the winner's row,
  and step 1 then compares it with its own configuration. A node whose read fails holds nothing
  and reads `NotCurrent`.
- **The latch is the durable truth, never a cache of it (#1738 P1 4176993795, P1 4176993801).**
  A process holds the latch row in memory once it has read or written it, and only ever moves
  from "none held" to "held". While it holds none, it re-reads the table before it may return
  `NotActivated`, so a row written by another node, or by an earlier deployment, is always seen.
  The re-read is one primary-key read, and it happens only before delivery and only for a
  command that needs Premium. After the row is held, no read is made.
- **A pre-delivery bypass is fenced against the latch (#1743 P1 4177047054).** Both sides take
  one transaction-scoped Postgres advisory lock with a fixed key, registered with the migration:
  - the latch insert runs in a transaction that first takes it exclusively
    (`pg_advisory_xact_lock`);
  - a `NotActivated` result is only ever produced inside a transaction that first takes it shared
    (`pg_advisory_xact_lock_shared`) and then reads the table. The consumer applies its bypass
    while that lock is held:
    - a durable consumer (GUILD-1's found, rank and job writes) writes in that same transaction,
      so its rows commit before any latch can;
    - an in-process consumer (the Wheel spell exception) applies the cast before the transaction
      commits and the lock is released.
  A latch commit therefore waits for every bypass that already read an empty table, and every
  read after the latch commit sees the row. No bypass applies after the durable switch-over.
  The gate (above) is the only way to obtain `NotActivated`, so no caller sees it outside such a
  transaction. A lock or read failure gives `NotCurrent`. The gate holds a transaction only
  before delivery and only for a command that needs Premium, for the length of that command.
- The status is decided in this order:
  1. **A row is held.** Premium has been delivered, so the status is `Current` or `NotCurrent`,
     never `NotActivated`:
     - the configured activation is `None`, or its id or `S` differs from the held row's (a
       configuration or deployment rollback, or the losing node of a mixed rollout):
       `NotCurrent` (#1738 P1 4176993801, #1743 P1 4177047049);
     - the clock reads `None`: `NotCurrent`;
     - `lower < S`, so the window still straddles `S`: `NotCurrent`;
     - otherwise the refresher's `PremiumConsumer::premium_current(account_id, now)`:
       `Current` or `NotCurrent`.
  2. **No row is held.** The shared lock is taken and the table is re-read in the caller's
     transaction. A failed lock or read gives `NotCurrent`. A row found is held, and step 1
     applies.
  3. **The table is empty and the activation is `None`** (the default and the production
     composition): `NotActivated`.
  4. **The table is empty, an activation exists and the clock reads `None`:** `NotCurrent`. Once
     an activation exists, an unknown time never reopens the pre-delivery behaviour.
  5. **The table is empty, an activation exists and `upper < S`,** so the switch-over has
     certainly not happened: `NotActivated`.
  6. **The table is empty, an activation exists and `upper >= S`,** so the switch-over may have
     happened, and no bypass applies. The gate latches in three steps (#1743 P1 4177096277):
     - It rolls back its shared transaction, which releases the shared lock.
     - It opens a new transaction, sets `lock_timeout` to 5 s, takes the exclusive lock and runs
       the insert. That transaction waits only for other connections' in-flight bypasses, each of
       which lasts one command, and never for its own caller.
     - It holds the returned row, or the winner's row on conflict, and step 1 applies.

     A lock timeout or a failed write gives `NotCurrent` and holds nothing. The next command
     retries from step 2.
  Once any row exists, nothing returns `NotActivated` again on any node: not a clock rollback, a
  VM restore, a restart, a larger uncertainty, a node that booted before `S`, nor a deployment
  that drops or changes the activation.
  Before delivery, with no activation, a configured snapshot source alone is `NotActivated` and
  grants nothing. PREM-WIRE-1 builds only the gate. Setting a production activation needs PREM-1's activation record: PREM-1b merged, PREM-P live, the
  cross-repository end-to-end test and the `PROD-ENTITLEMENTS-01` §6.6 rollout evidence. It also
  needs separate owner authority, so no packet in this decision sets it.

### 1.3 Who wires each Premium consumer

| Consumer | Rule | Owner | When |
|---|---|---|---|
| Spell cast check | PREMIUM-ACTIVATION §4.6 (PREM-4) | PREM-WIRE-1 | now (§2.1). `CasterState` carries the `PremiumStatus`, not a boolean. A Premium spell needs `Current`, except that a spell with `requirements.wheel_unlock` skips the Premium check while the status is `NotActivated` (WHEEL-0 §6.2, WHEEL0-PS-1; #1738 P2 4176993804) |
| Yell below level 20 | CHAT-0 §6 | the later of CHAT-1b-2b and PREM-WIRE-1 | if CHAT-1b-2b is later, it calls the §1.2 seam; if PREM-WIRE-1 is later, it touches only the one `chat/spam.rs` call site, after CHAT-1b-2b merges and under a control plane lease |
| Promotion, soul maximum, death input | §4.2, §4.3 (PREM-2) | PREM-2b, together with PREM-5 | after NPC-TALK-1 (§3); before that nothing can be promoted (§0, finding 5) |
| Premium areas | §4.5 (PREM-3) | PREM-3 | once the map bundle carries the area flag (§3) |
| Premium blessings and NPC services | §4.4 (PREM-5) | PREM-5 | after NPC-TALK-1 (§3) |
| Training statue | OFFLINE-0 §6 | STATUE-1 | with OFFLINE-1 |
| Guild founding, levels 1 and 2, leadership job | GUILD-0 §3.2, §3.3 (G1 a) | GUILD-1 | `NotActivated`: not required and the job writes nothing. `Current`: allowed. `NotCurrent`: `NOT_PREMIUM`, and a lapse keeps the rank (§2.2) |

Every consumer calls the §1.2 seam once per command and stores no result. A consumer with no
pre-delivery rule treats `NotActivated` as `NotCurrent`.

### 1.4 Acceptance needs

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
migration_lease: 0073 (reserved by the control plane for the activation latch table, §1.2)
owned_paths:
  - apps/game-server/migrations/0073_premium_activation_latch.sql
  - apps/game-server/src/durability/premium_activation_latch.rs
  - apps/game-server/src/durability/mod.rs   # module wiring only
  - apps/game-server/tests/support/premium_activation_latch_postgres_cases.rs
  - apps/game-server/src/premium/clock.rs
  - apps/game-server/src/premium/mod.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/mod.rs        # CasterState.premium becomes PremiumStatus; the Wheel exception (§1.3)
  - apps/game-server/src/spell/*tests.rs     # the existing Premium tests move to the status
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
acceptance: none beyond §1.4
```

Builds:

- `TrustedClock` with the system and fixed implementations (§1.1).
- `PremiumStatus<'g>`, `PreDelivery<'g>` and the runtime gate `with_premium_gate(account_id, …)`,
  with `premium_current` derived from it (§1.2). There is no bare `premium_status` function. Both sit behind the `PremiumActivation` gate, and production
  composition passes `None`.
- `cast_spell` takes the admitted account and runs inside the gate. `CasterState.premium` becomes
  the gate's `PremiumStatus`, read once at cast time, and the SPELL-D5 note is removed. The cast
  is applied inside the closure, so a `wheel_unlock` cast under `NotActivated` completes before
  the gate's transaction commits. A Premium spell
  needs `Current`; a `wheel_unlock` spell skips that check under `NotActivated` (§1.3).

Touch rules:

- The connection and test fakes change only by the added account argument.
- The packet touches no `chat/**`, protocol, registry or content file. Its one migration is the
  single-row latch table (§1.2: `id` SMALLINT primary key with `CHECK (id = 1)`,
  `activation_id` TEXT, `switch_over_us` BIGINT, `latched_at_us` BIGINT), with no UPDATE or
  DELETE grant. The advisory lock key is a named constant next to the latch code.
- If `gameplay_transport/mod.rs` is leased to another lane at allocation, the control plane
  serializes the two.

Acceptance tests:

- A Premium fixture spell is refused for a Free account (`NotAvailable`).
- The same spell is cast when the consumer reads current under a fixed synced clock.
- It is refused when the clock reads `None`.
- With a configured snapshot source whose consumer reads current, the spell is still refused
  when the activation is `None`, and when the clock reads 1 µs before the switch-over. It is cast
  at the switch-over instant.
- The production composition test asserts that the activation is `None`.
- Status table: no activation gives `NotActivated`, even with a source reading current.
  - An activation with a `None` clock gives `NotCurrent`.
  - 1 µs before the switch-over gives `NotActivated`.
  - At the switch-over, a consumer reading current gives `Current`, and one reading not current
    gives `NotCurrent`.
- Uncertainty window, with `S` and an uncertainty of 1,000 µs:
  - `now = S - 1,001` gives `NotActivated` and writes no latch;
  - `now = S - 1,000` writes the latch and gives `NotCurrent`;
  - `now = S + 999` gives `NotCurrent`;
  - `now = S + 1,000` with a current consumer gives `Current`.
- Rollback after crossing: after the latch is written, setting the clock back to `S - 10 s`
  gives `NotCurrent`, never `NotActivated`. The same holds after a restart. A latch write failure
  gives `NotCurrent` and holds nothing. A failed re-read gives `NotCurrent`.
- Two nodes on one Postgres (#1738 P1 4176993795): both start before `S`, node A crosses and
  writes the row, and node B, with its clock then set back to `S - 10 s` before its first
  post-boundary command, reads `NotCurrent`.
- Mixed rollout (#1743 P1 4177047049): two nodes on one Postgres with different activation ids
  both cross `S` and latch concurrently. Exactly one insert wins, both nodes hold the winner's
  row, and the losing node reads `NotCurrent` for every account. A second insert never adds a
  row.
- Bypass fence (#1743 P1 4177047054): node B takes the shared lock and reads an empty table, and
  node A then tries to latch. A's insert does not commit until B's transaction ends, and B's
  bypass write is committed before the row. A read that starts after A's commit sees the row
  and never returns `NotActivated`.
- Latch between read and cast (#1743 P1 4177076440): node B's gate reads an empty table for a
  `wheel_unlock` cast, and node A tries to latch while B's closure is still running (a test hook
  pauses it). A's insert commits only after B's cast is applied and B's gate commits. A cast
  that B starts after A's commit reads `NotCurrent` and is refused. `NotActivated` cannot be
  stored outside the closure; a compile-fail test shows it.
- Activation does not hang (#1743 P1 4177096277), on a real Postgres connection pool:
  - The first command with `upper >= S` on a node with an empty table latches the row and
    returns `Current` or `NotCurrent` within a 10 s `tokio::time::timeout`.
  - Its shared transaction has ended before the exclusive request; a test hook records the order.
  - Two nodes' first post-`S` commands, started together, both return within the same timeout.
    Exactly one insert wins, and neither gets a deadlock error.
  - With another connection's bypass paused while holding the shared lock, the latch waits. It
    completes once that bypass commits.
  - When the bypass is paused for longer than the 5 s `lock_timeout`, the command returns
    `NotCurrent`, holds nothing and writes no row. The next command after the bypass commits
    latches.
- Configuration rollback (#1738 P1 4176993801): with a row in the table, a runtime built with
  activation `None`, or with a different id or `S`, reads `NotCurrent` for every account, and
  never `NotActivated`.
- Wheel exception (#1738 P2 4176993804): a Premium fixture spell with `wheel_unlock` passes the
  Premium check under `NotActivated` and is refused under `NotCurrent`. A Premium spell without
  `wheel_unlock` is refused under `NotActivated`. The check is tested on `spell/mod.rs` directly,
  since the ready reader still refuses `wheel_unlock` until SPELL-WHEEL-GATE-1.
- An uncertainty of 5,000,000 µs is current; 5,000,001 µs is not.
- `STA_UNSYNC` and `TIME_ERROR` map to `None`; this is a unit test of the decoding function over
  a `timex` value, so no kernel state is needed.
- The non-Linux stub returns `None`.
- The V1 book's three spells cast for a Free account exactly as before.

### 2.2 GUILD-1

```yaml
task_id: OTV2-20261004-guild-1
decision: GUILD-0 §3, §4 (owner answers G1a, G2a); this decision §1.2, §1.3
worker: oteryn-hard-worker
review: hard, persistence and security review (Codex, final frozen head)
branch: claude/guild-1-20261004
base: main after ECON-RET-0 (#1738 §2.2) and PREM-WIRE-1 merge
depends_on: [ECON-RET-0, PREM-WIRE-1]
migration_lease: one number from the control plane at allocation
owned_paths: the guild module, its migration and tests, the guild event registration, and
  docs/agents/tasks/archive/OTV2-20261004-guild-1.md
acceptance: none beyond §1.4
```

Builds GUILD-0 §3 and §4:

- the guild, rank, member, invitation and account-leadership tables;
- the found, invite, join, leave, exclude, rank, resign and disband transactions;
- the formation and vice World jobs;
- the guild event, bound to ECON-RET-0's guild profile;
- the Premium rule through `PremiumStatus` (§1.2, §1.3), not a boolean. Founding and every move
  into levels 1 and 2 take the actor Account's status:
  - `NotActivated` needs no Premium, which is the G1 a pre-delivery bypass;
  - `Current` passes;
  - `NotCurrent` is refused with `NOT_PREMIUM`.
  The §3.3 daily job reads each leadership Account's status. On `NotActivated` it writes nothing.
  After activation it applies §3.3, and a lapse keeps the rank.

Acceptance tests: found and rank-to-vice under each of the three statuses; the job under
`NotActivated` writes nothing; after the switch-over a Free leader is handled per §3.3; a lapse
keeps the rank. The transactions run inside the §1.2 gate and use its `tx`; tests use a fixed
gate. A founding under `NotActivated` writes in the gate's transaction, so it commits before any
latch.

The packet has no wire. Its playable entry is GUILD-WIRE-1, which waits on GUILD-BANK-1 (#1738
§3).
GUILD-1 can still start now, because GUILD-0 says it does not wait for houses.

## 3. Held children and what releases them

| Child | Waits on | Released by |
|---|---|---|
| PREM-2b (promotion state, soul maximum, death input) | a promotion source | PREM-5, after NPC-TALK-1; packeted together with it |
| PREM-3 (Premium areas) | the area flag in the map bundle; Movement entry refusal | the map packets (another agent prepares maps) |
| PREM-5 (blessings, NPC services) | NPC-TALK-1 | NPC lane |
| A production activation | PREM-1's activation record (PREM-1b, PREM-P live, the cross-repository end-to-end test, the `PROD-ENTITLEMENTS-01` §6.6 rollout evidence) | separate owner authority |

## 4. Rejected options

- **Reading `SystemTime` with no error bound.** This fails PREMIUM-DELIVERY-0 §5: an
  unsynchronized node would grant Premium on a wrong clock.
- **An in-process NTP client or querying chrony over its socket.** Both add a protocol and a
  dependency when the kernel already exposes the disciplined error bound. A fork of a time crate
  is rejected for the same reason.
- **Caching `premium_current` in the session.** A cache would outlive an entitlement end or a
  loss of sync. The refresher already holds the entitlement, so reading per command is cheap.
- **A bare `premium_status() -> PremiumStatus`.** The lock would be released when the read
  returns, before the consumer acts on `NotActivated` (#1743 P1 4177076440).
- **A latch keyed by the activation.** Two nodes with different configured activations would
  both insert and both read `Current` (#1743 P1 4177047049).
- **Re-reading the latch without a fence.** A latch committed between the read and the bypass
  would let the bypass apply after the switch-over (#1743 P1 4177047054). A seeded row locked
  `FOR SHARE` needs an UPDATE grant on the latch, and the advisory lock does not.
- **Taking the exclusive lock in a second transaction while the caller's shared transaction is
  open.** The second transaction runs on another connection, so it waits for the caller, and the
  caller waits for it: the first post-`S` command hangs (#1743 P1 4177096277).
- **Upgrading the shared lock to exclusive in the caller's transaction.** This needs no second
  connection, but two nodes that both hold the shared lock and upgrade together deadlock. Postgres
  then aborts one only after `deadlock_timeout`. Releasing first and latching in a fresh
  transaction has no such cycle.
- **A latch read only at boot.** A node that booted before `S` would never see another node's
  row (#1738 P1 4176993795).
- **Keying the delivered state on the configured activation.** A deployment that drops the
  configuration would reopen the pre-delivery bypass (#1738 P1 4176993801).
- **A boolean `CasterState.premium`.** It loses the pre-delivery Wheel exception (#1738 P2
  4176993804).
- **Building PREM-2b now.** Promotion state with no way to promote is infrastructure no player
  reaches (§0, finding 5).

## 5. Decision queue for the control plane

1. **Route PREM-WIRE-1 now.** It has no dependency beyond PREM-1c-harden (merged).
2. **GUILD-1 after ECON-RET-0 (#1738 §2.2) and PREM-WIRE-1.**
3. **PREM-5 together with PREM-2b.** Packet them after NPC-TALK-1 is allocated.

## 6. Decision test

The mandatory answers (ARCHITECTURE_DECISION_DISCIPLINE):

1. **Must decide now?** YES. PREM-WIRE-1 is the next allocatable work of the Premium lane (D486
   item 4), and GUILD-1 needs its status.
2. **What is blocked?** The spell Premium check, the yell gate and GUILD-1.
3. **What becomes harder later?** The `PremiumStatus` enum is the gameplay read every Premium
   consumer binds to (§1.3), so changing its variants later touches every consumer. The latch
   table is append-only, so a delivered deployment cannot return to pre-delivery behaviour
   without a new decision and a migration.
4. **What would supersede it?** Any of these would reopen this decision:
   - PREM-P amending the activation semantics of PREMIUM-DELIVERY-0 §10.3;
   - a measured clock failure rate that makes `ntp_adjtime` unusable on the production nodes;
   - a measured cost of the pre-delivery latch re-read above the command budget.
5. **What is not decided?** The following stay with their owners:
   - Premium activation and its date (PREM-1's record and owner authority);
   - PREM-2b, PREM-3 and PREM-5;
   - the Wheel spell admission (SPELL-WHEEL-GATE-1);
   - the guild retention profile (ECON-RET-0, #1738).

| Question | Answer |
|---|---|
| Does a Free account's play change? | No |
| Can an unsynchronized node grant Premium? | No (§1.1) |
| Can a configured source grant Premium before activation? | No (§1.2) |
| Can a rollback of the clock, a node or the configuration reopen the pre-delivery behaviour? | No (§1.2) |
| Does any packet here touch `chat/**`? | No (§1.3) |
| Does any packet need a number not leased by the control plane? | No |
| Is any production or secret mutation authorized? | No |
