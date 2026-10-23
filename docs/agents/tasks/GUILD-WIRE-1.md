# GUILD-WIRE-1 — GUILD_V1 capability, commands and emblem (consolidated task card)

```yaml
task_id: GUILD-WIRE-1
title: "GUILD-WIRE-1: GUILD_V1 capability, commands, emblem"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "impl (protocol review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-WIRE-1 (row 113); App. 7 GUILD-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md   # §3.1-3.2, §4.2-4.3, §5.1, §9-11
  - docs/contracts/OTERYN_GAME_GUILDS_V1.md   # candidate; Q1 not accepted
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # PROTOCOL_OTERYN_V1_REGISTRY capability/command/result IDs, RESOURCE_LIMITS emblem row
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Guild core exists; no GUILD_V1 codec/commands from this card are present. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Implement the guild wire, registry/capability, native writer adapters and tests together. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2159, branch skeleton/guild-wire-1

## Goal

Capability `GUILD_V1`, the guild command/result/query wire, the emblem list, and the
guildhall variants of `HOUSE_INTENT`. The wire carries untrusted request data only; all
authority stays with the durable Guild/Guild bank/Hall writers.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| GUILD-1 (writers, occurrence/replay receipts, activity/query owner) | MERGED (#2136) as unregistered checkpoint; sources removed by #2280 (d25623a36) as dead code — BLOCKING until the Guild core is re-landed (`docs/agents/tasks/archive/GUILD-1.md`) |
| GUILD-BANK-1 (deposit/withdraw/claims) | BLOCKING — re-implementation pending (draft #2139 closing) |
| GUILDHALL-1 (guildhall bid/move_out, guild ACL/door variants) | BLOCKING for the Hall variants — re-implementation pending (draft #2153 closing) |
| HOUSE-WIRE-1 (`HOUSE_INTENT` / `HOUSE_QUERY` owner) | BLOCKING for the Hall variants — re-implementation pending (draft #2145 closing) |
| GUILD-CHAT-1 | open draft #2142; not a prerequisite of this wire |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/protocol-oteryn/src/guild.rs` (new, codec)
- `crates/protocol-oteryn/src/guild_tests.rs` (new)
- `docs/contracts/protocol-oteryn/v1/guild_v1.proto` (new)
- `crates/gameplay/dispatch/src/gameplay_transport/guild_intent.rs` (new, L5 request adapter returning untrusted data)
- `crates/gameplay/dispatch/src/gameplay_transport/guild_intent_tests.rs` (new)
- `docs/agents/tasks/GUILD-WIRE-1.md`

Slot-only (final commit at the queue turn): `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`,
`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`;
`crates/protocol-oteryn/src/lib.rs` `pub mod` + registry constants and
`capability_tables.rs` rows; `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`,
`connection.rs`, `capabilities.rs` wiring (capability not offered until CP decides).
No migration.

## Wiring points (verified on origin/main)

- `crates/protocol-oteryn/src/lib.rs` (4794 lines) — strict wire helpers, foundation UUIDv7 decoder; capability ids end at 30 on main
- `crates/protocol-oteryn/src/capability_tables.rs` (`GATED`, `SUPERSEDED`)
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (8261 lines) command chain `else if command.command_type == COMMAND_TYPE_...`
- `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` offered sets
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs` — sibling adapter pattern
- `crates/gameplay/domain/src/domain/character_name.rs` (`CharacterName`), `crates/gameplay/durability/src/durability/chat_text.rs` (`ChatText`) — reuse, no second normalizer
- `docs/contracts/protocol-oteryn/v1/foundation.proto`, `chat_v1.proto`

## Accepted decisions carried from the draft

- GUILD0 is accepted through ACCEPT-SOCIAL-MAP; `OTERYN_GAME_GUILDS_V1.md` remains candidate.
  CP confirmed no new acceptance of its Q1: no `decline` command and no other-guild name/rank
  badge expansion.
- Proto field/enum tags in the new schema are local; global capability, command,
  state-domain and error numbers are allocated by CP at the slot.
- Finding `GUILD-WIRE-P2-CURSOR-STORAGE` (accepted, fixed on the draft): invitation key is
  `(GuildId, CharacterId)` (§3.1), claim row key is `GuildId` (§5.1); cursors use those UUID keys.

## Reference behaviour built by the draft (re-implement, do not copy)

- Fifteen accepted guild command bodies, sixteen closed guild result variants, query
  sections/selectors, bounded keyset data and the `NONE` / `OWN_GUILD` / `OTHER_GUILD` emblem list.
- Fail closed on encode and decode: unknown/duplicate fields or oneofs, invalid enums, UTF-8
  or UUID shape, wrong wire types, missing required fields, excess sizes/counts. No World,
  Account, Channel, node or actor in any body.
- Edits carry nonzero expected revisions; rank count 3–20; amounts within the accepted bank
  ceiling; rank-name byte budget derived from the 512-byte projected entry budget. The durable
  writer still applies the canonical `0022 name_key` uniqueness rule.
- Adapter: reuses `CharacterName` and `ChatText`, checks title/message limits, keeps explicit
  empty metadata as data, and hashes the canonical original body before normalization. The
  digest is not an occurrence, operation ID or receipt.
- Emblem: native opaque 16-byte visibility identity + nonzero generation, one entry per
  identity, at most 256 entries; 31 bytes/entry, 8,448 bytes/list. Codec only.
- Cursors: guild-scoped cursors name their Guild; Account+World disband claims return
  `AccountWorldClaimsContextRequired` until the query owner supplies the Account+World
  context. Roster key = rank level then Character UUID; activity key =
  `occurred_at_unix_ms` + event UUID (never reinterpret the unit). Cursor data grants nothing.
- Draft evidence (historical): protocol tests 6/0, Clippy clean, adapter harness 4/0; no live
  dispatch or PostgreSQL run.

## Findings and open questions

Integration still owned elsewhere and required before activation:
1. GUILD-1/GUILD-BANK-1 writers with occurrence/replay receipts, current
   recovery/session/root/Account/World proofs, role/Premium/deadline checks and GUILD0-LO-01.
   Guild operations advance the guild revision, never `CharacterRevision`.
2. Query/claims owners: section privacy, page projections (100 entries, ≤512 encoded bytes
   each), keysets and `next_cursor` under the 64 KiB result bound.
3. Own name/rank and visibility-derived emblem updates; Q1 other-guild badges stay unresolved.
4. Hall bid/move_out and guild ACL/door variants through the real `HOUSE_INTENT` owner.

## Acceptance criteria

- [ ] Commands are rejected without `GUILD_V1`; older clients are unchanged.
- [ ] Emblem list bounded in RESOURCE_LIMITS.
- [ ] Wire + registry + capability + handler + tests land together.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (codec in `protocol-oteryn`, adapter in `dispatch`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs`), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`protocol-oteryn/src/lib.rs` and `connection.rs` are already over).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-protocol-oteryn guild
cargo test -p oteryn-gameplay-dispatch guild_intent
cargo test -p oteryn-gameplay-dispatch capabilities
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`
- `docs/contracts/OTERYN_GAME_GUILDS_V1.md`
- `docs/reference/tibia-manual/guilds.md`
- CP release: #1622 comment 6193914573
