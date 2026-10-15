# GUILD-WIRE-1 — GUILD_V1 capability, commands, emblem (skeleton task record)

```yaml
task_id: GUILD-WIRE-1
title: "GUILD-WIRE-1: GUILD_V1 capability, commands, emblem (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/guild-wire-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row GUILD-WIRE-1 (row 113); App. 7 GUILD-*; §5 Wave 5"
class: "impl (protocol review)"
needs_migration: no
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "crates/protocol-oteryn/src/guild.rs (new) + guild_tests.rs"
  - "apps/game-server/src/gameplay_transport/guild_intent.rs (new) + tests"
  - "docs/contracts/protocol-oteryn/v1/guild_v1.proto (new)"
  - "docs/agents/tasks/GUILD-WIRE-1.md"
slot_only_paths:
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
  - "crates/protocol-oteryn/src/lib.rs"
  - "apps/game-server/src/gameplay_transport/connection.rs"
  - "apps/game-server/src/gameplay_transport/mod.rs"
  - "apps/game-server/src/gameplay_transport/capabilities.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Capability `GUILD_V1`, guild commands, emblem, and guildhall variants of `HOUSE_INTENT`.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| GUILD-1 | NOT MERGED | **BLOCKING** |
| GUILD-BANK-1 | NOT MERGED | **BLOCKING** |
| GUILDHALL-1 (guildhall variants) | NOT MERGED | **BLOCKING** |
| HOUSE-WIRE-1 | NOT MERGED | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- crates/protocol-oteryn/src/guild.rs (new) + guild_tests.rs
- apps/game-server/src/gameplay_transport/guild_intent.rs (new) + tests
- docs/contracts/protocol-oteryn/v1/guild_v1.proto (new)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
- crates/protocol-oteryn/src/lib.rs
- apps/game-server/src/gameplay_transport/connection.rs
- apps/game-server/src/gameplay_transport/mod.rs
- apps/game-server/src/gameplay_transport/capabilities.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/connection.rs dispatch
- HOUSE_INTENT handler (HOUSE-WIRE-1)
- apps/game-server/src/gameplay_transport/capabilities.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] commands rejected without GUILD_V1; older clients unchanged
- [ ] emblem bounded in RESOURCE_LIMITS
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-protocol-oteryn guild
cargo test -p oteryn-game-server capabilities guild_intent
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`
- `docs/contracts/OTERYN_GAME_GUILDS_V1.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded owned implementation preparation — 2026-10-15

The active control plane released authoring of these six exact owned paths under
`#1622 issuecomment-6193914573` (local allocation
`/workspace/oteryn-prep/guild-wire-1-allocation.json`). This is preparation of the
existing draft, without production registration or activation. The original plan,
builder restrictions and every acceptance checkbox above remain in force.

The owning source is GUILD0 §3.1–3.2, §4.2–4.3, §5.1, §9–11, accepted through
ACCEPT-SOCIAL-MAP despite its historical CANDIDATE heading. The separate
`OTERYN_GAME_GUILDS_V1.md` remains candidate; CP confirmed no new acceptance of its
Q1. This component does not add its `decline` command or other-guild name/rank badge
expansion. Local protobuf field/enum tags in the new owned schema are proposals,
not allocations of global capability, command, state-domain or error numbers.

### Actual source component

- `protocol-oteryn/guild.rs` reuses the crate's strict wire helpers and foundation
  UUIDv7 decoder. It provides all fifteen accepted guild command body variants,
  sixteen closed guild-specific result variants, query sections/selectors,
  bounded keyset data and the NONE/OWN_GUILD/OTHER_GUILD emblem list. Unknown or
  duplicate fields/oneofs, invalid enums/UTF-8/UUID shape, wrong wire types,
  missing required fields and excessive sizes/counts fail closed on encode and
  decode. No World, Account, Channel, node or current actor is supplied by a body.
- Edits carry nonzero expected revisions. Rank count is 3–20; amount bounds use
  the accepted bank ceiling. The rank-name byte budget is derived from the
  accepted 512-byte projected entry budget with this proposed rank-entry framing,
  not a newly accepted naming grammar/resource-registry row. The durable writer
  must still apply its canonical `0022 name_key` and uniqueness rule; no second
  key normalizer was copied here.
- `gameplay_transport/guild_intent.rs` returns untrusted request DATA. It reuses
  actual `CharacterName` and `ChatText`, checks title/message scalar limits and
  retains explicit empty metadata data without granting an edit. It hashes the
  canonical original body before text normalization, so distinct original texts
  cannot silently share a body binding merely because they trim alike. This
  digest is not an occurrence, operation ID, receipt or replacement for the real
  writer's actor/World/request binding. No supplied bool/current/role/Premium
  callback, money writer or durable guild transition exists in this adapter.
- Emblem entries reuse the native opaque 16-byte visibility identity and nonzero
  generation, one entry per identity, at most the native 256 visibility entities.
  Their fixed framing yields 31 bytes per entry and 8,448 bytes per list. This is
  only the codec; membership resolution, current visibility, viewer-relative
  selection, changes across Channels and the accepted own guild name/rank domain
  remain with the real owners at CP. No other-guild names, title or position are
  encoded.

