# HOUSE-ACL-1 — house ACL tables and revisioned edits (skeleton task record)

```yaml
task_id: HOUSE-ACL-1
title: "HOUSE-ACL-1: house ACL tables and revisioned edits (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-acl-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-ACL-1 (row 121); App. 7 HOUSE-*; §5 Wave 5"
class: "hard (security review)"
needs_migration: yes
needs_registry: maybe
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house/acl.rs (new)"
  - "apps/game-server/src/durability/house_acl.rs (new)"
  - "apps/game-server/src/house/acl_tests.rs"
  - "docs/agents/tasks/HOUSE-ACL-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_house_acl.sql"
  - "docs/contracts/RESOURCE_LIMITS_REGISTRY.json (ACL caps)"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (only if Reg is confirmed)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

House ACL tables (guests, subowners, door lists) with revisioned edits per HOUSE-OWN-0 §10.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1a | NOT MERGED (Wave 5 skeleton) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house/acl.rs (new)
- apps/game-server/src/durability/house_acl.rs (new)
- `apps/game-server/src/house/acl_tests.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_house_acl.sql
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json (ACL caps)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (only if Reg is confirmed)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/durability/house_spell_acl.rs (spell `house_access` consumer)
- apps/game-server/src/gameplay_transport/map_door.rs (door access)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability

## Acceptance criteria

