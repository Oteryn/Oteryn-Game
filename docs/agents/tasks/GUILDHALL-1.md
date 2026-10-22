# GUILDHALL-1 — guildhall bid, guild rent, ban, GUILD_DISBAND and guild ACL (consolidated task card)

```yaml
task_id: GUILDHALL-1
title: "GUILDHALL-1: guildhall bid (split escrow), guild rent, ban, GUILD_DISBAND, guild ACL"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "hard (persistence, economy, security)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILDHALL-1 (rows 113, 119); App. 7 GUILD-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md   # §6.2 refund sources, GUILD0-LO-01, GUILD0-RL-14
  - docs/contracts/OTERYN_GAME_GUILDS_V1.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true   # House/Guild/Bank events, HousePrice/HouseRent burn arms
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2153, branch skeleton/guildhall-1

## Goal

Guildhall bid with split (Guild + leader Account) escrow, guild rent, guild ban,
`GUILD_DISBAND` release of the Hall and rank-derived guild ACL, per GUILD0. This task owns
the Hall bid custody and its release participant; House auction/property, ACL storage and
Inbox disposition stay with their House owners.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| BANK-1 | MERGED (#1766) |
| HOUSE-RUNTIME-1a | MERGED (#2163) |
| GUILD-1 (tables, guards, Resign/Disband writers, `GuildDisbandTransaction`) | MERGED (#2136) as unregistered checkpoint; sources removed by #2280 (d25623a36) as dead code — BLOCKING until the Guild core is re-landed (`docs/agents/tasks/GUILD-1.md`) |
| GUILD-BANK-1 (`apply_guildhall_escrow_in_transaction`, guild ledger) | BLOCKING — re-implementation pending (draft #2139 closing) |
| HOUSE-1a (auction, property/legacy anchor, `game_house_guild_disband_released_proven` delegate) | BLOCKING — re-implementation pending (draft #2137 closing) |
| HOUSE-1b (rent collection, disposition, Inbox steps) | BLOCKING — re-implementation pending (draft #2141 closing) |
| HOUSE-ACL-1 (property/ACL extension, guild-entry resolution) | BLOCKING — re-implementation pending (draft #2140 closing) |
| Junior/Dawnport source (`fee_payer_is_junior` still sees `NoStarterIsland`) | OPEN |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/housing/src/guildhall_escrow.rs` (new, L3 pure rules: split reserve/raise/lower arithmetic, ceilings)
- `crates/gameplay/housing/src/guildhall_escrow_tests.rs` (new)
- `crates/gameplay/durability/src/durability/guildhall.rs` (new, Hall bid custody, operation journal, release participant)
- `crates/gameplay/durability/src/durability/guildhall_tests.rs` (new)
- `apps/game-server/tests/guildhall_postgres.rs` (new)
- `docs/agents/tasks/GUILDHALL-1.md`

Slot-only (final commit at the queue turn): the migration (number: next free number at
allocation time (only one migration PR in the queue)); module registration in
`crates/gameplay/housing/src/lib.rs`, `crates/gameplay/durability/src/durability/core_modules.rs`
/ `lib.rs`; `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`; an amendment to
`docs/contracts/OTERYN_GAME_GUILDS_V1.md` if needed; command wiring in
`crates/gameplay/dispatch/src/gameplay_transport/connection.rs` and `capabilities.rs`
(new capability not offered until CP decides).

## Wiring points (verified on origin/main)

- `apps/game-server/migrations/0038_house_spell_acl.sql` — Character-only `game_house_ownership`; unchanged here
- `apps/game-server/migrations/0071_account_bank.sql`, `0073_character_progression_guard_wheel_union.sql` — closed Account bank entries / FeeBurnCause; immutable
- `crates/gameplay/durability/src/durability/bank.rs` (`fee_payer_is_junior`), `bank_audit.rs`
- `crates/gameplay/durability/src/durability/house_spell_acl.rs`, `world_house_instance.rs`
- `apps/game-server/src/node/guild_jobs.rs` (`run_guild_disband_pass`; unregistered) — Hall release runs inside the Guild disband transaction
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain; `crates/protocol-oteryn` `capability_tables::GATED`
- Reference only (removed from main): `git show d25623a36^:crates/gameplay/durability/src/durability/guild.rs`