### Cursor identity repair and explicit owner seams

Root finding `GUILD-WIRE-P2-CURSOR-STORAGE` was accepted before freeze: the first
proposal incorrectly assumed numeric invitation/claim row IDs. The accepted
invitation key is `(GuildId, CharacterId)` (§3.1); the claim row is keyed by
`GuildId` (§5.1). The corrected cursor uses those UUID keys. A guild-scoped cursor
names its Guild, while Account+World disband claims have no invented selected
Guild; their after-key is the actual Guild UUID. The guild-binding helper refuses
claims. The server explicitly returns `AccountWorldClaimsContextRequired` for a
claims cursor until the genuine query owner supplies its current Account+World
context/acceptance contract. This prepares key DATA and does not claim complete
accepted claims pagination or authorize any read.

The roster key is rank level then Character UUID. Activity carries the existing
game-event foundation `occurred_at_unix_ms` plus event UUID; GUILD-1's actual event
query owner must qualify exact timestamp conversion, descending keyset order and
the current 30-day window. Never reinterpret a native timestamp in another unit.
The query owner must independently resolve the selected guild, authorize each
section and compare the cursor binding; client cursor data alone grants nothing.

Concrete CP work still required, rather than replaced by detached fake writers:

1. Merge/bind the actual GUILD-1/GUILD-BANK-1 writers, native occurrence/replay
   receipts, independent current recovery/session/root/Account/World proofs,
   role/Premium/deadline checks and GUILD0-LO-01 transactions. Guild operations
   advance guild revision, never CharacterRevision.
2. Bind the actual query/claims owners; implement and qualify section-specific
   privacy, page projections (100 entries, each at most512 encoded bytes), actual
   keysets and next_cursor under the 64KiB result bound. No full read model, public
   projection or GC-24–26 runtime qualification is claimed by body parsing.
3. Compose accepted own name/rank and visibility-derived emblem updates. Q1
   other-guild badge names/ranks remain unresolved and are not silently accepted.
4. Compose guildhall bid/move_out and guild ACL/door variants through the actual
   HOUSE_INTENT owner. That native body is absent at this exact base; no competing
   House schema or generic House/Guild authority adapter is introduced here.
5. Allocate/review the global registry IDs/resource rows, register these modules,
   select/offer the capability and wire dispatch only in the CP slot after the
   original dependencies. Client/codegen/pins/migrations remain unmodified.

### Qualification boundary

Focused conformance fixtures cover literal wire bytes, all accepted command/result
variants, truncation/unknown/duplicate input, exact and excessive byte/count/value
bounds, real UUID key shapes, cross-guild/section mismatches, claims-context refusal,
opaque visibility identities and native name/text normalization/binding behavior.
The initial real compiler error `E0521` in the new local field-reader helper is
preserved as a code compilation failure, corrected by tying its borrowed slices
to the input lifetime. It is not labeled a test RED or a fixture/setup failure.

Final execution counts/statuses are recorded in
`/workspace/oteryn-prep/guild-wire-1-evidence/` after qualification. The intended
proof separates ordinary protocol-crate Cargo tests/Clippy, actual untransformed
whole-native unit-context metadata/Clippy, and the approved small DATA-parser
harness. That harness uses literal unchanged native ChatText constants/type/impl
and the complete unchanged CharacterName source, exact owned adapter/tests and
real newly built protocol/SHA-256 dependencies, with source hashes and excerpt
ranges. It substitutes no authority implementation and does not qualify a native
server runtime, current guild read/write, live capability gating or socket dispatch.

Final exact-source qualification: ordinary protocol Cargo tests **6 passed/0 failed**
(242 existing cases filtered); protocol all-targets Clippy **0**; owned Rust and
workspace formatting **0**. Actual unchanged whole-native unit-context metadata
Clippy passed with `-D warnings`, including the exact adapter/test source and real
new protocol dependency. The approved literal native DATA-parser harness executed
**4 passed/0 failed** (3 unchanged CharacterName controls filtered). Governance
validated22 required documents/9 lanes and **124 passed/0 failed**. No full server
binary, global native unit ELF, PostgreSQL test or live dispatch ran for this task.

The wrapper's initial missing `std::fmt` import was a retained harness setup
failure, corrected without any candidate Rust change. Dependency hash guards then
stopped execution before using a protocol rlib overwritten by another shared-target
job. An exact snapshot-only mtime refresh forced genuine Cargo dependency rebuild;
fresh actual whole-native Clippy passed under the same lock, then byte-bound real
protocol/SHA-256 artifacts were retained at unique paths before the harness ran.
There was no artifact/type substitution. The deliberate native Cargo sentinel
exit86/Cargo101 captures are dependency setup, never a successful Cargo test result.

All four Rust sources and the proposed proto are frozen at the executed hashes in
`source-manifest.json`; source/snapshot/excerpt/command and actual output bindings
are preserved beside it. This completes only the bounded owned preparation. Every
original acceptance checkbox and all real CP/dependency/owner integration gates
remain open; only root may publish this preparation to the existing draft.
