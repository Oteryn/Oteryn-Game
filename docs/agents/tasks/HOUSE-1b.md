# HOUSE-1b — house ownership second half (skeleton task record)

```yaml
task_id: HOUSE-1b
title: "HOUSE-1b: house ownership second half (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-1b
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-1b (row 119); App. 7 HOUSE-*; §5 Wave 5"
class: "hard (persistence, economy, security)"
needs_migration: yes
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house/transfer.rs, move_out.rs (new)"
  - "apps/game-server/src/house/*_tests.rs"
  - "apps/game-server/tests/house_move_out_*.rs"
  - "docs/agents/tasks/HOUSE-1b.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_house_transfer.sql"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Second half of house ownership per SOCIAL_MAP_PACKETS HOUSE-1b: move-out and transfer, item return to the inbox, overlay of house tiles.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1a | NOT MERGED | **BLOCKING** |
| INBOX-1a (MAIL-1) | MERGED (archive OTV2-20261004-inbox-1a) | ok |
| MAP-OVERLAY-1c | NOT MERGED (Wave 3) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house/transfer.rs, move_out.rs (new)
- `apps/game-server/src/house/*_tests.rs`
- `apps/game-server/tests/house_move_out_*.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_house_transfer.sql
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- inbox writer from INBOX-1a
- apps/game-server/src/durability/house_scope_handoff.rs
- map overlay (MAP-OVERLAY-1c)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- mint/burn + ledger (transfer price, item custody to inbox)

## Acceptance criteria

- [ ] move-out returns every house item to the owner inbox exactly once
- [ ] transfer is atomic with the bank ledger
- [ ] restart mid-transfer resumes from durable state
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (HOUSE-1b)
- `docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
- `docs/agents/tasks/archive/OTV2-20261004-house-rt-inbox-packets.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local implementation — bounded native disposition custody (2026-10-15)

Existing draft #2141; exact source `49caf3cacd4afdc14140e4d87e5beef8e5da7e70`,
`skeleton/house-1b`; accepted read-only main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.
The lead expanded the original card-only lease to the exact allocation's five paths after
user steering. The earlier report and actual evidence are preserved separately in
`/workspace/oteryn-prep/house-1b-evidence/design-phase/`; they are not runtime qualification.
Original plan row, unchecked acceptance criteria and CP rules above remain unchanged.
META 3.1.0, one author, local AUTHORING; worker makes no commit/push, root may batch-publish
the existing draft, and CP alone controls ready/merge/queue. No shared registration or
production migration/permission/content/registry changes were made.

### Accepted scope and prepared component

HOUSE-OWN-0 §5–§9/§15, HOUSE-CUSTODY-0 §3, SOCIAL-MAP §0.1/§2.4 and
HOUSE-RT-INBOX §1.2/§1.6 govern this component. Direct player house resale is deferred by
HOUSE-OWN-0; `TRANSFER` here means HouseInterior item custody to each item's actual
**reclaim subject**, never owner fallback. The skeleton's price/bank-ledger and “owner
Inbox” wording stays unchecked for CP correction. SOCIAL-MAP holds HOUSE-ACL/WIRE,
GUILD-BANK/HALL and every wire child; HIRE is a candidate, not permission.

`src/house/move_out.rs` prepares a real native DurabilityRoot semantic pass and an
**uninstalled delegated private schema**. The physical file is intended to become a
`durability::house_move_out` child at the slot, without widening any native helper visibility.
A new notice uses actual recovery, current Character/session/connection/lease/Channel
fences and native0038 owner/revision; it binds CommandRef, operation, original fences,
house, notice days and owner revision immutably. Notice1..30 days and all persisted times
use DB-clock Unix milliseconds. Same-command/operation changed bindings and another
operation for the same live property return semantic conflict. Accepted §6 cancellation is
implemented before the DB-clock due time with actual current-owner/session fences, retained
CommandRef/original-fence binding and immutable cancelled time/outcome. A single materialized
clock governs both deadline and stored time. CANCELLED history keeps its first cause and
original notice; a partial live-house uniqueness rule permits a fresh notice. Old cancellation
replay cannot affect that fresh operation. At/after due or after freeze cancellation is closed.
NOTICE/CANCEL reuse the native `character_item_fence_is_current` boundary, whose
`scope_of` currently admits Channel sessions only. House-scoped owner commands are not
qualified here: they require the actual admitted House grant/session publication owner,
not a parallel or weaker current-fence predicate. World jobs have no actor-session requirement.

Due freeze and custody steps use independently current native node incarnation and an
assigned **execution Channel in the same World**, not an active destination House assignment
or actor session. Thus unloaded houses and another legitimate World Channel process can
run the job. Root recovery/admission relation locks, handoff relation lock and native property
locks protect the current census. Freeze reads at most2001 sorted IDs, refuses overflow of
the accepted2000 PARITY_PENDING bound, and retains exact native source location/provenance/
item snapshots plus count and SHA256 of sorted ItemInstanceIds. Native HouseInterior and
entry fences prevent subsequent source mutation or a new PREPARED/COMMITTED entrant.
First cause and notice binding are immutable; this subset admits MOVE_OUT only.

`src/house/transfer.rs` locks real recipient Character roots before property, items by
ItemInstanceId and native Inbox counters by CharacterId. Each step selects at most100
unmoved census items. Its private SECURITY DEFINER verifies current native owner/source
and canonical sessions plus unreleased PREPARED/COMMITTED entrant reservations, then
removes the real0025 source/provenance and calls **the actual0076
`game_character_inbox_deliver`** in the same transaction. Full native custody, cause,
Inbox exclusivity and deferred proof guards stay active. Private operation/step/item proof
also commits atomically and binds real source/reclaim/returned ordinal; it is not a
registered HouseOperation event. Native OTI errors roll the entire step back. Exact
historical step replay returns original item/recipient/ordinal without redelivery or a new
authority grant. DRAINED requires full retained census delivery and no source item; it
**retains ownership and the content fence** pending actual final reconciliation.

### Qualification and authority model

`tests/house_move_out_postgres.rs` uses the native private-root path-loaded pattern already
used by character_authority_postgres. Its explicit crate gate defers autodiscovery until CP
registers the two owned durability children. Disabled-target metadata is only build hygiene;
zero disabled tests are not a behavioral pass. An outside-tree exact-source snapshot removes
only that gate and adds literal module registrations for separate enabled standalone metadata
qualification. Native-root case execution retains the exact marked test body; normal rlib
validation removes only #[test] attributes and appends a dispatcher, avoiding the known
full unit-test codegen OOM. Every transform and source hash is recorded; no native visibility,
authority implementation, feature, guard or production source is patched for validation.

The fourteen PostgreSQL cases cover: notice stale lease/changed binding/conflict/due/first cause;
unloaded101 mixed-reclaim items split100+1 and stable replay; missing native cause full
rollback/retry; frozen source/manifest tamper/deferred proof and runtime privilege refusal;
stale Channel and changed native owner; same-step race and same-house conflict; real
PREPARED entrant census/abort/post-freeze entry refusal; bounded2001-item refusal; real
COMMITTED canonical house session independent of released reservation; and a different
current World Channel worker with wrong-holder/foreign-World negatives. Four cancellation cases add stale
lease and cross-kind CommandRef reuse, exact retained replay after a fresh notice, immutable
first cause/time, both before-due and overdue cancel/freeze races, and an independently current
successor connection cancelling an older notice. The last uses a DB-owner persisted generation
operator with native guards active; it qualifies fence consumption, not reconnect E2E.

Fixture history is explicit: only preexisting ownership/item history is authored under
setup-only replication mode; all candidate operations run with all native guards enabled.
The second reclaim subject is created through actual native source observation and
bootstrap_character, including immutable creation/audit/name facts. This local source fixture
does not qualify Platform TLS/producer delivery. A retained earlier-day NOTICE fixture avoids
a day-long wait but is never current authority. The native game_house_access no-row stub
means COMMIT refusal alone cannot qualify the new fence. The actual pre-freeze PREPARE
is the positive control; a fresh post-freeze PREPARE has typed database refusal. A
separate INSERT through the actual native handoff trigger checks SQL23514 and the fence
message; production error mapping remains private. A separate owner-derived test-only
access function enables the actual canonical handoff/session positive control; it does not
qualify production grants/access policy or exits.

AuthorityInvariant × ConsumerBoundary × MutationOperator: identity/binding (CommandRef,
house World/key, operation, owner revision, source/reclaim snapshots); current liveness
(actual recovery, Character lease/session for notice, node and independent assigned World
Channel for jobs, canonical occupants/reservations before custody); temporal/provenance
(DB due time, immutable first cause/census, native same-transaction source→Inbox proof; strict cancellation deadline and retained terminal history).
Missing cause, stale lease/Channel/session, mismatched World/holder/owner, changed immutable data,
forged moving proof and concurrent/replayed step have actual focused negatives. Current
authority is never reconstructed from the retained disposition or supplied as a boolean.
Cancel/freeze races use two actual ready native Roots whose real reconciled authorities are
opened before racing mutation passes, avoiding a fixture race for one idle pool holder.
A session is NOT_APPLICABLE to accepted World-job execution; historical replay grants no
current controller. Native admission EXCLUSIVE relation locks serialize Root passes; a
reviewed hypothetical actor/property deadlock was rejected on that actual boundary.

Final exact-source qualification: **14/14 PostgreSQL cases PASS, zero ignored**, including
real native source→Inbox commits,100/101 batching, rollback, replay, races and cancellation.
Whole production library metadata, original gated target metadata, **enabled original
standalone private-root target metadata**, exact native original-case Clippy with warnings
denied, normal cfg(test) native rlib and real-case driver compilation all PASS. Scoped
rustfmt, governance validator,124 governance tests, diff/scope/preserved-prefix checks PASS.
This is bounded native/private PostgreSQL qualification, not full gameplay/runtime E2E or
Cargo all-target/full unit-test execution. The explicit original target gate yields no
behavioral tests and is never counted as a test pass.

Evidence: `/workspace/oteryn-prep/house-1b-evidence/index.md`,
`entrant-green-qualification.json`, `entrant-green-pg-tests.log`, final source hashes and
whole-owned patch. Earlier initial setup/compile0/10, intermediate8/10, test-only lint and
13/14 entrant fixture-order failures remain real raw records, separately classified, not
product RED. The final repair captures the actual native PREPARED row before COMMIT, whose
access refusal can delete it; native COMMIT/abort semantics and all guards stay unchanged.
Root owns final independent closure and freeze/publication choice; READY_TO_FREEZE applies
to this bounded component only, with every production integration gate below retained.

### Exact production integration gates; no completed release claim

1. CP must compose/install the private schema and literal durability child registrations,
   production privilege boundary and registered **HouseOperation event**. Private atomic
   custody evidence does not replace the shared registry/audit contract. No current runtime
   caller or production permission is installed; no production mutation is claimed.
2. Actual HOUSE1a acquisition/property/Account-slot reconciliation and final owner release
   must consume the complete drained census, retain immutable outcomes and clear the fence
   atomically. Native0038 has an active same-World owner and strict successor guard, not the
   complete accepted tenancy/slot writer; prepared HOUSE1a is arithmetic only, not merged API.
   Owner stays present here; restart/retry reads retained private rows, not a claimed final release.
3. Independent current house map/content activation and house content projection revision,
   actual grants/guild policy and runtime exit/quiesce must be bound by their owners. Native
   handoff Entry+reservation and canonical session census exist; ExitNotAdmitted remains.
   No closure/open/quiescent/current-guild boolean or forged HousePresenceProof is accepted.
4. BED-0 §6 disposition freeing belongs in the same fence transaction, before any owner
   change; preserve first freed cause/time in native Unix milliseconds and retain sleepers for
   actual admission settlement. This subset does not install that hook, regenerate food/vitals
   or grant XP. It must not run as production until composed with the BED owner.
5. Rent/eviction eligibility, reset per-item retirement and retained final outcomes
   remain additional accepted lifecycle work. Bank-only house burns use the shared registered
   HouseOperation cause; current FeeBurnCause::CharmUnassign is not reused or invented.
   No direct resale, guildhall, ACL/wire hold bypass, reset/overlay writer or Inbox callback.
