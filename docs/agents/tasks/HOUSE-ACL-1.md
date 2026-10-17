# HOUSE-ACL-1 — house ACL tables and revisioned edits (skeleton task record)

```yaml
task_id: HOUSE-ACL-1
title: "HOUSE-ACL-1: house ACL tables and revisioned edits (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-acl-1
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
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

## 2026-10-17 current published source — D1420 requalification required

The current draft source checkpoint is
`0ca09342c7283a3ea4e28a7bc1391bf3b199578b`, sole parent
`20b751c889936cb311ca61d0374512e83e374470`, tree
`3c4514f571626edc32f7ebdcfbe7a5c9ff1597b9`. A fresh read of #2140 confirmed this
head and its draft/open disposition. The nullable-owner correction is already
published; this successor changes only this owned task record. It adds no Rust,
schema, shared registration, runtime grant or gameplay activation.

The published writer and reader decode a nullable property owner kind and return
the existing typed `AclError::NotOwner` for VACANT, AUCTION and RETIRED properties.
The existing native regression exercises both actual Root entrypoints and requires
unchanged ACL revision with no operation receipt or change-outbox row on refusal.
The three retained ACL inputs are unchanged:

| Source | SHA-256 |
| --- | --- |
| `apps/game-server/src/durability/house_acl.rs` | `2dd835a3b9873acb54008c478405c1090e81922d9ea3f2bb488967768f462b1c` |
| `apps/game-server/src/house/acl.rs` | `948735ef66be8456fdba602e5619c934db1e48652376e74eac9c93ea22a75b5a` |
| `apps/game-server/src/house/acl_tests.rs` | `7da6c7dc7c907309f593f5ea30a9cf08b87155fd9366bf7a609ccf521e6f0457` |

### Honest native disposition

[CP D1420](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6239666536)
requires touching the `src/lib.rs` of each changed crate, at minimum
`apps/game-server/src/lib.rs`, inside the actual canonical lease before every
Cargo command. This includes locked fetch, LIST and execution. The operation must
preserve the pinned source bytes. Earlier native Cargo results without that step
need a repeat and remain historical evidence.

The recorded Molehill-PC run beginning 2026-10-17 13:20:10 UTC passed nine ACL
cases: four pure and five native PostgreSQL cases, zero failed or ignored, 10.12s.
That run used the disclosed main-f010 source assembly, genuine unchanged native
Root/harness dependencies, this task's three ACL leaves and the WIRE leaves. It
predates D1420 and did not include its required touch step. The result is retained
in the historical section below; it is not current D1420 qualification, an
exact-published-head native result or a production activation result.

| Required evidence | Current disposition |
| --- | --- |
| Nullable-owner source checkpoint and retained source identities | Published at 0ca09342; unchanged by this card. |
| Earlier native formatting and source reviews | Retained source evidence; no new compile or runtime result inferred. |
| Historical ACL inventory | Nine cases: four pure plus five PG, including the nullable-owner regression. |
| D1420 native compilation, LIST9 and execution of those nine cases | NOT_RUN for the required repeat. Pin the complete real source/dependency assembly and report its identity explicitly. |
| Exact published-head and integrated runtime qualification | NOT_FULLY_QUALIFIED; remains separate from an assembled native component run. |

A repeat requires its actual allocated native window and the existing resource,
source/HEAD, lease and final-integrity checks. This card starts no additional job
in the active four-child House queue. [D1329](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233047791)
remains mandatory: use the existing shared PostgreSQL17.6 service on
`127.0.0.1:55432`, the adjacent `in-pg-netns.sh` and an own disposable database;
run `cargo fetch --locked` first and clean up that own database. No new PostgreSQL
server/container or other database is part of this task.

### Actual door/currentness and integration gates remain open

A fresh read of [#2167](https://github.com/Oteryn/Oteryn-Game/pull/2167) found
`8702779595ec0611add17200f08328fa2e23b503`, still draft/open and unmerged.
D1348 therefore continues to hold `QualifiedHousePlacement::door_cell`, its
real generation-qualified ACL consumer and the associated content tests until
#2167 merges. The recovery proposal remains NOT_APPLIED. Its data-only door
coordinate must not stand in for current active content or admission authority.

The actual consumer must bind the same World/House row and transaction, retain
the required SHARE lock, and compare both placement and compatible-content digests
against the genuine current qualified source. Missing or ambiguous doors and
missing/mismatched source preserve the existing legacy behavior. The canonical
property/Guild lifecycle, House-session fence, shared spell/door/GUI consumers,
legacy0038 compatibility, registered House event and definer-only runtime
integration remain unfinished criteria. The native ACL fixture's minimal private
property/Guild schema does not qualify any of those producers.

The complete recovery appendix and its original patch bytes, SHA-256
`38c73cef567300e641310f4d911660cffedce8bfc544d2c5396f2aa63d3d3c4e`, are preserved
unchanged. All original acceptance boxes and scope restrictions remain. Root is
the sole publisher; ready, review triggers and merge remain CP-owned.

## Historical 2026-10-17 nullable-owner source-assembly result

This packet follows CP's ordinary main merge at
`20b751c889936cb311ca61d0374512e83e374470`. The three changed preimages were
read at that head and match the retained baseline. Root is the sole publisher
under [D1345](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418).
The PR remains draft and NOT_FULLY_QUALIFIED.

The canonical property shape permits `owner_kind = NULL` for unowned states.
The reader/writer previously decoded that valid database value as a required
string, producing a SQLx decoding failure. They now decode `Option<String>` and
return the existing typed `AclError::NotOwner` for an unowned property. Character
and Guild owner checks, current session fencing, receipts, replay and revision CAS
retain their existing behavior; no grant or current-authority source is added.

The new native case
`native_acl_unowned_property_returns_not_owner_without_receipt_or_decode_error`
covers VACANT, AUCTION and RETIRED rows through both actual Root entrypoints. Each
refusal leaves zero operation receipts, zero outbox rows and unchanged global ACL
revision. Its prerequisite remains the already disclosed minimal uninstalled
property/Guild fixture; this test does not qualify HOUSE-1a lifecycle or runtime
property grants.

Root executed the frozen formatted sources on Molehill-PC, slot B, beginning
2026-10-17 13:20:10 UTC. The source assembly was native main
`f010472bcb6794a2cbc56dfaf755bd122c10225e`, this task's three real ACL leaves,
four real #2145 WIRE leaves and literal own module registrations. The existing
native `bestiary_postgres_harness` supplied the Root; no helper or authority was
replaced. `cargo fetch --locked` passed before the shared PostgreSQL namespace
run, with D1342 jobs=4 and MemAvailable about40GiB.

**PASS, bounded:** all nine `durability::house_acl::tests::` cases passed
(four pure and five native PG), zero failed or ignored, elapsed10.12s. This
includes the new nullable regression. The unchanged rules source is a validation
input, not a changed file in this publication packet.

| Frozen source | SHA256 |
|---|---|
| `durability/house_acl.rs` | `2dd835a3b9873acb54008c478405c1090e81922d9ea3f2bb488967768f462b1c` |
| `house/acl.rs` | `948735ef66be8456fdba602e5619c934db1e48652376e74eac9c93ea22a75b5a` |
| `house/acl_tests.rs` | `7da6c7dc7c907309f593f5ea30a9cf08b87155fd9366bf7a609ccf521e6f0457` |

This is source-assembly evidence, not an exact-published-head or production-wired
qualification. Root must read back the successor's sources and requalify its
identity; full package/Clippy and live House activation are not claimed here.

D1348 holds `QualifiedHousePlacement::door_cell`, its real generation-qualified
ACL consumer, and associated content tests until #2167 merges. None of those
shared hunks or module/migration/dispatch mounts is in this nullable-only packet.
The recovery archive below remains a held proposal, not applied source. Actual
House-session fencing, active-bundle door qualification, legacy0038 compatibility,
canonical property/Guild lifecycle, registered House events and definer-only runtime
integration remain separate unfinished criteria.

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

## 2026-10-17 source proposal recovery archive — NOT_APPLIED

At the owner's request, the previously external source proposal is preserved
below inside this task record, rather than only in a local handoff or PR comment.
The `patch_lines` array joins to the original UTF-8 patch bytes and SHA256.
The original base and historical source-only review are retained for recovery;
they do not qualify the current branch or authorize applying shared hunks.
Recheck the actual successor source and obtain the required CP shared slot,
migration allocation and independent review before implementation.
**Shared proposal NOT_APPLIED; native execution NOT_RUN; runtime acceptance
and all existing integration gates remain unchanged.** This appendix changes
only this owned task record and introduces no executable implementation.

Previous reviewable handoff: https://github.com/Oteryn/Oteryn-Game/pull/2140#issuecomment-6230752734.

```json
{
  "archive_version": 1,
  "owning_pr": 2140,
  "status": "SOURCE_PROPOSAL_NOT_APPLIED_NOT_RUNTIME_QUALIFIED",
  "original_source_sha": "b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16",
  "patch_sha256": "38c73cef567300e641310f4d911660cffedce8bfc544d2c5396f2aa63d3d3c4e",
  "patch_lines": [
    "--- a/apps/game-server/src/content/project/native_house_tiles.rs\n",
    "+++ b/apps/game-server/src/content/project/native_house_tiles.rs\n",
    "@@ -48,6 +48,16 @@\n",
    "     }\n",
    "     pub(crate) fn has_door(&self, id: u32) -> bool {\n",
    "         self.doors.values().any(|number| *number == id)\n",
    "+    }\n",
    "+    /// Reverse lookup inside this qualified placement. An absent or ambiguous\n",
    "+    /// number supplies no cell; generation/currentness remains with the owner.\n",
    "+    pub(crate) fn door_cell(&self, id: u32) -> Option<LogicalCell> {\n",
    "+        let mut cells = self\n",
    "+            .doors\n",
    "+            .iter()\n",
    "+            .filter_map(|(cell, number)| (*number == id).then_some(*cell));\n",
    "+        let cell = cells.next()?;\n",
    "+        cells.next().is_none().then_some(cell)\n",
    "     }\n",
    "     pub(crate) fn door_at(&self, cell: LogicalCell) -> Option<u32> {\n",
    "         self.doors.get(&cell).copied()\n",
    "@@ -158,6 +168,50 @@\n",
    " mod tests {\n",
    "     #![allow(clippy::unwrap_used)]\n",
    "     use super::*;\n",
    "+    fn placed_doors(doors: BTreeMap<LogicalCell, u32>) -> QualifiedHousePlacement {\n",
    "+        QualifiedHousePlacement {\n",
    "+            house: ProjectV2DefinitionRef {\n",
    "+                family: ProjectV2Family::House,\n",
    "+                key: \"oteryn:content.house.door_lookup\".into(),\n",
    "+                revision: \"door-lookup-r1\".into(),\n",
    "+            },\n",
    "+            cells: doors.keys().copied().collect(),\n",
    "+            entry: LogicalCell { x: -1, y: -1, z: 7 },\n",
    "+            doors,\n",
    "+        }\n",
    "+    }\n",
    "+    #[test]\n",
    "+    fn door_cell_absent_number_has_no_coordinate() {\n",
    "+        let placed = placed_doors(BTreeMap::new());\n",
    "+        assert_eq!(placed.door_cell(0), None);\n",
    "+        assert_eq!(placed.door_cell(7), None);\n",
    "+    }\n",
    "+    #[test]\n",
    "+    fn door_cell_unique_number_round_trips_actual_door_map() {\n",
    "+        let cell = LogicalCell { x: 12, y: 31, z: 7 };\n",
    "+        let placed = placed_doors(BTreeMap::from([(cell, 7)]));\n",
    "+        assert_eq!(placed.door_cell(7), Some(cell));\n",
    "+        assert_eq!(placed.door_at(cell), Some(7));\n",
    "+        assert!(placed.contains(cell));\n",
    "+        assert_eq!(placed.door_cell(8), None);\n",
    "+    }\n",
    "+    #[test]\n",
    "+    fn door_cell_duplicate_number_never_selects_first_or_last_cell() {\n",
    "+        let first = LogicalCell { x: 1, y: 1, z: 7 };\n",
    "+        let middle = LogicalCell { x: 2, y: 1, z: 7 };\n",
    "+        let last = LogicalCell { x: 3, y: 1, z: 7 };\n",
    "+        for doors in [\n",
    "+            BTreeMap::from([(first, 7), (middle, 8), (last, 7)]),\n",
    "+            BTreeMap::from([(last, 7), (middle, 8), (first, 7)]),\n",
    "+        ] {\n",
    "+            let placed = placed_doors(doors);\n",
    "+            assert!(placed.has_door(7));\n",
    "+            assert_eq!(placed.door_at(first), Some(7));\n",
    "+            assert_eq!(placed.door_at(last), Some(7));\n",
    "+            assert_eq!(placed.door_cell(7), None);\n",
    "+            assert_eq!(placed.door_cell(8), Some(middle));\n",
    "+        }\n",
    "+    }\n",
    "     #[test]\n",
    "     fn omission_retains_unknown_on_actual_accepted_room() {\n",
    "         let world = crate::foundation::WorldId::decode(&[\n"
  ],
  "historical_source_manifest": {
    "state": "SOURCE_PROPOSAL_ROOT_REVIEW_PENDING_NOT_APPLIED",
    "pr": 2140,
    "cp": "D1228 https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226099560",
    "source_commit": "b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16",
    "source_path": "apps/game-server/src/content/project/native_house_tiles.rs",
    "source_git_blob": "3e2f53a79960b1e8a14113d63038638788efdb3a",
    "original_sha256": "55a58937a6d2b71a96ba7f6d4496602502aabf3dce4430dde25043b46f320bbc",
    "proposed_sha256": "b4f32977ffa5888b03dfba28129ed27d434ef179a1a7372b6282405bf4864d7d",
    "patch_sha256": "38c73cef567300e641310f4d911660cffedce8bfc544d2c5396f2aa63d3d3c4e",
    "API": "pub(crate) fn QualifiedHousePlacement::door_cell(&self, id:u32)->Option<LogicalCell>",
    "behavior": "Returns the sole matching cell; None for absent or multiple matching cells, independent of insertion/map order. Does not change has_door/door_at, qualification or generation authority.",
    "source_checks": {
      "official_rustfmt_1_94_edition2024_skip_children": "PASS",
      "git_apply_check_against_exact_snapshot": "PASS",
      "diff_scope": "Only accessor hunk and3actualQualifiedHousePlacement unit tests; original qualification source and original test unchanged"
    },
    "focused_tests": {
      "authored": 3,
      "cases": [
        "absent/empty",
        "uniqueactualdoor_map_roundtrip/missingotherid",
        "duplicatedid withuniqueneighbor andbothinsertionorders"
      ],
      "uses": "Real existing struct/type and BTreeMap in native source test module; no portablecopy/facsimile or authoritymock",
      "executed": "NOT_RUN: no native Rust/PC qualification in this sourceproposal"
    },
    "consumer_status": "PENDING. No #2140 consumer checkout/exacthunk was supplied/authorized here. Do not invent caller wiring, digest accessor or transactional authority.",
    "consumer_required_binding": [
      "Exact World/House game_world_house_instances row SHARElocked in same callerTX",
      "Both qualified_placement_digest and compatible_content_digest matchactiveQualifiedHouseTiles",
      "Only positivelegacylist_id; missingplacement/mismatch/absent/duplicatecell remainslegacy",
      "Keep wildcard/receipts/FKs unchanged and addDoor(TilePos) besidelegacy"
    ],
    "shared_order": "D1228: account for2167successor before appending exactsharedhunk; current user forbids merge/rebase, so no merging performed",
    "checkout_changes": false,
    "pc_calls": false,
    "commit_push_merge": false,
    "new_cp_comment": false
  },
  "historical_source_review": {
    "state": "PASS_SOURCE_ONLY",
    "reviewer": "Root independent of author checkpoint_policy_medium",
    "source_commit": "b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16",
    "patch_sha256": "38c73cef567300e641310f4d911660cffedce8bfc544d2c5396f2aa63d3d3c4e",
    "findings": [],
    "verified": [
      "Exact requested native API uses existing BTreeMap<LogicalCell,u32>",
      "Returns sole match only; empty/multiple None preserves unknown coordinate",
      "No authority or placement/digest qualification fabricated by lookup",
      "Tests construct actual QualifiedHousePlacement/native ProjectV2DefinitionRef",
      "Original qualifier and omission behavior unchanged"
    ],
    "native_test_execution": "NOT_RUN",
    "consumer": "Pending real sameTX SHARE-lock and active content/placement digest comparison; legacy wildcard/receipts/FK preserved",
    "shared_apply": "NOT_APPLIED"
  }
}
```
