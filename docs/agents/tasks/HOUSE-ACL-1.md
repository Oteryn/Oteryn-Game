# HOUSE-ACL-1 — house ACL tables and revisioned edits (consolidated task card)

```yaml
task_id: HOUSE-ACL-1
title: "HOUSE-ACL-1: house ACL tables and revisioned edits"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "hard (security review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row HOUSE-ACL-1 (row 121); App. 7 HOUSE-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md        # §9-§10
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md      # §4.1, §10, LO-01
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md                    # §1.2
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md # §5.1 door decision
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md                        # §0.1
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true   # ACL caps in RESOURCE_LIMITS_REGISTRY; House event; protocol only if confirmed
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2140, branch skeleton/house-acl-1

## Goal

House ACL (guests, subowners, exclusions, guild entries and per-door lists) with revisioned
edits per HOUSE-OWN-0 §10. One ACL source of truth is read by door, spell and GUI consumers;
only owner/subowner may edit, checked server-side; every edit carries a revision and stale
revisions are rejected.

## Dependencies (re-verified on origin/main, 2026-10-22)

| Dependency | State |
|---|---|
| ACCEPT-SOCIAL-MAP-0 | MERGED (#1771) |
| HOUSE-RUNTIME-1a (house scope/handoff) | MERGED (#2163) |
| HOUSE-1a (canonical `game_house_properties`, lifecycle, composed `game_house_access` body) | BLOCKING — re-implementation pending (draft #2137 closing) |
| GUILD-1 Guild rows/membership/ranks | MERGED (#2136) as checkpoint, but Guild core sources were dropped by the ADR-0022 move (#2280); must be re-landed before guild-owned ACL entries go live |
| MAP-OVERLAY-1c (`QualifiedHousePlacement::door_cell`, generation-qualified door consumer, D1348) | BLOCKING for the door consumer — re-implementation pending (draft #2167 closing) |
| HOUSE-RUNTIME-1b (named-House session fence / admission rebind) | BLOCKING for House-scoped edits — re-implementation pending (draft #2165 closing) |

## Owned paths (ADR-0022 layout)

- `crates/gameplay/housing/src/house_acl.rs` (new, L3 pure rules: entry bounds, role hierarchy, exclusion and owner-first composition)
- `crates/gameplay/housing/src/house_acl_tests.rs` (new)
- `crates/gameplay/durability/src/durability/house_acl.rs` (new, Root writer/reader, revision CAS, receipts)
- `crates/gameplay/durability/src/durability/house_acl_tests.rs` (new, native PG cases)
- `docs/agents/tasks/HOUSE-ACL-1.md`

Slot-only (final commit at the queue turn): the ACL migration (number: next free number at
allocation time (only one migration PR in the queue)); `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
(ACL caps); `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (House event);
`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` only if a wire is confirmed; module
registration in `crates/gameplay/housing/src/lib.rs` and
`crates/gameplay/durability/src/durability/core_modules.rs` / `lib.rs`.

## Wiring points (verified on origin/main)

- `crates/gameplay/durability/src/durability/house_spell_acl.rs` (0038 spell `house_access`, per-list ACL, wildcard text)
- `crates/gameplay/dispatch/src/gameplay_transport/map_door.rs` (door access)
- `crates/gameplay/content/src/content/project/native_house_tiles.rs` (`QualifiedHousePlacement::has_door` / `door_at`; `door_cell` is not on main — MAP-OVERLAY-1c owns it)
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain and `capabilities.rs` `GATED` table (no capability offered until CP decides)
- HOUSE-WIRE-1 maps the ACL edit/read to the wire after this lands.

## Accepted decisions carried from the draft

- **Data model (uninstalled proposal):** canonical `game_house_properties` (identity, owner kind
  Character+Account or Guild, lifecycle, property `revision`, global `acl_revision`) is HOUSE-1a's;
  `game_house_acl_entries` stores bounded Character / exclusion / guild entries by role or native
  door position. An ACL operation occurrence is bound one-to-one to a native CommandRef and the
  exact request bytes, with an immutable result/change receipt.
- A successful replacement and its receipt commit in one transaction, increment the global ACL
  revision exactly once and never advance CharacterRevision. Refusal/conflict/rollback writes nothing.
- A historical receipt is data only: replay under a stale generation returns the receipt, a new
  write under that generation is rejected; a receipt never regains current authority after
  owner/session replacement.
- **Lock order (GUILD0 LO-01):** recovery/admission → operation occurrence → acting Character root
  → required guilds sorted by GuildId → member rows → property → ACL entries. Guild sources are
  read unlocked, then reread under locks; a guild discovered missing after the property lock means
  rollback/retry, never a late guild lock.
- **Nullable owner:** a property `owner_kind` is NULL for VACANT, AUCTION and RETIRED; reader and
  writer decode `Option<String>` and return typed `AclError::NotOwner` with no receipt, no outbox
  row and an unchanged ACL revision.
- **0038 legacy:** native spell tables (`game_house_ownership`, per-list `game_house_acl`, 100
  members, wildcard text) are not silently reinterpreted as the canonical model. Native
  `game_house_access` stays a fail-closed stub; HOUSE-1a owns its composed body and the grant-part
  signature, and this task replaces only the grant part at its slot. No second live ACL truth.
- **Door grant:** `door_list_granted` is only the selected list's grant fact, not the full
  HOUSE-RUNTIME0 §5.1 door decision. Absent or empty lists give no grant; the runtime resolves
  the admitted door and its restrictive list itself. No caller boolean or guessed catalogue.
- Guild owner resolution uses the same locked member/rank rows. Exclusions apply to non-owner
  grants; owner-first composition bypasses them.
- **D1348 door consumer:** the consumer binds the same World/House instance row SHARE-locked in
  the caller transaction and compares both `qualified_placement_digest` and
  `compatible_content_digest` against the active qualified tiles; only a positive legacy
  `list_id` qualifies; missing/mismatched/ambiguous falls back to legacy behaviour; wildcard,
  receipts and FKs stay unchanged; add `Door(TilePos)` beside legacy.
- Held proposal (MAP-OVERLAY-1c scope): `door_cell(id) -> Option<LogicalCell>` returns the sole
  matching cell, `None` for absent or duplicate numbers independent of map order, with three unit
  tests (absent, unique round-trip, duplicate both insertion orders). Not applied by this task.
- No supplied owner/consent/current boolean or row-derived controller proof is accepted. The
  native Channel session fence is the initial entry port; named-House session rebind must consume
  the shared native fence, never a relaxed predicate.

## Reference behaviour and qualification (draft result)

- Pure (4): 200/201 entry bound and duplicates; EXACT / AT_LEAST / removal / disbanded rules;
  owner-first hierarchy; native max door coordinates.
- Native PG (5): property-global revision CAS with two real Roots/pools and one typed stale loser;
  exact occurrence replay/conflict; receipt-insertion failure rollback; immutable receipt/outbox and
  unchanged CharacterRevision; owner/subowner/exclusion permissions; independent Character/World/
  Channel/connection/lease/scope/node/session-binding rejection; guild leader/rank/state reads;
  `native_acl_unowned_property_returns_not_owner_without_receipt_or_decode_error` (VACANT, AUCTION,
  RETIRED through both Root entrypoints).
- Draft: 9/9 PASS on a source assembly; an earlier run failed on private property-DDL syntax and
  three Clippy lints. Not exact-head or production qualification.

## Findings and open questions

- The test property/guild schema is a minimal fixture, not HOUSE-1a's lifecycle or GUILD-1's
  rank mutation; runtime-role grants, definer-only mutation, House event emission and activation
  are not qualified by these tests.
- Remaining integration criteria: HOUSE-1a lifecycle/current ownership; GUILD-1 lifecycle/rank
  owner; composed access function with unchanged native signature; House-session fence/admission
  rebind; restricted-door/catalogue composition; shared spell/GUI/door/runtime revalidation and
  notification source; registered House event/audit envelope; definer-only runtime grants and
  immutable mutation guards. The private outbox is candidate evidence only.
- Stored access observations authorize no relocation, item or socket action by themselves.

## Acceptance criteria

- [ ] Every ACL edit carries a revision; a stale revision is rejected with a typed loser.
- [ ] Only owner/subowner may edit; checked server-side from locked current rows.
- [ ] Door and spell access read the same ACL.
- [ ] Unowned properties return `NotOwner` without receipt or decode error.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] One mechanic per module file in its ADR-0022 layer.
- [ ] New tests in their own child files.
- [ ] Edits to mod.rs/lib.rs/connection.rs/capabilities.rs are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing house_acl
cargo test -p oteryn-gameplay-durability house_acl
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md §10
- docs/agents/tasks/archive/OTV2-20260929-house-doors.md
- docs/reference/tibia-manual/houses.md
- Draft #2140 (CP decisions D1228, D1345, D1348, D1420 on PR #2131)