## Accepted decisions carried from the draft

- **D1098:** GUILD-1 creates `GuildDisbandTransaction`; this task supplies the real bid
  release participant; HOUSE-1a owns property/auction/Inbox ports.
- **D1100 (accepted):** after a leader-Account change a raise may be paid wholly by Guild gold;
  any raise needing extra Account escrow is refused. Refunds stay bound to the original
  contributors (GUILD0 §6.2).
- **D1167:** this task only consumes `game_house_guild_disband_released_proven`; it must return
  strictly TRUE. Missing, FALSE and NULL fail closed; DB errors roll back the whole
  transaction. GUILD-1 owns the real caller, reservation/cursor/events and context getters.
- No surrogate Character owner for Guild ownership, no second ACL validator or owner table,
  no boolean authority callback, no fee-debit substitution.

## Reference behaviour built by the draft (re-implement, do not copy)

- Bid source `game_guildhall_bids`: bid/guild/World/House/auction identities, fixed funding
  Account and Character, max and snapshotted rent, guild/account escrow parts,
  HELD/WON/RELEASED, revision and last operation identity. Active parts sum to max + rent;
  terminal parts are zero. SQL enforces World/Character/Guild World consistency.
- Raising draws Guild escrow first; lowering returns Account first. Headroom is the sum of
  HELD parts, never an assumed zero. Ceilings: BANK0-RL-01 (guild) and HOUSEOWN0-RL-13 (Account).
- Lock order (GUILD0-LO-01): occurrence → Character → Guild → member → property →
  auction/bid → ban → Guild balance → Account balance. World release jobs take no Character
  root/session lock and recheck node possession plus a Channel assignment in that World.
- Each step writes an immutable same-xid operation, advances the bid revision and calls the
  GUILD-BANK participant so both contributors use their native ledger chains. Terminal
  custody and operations cannot be deleted or truncated.
- Release: consumes all original custody once regardless of later leaders; replay reads the
  retained outcome without restoring authority; a changed occurrence binding or stale
  predecessor cannot create another value grant. An empty HELD row (zero-rent bid lowered to
  zero) terminalizes with a revision advance and zero ledger entries; only `BID_RELEASE`
  admits that transition.
- Disband participant: Hall seals its occurrence lock and `pg_current_xact_id` before the Guild
  lock; one HELD bid per Guild per pass; returns `WaitingForHouseDisposition` after bid
  progress. `game_guildhall_assert_disband_released(UUID,UUID)` rejects remaining HELD bids or
  a bad Guild World binding and, with the D1167 delegate, guards Bank payout and terminal state.
  Never caught inside the refund transaction.
- Release batching keeps GUILD0-RL-14 (50 bids / 200 lines); one bid uses at most four value lines.
- Draft evidence (historical): 11/11 native PG 17.6 bodies on the bid refund participant
  before D1167; the D1167 successor was not compiled or run.

## Findings and open questions

- Still missing (owned elsewhere, required before activation): House catalogue/auction
  admission and settlement, Guild property/slot/ACL/fence final release, rent/grace/burn,
  eviction → Guild-only ban, rank-derived ACL via HOUSE-ACL-1, canonical occurrence →
  CommandRef binding, same-transaction admitted House/Guild/Bank events and
  HousePrice/HouseRent burn arms.
- Auction/property/ban rows used by the draft tests were fixture data, not House writer proof.

## Acceptance criteria

- [ ] Split escrow refunds each contributor exactly once.
- [ ] Disband releases the Hall and pays out via the ledger (release before payout).
- [ ] Guild ACL derives from rank.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (pure split arithmetic in `housing`, storage in `durability`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing guildhall
cargo test -p oteryn-gameplay-durability guildhall
cargo test -p oteryn-game-server --test guildhall_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`
- `docs/contracts/OTERYN_GAME_GUILDS_V1.md`
- `docs/reference/tibia-manual/guilds.md`
- `docs/agents/tasks/archive/OTV2-20261010-guild-0-contract.md`
- CP coordination: #2131 (D1098, D1100, D1102, D1122, D1167)