- [ ] every ACL edit carries a revision; stale revision rejected
- [ ] only owner/subowner may edit; checked server-side
- [ ] door and spell access read the same ACL
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house_acl
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md §10
- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md §0.1
- `docs/agents/tasks/archive/OTV2-20260929-house-doors.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local owned implementation design (2026-10-15)

CP comment6193914573 releases this previously held module for independent owned-path drafting.
Acceptance is HOUSE-OWN0§9–10, GUILD0§4.1/§10 and HOUSE-RT-INBOX§1.2 at main@ca5ea593;
the skeleton's old CANDIDATE/dependency prose remains historical and its unchecked completion
criteria are preserved. HOUSE1a remains an integration dependency, not an assumption of installed
property tables. No shared registration, migration number, grants, content/pins or wire changes.

The prepared owner is `durability/house_acl.rs` with the owned `house/acl.rs` as its neutral
rules child. It reuses native HouseId, CharacterId, WorldId, CommandRef, Root/reconciled recovery,
current session/lease/scope/node fences and the existing native UUIDv7 validator/generator.
No supplied owner/consent/current bool or row-derived controller proof is accepted. The native
Channel session fence is the initial actual entry port; named-House session/admission rebind
must consume its genuine shared native fence at CP, never relax a predicate or copy a guard.

Private schema proposal (UNINSTALLED): accepted `game_house_properties` identity, current owner
kind/Character+Account or guild, lifecycle state, property `revision` and global `acl_revision`;
`game_house_acl_entries` stores bounded Character/exclusion/guild entries by role or native door
position; an ACL operation occurrence is bound one-to-one to native CommandRef and exact request
bytes, with immutable result/change evidence. A successful replacement and its evidence commit
in one native transaction, increment only the global ACL revision once and never advance
CharacterRevision. Refusal/conflict/rollback writes nothing. A historical receipt is data only
and must not regain current authority after owner/session replacement.

GUILD0-LO-01 order: recovery/admission, operation occurrence, current acting Character/root,
required guilds sorted by GuildId, member rows, property, ACL entries. Required guild sources
are first read without row locks, then reread under locks; discovering a missing guild after
property lock means rollback/retry, never a late guild lock. Guild rows/membership/ranks remain
explicit coordinated candidate dependencies, not merged/current production membership. Actual
SQL reads in isolated PG qualify source semantics; no caller membership callback grants access.

The native0038 spell tables (`game_house_ownership`, per-list `game_house_acl`,100 members and
wildcard text) are not silently reinterpreted as the accepted canonical property/global revision
model. Native `game_house_access` is still a failclosed stub; HOUSE1a owns its final composed body
and the exact future grant-part signature. This task may replace only the grant part at its CP
slot once that signature exists. Shared spell/GUI/door/admission consumers and their registered
HouseEvent/definer/grant integration must then consume this same source; no second live ACL truth
is activated now. Door positions are native coordinates/data, not proof of an admitted physical
door/current content/placement; actual catalogue and physical actor checks remain with their
native owners at integration. Stored access observations likewise authorize no relocation/item
or socket action by themselves.

The read result's `door_list_granted` is deliberately only the selected list's grant fact,
not the full HOUSE-RUNTIME0§5.1 physical-door decision. Absent and empty lists yield no list
grant; the runtime must independently resolve the admitted door and whether it has a restrictive
list before composing the invitation/owner/door rule. No guessed catalogue/list-presence or
caller boolean is a grant. Guild owner resolution uses the same locked current member/rank rows;
exclusions apply to nonowner grants, while the accepted owner-first composition bypasses them.

The qualification property prerequisite is a minimal shape, not HOUSE1a's complete property
schema/lifecycle. Its fixture does not synthesize rent, paid/grace dates, auctions or ownership
transitions. The coordinated GUILD1 row prerequisite is likewise not lifecycle authority.
The native test Root uses its unchanged administrative fixture connection; runtime-role grants,
owner-definer-only mutation, registered HouseEvent emission and activation are NOT qualified by
these tests and remain CP gates. Native Channel fencing is qualified; named-House fencing and
the prepared House-session owners' exact current entry must be composed at CP.

## Frozen local qualification (2026-10-15)

Prepared actual native Root writer/reader plus canonical ACL rules within the four owned paths.
The exact-source disposable native unit context passed metadata and Clippy (`-D warnings`),
owned/workspace format, and an eight-case bridge run: four pure rule cases and four genuine
PostgreSQL17.6/native Root cases. The native test bodies are unchanged; only eight test attrs,
parent visibility/forwarders and one literal snapshot-only durability registration differ.
No full native unit ELF, full package test run or production-wired gameplay test is claimed.

Native PG covers property-global revision CAS, two genuine Roots/pools contending with one typed
stale loser, exact occurrence replay/conflict, evidence-insertion failure rollback, immutable
receipt/outbox, CharacterRevision unchanged, current owner/subowner/exclusion permissions,
independent Character/World/Channel/connection/lease/scope/node/session-binding rejection and
current guild leader/rank/state reads. A historical stale-generation replay returns receipt data;
a new write under that generation rejects. Pure cases cover 200/201 and duplicate entries,
EXACT/AT_LEAST/removal/disbanded rules, owner-first hierarchy and native max door coordinates.

The first authored candidate passed four pure cases but its four PG cases failed on a private
property-DDL syntax error before core assertions; its original Clippy failed three style lints.
Those actual failures are retained, with corrected exact DDL rollback preflight and final native
GREEN evidence. They are not described as environmental failures or behavioral RED qualification.
No Rust changes were made after the final source-bound batch began.

Local evidence: `/workspace/oteryn-prep/house-acl-1-evidence/` (sources, exact overlay manifests,
raw logs/statuses, DDL rollback preflight, scope and boundary qualification). Completed first
snapshots are losslessly archived with GNU comparison, SHA and restore commands; old own source
bytes remain separately available. Root controls export/publication and independent review.

Remaining integration criteria stay open: admitted HOUSE1a property lifecycle/current ownership
and GUILD1 lifecycle/rank mutation owner; owner/ACL/fence composed access-function with unchanged
native signature; House-session current fence/admission rebind; actual restricted-door/catalogue
composition; shared spell/GUI/door/runtime revalidation and notification source; registered
HouseEvent/audit envelope and definer-only runtime grants/immutable mutation guards. The private
outbox is candidate evidence only. Existing0038 legacy per-list/wildcard spell behavior is not
silently changed. No migration numbering, shared registration/wiring, registry or content changes.
