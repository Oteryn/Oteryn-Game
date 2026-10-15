# GUILDHALL-1 — guildhall bid, guild rent, ban, GUILD_DISBAND, guild ACL (skeleton task record)

```yaml
task_id: GUILDHALL-1
title: "GUILDHALL-1: guildhall bid, guild rent, ban, GUILD_DISBAND, guild ACL (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/guildhall-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row GUILDHALL-1 (rows 113, 119); App. 7 GUILD-*; §5 Wave 5"
class: "hard (persistence, economy, security)"
needs_migration: yes
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/guild/guildhall.rs (new)"
  - "apps/game-server/src/durability/guildhall.rs (new)"
  - "apps/game-server/src/guild/guildhall_tests.rs"
  - "docs/agents/tasks/GUILDHALL-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_guildhall.sql"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "docs/contracts/OTERYN_GAME_GUILDS_V1.md (amendment if needed)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Guildhall bid with split escrow, guild rent, guild ban, `GUILD_DISBAND` and guild ACL per GUILD0.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| GUILD-1 | NOT MERGED (Wave 4) | **BLOCKING** |
| GUILD-BANK-1 | NOT MERGED (Wave 4) | **BLOCKING** |
| HOUSE-1b | NOT MERGED | **BLOCKING** |
| HOUSE-ACL-1 | NOT MERGED | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/guild/guildhall.rs (new)
- apps/game-server/src/durability/guildhall.rs (new)
- `apps/game-server/src/guild/guildhall_tests.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_guildhall.sql
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- docs/contracts/OTERYN_GAME_GUILDS_V1.md (amendment if needed)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- guild tables (GUILD-1), guild ledger (GUILD-BANK-1)
- house ownership writers (HOUSE-1a/1b), ACL (HOUSE-ACL-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- mint/burn + ledger (split escrow, guild rent)

## Acceptance criteria

- [ ] split escrow refunds each contributor exactly once
- [ ] disband releases the hall and pays out via ledger
- [ ] guild ACL derives from rank
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server guildhall
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md`
- `docs/contracts/OTERYN_GAME_GUILDS_V1.md`
- `docs/reference/tibia-manual/guilds.md`
- `docs/agents/tasks/archive/OTV2-20261010-guild-0-contract.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## 2026-10-15 bounded native preparation packet

Exact lease: four original source/test/card paths on `skeleton/guildhall-1`, unchanged
base `5169621b26d085e204c93ccc65685748cd31529c`; accepted inputs are main
`ca5ea5930edf0d609e91d7a3f5436e7ca2fde610`. CP released HELD for private native
preparation; no migration, registration, content, shared wiring or runtime activation.
The earlier task instruction permitting shared registration edits is superseded by
this exact allocation. Root alone owns publication to the existing draft.

Native residue: 0038 has Character-only `game_house_ownership`; 0071/0073 Account bank
entries and FeeBurnCause remain closed. Prepared GUILD-1 exports only pure domain rules.
Concurrent GUILD-BANK, HOUSE-ACL and HOUSE1b work is proposed/unmerged, never installed
or current authority. HOUSE1b retains actual Character-only ownership and cannot be
used with a surrogate leader Character as Guild ownership. HOUSE-ACL owns the accepted
property/ACL extension and current guild-entry resolution; this task will not create
a second ACL validator or owner table.

The proposed useful financial component is split-bid custody with current native Root
recovery/session-generation/root checks and independently locked guild/leader/property/
auction facts, then real same-transaction guild and Account ledger participants. Its
private bid source has fixed funding Account/Character and two escrow parts, preserving
same-source refunds after leadership changes. Resizing draws guild first; lowering
returns Account first. Release is terminal and replay-safe. GUILD0-LO-01 remains the
combined order; headroom is the sum of HELD parts, never an assumed zero.

The own private `game_guildhall_bids` source shape is proposed, not an installed external
schema: bid/guild/World/House/auction identities, fixed funding Account and Character,
max and snapshotted rent, guild/account escrow, HELD/WON/RELEASED state, revision and
last operation identity. Active parts sum to max+rent; terminal parts are zero. Native
World/Character and proposed Guild World consistency must be enforced by SQL. The
GUILD-BANK owner will consume this one source for both mandatory reserve-headroom guards;
its real ledger hook and independently qualified candidate schema must compose before
any value writer is claimed executable. No boolean authority callback or fee-debit
substitution is admitted.

Cross-domain gates: native HousePrice/HouseRent FeeBurn audit arms and house/guild/bank
events remain closed; first-bid catalogue admission/auction creation and settlement
ranking belong to the real House owner; Guild ownership/ban/disposition must compose
with that owner and actual HouseInterior/Inbox proof, not a new independent property
store. Current membership and Guild ACL are coordinated with HOUSE-ACL. Raising an
existing bid after a leader Account change needs an accepted source-binding policy:
the fixed original Account cannot silently be debited by the new leader or replaced.
Until resolved, no such Account top-up is authorized. Rent, disposition/disband and
ban APIs must not claim integration through unavailable shared hooks.

All schema/authority fixture data is explicitly synthetic independent DATA if native
admission/claim pipelines do not exist; native guards and current proofs are never
relaxed. Qualification evidence starts in `/workspace/oteryn-prep/guildhall-1-evidence/`.
All original acceptance criteria remain open.

### Implemented component boundary (qualified private preparation; no activation)

`guild/guildhall.rs` contains checked split arithmetic used by the actual SQL component,
with the existing BANK0-RL-01 guild ceiling and HOUSEOWN0-RL-13 Account hard ceiling.
`durability/guildhall.rs::guildhall_escrow_financial_step` is a **private uninstalled
financial-component wrapper**, not success of `guildhall_bid`. It uses the genuine native
Root semantic pass/recovery capability, admission relation locks and independently current
native Character/session/assignment/incarnation/guard/root checks. Current leader membership
and ACTIVE guild are loaded under the candidate guild/member locks. World release jobs take
no Character root/session lock and independently recheck current node possession plus an
actual Channel assignment in the World. World and Channel remain different identities.

The proposed `game_house_auctions` read contract is explicitly fixture DATA from an absent
House owner: snapshotted rent, current price, phase and close time. Property/auction/ban facts
are stored independently from the financial request. This does **not** qualify their native
catalogue/admission/control source, first-bid creation, proxy ordering/effective eligibility,
anti-sniping, settlement, rent or controller publication. Registration must not route a real
House command to this wrapper until the accepted House owner composes those missing parts.
No runtime auction/property/ban writer, invented authority token or Boolean authority callback
is provided. The original Character-only 0038 ownership engine is unchanged.

The financial transaction takes the accepted occurrence→Character→Guild→member→property→
auction/bid→ban→Guild balance→Account balance order. It fixes the funding Account/Character,
writes an immutable same-xid source-specific operation, advances the exact bid revision, and
invokes the real candidate Guild bank `apply_guildhall_escrow_in_transaction` participant.
Both contributors use their native immutable ledger chains; neither fee-debit nor admitted
numeric-kind substitution is used. Lowering returns Account first. A release consumes all
original custody once, independently of a later leader, and a retry reads retained financial
outcome data without restoring authority. A changed occurrence binding or stale predecessor
cannot create another value grant. Terminal custody/operations cannot be deleted/truncated.
Original funding Character lifecycle is not required for owned refunds; its immutable World
and Account binding still must match. Unknown/stale current actor evidence fails closed.

All proposed DDL remains **UNINSTALLED**. Prerequisite order is the exact coordinated Guild
prerequisite, HOUSE-ACL property fixture, own bid+operation source, Guild bank schema, Guild
bank Hall-ledger extension, own Hall guards and proposed auction/ban read contract. The
HOUSE-ACL property constant is copied as hash-bound independent fixture data only; there is
no competing property writer or duplicate Guild ACL parser. Guild bank and Hall consume one
HELD source and its mandatory combined source headroom guards.

The immutable journal retains custody amounts, before/after balances, request binding and
exact ledger identities. It is **not** an admitted House/Guild/Bank event envelope. Required
same-transaction admitted House+Guild(+Bank) events, HousePrice/HouseRent burn variants,
canonical shared operation occurrence/CommandRef binding, complete shared House writer,
Guild ban lifecycle and `GUILD_DISBAND`/Inbox disposition remain inseparable production gates.
No rent sink, GuildDisband completion, ownership grant or ACL runtime claim is made.
Native `fee_payer_is_junior` is reused; this exact floor's starter-island source still returns
`NoStarterIsland`, so real current Dawnport/junior integration is also not qualified here.

Evidence is durable at `/workspace/oteryn-prep/guildhall-1-evidence/`. Validation snapshots may
register the literal owned modules and exact unmerged Guild bank proposal solely outside the
allocated checkout. Native unit-context metadata Clippy is distinct from small-driver
execution of unchanged selected bodies. No fake native Root/fence/types are substituted.
All acceptance checkboxes above remain open for the integrated feature.


### Final local qualification and review handoff

Final source-bound outside-tree run `guildhall-1-evidence/attempt-2` passed **6/0**:
three arithmetic bodies and three actual PostgreSQL **17.6** native Root cases. Funding
uses native MINT, Account bank deposit and Guild bank transfer. Coverage includes split
reserve/raise, Account-first lower, original-source release, two genuine independently
ready Roots racing the same release occurrence, replay/conflicting binding, current native
session rejection, actual current guild/member/ban/auction changes, wrong native node
secret, independently committed native Channel-assignment revocation, stale predecessor,
forbidden custody/journal rewrites and terminal retention. The revoked-assignment case
logs the actual native `Committed` Revoke receipt before checking the financial job's
`NotCurrent` rejection; no authority is reconstructed from the saved custody row.

Peer P2 **fixed**: an admissible zero-rent bid lowered to zero remained HELD and generic
resize returned `NoChange` on release. The exact actual-domain regression first failed
**0/1**, then passed **1/0**. Distinct validated release now terminalizes an empty HELD
row with revision advance and an immutable same-xid operation, with zero ledger entries.
Only `BID_RELEASE` admits that zero transition. The final native PG case verifies the
funded rent-zero lower-to-zero path, terminal state, unchanged ledger/balance counts and
retained replay. Initial zero-value resize still reports `NoChange` for the financial
component; this is not a full auction-admission decision.

Attempt1 actually executed **5/1**. The sole failure was the new native Revoke fixture
setup: the supplied actor did not equal `session_user` and the shared fixture had only
Assign1 permission, not Revoke3. The old typed return was not printed; an exact own-DB SQL
probe plus the unchanged native first authorization branch establish `NotGranted`.
Attempt2 changes only the test fixture: independently installed exact-scope Revoke3 data
and the actual authenticated fixture role. Its typed outcome is logged, and rejection
returns through cleanup. No production predicate, grant or native guard was relaxed.
The exact leaked own fixture DB was removed; historical failure inputs/logs remain.
A separate real rollback-only DDL preflight found the earlier unparenthesized PL/pgSQL
CASE delimiter; the corrected guard is used by the actual passing PG schema installation.

Final Rust bytes passed `cargo fmt --all --check` and **original native unit-context
Clippy metadata with `-D warnings`**. The real Cargo capture deliberately exits with
sentinel86/Cargo101 before global root codegen; that is **not** a test/check pass.
Runtime execution uses a normal native cfg(test) rlib plus small six-case driver,
preserving native Root/authority/fixture types. Recorded snapshot-only transformations
remove selected owned test attributes, minimally expose/forward their unchanged bodies
and remove the sibling bank test registration only after original-unit metadata checks.
Baseline unused cfg(test) scaffolding warnings from that rlib are retained separately.
Ordinary Cargo on the unregistered allocated branch does not exercise these new modules.
No full global native test ELF, production install or runtime/controller qualification
is claimed. Governance validation and its 124 tests are recorded in final evidence.

Exact candidate foreign input hashes are retained under `foreign-candidate-inputs`:
Guild bank production `4301336b302cfb40acba503e3e5be503d7254256cf9e8d77c9d8d0049ebd3905`,
audit `21bfe1fd2384908879677d4516c93e3be9c31d12bf41582eba648a703834deb0`,
tests `ac4853c47fdaa35de696ce009c63a782aeecea9d16685e468e3c7a0e7bebd6be`.
These remain proposed/unmerged dependencies, not installed truth. The HOUSE-ACL property
fixture constant is bound to `0ebf6f849126523b3bf8a09ba7a263b58229a3852e035542263418935630a36f`.
Current own raw schema hashes, final four-file hashes, literal registration roots, both
attempt manifests/logs, tiny RED/GREEN bytes, source-only review dispositions and restoration
instructions are durable at `/workspace/oteryn-prep/guildhall-1-evidence/`.

Remaining CP gates include complete GUILD1/current Dawnport source admission, real House
catalogue/auction/property owner, canonical occurrence-to-CommandRef binding, required
same-transaction admitted House/Guild/Bank events, HousePrice/HouseRent burn arms, guild
ban lifecycle, physical GuildDisband/Inbox disposition, accepted Guild ACL integration,
and shared module/caller/schema activation. Release batching must retain GUILD0-RL-14's
50-bid/200-line ceiling; this component executes one bid, with at most four value lines.
No new ceiling, event number, migration number, registry or runtime wiring was introduced.
All original integrated acceptance criteria remain open. The worker leaves only this
four-file local diff, with no commit/push/lifecycle action; the lead owns the existing draft.
