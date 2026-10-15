# ITEM-IMBUE-1 — imbuing (skeleton task record)

```yaml
task_id: ITEM-IMBUE-1
title: "ITEM-IMBUE-1: imbuing"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2144
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-IMBUE-1 (row 72); §7 'ITEM-TIMED-1' IMBUE-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-imbue-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Imbuements: content (24 types x 3 tiers, slots, shrines), item-state table with apply/clear/ImbueCause/checkpoints/expiry, ticking runtime and effects, scrolls and etcher, `IMBUE_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

IMBUE-CONTENT-1 (content), IMBUE-1 (hard), IMBUE-RT-1 (hard, combat), IMBUE-SCROLL-1 (hard), IMBUE-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| NPC-ECON-1 (per child) | BLOCKING |
| GOLD-FEE-2 | MERGED |
| ATTACK-1b | MERGED (#1798) |
| SPEED-1 | MERGED |
| PREMIUM-1 | MERGED (#2110) |
| NPC-TRADE-1 | MERGED |
| ITEM-SEM-USE / USE-WIRE-V1 / ITEM-MOVE-WIRE-1 | MERGED (archive records) |
| acceptance of IMBUE_FORGE0 | ACCEPTED via protected ACCEPT-BATCH; entry and child integration gates remain |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (exclusive bounded root allocation)

- `apps/game-server/src/domain/imbuement.rs`
- `apps/game-server/src/durability/imbuement.rs`
- `apps/game-server/src/durability/imbuement_tests.rs`
- `docs/agents/tasks/ITEM-IMBUE-1.md` (packet/notes)

This exact allocation supersedes the family's broad suggestions. No shared module
registration or source authoring/regeneration path is allocated here.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/domain/timed_item.rs and timed_item_host.rs (checkpoint/expiry pattern)
- ability/combat pipeline hook for IMBUE-RT-1
- connection.rs dispatch; capabilities.rs `GATED` (`IMBUE_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + imbue writer + durability
- gold fee burn + imbuement row + ledger (mint/burn + ledger)
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Apply charges fee and writes state atomically; clear is idempotent.
- [ ] Ticking checkpoints survive restart; expiry writes exactly once.
- [ ] Effects pass combat parity tests.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md §3-§7, §14
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded authoring packet, before code (2026-10-15)

Existing draft #2144, exclusive `skeleton/item-imbue-1` at unchanged base
`470891e2d579d60ebcf8755085ba8a32e825c065`. Root supplied META 3.1.0 and exact
four-path lease; no worker commit, push, freeze, lifecycle or remote mutation.
Accepted-source baseline is primary `3ef900c160c9713092a86bfcd1a8377c7587c898`.
IMBUE-FORGE-0 §§3, 4, 6, 7, 15 supplies the tier/access, row binding and bounds:
3 slots (indices 0..2), remaining 0..72,000,000 ms, checkpoint transaction 10 items,
10 participants and 40 work (one participant plus at most 3 row writes per item).
The 60 s ticking interval belongs to future runtime, not a new clock here.

### Actual residue and source limits

No Rust imbuement state/writer/effect/protocol or SQL table exists on this base.
Item slot metadata exists, but its tier codec Two/Three/Ten cannot express Basic-
only maximum 1. Authoring has 24/72 draft entries, DRAFT_NOT_RUNTIME_READY; canonical
Imbuement and Quest indices are READY_UNPOPULATED. Unknown per-family limits and
conflicting source/effect profiles remain. Draft fees differ from the brief's
reference examples. No candidate fee, category map, source profile or schema is
silently made admitted content. Content changes need their own exact lease.

### Approved useful implementation

Prepare private structurally validated slot/binding records and a deterministic
checkpoint-delta/batch planner, no SQL or installed schema. A row preserves item,
slot, definition key/content revision, category/tier, applying occurrence and
TransactionId; a checkpoint changes only remaining time downward. Content revision
is not a monotone state counter. No duplicate timer is written: native timed lanes
already exist and cannot be blindly reused as imbuement content-revision truth.

Use existing PremiumGate for the Premium-only subcheck. With no native current
quest reader, expose only outstanding access requirements, not an apply grant:
shrine/scroll-create still require qualified quest and item bindings; completed-
scroll apply still requires qualified item rules. No quest bool/proof is invented.
Unknown slot/tier rules fail closed. Structural slot/category checks do not prove
that any source is admitted, an account is current, or a transaction may execute.
Tests exercise missing rules, native closed/open Premium classification, slot/
category collisions, preserved bindings, monotone time and exact batch ceilings.

### cross_domain_findings / pending owner hooks

- Admitted catalogue/Quest reader and Basic-capable item rules are absent. Future
  current Premium must come from the existing fence owner at the real boundary;
  this code never opens a gate. Raw/persisted records cannot reconstruct current
  session, lease, scope, ownership, content admission or quest authority.
- CP must implement the accepted table/guards, apply/clear/expiry audit/receipt,
  new admitted FeeBurnCause arms and item-only fee/ledger composition in one
  transaction, exact reserved revision-set replay/REVISION_CHANGED records, and
  current fences/root/item locks. No guessed fee, event/command/capability ID or
  dead writer is supplied. Native fee and timed models remain their owners.
- The batch planner is not a checkpoint commit or CAS proof. Future owner must
  independently read current rows, compare the complete before binding, make all
  batches commit before declaring an interval complete, freeze ticking/effects on
  failure, retry unchanged live values, and audit-delete expiry separately. Zero
  remaining rows still occupy slots/categories until their audited deletion.
- Combat/effects, capacity's committed-state agreement, ticking, 60 s accounting,
  ambiguous writes, transfer/stop/restart/concurrency and wire/client integration
  remain unqualified. Full family acceptance boxes stay open. No schema is
  installed and no runtime is enabled by this unregistered component.

Durable evidence is preserved directly in
`/workspace/oteryn-prep/item-imbue-1-evidence/` (no private target/ELF copies).

### Local qualification boundary (2026-10-15)

The exact final three Rust sources pass 12 focused tests (12 passed, 0 failed,
0 ignored). A dependency-free isolated harness imports the actual owned modules
and test source by absolute path; it includes four verified literal native source
excerpts for ItemInstanceId/UUID validation and PremiumGate. It does not substitute
validators or grant authority. Open Premium fixtures exercise the existing native
constructor's classification only, not a current fence read or mutation permission.
The initial 12/0 run precedes fixture strengthening (unique per-row applying IDs
and isolated duplicate-slot/category cases); its separate source manifest/log is
retained. The final tests bind tested-source-manifest.json, not that initial run.

A binding's transaction_id is the original applying transaction, not a later
checkpoint transaction. Category is raw projected content data, not a proposed
installed column or a catalogue proof. No timer/state revision counter is added.
The complete before binding is preserved even when applications from distinct
content revisions coexist. SQL/PG, current owner/session fences, durable CAS,
clock/failure suspension, audit/fees, admission, runtime and protocol integration
are not qualified by these local tests. No family acceptance box is closed.

The exact source-bound harness also passes all-target Clippy with warnings denied.
No full-server build is claimed for these unregistered modules. Governance
validation passes for 22 policy documents / 9 lanes; its unit suite passes 124
checks. The durable folder includes tested/final source manifests, four literal
native excerpt hashes, immutable registration-root copies, local.patch and exact
restoration instructions. Whole owned diff self-review found no known issue in
this bounded data-only scope; independent qualification and all owner hooks above
remain required.
Direct rustfmt --check on all three owned Rust files and workspace cargo fmt
--all --check pass. Final source verification confirms tested bytes and unchanged
native/registration roots. No commit or publication was performed.
