# Architect batch: TIMED-RT-1b/1c, TIMED-WIRE-1, FORGE-1a/1b and PROF-SHAPE-1b packets

```yaml
decision_id: ARCH-BATCH-TIMED-FORGE-PROF-PACKETS-V1
status: CANDIDATE
date: 2026-10-03
owner: Sol Supervising Architect
requested_by: control plane (D358 topic request after FAMILIARS-0-FIX-3)
writes_on_other_prs: none
amended_by: ARCH-PACKET-FIX-1 (§2.3 rows, §2.5 RL-04, §2.6 rows and the Light owner; #1687 round-3 P1s 4174692081 and 4174692089; #1689 P1s 4174761344, 4174761349, 4174781962 and 4174781965); ARCH-RT-CHECKPOINT-ROWS-1 (§1.1 and §2.3: the first item and the narrowed RT-1b scope; #1689 P1 4174803489; #1692 P1s 4174859933 and 4174875997); ARCH-SLOT-WIRING-1 (§1.1, §2.3 slot call sites; #1692 P1 4174906452); ARCH-ITEM-PACKETS-AMEND-1 (§1.1 and the §2.3 call_sites: slot-to-Ground drops; #1696 P1 4175041166)
```

This bundle splits and packets three requested topics:

- TIMED-RT-1b: the timed-item writes that #1681 (TIMED-RT-1a, migration 0054) refuses;
- TIMED-WIRE-1: the timed-item wire, including `show_count` (D397);
- FORGE-1 and PROF-SHAPE-1b, in dependency order (D393, D394).

The bundle changes no code, no contract and no wire. The rulings in §1 are architecture rulings
under the accepted decisions they cite; §1.5 gives their analysis. Live PR and Issue state governs. The dependency notes
record the state when this was written: #1681 and #1685 open, carrier #1675 open; ITEM-MOVE-2a,
ITEM-MOVE-2b, EQUIP-RT-1, GOLD-FEE-2, FORGE-CONTENT-1 and TIMED-CONTENT-1 not on `main`.

## 0. Leases, order and shared files

Proposed leases (the control plane confirms them; a worker that needs another number stops and
asks):

| Packet | Migration | Capability / command / state domain |
|---|---|---|
| TIMED-RT-1b | 0058 | none |
| FORGE-1a | 0059 | none |
| PROF-SHAPE-1b | 0060 | none |
| FORGE-1b | 0061 | none |
| TIMED-RT-1c | 0062 | none |
| TIMED-WIRE-1 | none | capability 11 `TIMED_ITEMS_V1`, already leased (D353); no command, no state domain |

Cap 12, cmd 21 and domain 14 stay free: no packet here needs them. FORGE-WIRE-1 and
PROF-SHAPE-WIRE-1 lease theirs at allocation.

Order:

| Packet | Starts when |
|---|---|
| FORGE-1a | now (no unmerged dependency) |
| TIMED-RT-1b | #1681 and TIMED-CONTENT-1 have merged |
| PROF-SHAPE-1b | #1685 and FORGE-1a have merged |
| TIMED-WIRE-1 | TIMED-RT-1b and carrier #1675 have merged |
| FORGE-1b | FORGE-1a, FORGE-CONTENT-1, GOLD-FEE-2 and ITEM-MOVE-2a have merged |
| TIMED-RT-1c | TIMED-RT-1b, ITEM-MOVE-2a, ITEM-MOVE-2b and ITEM-USE-1 have merged |

**Shared files.** The packets own disjoint code paths. Four files are append-only registers that
every persistence slice touches:

- `apps/game-server/src/durability/mod.rs` (one `mod` line and one linkage test each);
- `apps/game-server/tests/durability_postgres.rs` (one `mod` line each);
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (each packet's own rows only);
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` (each packet's own
  paragraphs only).

A packet edits only its own lines there. The second of two concurrent packets merges `main` and
keeps both sides (a union, as D391). No packet edits another packet's lines.

## 1. Rulings

### 1.1 TIMED-RT-1 is split again (RT-1b now, RT-1c after the move slices)

D391 left everything that 0054 refuses to "RT-1b, after TIMED-CONTENT-1, ITEM-MOVE-2a/2b and
EQUIP-RT-1". Most of it does not need those slices. The minimum-sufficient split:

- **RT-1b: expiry and the hosting runtime.**
  - Admits `Expire {TimeExhausted | ChargesExhausted}` (causes 2 and 3) with the expiry
    transform, the expiry burn and the composed expiry burn (TIMED-ITEM-0B §8, §12).
  - Wires the lanes into the hosting runtime for the live places that need no move slice: an item
    already in its `CharacterEquipment` slot at login, respawn or arrival (TIMED-ITEM-0B §5.1
    places 1 and 2), and the exercise binding (place 3) as an API for EXERCISE-1.
  - Adds the §6.1 checkpoint points (cadence, logout, channel transfer, death) and the expiry
    audit event.
  - With this, soft boots worn at login run down and become worn soft boots. That is a playable
    effect without any move slice.
  - **Narrowed (ARCH-RT-CHECKPOINT-ROWS-1).** RT-1b hosts at login and logout only (§2.3).
    - The slot call sites go to whichever of RT-1b and ITEM-MOVE-2a merges second, and those of a
      slot-to-Ground drop to whichever of RT-1b and ITEM-MOVE-2b merges second (§2.3 Scope).
    - The exercise binding and the composed writers move to EXERCISE-1.
    - `main` has no character channel transfer, death, respawn or arrival path. Whichever PR adds
      one wires the timed host for it, as a merge condition (§2.3 Scope).
- **RT-1c: everything that rides a move or a use.**
  - `SetDeadline`, `ClearDeadline`, `PutOut` and `Expire {Deadline}` (causes 4-7), the equip forms
    (§9.2) and the `Light` use form (§9.3).
  - Torches in slots and on the Ground (§10.1-§10.5), the Ground deadline scheduler and
    `TIMEDITEM0B-RL-06`.
  - The ITEM-MOVE-WIRE-1, ITEM-USE-0, MARKET-0 and WORLD-INTERACTION-0 amendments.
- **EQUIP-RT-1 is not a dependency of RT-1b.** Live eligibility comes from placement and ownership
  facts alone (§5.1). "Active" and every effect of an active item are TIMED-FX-1's, after
  EQUIP-RT-1. Charge use by protection (§7) is also TIMED-FX-1's, because it needs the EQUIP-0
  §3.1 stage. RT-1b spends no charge except through the exercise API.

### 1.2 `show_count` reaches the runtime in TIMED-WIRE-1 (D397)

- D397 kept the OTRPA04 codec unchanged in TIMED-CONTENT-1. The runtime reads `charges.count`
  only (`ReferenceItemCharges`).
- The `ITEM_GROUP_CHARGES` group is written in both projections, so changing its layout would also
  change the client's artifact. TIMED-WIRE-1 therefore leaves it as it is and adds one new item
  group, `ITEM_GROUP_CHARGES_DISPLAY`, holding `show_count`. It takes the next free group tag on
  `main` at allocation (17 today).
- The new group is written only in the server-authoritative projection, as `ITEM_GROUP_TEMPORAL`
  is. The content schema already has `charges {count, show_count}`.
- The server-authoritative artifacts are regenerated and their digests re-pinned in the same PR.
  That is why TIMED-WIRE-1 starts only after carrier #1675 merges.
- There is no new magic and no client-projection change. The server decides whether to send
  `charges` (TIMED-ITEM-0B §13), and the client only shows what it receives.
- A definition with `show_count` false sends no `charges` field. `remaining_s` follows the
  duration rule of §13 unchanged.

### 1.3 FORGE-1 is split at the dust asset

PROF-SHAPE-1b needs only the dust balance and its ledger (D394). It does not need tiers, fusion
or the GOLD-FEE-2 bank part.

- **FORGE-1a: the dust asset.**
  - `game_character_forge_dust` and its immutable ledger (`GAIN`, `SPEND`, `CONVERT`,
    `LIMIT_RAISE`), IMBUE-FORGE-0 §9, under BANK-0 §3's pattern.
  - The composition amendment (rule 1 covers the acting character's dust rows; lock order: rule
    4's, then the dust row).
  - The DUR-03 §18 naming of forge dust.
  - The `dust_limit` column (initial 100, `IMBFORGE0-RL-08`). Nothing in FORGE-1a raises it;
    the `CONVERT` and `LIMIT_RAISE` entry kinds are reserved for FORGE-1b.
  - A `SPEND` API with a closed spending cause, and the `GAIN` writer with a closed cause
    reserved for FORGE-CREATURE-1. Production wires no gain yet; PostgreSQL cases exercise it.
- **`DustLimit` is a forge operation, so it is FORGE-1b's.** IMBUE-FORGE-0 §10 gives every forge
  operation a `ForgeCause`, its DUR-03 §15/§39.3 admission and a history row (`IMBFORGE0-RL-12`).
  Splitting those from the other forge operations would build the forge operation frame twice.
- **The dust limit price is a ruleset formula, not content.**
  - Raising the limit by one costs (limit − 75) dust, up to 225 (`IMBFORGE0-RL-08`, Canary,
    `PARITY_PENDING`).
  - It is bound by the ruleset/formula revision of the occurrence (IMBUE-FORGE-0 §10), so
    `DustLimit` needs no FORGE-CONTENT-1 row.
- **FORGE-1b: everything else in IMBUE-FORGE-0 §8-§10.** `DustLimit`, the tier table, fusion,
  convergence, transfer, the conversions, `ForgeCause`, the `FeeBurnCause` forge variants, the
  history and the forge DUR-03 rows. It keeps the original FORGE-1 dependencies.

### 1.4 PROF-SHAPE-1b replaces 0055's zero-cost pin

- 0060 replaces the `..._line_no_ledger` CHECK with PROFICIENCY-1B §7.1's deferred guard. The
  transaction's dust ledger `SPEND` equals `dust_spent`, with no entry when it is 0. The orb BURN
  quantity equals `orbs_spent`.
- The writer lifts `NOT_ADMITTED` for the operations whose costs are evidenced. Admission by
  evidence (§3.3) is unchanged, so a cost without evidence still answers `NOT_ADMITTED`.
- The orb BURN follows the existing one-item stack BURN shape of `item_fee_burn.rs` under
  `ProficiencyCause`. It does not add a new item writer.

### 1.5 Decision analysis

ARCHITECTURE_DECISION_DISCIPLINE.md applies to each ruling above.

**1.1 Split TIMED-RT-1 into RT-1b and RT-1c.**
- Problem: D391 put all of 0054's refused writes behind four unmerged slices, so no timed item
  runs down in play.
- Constraints: TIMED-ITEM-0B §5-§13 unchanged; session-generation fencing; no move or use path
  without ITEM-MOVE-2a/2b and ITEM-USE.
- Options: (a) keep one RT-1b behind all four slices; (b) split at the first write that needs a
  move or a use.
- Trade-offs: (a) is one review but blocks the playable effect for weeks; (b) adds one packet
  and gives a playable effect after #1681 and TIMED-CONTENT-1.
- Risks: (b) could hard-code a hosting path RT-1c must reopen. Mitigation: RT-1b uses only
  TIMED-ITEM-0B §5.1's placement facts, which RT-1c extends without change.
- Recommendation: (b).
- Future impact: none on schema or wire; RT-1c adds causes 4-7 on the same lanes.
- Decision test: 1 YES; 2 TIMED-RT-1b allocation after #1681; 3 nothing (no schema or wire is
  fixed beyond TIMED-ITEM-0B); 4 evidence that the hosting runtime needs EQUIP-RT-1 facts;
  5 TIMED-FX-1 effects and charge use by protection.

**1.2 `show_count` in a new server-authoritative group.**
- Problem: the runtime cannot read `show_count` (D397), and the existing charges group is in both
  projections.
- Constraints: the client projection and its artifact stay unchanged; D397; carrier #1675 pins.
- Options: (a) widen `ITEM_GROUP_CHARGES`; (b) add `ITEM_GROUP_CHARGES_DISPLAY`, server-only.
- Trade-offs: (a) reuses a group but changes the client artifact and its digest; (b) costs one
  group tag and keeps the client artifact byte-identical.
- Risks: a tag collision with a concurrent group; mitigated by taking the next free tag at
  allocation.
- Recommendation: (b).
- Future impact: one more server-authoritative group; a later client need can project it without
  touching `ITEM_GROUP_CHARGES`.
- Decision test: 1 YES; 2 TIMED-WIRE-1; 3 one group tag; 4 evidence that the client must read
  `show_count` itself; 5 the tag number and any client projection.

**1.3 Split FORGE-1 at the dust asset.**
- Problem: PROF-SHAPE-1b needs only dust (D394), but FORGE-1 waits on FORGE-CONTENT-1,
  GOLD-FEE-2 and ITEM-MOVE-2a.
- Constraints: IMBUE-FORGE-0 §9-§10 and §15; BANK-0 §3; DUR-03 §28.
- Options: (a) one FORGE-1; (b) FORGE-1a (the dust asset only) now, FORGE-1b (every forge
  operation, `DustLimit` included) later.
- Trade-offs: (a) one review, but PROF-SHAPE-1b waits on three unrelated slices; (b) one more
  packet, and PROF-SHAPE-1b is unblocked.
- Risks: a dust schema FORGE-1b must change. Mitigation: FORGE-1a implements IMBUE-FORGE-0 §9 as
  written, with the closed `CONVERT` entry kind reserved.
- Recommendation: (b), with the dust limit price as a ruleset formula bound by revision.
- Future impact: FORGE-1b only adds causes and shapes.
- Decision test: 1 YES; 2 FORGE-1a and PROF-SHAPE-1b; 3 the dust ledger schema, which
  IMBUE-FORGE-0 §9 already fixes; 4 Reference evidence of a different limit price; 5 tiers,
  fusion, transfer and conversions.

**1.4 Replace 0055's zero-cost pin with a deferred guard.**
- Problem: 0055 forbids any value line, so evidenced dust and orb costs cannot commit.
- Constraints: PROFICIENCY-1B §7.1-§7.2; admission by evidence (§3.3); no new item writer.
- Options: (a) drop the CHECK; (b) replace it with the deferred guard that ties lines to the
  recorded costs.
- Trade-offs: (a) is simpler but lets a writer record a cost it did not take; (b) keeps the
  ledger, orb line and receipt provably equal.
- Risks: a guard that fires too early; it is deferred to commit, as 0023's fee lines.
- Recommendation: (b), with the orb BURN in the `item_fee_burn.rs` shape.
- Future impact: none on wire; PROF-SHAPE-WIRE-1 reads the same receipt.
- Decision test: 1 YES; 2 PROF-SHAPE-1b; 3 nothing beyond PROFICIENCY-1B; 4 a cost shape that
  needs more than one dust line or one orb line; 5 costs still without evidence.

## 2. Packets

### 2.1 FORGE-1a

```yaml
task_id: OTV2-20261003-forge-1a
decision: IMBUE-FORGE0 §9 (the dust asset; no forge operation); this bundle §1.3
worker: oteryn-hard-worker   # persistence, a new value asset
review: independent persistence and economy review (Codex, final frozen head)
branch: claude/forge-1a-20261003
base: main
migration_lease: 0059
depends_on: []
owned_paths:
  - apps/game-server/migrations/0059_character_forge_dust.sql
  - apps/game-server/src/durability/character_forge_dust.rs
  - apps/game-server/src/durability/character_forge_dust_audit.rs
  - apps/game-server/src/domain/forge_dust.rs
  - apps/game-server/src/domain/mod.rs                 # one mod line
  - apps/game-server/src/durability/mod.rs             # shared register (§0)
  - apps/game-server/tests/character_forge_dust_postgres.rs
  - apps/game-server/tests/support/character_forge_dust_postgres_cases.rs
  - apps/game-server/tests/durability_postgres.rs      # shared register (§0)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # IMBFORGE0-RL-08; own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §18 forge dust; own paragraphs only
  - the composition decision's rule 1 paragraph (the file BANK-0 §7.2 amended; named at allocation)
  - docs/agents/tasks/archive/OTV2-20261003-forge-1a.md
validation:
  - cargo fmt --all --check
  - cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test character_forge_dust_postgres --quiet
  - python3 tools/agents/validate_governance.py; git diff --check
```

Acceptance:

- **Balance and ledger.**
  - The balance stays within 0 to `dust_limit`, and `dust_limit` within 100-225.
  - Every balance change has exactly one ledger entry in the same transaction, chained by
    `last_entry_id`.
  - Ledger entries are immutable: no UPDATE, DELETE or TRUNCATE.
  - Grants as BANK-0 §3: the runtime role never deletes.
- **Gain above the limit.** A gain above the limit credits up to the limit and records the lost
  part in the entry (IMBUE-FORGE-0 §9).
- **No forge operation.** FORGE-1a writes no `ForgeCause`, receipt or history. `SPEND` and `GAIN`
  run inside the caller's transaction under the caller's cause, receipt and DUR-03 admission
  (PROF-SHAPE-1b's `ProficiencyCause`, later FORGE-1b and FORGE-CREATURE-1).
- **Rows.** `IMBFORGE0-RL-08` (dust limit 100-225) is registered here, with tests that 100 and 225
  are accepted and 99 and 226 rejected by the CHECK.
- **Tests.** Lock order: rule 4's locks, then the dust row; a concurrent spend and gain
  serialize. A `SPEND` above the balance is refused before any write.
- **Not in scope.** `DustLimit`, tiers, fusion, transfer, conversions (FORGE-1b), the wire (FORGE-WIRE-1),
  dust from kills (FORGE-CREATURE-1) and the proficiency sink (PROF-SHAPE-1b).

### 2.2 PROF-SHAPE-1b

```yaml
task_id: OTV2-20261003-prof-shape-1b
decision: PROFICIENCY-1B §5, §7 (dust and orb lines), IMBUE-FORGE0 §9 amendment; this bundle §1.4
worker: oteryn-hard-worker   # persistence, value sink
review: independent persistence, economy and determinism review (Codex, final frozen head)
branch: claude/prof-shape-1b-20261003
base: main after #1685 and FORGE-1a merge
migration_lease: 0060
depends_on: [#1685, FORGE-1a]
owned_paths:
  - apps/game-server/migrations/0060_proficiency_modification_value_lines.sql
  - apps/game-server/src/durability/character_proficiency_modification.rs
  - apps/game-server/src/domain/weapon_proficiency.rs
  - apps/game-server/tests/support/character_proficiency_modification_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # DUR03-RL-01/-02/-03/-06-PROF; own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §15 ProficiencyCause sink, §39.3 shapes; own paragraphs only
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md  # §9: the dust sink admitted
  - docs/agents/tasks/archive/OTV2-20261003-prof-shape-1b.md
validation:
  - cargo fmt --all --check
  - cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test character_proficiency_modification_postgres --quiet
  - python3 tools/agents/validate_governance.py; git diff --check
```

Acceptance:

- **Dust burn shape.** The receipt, its two lines and one dust ledger `SPEND` are in one
  transaction (PROFICIENCY-1B §7.2), in the lock order modification rows, the dust balance row,
  then the orb stack.
- **Orb burn shape.** One BURN of one unit of the found orb stack only when the evidenced count is
  1. The stack keeps its identity or retires at zero. A count of 0 still requires the orb
  (`NO_ORB`).
- **Deferred guard.**
  - The `SPEND` amount equals `dust_spent`, with no entry when it is 0, and the orb BURN quantity
    equals `orbs_spent`.
  - The finding-4174228267 test: a writer that records 1 dust for a 1,000-dust operation is
    refused.
- **Admission.** `INSUFFICIENT_DUST` and `NO_ORB` are checked before the transaction. Operations
  without evidenced costs still answer `NOT_ADMITTED`.
- **Integrity.** `verify_character_integrity` also checks each line's ledger entry and orb line.
- **Rows.** Max and max+1 tests for `DUR03-RL-01-PROF` (1), `-02-PROF` (1), `-03-PROF` (1) and
  `DUR03-RL-06-PROF` (participants 1, work units 4), as PROFICIENCY-1B §7.2.
- **Not in scope.** The wire (PROF-SHAPE-WIRE-1).

### 2.3 TIMED-RT-1b

```yaml
task_id: OTV2-20261003-timed-rt-1b
decision: TIMED-ITEM-0B §5.1 (place 1 at login), §5.2-§5.3, §6.1 (cadence and logout), §8, §12 (checkpoint and expiry shapes), §14; this bundle §1.1
worker: oteryn-hard-worker   # persistence, session-generation fencing, runtime ownership
review: independent persistence and determinism review (Codex, final frozen head)
branch: claude/timed-rt-1b-20261003
base: main after #1681 and TIMED-CONTENT-1 merge
migration_lease: 0058
depends_on: [#1681, TIMED-CONTENT-1]
owned_paths:
  - apps/game-server/migrations/0058_item_timed_expiry.sql
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/item_timed_state_audit.rs
  - apps/game-server/src/domain/timed_item.rs
  - apps/game-server/src/domain/timed_item_host.rs     # new: the hosting runtime's lanes, login eligibility, the cadence and logout §6.1 points
  - apps/game-server/src/domain/mod.rs                 # one mod line
  - apps/game-server/src/durability/mod.rs             # shared register (§0)
  - apps/game-server/tests/item_timed_state_postgres.rs
  - apps/game-server/tests/support/item_timed_state_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # §12 checkpoint and expiry shape rows; RL-01/-02/-04/-05 only where #1681 did not register them; own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §39.3 checkpoint and expiry shapes; own paragraphs only
  - docs/agents/tasks/archive/OTV2-20261003-timed-rt-1b.md
call_sites: login and logout only call into timed_item_host; the worker names each file at allocation and adds only the call. `main` has no character channel transfer, death, respawn or arrival path to call from; slot call sites belong to whichever of RT-1b and ITEM-MOVE-2a merges second, and slot-to-Ground drop call sites to whichever of RT-1b and ITEM-MOVE-2b merges second (§2.3 Scope). It does not edit foundation/runtime_actor_carrier.rs while #1675 or #1682 is open.
validation:
  - cargo fmt --all --check
  - cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_timed_state_postgres --quiet
  - python3 tools/agents/validate_governance.py; git diff --check
```

Acceptance:

- **Scope (narrowed; ARCH-RT-CHECKPOINT-ROWS-1).** RT-1b is:
  - migration 0058 (expiry);
  - the amendments it needs to the guards that 0011 and 0023 define, made in 0058 by replacement
    (0011 and 0023 are not edited);
  - the five §12 rows (checkpoint, composed checkpoint, expiry transform, expiry burn and composed
    expiry burn);
  - a `timed_item_host` lib wired at login and logout only.

  Moved out of RT-1b (#1692 P1 4174875997):
  - **Slot call sites** (equip, unequip, swap) go to whichever of RT-1b and ITEM-MOVE-2a merges
    second (#1692 P1 4174906452).
    - That PR wires them, as a merge condition with tests. When RT-1b is second, the worker names
      ITEM-MOVE-2a's call-site files at allocation.
    - The ITEM-MOVE-WIRE-1 decision §4 carries the same condition for ITEM-MOVE-2a.
    - The two slices are not ordered.
    - An item moved into a slot stays non-live until the slot call sites are wired.
    - Leaving a slot stops the item (TIMED-ITEM-0B §5.3) and waits until its lane is empty;
      a rejected move makes it live again from the row (§9.1). Both have tests (#1696 P1s).
    - A drop from a slot to Ground (ITEM-MOVE-2b §5) is also a move out of a slot. Its call
      sites go to whichever of RT-1b and ITEM-MOVE-2b merges second, with the same stop, empty
      lane and rehost rules and tests, success and rejection. ITEM-MOVE-WIRE-1 §5 carries the same
      condition (#1696 P1 4175041166). RT-1c cannot own them: it starts after ITEM-MOVE-2b, so
      ordering 2b after RT-1c would be circular (item batch §1.7).
  - **The exercise binding** (place 3) moves to EXERCISE-1, with the two composed writers
    (composed checkpoint and composed expiry burn).
  - **Lifecycle paths: the owner is whichever PR adds them.** `main` has no character channel
    transfer, death, respawn or arrival path. RT-1b therefore has no call site to wire for them,
    and no live item can pass through one. The first PR that adds any of these paths must wire
    the timed host for that path, as a merge condition, with tests:
    - channel transfer and death drain every lane of the actor first (TIMED-ITEM-0B §6.1);
    - respawn and arrival rehost the items in their slots (§5.1 place 2), from the last durable
      checkpoint.

    That PR may be ITEM-MOVE-2a, EXERCISE-1 or the lifecycle path's own slice.

  Each receiving slice carries what it receives here, with its tests.
- **First item: the checkpoint rows (#1689 P1 4174803489, D317 follow-up).**
  - #1681 (RT-1a) merged its checkpoint writer without the §12 checkpoint and composed checkpoint
    shape rows. RT-1b's first commit registers both, before any other RT-1b work.
  - The checkpoint row has max and max+1 tests on the DUR-03 ceilings §12 names (`DUR03-RL-01` 1,
    `DUR03-RL-07-EVENTS` 1, `DUR03-RL-08` 3, the location lines, transform I/O, participants and
    work units). The tests run against RT-1a's merged writer, `commit_timed_checkpoint`.
  - RT-1a has no composed writer: `commit_timed_checkpoint` writes a plain record with no build
    receipt (#1692 P1 4174859933). So the composed checkpoint row gets max and max+1 tests on the same
    ceilings at the shape level only, with no writer behind them.
  - EXERCISE-1 adds the composed checkpoint writer. Its writer-backed max and max+1 tests, which
    it also adds to the row's `boundary_tests`, are a merge condition of EXERCISE-1.
  - The rest of RT-1b is not reviewed until these rows are on its branch.
- **Guard (0058).**
  - Admits causes 2 and 3 with §8's three shapes.
  - Causes 4-7 stay refused until RT-1c.
  - An expiry is keyed at the lane's expected revision and never stores 0 charges or 0 ms for a
    timed definition.
- **Expiry transform.** In place (`PRESERVE_INSTANCE`), the row reset to the target's full values,
  or set spent when the target is not timed.
- **Expiry burn.** To `RETIRED` with one location line; the inert row stays and the record
  carries the before values.
- **Composed expiry burn.** The writer, with the build receipt in §6.3's lock order (EXERCISE-0
  §5.3), is EXERCISE-1's. RT-1b registers only its row (Rows below).
- **Lane (§5.2).**
  - Expiry waits for an in-flight checkpoint (the D285 race).
  - An unexpected revision with current fences retries an expiry once at the same definition.
  - The §5.2 durable-exhaustion tests, including the crash between checkpoint and expiry.
- **Host.**
  - Items in their slot at login become live (place 1).
  - Logout waits for every lane of the actor to be empty.
  - A crash returns at most one checkpoint interval (§6.2).
- **Evidence.** One audit event per expiry (§8).
- **Rows (TIMED-ITEM-0B §12, §14).**
  - RT-1b registers the §12 expiry shape rows it admits: expiry transform
    (`DUR03-RL-04-TIMED-EXPIRY`), expiry burn and composed expiry burn.
  - Expiry transform and expiry burn have max and max+1 tests on the same DUR-03 ceilings as the
    checkpoint row, against RT-1b's own writers.
  - Composed expiry burn is handled like the composed checkpoint row (first item): shape-level
    tests in RT-1b, and writer-backed tests in EXERCISE-1 as one of its merge conditions.
  - `TIMEDITEM0B-RL-01`, `-02`, `-04` and `-05` are registered by #1681 (RT-1a), with unit tests
    on the lane. RT-1b proves each one again at the host boundary it adds, with max and max+1
    tests:
    - **RL-01:** the host's checkpoint cadence is accepted at 60 s and refused at 61 s, and a live
      item hosted at login checkpoints within 60 s;
    - **RL-02:** a host write whose outcome is known by 2,000 ms keeps the lane open, and one
      still unknown at 2,001 ms holds the lane for reconciliation;
    - **RL-04:** the host lib accepts 11 live items for one actor and refuses a 12th before any
      lane opens. The test drives the lib directly, because the login wiring reaches at most ten
      equipment slots. EXERCISE-1 repeats it with the binding as the 11th;
    - **RL-05:** a host expiry or checkpoint issued while the lane has a write in flight waits
      for it and is never sent as a second write.
  - If one of those rows is missing on `main`, RT-1b registers it, with the same tests.
  - The use form and put out rows of §12 are RT-1c's (§2.6).
- **Not in scope.**
  - RT-1c's causes and forms (§1.1).
  - Charge use by protection and every active effect (TIMED-FX-1).
  - The wire (TIMED-WIRE-1).
  - The call sites and writers moved out above (Scope), which belong to the slices named there.

### 2.4 TIMED-WIRE-1

```yaml
task_id: OTV2-20261003-timed-wire-1
decision: TIMED-ITEM-0B §13; D397; this bundle §1.2
worker: oteryn-hard-worker   # protocol-oteryn wire format
review: protocol review (Codex, final frozen head)
branch: claude/timed-wire-1-20261003
base: main after TIMED-RT-1b and carrier #1675 merge
capability_lease: 11 TIMED_ITEMS_V1 (D353)
depends_on: [TIMED-RT-1b, "#1675"]
owned_paths:
  - apps/game-server/src/content/reference_playable.rs     # ReferenceItemCharges.show_count
  - apps/game-server/src/content/reference_artifact.rs     # ITEM_GROUP_CHARGES_DISPLAY, server-authoritative projection only
  - the pinned server-authoritative Reference artifacts and their digest pins (regenerated; files named at allocation)
  - crates/protocol-oteryn/                                # the optional fields (files named at allocation)
  - the game-server item presentation encoder and Look text (files named at allocation)
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 11 row, re-measured max_payload_bytes
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # TIMEDITEM0B-RL-03; own rows only
  - docs/agents/tasks/archive/OTV2-20261003-timed-wire-1.md
validation:
  - cargo fmt --all --check
  - cargo clippy --locked --workspace --all-targets --quiet -- -D warnings
  - cargo test --locked -p protocol-oteryn --quiet
  - cargo test --locked -p oteryn-game-server --quiet
  - the protocol registry validator; python3 tools/agents/validate_governance.py; git diff --check
```

Acceptance:

- **Codec.**
  - `show_count` round-trips through `ITEM_GROUP_CHARGES_DISPLAY` in the server-authoritative
    projection.
  - The client projection's bytes are unchanged; a test compares them before and after.
  - A charged definition without the group in a server-authoritative artifact is refused.
- **Fields.** `charges` and `remaining_s` are sent only with capability 11, and only under §13's
  rule:
  - `charges` only when `show_count` is true;
  - a deadline item's `remaining_s` is clamped to 0.
- **Updates.**
  - At most one time update per `TIMEDITEM0B-RL-03` (60 s), with max and max+1 tests.
  - A charge change at most once per second per item.
- **Look.** The Look text comes from the same values.
- **Payload.** The largest affected message (a 30-entry container view, `ITEMV0-RL-01`) is
  measured, and its `max_payload_bytes` is re-registered when it grows.
- **Without the capability.** Nothing changes.

### 2.5 FORGE-1b

```yaml
task_id: OTV2-YYYYMMDD-forge-1b
decision: IMBUE-FORGE0 §8-§10 (all but the dust asset); this bundle §1.3
worker: oteryn-hard-worker   # persistence, economy, deterministic RNG
review: independent persistence, economy and determinism review
base: main after FORGE-1a, FORGE-CONTENT-1, GOLD-FEE-2 and ITEM-MOVE-2a merge
migration_lease: 0061
depends_on: [FORGE-1a, FORGE-CONTENT-1, GOLD-FEE-2, ITEM-MOVE-2a]
owned_paths:
  - apps/game-server/migrations/0061_item_tiers_and_forge_operations.sql
  - apps/game-server/src/durability/item_forge.rs
  - apps/game-server/src/durability/item_forge_audit.rs
  - apps/game-server/src/domain/forge.rs
  - apps/game-server/src/domain/mod.rs                 # one mod line
  - apps/game-server/src/durability/mod.rs             # shared register (§0)
  - apps/game-server/tests/item_forge_postgres.rs
  - apps/game-server/tests/support/item_forge_postgres_cases.rs
  - apps/game-server/tests/durability_postgres.rs      # shared register (§0)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # DUR03-RL-01/-03-FORGE, IMBFORGE0-RL-03/-04/-07/-11/-12, the forge DUR03-RL-06 rows (DustLimit included); own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §15 ForgeCause, §39.3 forge shapes; own paragraphs only
  - docs/agents/tasks/active/OTV2-YYYYMMDD-forge-1b.md
validation: as FORGE-1a, with --test item_forge_postgres
```

Acceptance: IMBUE-FORGE-0 §8-§10 operation by operation, `DustLimit` included:

- `DustLimit` costs (limit − 75) dust and raises the limit by 1 under `ForgeCause::DustLimit`,
  with one receipt and one history row; a replay returns the receipt; refusals (insufficient dust,
  the limit at 225) come before the transaction;

- each operation's lines are in one transaction;
- costs are spent on failure;
- the roll comes from `forge_fusion`, and its outcome is stored in the receipt and never rerolled;
- the terminal `REVISION_CHANGED` is written receipt-only;
- every refusal comes before the transaction;
- history rows are capped by `IMBFORGE0-RL-12`;
- the bonuses are content with Canary rates (`PARITY_PENDING`);
- **throughput (`IMBFORGE0-RL-04`, IMBUE-FORGE-0 §15: measured before FORGE-1 ships):**
  - A PostgreSQL measurement case in `item_forge_postgres_cases.rs` runs concurrent forge
    commits for distinct characters on one channel. It covers every forge shape FORGE-1b admits,
    `DustLimit` included, on the PostgreSQL version CI pins.
  - It reports sustained commits per second and the database p99 commit latency. The full run
    is selected by an environment variable; CI runs a short version that checks only that the
    harness completes and asserts no timing.
  - The worker records the method, the host and the measured values in the task record.
  - `IMBFORGE0-RL-04` is one registry entry: the database p99 forge commit latency, in
    milliseconds. Its `hard_maximum` is an upper bound, as the registry defines it. It is a
    qualification budget, not a runtime cutoff, as `MAP01-VIEWPORT-US` is registered.
  - The throughput is not a registry limit. A `hard_maximum` cannot express a floor. The
    qualified rate, in forge commits per channel per second, is the entry's workload: its
    `resource` names it and its `notes` record it.
  - Boundary tests:
    - a full run at the qualified rate whose p99 is within the budget passes qualification;
    - a p99 over the budget fails it.
  - The harness fails a run that does not sustain the qualified rate before it compares any
    p99. That is a precondition of the measurement, not a registry comparison.
  - The worker reports both values in its FREEZE. The architect accepts them there; the worker
    does not pick either number. If no measurement can run, the worker stops and reports, and
    FORGE-1b does not merge without one. IMBUE-1 measures the imbuing part of RL-04 under its own
    packet.

The owned paths are refreshed at allocation against the GOLD-FEE-2 and ITEM-MOVE-2a modules
then on `main`.

### 2.6 TIMED-RT-1c

```yaml
task_id: OTV2-YYYYMMDD-timed-rt-1c
decision: TIMED-ITEM-0B §9, §10.1-§10.5, §12 (causes 4-7); this bundle §1.1
worker: oteryn-hard-worker
review: independent persistence and determinism review
base: main after TIMED-RT-1b, ITEM-MOVE-2a, ITEM-MOVE-2b and ITEM-USE-1 merge
migration_lease: 0062
depends_on: [TIMED-RT-1b, ITEM-MOVE-2a, ITEM-MOVE-2b, ITEM-USE-1]
owned_paths: the remainder of the TIMED-RT-1 packet in TIMED-ITEM-0B §21: the move modules, the Ground step scheduler, and the ITEM-MOVE-WIRE-1, ITEM-USE-0, MARKET-0 and WORLD-INTERACTION-0 pointers, named at allocation; the ITEM-USE-1 module that defines `ItemUseCause` and its transaction mapping, for the `Light` variant and its use form only; plus 0062 and the RT-1b modules
validation: as TIMED-RT-1b
```

Acceptance: TIMED-ITEM-0B §15 conditions 3 and 5, D360's tests, and the max and max+1 tests for
the §9.2 and §10.5 rows, the §12 use form (`ItemUseCause::Light`) and put out (`PutOut`) shape
rows, and `TIMEDITEM0B-RL-06`. RT-1c registers each row it admits.

`ItemUseCause::Light` is RT-1c's, not ITEM-USE-1's. ITEM-USE-1 provides the `ItemUseCause` shapes
and the use path (ITEM-USE-0 §4) without `Light`. RT-1c adds the `Light` variant and its §9.3 use
form (the TIMED-ITEM-0B amendment of ITEM-USE-0 §4.2 and §6). It registers the use form row in the
same PR, before the variant can be reached. No `Light` form merges ahead of its §12 row.
