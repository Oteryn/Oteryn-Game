# HOUSE-1a — house ownership, auction and rent (skeleton task record)

```yaml
task_id: HOUSE-1a
title: "HOUSE-1a: house ownership, auction and rent (skeleton)"
mode: IMPLEMENT
status: guild_disband_port_authored_pending_contract_and_native_qualification
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-1a
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-1a (row 119); App. 7 HOUSE-*; §5 Wave 5"
class: "hard (persistence, economy, security)"
needs_migration: yes
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house/ (new: ownership, auction, rent)"
  - "apps/game-server/src/durability/house_ownership.rs (new)"
  - "apps/game-server/src/house/*_tests.rs"
  - "apps/game-server/tests/house_ownership_*.rs"
  - "docs/agents/tasks/HOUSE-1a.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_house_ownership.sql (number leased by CP at queue turn)"
  - "apps/game-server/src/lib.rs / mod.rs module wiring"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

House ownership, auction (bid escrow) and rent per HOUSE-OWN-0 §3-§9 and SOCIAL_MAP_PACKETS HOUSE-1a.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BANK-1 (NPC-ECON-1) | MERGED (archive OTV2-20261004-bank-1) | ok |
| HOUSE-RUNTIME-1a | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-RUNTIME-1b | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-RUNTIME-1c | NOT MERGED (Wave 3) | **BLOCKING** |
| PREM-WIRE-1 (PREMIUM-1) | MERGED (#2110) | ok |
| HOUSE_OWN0 acceptance | ACCEPTED by ACCEPT-SOCIAL-MAP-0 #1771; current main status reconciled | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house/ (new: ownership, auction, rent)
- apps/game-server/src/durability/house_ownership.rs (new)
- `apps/game-server/src/house/*_tests.rs`
- `apps/game-server/tests/house_ownership_*.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_house_ownership.sql (number leased by CP at queue turn)
- apps/game-server/src/lib.rs / mod.rs module wiring
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/house_scope/mod.rs, handoff.rs (house scope)
- apps/game-server/src/durability/world_house_instance.rs
- apps/game-server/src/content/project/native_house_tiles.rs
- bank writer from BANK-1 (archive record)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- mint/burn + ledger (bid escrow, rent debit through the bank ledger)

## Acceptance criteria

- [ ] auction bid escrow and refund are exactly-once under the session fence
- [ ] rent debit idempotent per period; eviction path deterministic
- [ ] Premium gate fails closed
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house
cargo test -p oteryn-game-server --test '*house*' (Postgres e2e)
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md §3-§9
- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (HOUSE-1a)
- `docs/reference/tibia-manual/houses.md`
- `docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local preparation (2026-10-15)

- Existing draft #2137, `skeleton/house-1a`, inspected source
  `212540a23cfb5f1654367567ee7b9bba29f0e07d`. This preparation is restricted to
  `src/house/auction.rs`, its `auction_tests.rs`, and this record. No module registration,
  migration, registry, runtime, content pin, regeneration, commit or publication is included.
- Accepted basis: HOUSE-OWN-0 §4 and §12, accepted by ACCEPT-SOCIAL-MAP-0 #1771
  (the acceptance document explicitly names `HOUSE-OWN0-PHYSICAL-HOUSE-OWNERSHIP-V1`),
  and SOCIAL-MAP-PACKETS-1 §1.3, §1.9 and §2.3. The older brief's CANDIDATE header does
  not introduce an ACCEPT-BATCH #2207 dependency for this allocation.
- Scope discrepancy: this skeleton broadly mentions rent/grace. The accepted §1.3 split
  assigns ongoing rent collection, grace, warnings, eviction, move-out and disposition to
  HOUSE-1b. HOUSE-1a covers acquisition/auction and the first rent reserved with a bid.
  The packet locates pure rules under `domain/house_auction.rs`; the local allocation instead
  owns `house/auction.rs`. Final placement/registration remains a CP-slot decision.
- Prepared pure subset: proxy price and chronological maximum ties, the 256-bidder bound,
  maximum-plus-snapshotted-first-rent arithmetic and raise/lower deltas, escrow-return
  headroom arithmetic, seven-day opening and fifteen-minute anti-sniping deadlines.
  Existing `domain::AccountId` and bank-style `u64` gold are reused. No new identity,
  money wrapper, table, schema, authority proof, escrow writer or host action is invented.
- A quote is a calculation over the supplied qualified bid set, not settlement authority.
  The future writer must independently check current Character/Account/World binding,
  property state/ownership, housing slot, ban, eligibility/Premium, held escrow and bank facts
  under the accepted lock/fence order, then update escrow, balance, ledger and event atomically.
  Settlement must exclude currently ineligible bids before calculating the proxy price.
  Maximum-reached order is a serialized durable input, not a client timestamp or tie fallback.
  The pure tests cover input ordering, not actual concurrent writers or exactly-once durability.
- `cross_domain_finding: HOUSE-1a-BANK-HEADROOM`: SOCIAL-MAP §1.12 requires the one shared
  credit-headroom check and architect escalation if a credit path does not use it. On this
  supplied source, `durability/bank.rs` deposit and receiver-transfer paths directly apply
  `checked_add`/`BANK_BALANCE_MAX` (around lines 1473 and 1781), with no shared headroom
  helper; `bank_audit::BANK_BALANCE_MAX` is still 999,999,999,999. The detached arithmetic
  does not implement the RL-15 deferred guard, change that ceiling or integrate any credit.
  CP/architect must resolve and allocate this bank seam before acquisition integration.
- HOUSE-RUNTIME-1a/1b/1c and the exit gate remain runtime/slot dependencies. The detached
  subset assumes no unmerged schema. Premium proof consumption and batch-gate-before-lock
  wiring (§1.4), nomination/eligibility, one bid/slot per Account+World, property/tile guards,
  escrow custody, settlement, release jobs, replay bindings, audit, migrations and resource
  registration remain incomplete. No auction can open from this detached module.

### Source-accurate local harness

`/workspace/oteryn-prep/house-1a-evidence/harness.rs` includes the allocated auction source
by path and the verbatim existing `domain/mod.rs` prefix containing `DomainError`, UUIDv7
validation and the existing strong-ID definitions. The harness adds no replacement identity
implementation. Its extraction bounds and hashes are retained in `harness-provenance.txt`:

- Existing domain source SHA-256:
  `93732d431fddb5f2ca6aefc95cc1c61d5b9c513d8110e848d52e6e31762f60df`.
- Verbatim prefix SHA-256:
  `5bac8f9bdf403a115c84d6201dceb8350bfb91c64fb4f91620806fcf9999f0f1`.
- Harness SHA-256:
  `8d1c33564c46e13d5d948e48a8b61b3ffe989d9a887c599b792d1f0b50faaef6`.

Focused formatting, `rustc --test` and `clippy-driver --test --emit=metadata -D warnings`
use Rust 1.94.0 under `flock /tmp/oteryn-prep-rust.lock`, shared
`CARGO_TARGET_DIR=/workspace/oteryn-prep/cargo-target` and `CARGO_BUILD_JOBS=2`.
These qualify only the detached source subset; package-wide checks, PostgreSQL scenarios,
independent exact-head review and final CP/architect/slot acceptance remain pending.

Local result: **10 passed, 0 failed, 0 ignored**, covering proxy/no-reserve pricing,
six input-order permutations and changed-maximum chronology, duplicate/ambiguous inputs,
256/257 bidders, reserve/raise/lower conservation, gold ceiling/overflow, escrow headroom,
closed-at-deadline/anti-sniping boundaries and time overflow. Focused `rustfmt --check`
and `clippy-driver -D warnings` passed. Governance validation passed (22 documents, 9 lanes),
governance unit tests **124 passed**, and `git diff --check` passed. No package-wide or
PostgreSQL result is claimed for these unwired rules.

Prepared source SHA-256:

- `auction.rs`: `0fa5ed8c22c5204d09a49139031d21bec45f22ba55537581c66a0c11b604ed34`.
- `auction_tests.rs`: `727b9289add1ce88939b881613aefcb2da68211bf8e7de7e0e0ed9f3465a12ab`.

Logs/provenance and the small standalone executable remain under
`/workspace/oteryn-prep/house-1a-evidence/` (about 6.2 MiB); no server binary was built.
Whole-diff self-review and owned-path check: only the two allocated Rust files and this
record differ; no shared wiring or durable authority/value mutation was prepared.

## ROOT-AJ-20261016-C — independent ordinary-property storage

Direct owner assignment, reiterated by CP comment
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6195906702,
authorizes this family writer without an additional lease number or a new CP decision.
This continuation starts from #2137 `e467fb5f0274ed0a8212e8fcedefd0671fbed215`.
The earlier detached-auction qualification above remains historical; it does not qualify
this new source or a future frozen candidate.

`apps/game-server/src/durability/house_ownership.rs` now prepares the independent
ordinary-property storage unit required by accepted HOUSE-OWN-0 §3 and HOUSE-TENURE
§4.2 / owner answer Q1a. Its SQL fragment creates canonical properties, World-global
tiles and one personal housing slot per Account+World. It preserves the legacy `0038`
ownership anchor used by `0047` instance bindings and immutable ACL receipts. The
canonical property and retained legacy row must agree at transaction commit; ownership
generation remains separate from property lifecycle and global ACL revisions. An
acquisition must succeed an auction with a new receipt; a release increments ownership
generation, clears both owners, retains the acquisition receipt and removes the slot
atomically. The legacy owner trigger is replaced to permit this NULL-owner release.
Neither the property nor its legacy anchor may be deleted. A retired property cannot
return to `VACANT` or `AUCTION`. Deferred slot checks cover both old and new property
keys, so moving a slot cannot strand its previous owned property.

This unit grants no runtime or control-role ownership writer. It provides no auction
settlement, bank debit, escrow, Premium proof, Game event or production installer. The
receipt UUID binds storage generations; it is not current acquisition authority. The
initial fragment is explicitly ordinary-house-only: guildhall mutation and its genuine
guild lifecycle proofs remain with that later owner. HOUSE-1a acquisition/first rent
does not absorb HOUSE-1b's ongoing rent, grace, eviction or disposition implementation.

Pre-existing legacy ownership is an exact import boundary: `0038` stores no first-rent
or `paid_until` facts. Installation refuses unreconciled legacy rows without changing
them. It does not infer tenancy dates, discard their receipts or replace immutable
instance bindings. Any actual import must supply accepted original tenancy evidence.

Five native PostgreSQL17.6 tests are prepared in the same owned module. They use the
unchanged native migration set and administrative native-root fixture seeding, with
all property/anchor/slot guards enabled for the actual cases. Cases cover durable
release/reload with instance and immutable receipt retention and CharacterRevision
unchanged; incomplete release rollback and independent anchor mutation/deletion
refusal; two real connections competing for one Account+World slot; terminal retirement;
and failclosed legacy import without invented rent. These are storage invariants,
not qualified session, Premium, economy or gameplay acquisition cases.

Validation of this continuation: official Rust1.94 `rustfmt` with edition2024 and
`skip_children=true`, and diff whitespace checks passed. Native compilation, Clippy,
all five PostgreSQL cases and RED/GREEN execution are **NOT_RUN**. D1011 requires
the approved PC executor/global slot; its current allocation/helper is pending, not
declared globally unavailable. Module registration and numbered migration installation
remain CP-slot work, so existing PR CI cannot qualify an unregistered source file.
No native checks, commit, push, shared-file changes, schema installation or paid review
were performed in this continuation. Source review is independent and remains distinct
from the mandatory native qualification before readiness.

The bank finding is scoped to live reserve/credit/settlement integration, not a blanket
block on this independent storage unit. SOCIAL-MAP §1.12 still requires resolving the
one shared credit-headroom seam before those acquisition operations are activated.
Remaining acquisition gates include current session/recovery fences, accepted runtime
exit dependencies, Premium before locks, native occurrence/economy/event evidence,
catalogue admission and tile/Ground guards, and canonical access-function integration.

### ROOT-C-D-TERMINAL-PORT — new isolated authoring generation

Explicit D1098/Root allocation adds the real caller-transaction Guildhall release port
in `durability/house_ownership.rs` and its private `house/guildhall_release.rs` child.
Source parent is frozen `c75a0dbe585c746e0f5fca3c1b4000527cb049d2`; that candidate,
its bundle and PC snapshot remain unchanged. This new generation is AUTHORING.

Canonical storage now distinguishes Guild ownership from Character/Account ownership;
a Guildhall occupies the unique Guild owner binding and never a personal Account slot.
The port consumes the genuine D `GuildDisbandTransaction`; it creates no replacement
Guild context, session, actor, pool getter or independent transaction. A source-derived
next-at-most-100-recipient reservation acquires sorted native deletion pins before D's
Guild spine and stamps protected evidence last. Every use checks the same physical
transaction and database marker. Actual Guild/spine locks are reacquired before property
and legacy-anchor locks because a D in-memory xid alone survives SAVEPOINT rollback.
Participant errors explicitly roll back a protected SAVEPOINT before returning.

The bounded House SECURITY DEFINER entrypoints grant runtime EXECUTE only. Protected
C tables, canonical ownership, ACL and BED receive no runtime DML grant. Their actual
writer captures an immutable native custody census, explicitly frees genuine BED rows
with `disposition`, transfers flat House items through native `game_character_inbox_deliver`,
retains immutable delivery/step evidence, deletes live ACL while preserving receipts and
editors, and clears canonical/legacy Guild ownership atomically with revision successors.
The immutable same-transaction terminal journal requires zero native House sources,
occupants, admission reservations, live ACL, personal slots and open sleepers. Only that
proof permits the typed Guild operation's terminal state5 and real fence release; ordinary
MOVE_OUT/DRAINED history remains independently fenced.

The real common item/entry/provenance/ItemInstance hooks are C2141 candidate
`d77b6a820472329dd452bc2ce3ab32aabc06cdfb`, ABI1. This writer checks all four enabled
native triggers and their actual functions/operation coverage before freeze. Its hooks
are not a parallel content flag; late native writers use the accepted-proposal NOWAIT
anchor fallback. The Guild hook itself acquires no earlier blocking root/Guild/property lock.

Focused tests are authored in `house/guildhall_release_tests.rs` for a private child mount
inside the genuine D2136 native fixture. Required production-source composition is
D2136 `caf3c375c02c97db1bb667f384529a110bcfe7d9`, D2153
`498e44e7ed20b91d121bc4d531cc7d3e49dce6de`, actual D2139 Bank source from the
verified eight-module `d-terminal-sources/manifest.json`, actual BED2147, and the C2141
candidate above. No fake Guild tables/context, fake Inbox, or replacement native functions
are used. Initial retained fixture state is administrative seeding; native release guards
are active for cases. The separate literal runtime-role case is not qualified by that seeding.
Twelve cases cover actual Inbox release/rollback/reload; 100+1 bounded work and Root reconstruction;
actual native frozen mutation boundaries; reservation and release SAVEPOINT invalidation;
D-context lock loss and real lock reacquisition; exact second-delivery fault with full
participant footprint conservation even if the caller commits after error; runtime direct
DML denial; missing native capability; changed recovery, foreign World and canonical
revision resnapshot. Native compilation and all focused cases are **NOT_RUN**.

Activation remains blocked on the explicitly proposed, unaccepted recipient-pin/late
NOWAIT lock-order amendment; D's actual occurrence/spine and pre-financial caller
composition; D's protected SQL terminal delegate/current-authority installation; genuine
D runtime table/authority grants (its current fragments grant none); and module/schema
registration plus native qualification. The directly callable definer writer independently checks the full Root-provided recovery
record, the actual admission lock set, secret-bearing native node possession, current World
assignment and actual Guild spine. It stamps protected immutable authority evidence only
after all those locks. Consumers recheck current facts and that same-transaction stamp;
SAVEPOINT rollback losing any of the locks also removes the stamp. Raw node secrets are
bound as a separate SQL parameter and are never serialized or retained. The Rust leaf
requires the separately proposed D context reference getters and native-owned private
secret accessor, neither of which is silently applied in these owned paths. Deferred Guild
revision changes require the actual same-transaction D step and complete event successor.
The protected Guild/world terminal SQL predicate supports D's deferred state3→4 check
only with current physical-transaction marker and actual release/absence, never history.
No migration installation, PC call, native execution, push or publication occurred.
