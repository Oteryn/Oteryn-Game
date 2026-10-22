# ITEM-FORGE-1 — exaltation forge (skeleton task record)

```yaml
task_id: ITEM-FORGE-1
title: "ITEM-FORGE-1: exaltation forge"
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-FORGE-1 (row 73); §7 'ITEM-TIMED-1' FORGE-*/TIER-EFFECT-1/MARKET-TIER-1 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-forge-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
MERGE_AUTHORITY: CP_ONLY
```

## Goal

Exaltation forge on top of FORGE-1a (dust asset, merged): tiers, fusion, transfer, conversions and ForgeCause (FORGE-1b), content, `FORGE_V1` wire, influenced/fiendish creatures, tier effects, Market tier wares.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

FORGE-CONTENT-1, FORGE-1b (hard), FORGE-WIRE-1 (Reg), FORGE-CREATURE-1 (combat), TIER-EFFECT-1 (combat), MARKET-TIER-1 (after MARKET-1). FORGE-1a MERGED (#1691).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| FORGE-1a | MERGED (#1691, migration 0059) |
| GOLD-FEE-2 | MERGED |
| GAME-AI-01 runtime | MERGED (foundation) |
| ATTACK-1b | MERGED |
| SPELL-D3 | MERGED |
| PREMIUM-1 | MERGED (#2110) |
| MARKET-1 (MARKET-TIER-1) | BLOCKING |
| acceptance of IMBUE_FORGE0 | ACCEPTED by ACCEPT_BATCH §1 (owner item 5) (#2209); historical brief header does not override the acceptance |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `crates/gameplay/domain/src/domain/forge.rs`
- `crates/gameplay/durability/src/durability/forge.rs`
- `crates/gameplay/durability/src/durability/forge_tests.rs`
- `apps/game-server/src/combat/tier_effects.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/forge.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/forge_tests.rs`
- `crates/protocol-oteryn/src/forge.rs`
- `crates/protocol-oteryn/src/forge_tests.rs`
- `apps/game-server/tests/support/forge_cases.rs` (was `tests/forge_postgres.rs`, removed by D1430)
- `docs/agents/tasks/ITEM-FORGE-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

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
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- crates/gameplay/durability/src/durability/core_modules.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- crates/gameplay/domain/src/domain/forge_dust.rs, durability/character_forge_dust.rs, character_forge_dust_audit.rs (FORGE-1a; extend, do not fork)
- crates/gameplay/content/src/content/item_forge289_promotion.rs, item_forge3332_promotion.rs
- monster loot/death chain for FORGE-CREATURE-1
- connection.rs dispatch; capabilities.rs `GATED` (`FORGE_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + forge writer + durability
- fusion = item BURN + tier MINT + dust/gold ledger in one transaction
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Fusion/transfer conserve value per IMBUE_FORGE0 §8-§10; failure paths deterministic and replay-stable.
- [ ] Tier caps per classification enforced.
- [ ] Forge history bounded by `IMBFORGE0-RL-12`.
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

- docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md §8-§14
- archive OTV2-20261003-forge-1a.md
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — 2026-10-15 (draft #2146)

Base `333b3c8497c0a9523232e3d9b0a13eda33c31f09`; current allocation is the
exclusive six-path lease beside the checkout. The wider historical owned list
above does not grant registration, transport, protocol, migration or publication
writes in this preparation. No merge/rebase or production wiring is performed.

### Accepted and executable subset

ACCEPT_BATCH §1 (owner item 5) accepts IMBUE-FORGE-0; the owning §8 states:
“`game_item_tiers`: one row per item instance with `tier` 1-10. No row is tier 0.
The row belongs to the ItemInstance and every TRANSFER keeps it”. It also says a
tier change is `STATE_MUTATION` / `PRESERVE_INSTANCE`. This draft prepares only:

- `domain/forge.rs`: the 0..10 tier value and pure structural checks described
  in the 2026-10-16 authoring continuation below.
- `durability/forge.rs`: private schema SQL with actual ItemInstance FK, tier
  bounds, uniqueness and immutable row identity, plus a raw single-statement
  reader. Existing item/no tier row returns zero; unknown/malformed item identity
  and corrupt stored tier fail closed. No schema installation or writer runs in
  production. The SQL is preparation for the future inseparable migration/writer
  bundle, not a numbered migration or a production-ready permissions design.
- `tests/forge_postgres.rs`: exact-source path loading while modules remain
  unregistered; actual native UUID validation, native migrations and PostgreSQL
  17.6. Fixture insertion is explicitly migration-owner setup, not a qualified
  MINT or proof of current gameplay authority.

`StoredItemTier` is raw persisted data, not current authority, ownership,
compatibility or forge permission. A reader acquires no gameplay fence/root lock.
The future writer must resolve those facts independently and re-read under its
actual locks/fences. SQL primary/FK/range/identity constraints do not enforce
Content classification/max tier, non-stackability, imbuement restrictions or
value conservation. No raw tier may authorize fusion, transfer or effects.

### cross_domain_findings / CP-slot integration packet

1. FORGE-1a already owns the dust balance/ledger; no duplicate dust asset is
   introduced. Its admitted native causes are creature-kill gain and proficiency
   spend, not Forge. Conversion/limit-raise ledger kinds do not admit Forge causes
   or supply a shared arithmetic writer.
2. Native fee causes/structural sequencing admit CharmUnassign only; item-operation
   audit has no Forge cause. Forge fee/audit/asset shapes, DUR-03 guards, typed
   terminal/replay outcomes and the actual shared commit boundary must land with
   the migration and writer. No invented receipt/request codec/digest or fallback
   fee cause is provided here.
3. `rulesets/items/exaltation-forge/index.json` is `READY_UNPOPULATED`.
   Authoring classification/max-tier imports and `ProjectV2ItemForgeProfile` are
   not an admitted executable recipe/effect provider. Native ItemDefinitionFacts
   does not currently expose forge eligibility. Content revision/ruleset/profile
   binding, recipe costs/chances/material definitions, class caps and tier effects
   need their owning current-content seams. Canary numbers remain
   `PARITY_PENDING`; none are promoted into executable policy in this draft.
4. Current ItemInstance lifecycle/ownership/location, Content compatibility,
   classification, non-stackability, imbuements and live session-generation
   authority must be qualified by the eventual real writer. Raw input arguments
   and the persisted tier row cannot manufacture that authority. This preparation
   does not perform a gameplay TRANSFER, economic mutation or concurrency/replay
   operation; those acceptance criteria remain open.
5. At CP slot revalidate against then-current main and the actual ITEM-LOOP,
   ITEM-MOVE and MARKET dependencies. Install schema, grants/roles, Forge writer,
   audit/cause shapes and native registrations only in the accepted inseparable
   bundle; add current-fence, revision-change, ambiguous commit, replay,
   concurrency, transfer preservation and conservation integration tests there.

### Validation evidence

Durable commands, original logs, source hashes and honest qualification limits:
`/workspace/oteryn-prep/item-forge-1-evidence/index.md`. This is bounded private
schema/read preparation; family acceptance checkboxes above remain intentionally
open. No runtime forge, fee/dust mutation, effects, content generation or pins are
implemented or enabled.

## Authoring continuation — 2026-10-16

The current chat assigns one exclusive author for this PR, recorded in
`/workspace/oteryn-family/AUTHOR_ASSIGNMENTS.json`; the earlier private lease
reference above is historical preparation evidence. The continuation stays in
the assigned forge core, test and task-record paths.

`ForgeItemShape` now consumes the existing `ProjectV2ItemForgeProfile` and native
`ItemDefinitionFacts`/`ItemStackClass`. It refuses absent profiles, classifications
outside 1-4, caps outside 1-10, unknown or stackable definitions and stored tiers
above the supplied definition cap. Caps are Content inputs, not Canary constants.
Standard fusion checks the complete definition identity including revision,
consistent profiles, equal tiers and space for the success tier. Standard
transfer checks the shared class, an untiered receiver, donor tier at least two
and the receiver's cap for donor tier minus one. Convergence transfer enforces
the same transfer prerequisites and class four on both definitions, then keeps
the donor tier subject to the receiver cap. These are reusable pure predicates,
not a recipe executor or proof of current gameplay eligibility.

The actual writer still must resolve profile-to-definition binding from admitted
current Content and validate distinct live instances, main-backpack placement,
absence of imbuements, session-generation and other authority under its actual
locks. No authoring profile, returned shape or raw stored tier grants that
authority. Convergence fusion (same-slot facts are absent from the supplied native
types), RNG, costs, bonuses, receipts, history, schema installation,
mutation, wire, effects and the existing shared integration blockers remain open.

Five focused test groups in `durability/forge_tests.rs` cover missing/malformed
profiles, class/cap boundaries, unknown and stackable facts, full definition
revision mismatch, fusion cap/equal-tier failures and transfer donor/receiver
constraints, including convergence class requirements and preserving tier ten
only when the receiver cap permits it. The existing exact-source `forge_postgres`
harness loads these tests using production Content/native item types. Database
cases retain their canonical
PostgreSQL 17.6 route. No storage reader defect was established in this sweep.

Validated with native Rust 1.94.0:
`cargo test --locked -p oteryn-game-server --test forge_postgres --quiet -- --test-threads=1`
passed 9/9 tests (zero failed, ignored or filtered): five new structural predicate
groups, the existing tier-value test and three actual PostgreSQL 17.6 cases
requiring `server_version_num = 170006`. The canonical PostgreSQL URL was set.
`forge_postgres` is an automatically discovered integration target under Cargo's
default `autotests = true`; it has no explicit `[[test]]` entry. The disabled
`character_forge_dust_postgres` entry is a separate target.
`cargo fmt --all --check`, `git diff --check` and the governance validator passed.
The unchanged governance test tree `c32e3d282081025220746cd71779187496cea77c`
passed its shared 124-test suite. Evidence:
`/workspace/oteryn-family/forge-native-test.log`,
`/workspace/oteryn-family/item-forge-1-fmt.log`,
`/workspace/oteryn-family/item-forge-1-governance.log` and
`/workspace/oteryn-family/VALIDATION_PROGRESS.md`.
Strict package Clippy, the full package test suite and full-head CI qualification
remain pending. These checks prove the supplied-fact predicates and existing
private tier schema/reader, not a live Forge writer, current Content binding,
atomic economy operation or runtime dispatch. The family acceptance checkboxes
and shared integration blockers remain open. Historical validation above does
not validate this continuation.


### Terminal private qualification checkpoint (2026-10-16)

The frozen private test input is `PC_SOURCE_PACKET.json` SHA-256
`ce0c7dc10623a9b54971a7b5ddff1f9cf2101aed34b4123efc9f2c8675ea792d`,
full source manifest `75fe38c2dd11e827a26d637c0747722175141c6ff8ea86e8ec88ff2065883e4b`.
Its historical task-card bytes precede this documentation candidate.

Historical combined run `evidence-run-1792167456282407854` in
`PC_FINAL_EVIDENCE.json` passed fmt and strict package all-target Clippy but
failed package tests (exit 101): library 3150 passed / 29 ignored; later
`character_authority_postgres` 1490 passed / 1 failed at
`postgres_target_aggregation::every_postgres_target_runs_its_cases_in_a_ci_run_target`.
No focused PG stage executed in that historical run. Launch 041 subsequently
held before tests on the occupied canonical PG lock (observation 043), neither
a focused test PASS nor a test FAIL.

Actual focused run `evidence-run-1792176419124648929` is retained in
`PC_FOCUSED_FINAL_EVIDENCE.json`, including exact native commands, logs and
hash-verified ELF archive receipts. Runner V3 SHA-256
`6f86ba9abef7c0dbb611a9bb59b93f3fcc39531253f58016ce67907700fac14a`;
recipe V3 `8aedcff0eda55fb0d6b61ffa023aaa118e7bad1bc95e42064d7298bc30dace56`.
Forge passed 9/9, including three actual PostgreSQL 17.6 cases. Imbue passed
22/22 (12 pure + 10 actual PostgreSQL 17.6 cases, each asserting 170006),
including complete-before-binding, with the same unchanged Root deadlines.
Storage passed 7/7 pure native depot tests in the test-only registration overlay;
its four actual PostgreSQL cloud cases remain separate historical evidence.
The earlier cloud Imbue 21/22 `RootPassDeadlineExceeded` failure remains retained;
this later PASS establishes no causal diagnosis for that earlier failure.

Cleanup is clean, source mode plain, with no remaining owned process group.
`PC_CALL_047_RESULT.json` attests `RELEASED_NO_ACTIVE_CLAIM`, supervisor 163594,
start ticks 2603394, boot `1de2caf4-8501-4962-98e7-73177714c673`.
These are private fixture results, not production installation or exact-head CI.

The exact shared CI remedy `CP_PG_TARGET_REGISTRATION.patch` and rationale `.md`
remain UNAPPLIED: register the existing Storage/Forge targets in
`.github/workflows/merge-group-gate.yml` through the existing helper. This shared
workflow slot and qualifying package/CI results remain required for this target.

Other existing product integration slots and full mechanic acceptance remain gated.
No Rust, workflow or shared source changed here; no commit, publication, new PR,
ready, merge or complete package qualification is claimed.

### Local recovery checkpoint (2026-10-16)

Owned Rust bytes retain the qualified native9/9 result, including3 actual
PostgreSQL17.6 cases. Imbue's later normal-package PASS belongs to its different
frozen input. This Forge-only full normal package is NOT_RUN; the combined
input's actual aggregation-guard FAIL above remains RED.
Publication is held under root AGENTS.md47 (run local changed-path checks before
pushing and fix all failures of a run in one push) and CONTEXT_ROUTING.md68
(Rust changes require fmt, strict all-target Clippy and normal package tests).
The two-line shared workflow registration remedy remains unapplied pending its
CP slot. The owned candidate is secured as a local commit and verified recovery
bundle outside this checkout; exact identifiers are in the family recovery record.
No shared file, ready, merge or rebase is part of this checkpoint.

### D1122 owned-source checkpoint — NOT_FULLY_QUALIFIED

CP D1122 (https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
permits one coherent checkpoint push to this existing draft before full
qualification, with passing fmt/diff checks and explicit RED/NOT_RUN disclosure.
This supersedes the earlier AGENTS.md47 publication hold only for that bounded
checkpoint; full checks remain required before a qualified frozen review head.
No ready, merge, review trigger or CI restart is authorized by this record.

This increment changes only this task card. The already reviewed owned
structural predicates and meaningful regression tests retain their exact Rust
bytes from local recovery commit `46120db4f14ddab2e45281d91561e34ecd673922`.
Retained native evidence is 9/9, including three actual PostgreSQL17.6 cases;
private validation-overlay fmt/Clippy and combined-input results keep their
original source/registration provenance. They are not a fresh checkpoint PASS.
The historical combined package aggregation-guard failure remains RED.

Own full normal package, exact-checkpoint all-target Clippy, native module
registration/compilation, a fresh canonical-PG target invocation and new exact-head
CI are NOT_RUN for this checkpoint. No Imbue or Loop result is borrowed.
Current formatting/governance checks, if run by the executor, are recorded
separately; no execution is claimed by this authoring entry.

D1145's atomic fee/dust/item/audit/receipt/RNG writer remains after #2156.
FORGE-CONTENT executable recipes are missing; the dust limit remains
OWNER/PARITY_PENDING under D1163, without guessed cap/cost constants. Migration
0103 is reserved, not installed. D1147/D1182 workflow target and protected-pin
work follow Storage's merge to main and the required CP pin approval. This
increment changes no workflow, pin, module registration, schema or shared source.
It does not enable a runtime Forge operation or close mechanic acceptance.

### D1230 accepted dust policy — NOT_FULLY_QUALIFIED

CP D1230 (comment6226138153) supersedes the earlier dust-policy
OWNER/PARITY_PENDING statement for these exact accepted facts. GUI and CIPSOFT
S1 establish `limit(n)=100+20n` and next cost25 at n0. General next cost`25+n`,
maximum n225 and resulting4600 are separately **PARITY_WIKI W1**. GUI establishes
60 dust→3slivers and50slivers→1core. CP D1230f (comment6226341619) states that
Forge has no gold→dust conversion: no such function is implemented. The GUI
shows only the current unlocked limit; the wiki maximum is not a display value.
The evidence source here is the exact accepted CP packet; no screenshot/GUI or
zip inspection by this worker is claimed.

The current increment adds only owned pure checked limit/next-upgrade quotes
and single-conversion resource projections in domain/forge.rs, plus four owned
unit tests covering formula/base/ceiling, exact resource costs, underfunding and
overflow. It changes no Content recipe or existing Forge dust writer/schema.
The earlier `limit−75`/325 formula was absent from both owned files. Unknown or
unaccepted Forge facts remain unknown; no item recipe, roll or cost is inferred.

Under CP D1160, the only registration change is `#[cfg(test)] mod forge;` in the
native domain root. It enables this owned source for native library tests,
without runtime registration or an additional target/runner. The actual owned
file contains five pre-existing pure cases plus four new cases (nine total).
Actual native command:
`cargo test --locked --offline -p oteryn-game-server --lib domain::forge::tests:: --quiet -- --test-threads=1`.
It passed all nine cases (five existing plus four new),0failed/ignored, PG URLunset;
exit0 and full log hash907e89cc3683e0fcef5a766b6dd1100ebc4e2922ef14492576e826092db53794
are retained in `/workspace/oteryn-family/forge-d1230/native-pure.json`.
Independent source review and changed-file fmt/diff/governance PASS. Current
strict Clippy/full package/PG/runtime and exact-head CI remain NOT_RUN; this
D1122 draft checkpoint is NOT_FULLY_QUALIFIED.

The earlier native9/9 evidence, including three PostgreSQL17.6 cases, remains
bound to the original source/context preserved by the D1122 checkpoint and
recovery commit46120db4f14ddab2e45281d91561e34ecd673922. That result is not reused
as qualification of the changed D1230 policy or test registration. D1145's shared
atomic writer remains gated after#2156; FORGE-CONTENT recipes, runtime operation
and full mechanic acceptance remain pending. No runtime, GUI or complete
qualification is claimed.

### 2026-10-17 — test-source registration repair (AUTHORING)

The continuation starts from draft #2146 head
`86bf82f27326fc81ed8f1c6bd1d88f3b9a751847`. Source inspection found that
`domain/forge.rs` included the same nine pure cases in both the native library
and the path-loaded `forge_postgres` integration target. The repository's
duplicate-test-source guard has no Forge exception.

The repair mounts `durability/forge_tests.rs` once, as the native
`domain::forge_tests` sibling module, and uses explicit imports from
`crate::domain::forge`. Both native module mounts remain test-only under
`#[cfg(test)]`, preserving the D1160 registration boundary. The nine tests' assertions,
Forge algorithms and PostgreSQL cases are unchanged. The focused library filter
is now `domain::forge_tests::`; the historical `domain::forge::tests::` filter
does not select these cases.

The exact repository checker rejected the original source-derived two-harness
inclusion graph and found no duplicate for the repaired graph. This is a static
reproduction, not Cargo dep-info or an executed native test suite.
`git apply --check --whitespace=error-all` passed against the exact originals.
Independent source review found no blocking issue. Fresh native fmt, strict
all-target Clippy, package tests, canonical PostgreSQL and exact-head CI remain
pending for this authoring candidate.

Current protected-main workflow blob
`d7cfbf90b2b81c43f92c010660f41d242a3cf840` contains the canonical
`run_registered_target` helper but no `forge_postgres` invocation. The minimum
shared integration delta is
`run_registered_target forge_postgres apps/game-server/tests/forge_postgres.rs`;
its workflow/pin slot remains with the publication coordinator. This owned
repair does not install that delta or claim full package qualification.

D1145's atomic writer, the shared Forge dust schema/cause changes, accepted
executable Content recipes and runtime operation remain open dependencies.
At the inspected PR head, the production dust model admits limits 100–225; the D1230 pure
limit policy is not a production migration or a substitute for that integration.
No mechanic acceptance box is closed by this test registration repair.

### 2026-10-17 — PG target registration and workflow pins (AUTHORING)

This continuation repairs draft head
`86bf82f27326fc81ed8f1c6bd1d88f3b9a751847`. The current user instruction to
complete and update this existing draft, with the lead as the sole writer,
authorizes this bounded CI repair now. D1147 and D1182 above retain their
historical Storage-first sequencing and exact Storage pin approval; they are
not represented as approval of the new Forge workflow hash. No other PR is
merged or rebased for this draft checkpoint.

Current main `cecbfda32b8d4cbb830566cf1b7d1173bef2a6cd` and this original
Forge head share workflow blob `d7cfbf90b2b81c43f92c010660f41d242a3cf840`.
The workflow now appends only its own existing target through the unchanged
helper:
`run_registered_target forge_postgres apps/game-server/tests/forge_postgres.rs`.
All four existing target registrations, exact source/metadata checks and the
PostgreSQL 17.6 service and routed admin environment remain unchanged.

The resulting workflow Git blob is
`bb57d117da6eeb58d61d6ca946605ab662c02490`. Only the existing `APPROVED`
constant in `tools/repository/test_validate_merge_group_pg_sim.py` and
`EXPECTED_MERGE_GROUP_GATE_BLOB` in
`tools/repository/validate_repository_policy_core.py` change to that exact hash.
No validator logic, aggregation or duplicate-source guard, allowlist or helper
behavior changes. The one-line workflow edit alone would leave both protected
pin checks inconsistent, so these three lines form one repair.

The unchanged native `tests/support/postgres_target_aggregation.rs` reads
`run_registered_target` declarations and requires standalone PG cases to be
included by a declared target or run directly. This registration selects the
existing `forge_postgres` target's four cases: one pure tier-bound case and three
actual PostgreSQL 17.6 schema/read cases. It adds no fixture or target and makes
no new production Forge operation available. The separate test-source repair
retains all nine pure assertions under the `domain::forge_tests::` filter.

Exact-source review and hash checks cover the authored input. Governance,
repository policy and workflow regressions, native aggregation and Cargo
dep-info guards, fmt, strict all-target Clippy, the normal package, canonical
PG execution and exact-head CI remain pending for the assembled candidate.
Historical receipts above retain their original source and execution scope;
this checkpoint remains NOT_FULLY_QUALIFIED. D1145's atomic writer and executable
Content dependencies remain open.

### 2026-10-17 — native qualification through the shared PG (D1329)

The registration candidate was executed on Molehill-PC in a genuine Git
worktree at base `86bf82f27326fc81ed8f1c6bd1d88f3b9a751847`, with a separate
Cargo target directory for this PR. The tested seven-file patch before this
receipt-only card addition has SHA-256
`74183ed05d483f1637db935d7e57ea34c45cd652c34bebab384c8b16dff9b0f2`.
Its Rust, workflow and validator bytes are unchanged by this receipt.

| Executed check | Result |
| --- | --- |
| `cargo +1.95.0 fmt --all --check` | PASS |
| Native governance and its Python tests | PASS, 124 tests |
| Strict package Clippy, all targets, `-D warnings` | PASS, 192.37 seconds |
| Actual Cargo artifact/dep-info duplicate-source guard | PASS, 56 test-harness units and 11 existing allowlisted targets |
| `--lib domain::forge_tests::` | 9 passed, 0 failed, 0 ignored |
| Existing `--test forge_postgres` target | 4 passed, 0 failed, 0 ignored; one pure case and three actual PostgreSQL 17.6 cases |
| Governance, repository policy, governance lifecycle, complete merge-group PG workflow regression programs | All four PASS with actual PowerShell 7.6.7 |

The complete workflow regression exercised its real Atlas/World aggregation,
native-target failure positions, workflow mutations and fan-in rejection cases.
The earlier missing-PowerShell attempt remains recorded as a failed preparation
attempt and is not substituted for this successful execution.

PG execution followed the current user D1329 instruction: first
`cargo fetch --locked`, then the exact owner-provided `in-pg-netns.sh` beside
the task copy of the supplied env. Its isolated namespace maps namespace5432
to the existing shared PostgreSQL17.6 at host127.0.0.1:55432. The helper has
SHA-256 `368fd9867c97ca1d59aa8b9f7123b77f0e23fa8b6a16fbb5ace4d18b6e1bfd99`.
No private PG server/container was started for these runs. The three PG cases
executed their exact17.6 version checks and cleaned their own databases.

Raw logs and source-bound JSON receipts are retained under
`/home/mole/work/pr4-20261017-1135/logs/` on Molehill-PC. The strict Clippy log
has SHA-256 `ed3e9c9a67c1849e2fa245fb4ab3d7e35018ca0854262fddff09b8afaeb06882`,
the duplicate guard log `db5b5f4338644ceb286e11783c17465770d3b88c1b71c9d8362e456c0414f2f2`,
the nine-case pure log `bbc206606b95109c6074e7a9ffc6384a2efbfeb868ce42507cbefd0e045ce6ca`,
and the four-case PG log `eac8642a9ca96a7f20f6a83f70da2b7a999e55571413416e99d47827deb2d4c1`.
The complete PG workflow-regression log has SHA-256
`e79d24b3539dea374bc6543152bfabce1a4c941371d0eb95bf158b335718135d`.

At this draft publication checkpoint, the full normal package is still running;
its partial output is not a package PASS. Exact successor CI awaits publication.
The qualified changes repair test registration and canonical routing. They do
not install the native dust LIMIT_RAISE proposal, atomic Forge recipes, current
world-object admission or complete mechanic acceptance. This remains a D1122
draft checkpoint, with no ready or merge claim.

### 2026-10-17 — material-free native Dust LIMIT_RAISE, qualified v3 checkpoint

This continuation implements the material-free D1230 dust-limit operation in
the existing native FORGE-1a domain, ledger and audit owner. It adds the closed
`ForgeDustLimit` receipt/event shape and its database proof proposal. The
existing Forge test-registration repair remains present; no second dust asset
or material recipe is introduced.

The actual qualified input is the complete private Git composition
`40d2cb77e4bee6e2ea86e79fc02aecdcfa323a15`, with parents
`d992947c085133322a0b567fb70a0d654e1d9c30` and
`f010472bcb6794a2cbc56dfaf755bd122c10225e`. Its base tree is
`96377f486048780aca9879f9507395acb30a3582`; the coordinator captured the exact
16-file native input as tree `8a0f7d3a0b688ac1d277c0649f623427f016c1b7`.
The source capture has SHA256
`13e8d5f53b281e6a10c7334292238d11848f0eb9911f652515e6a5f332bd9e7e`,
and the actual v3 formatted-source receipt has SHA256
`c8a4aa7a3f613c2febb71e8df3c5074035a6af6a36c25d5d8116a2b890176db6`.
This composition has no Items87e parent. This appended task-card receipt is a
separate seventeenth file and does not change the 16 qualified source files.

#### Implemented behavior

[D1230](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6226138153)
defines the unlocked limit as `100 + 20*n`. For `n` in 0..224, a successful
upgrade burns `25+n` dust and advances to stage `n+1`. There are at most 225
paid upgrades, ending at a limit of 4,600. The base/increment and initial cost
retain their GUI/CIPSOFT evidence; the general cost and maximum remain
explicitly `PARITY_WIKI` under that decision. The pure suite and the native PG
transition case cover every paid stage. This operation consumes no material
ItemInstance, gold, recipe selection or RNG draw.

The caller supplies the Character occurrence and expected stage. The owner
validates actual recovery/fence evidence and reads active Content under the
native activation lock. The caller cannot choose the debit, resulting limit,
Content digest, formula revision or simulation revision. Commit takes the
actual Character-root lock before the dust row, revalidates current native
authority, and rejects a stale quote without charging the next stage's price.

A successful physical transaction writes the dust ledger entry, guarded
balance/limit update, closed type-2 audit/outbox event, immutable receipt and
bounded history row together. Deferred proof requires their exact
same-transaction counterparts and digest; an old or unrelated receipt cannot
prove a new effect. CharacterRevision is read under the root lock and remains
unchanged. Existing Gain still records credited/lost dust and respects the
unlocked limit; Spend retains its overdraft and Proficiency receipt-pairing
guards. Existing native audit arms, including HOUSE, remain intact.

The same occurrence replays its retained Raised or terminal RevisionChanged
receipt, including a new CommandRef under valid recovery evidence. Changed
bound Content, formula or profile terminally refuses an uncommitted occurrence
with no ledger, state, audit or history effect. Reactivating old Content cannot
reactivate it. The existing three-work-unit retry budget and bounded
reconciliation path remain explicit. The rollback case injects a known final
receipt-trigger noncommit; it does not claim a network-ambiguous COMMIT test.

The append proposal excludes writes and verifies the complete historical
Gain/Spend chain, reachability and final balance agreement before changing
constraints. Every old limit must be 100. An unexplained historical limit or a
detached cycle aborts without inventing paid unlock history. These refusals
were tested in the harness databases; no deployed-data preflight is claimed.

#### Completed native v3 evidence

The coordinator executed v3 on Molehill-PC using Rust 1.95.0, jobs 2 and the
separate `targets/feature2146` directory. The complete terminal receipt bundle
`native-receipts-v3.json` has SHA256
`4d478fc6622c72f816bccac1676aa5809af8c0717d4e3937ba265e30be4b6a4e`.
Governance, Clippy, the actual duplicate-source checker and every test stage
carry the same complete 16-file formatted source mapping.

| Executed gate | Terminal result | Actual log SHA256 |
| --- | --- | --- |
| `cargo +1.95.0 fetch --locked` | Completed, exit 0; before offline PG execution | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `cargo +1.95.0 fmt --all` | Completed, exit 0; resulting bytes bound by the formatted-source receipt above | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Native governance validator | PASS, exit 0 | `ed5de0b194f14550187fbb13e78364869d9be5848032f691f807fecb2abe0a1f` |
| Strict package Clippy, all targets, `-D warnings` | PASS, exit 0, 47.48 seconds | `8983efcfd7271cbb9801e41e5087aefae29ed8508a1c51636ec51445d051f67d` |
| Actual Cargo artifact/dep-info duplicate-source guard | PASS; 56 test-harness units, existing allowlist unchanged | `db5b5f4338644ceb286e11783c17465770d3b88c1b71c9d8362e456c0414f2f2` |
| `--lib forge_dust_limit_audit_tests::` | 5 passed, 0 failed, 0 ignored | `8dabef3dcc7f00c0144cef76bf42cfc1de42b0f546f99da0a0129e6c134bcc15` |
| `--test character_authority_postgres forge_dust_limit_postgres_cases::` | 11 passed, 0 failed, 0 ignored | `588102a3d33835a67d1f19a7501cb382153c2c6e76b7512f4f3a676e28b8dc67` |
| `--test character_forge_dust_postgres character_forge_dust_postgres_cases::` | 8 passed, 0 failed, 0 ignored; 48.48 seconds | `31a56a163b2ddae27af89e173093cc07cceeaec1c2454b9308b37b4bbc14010d` |
| Full normal `oteryn-game-server` package, PostgreSQL URL absent | 10,269 passed, 0 failed, 37 ignored; 58 groups; 547.82 seconds | `c038b77d73b77e11b0f57d2ac2acb11a4c51782e9f5f17a5c35cfee72396a42e` |

Fetch and formatting record their pre-format input. The resulting native case
file changes from blob `3fdf422ba0a1808bc7003e72fa85dad56e540b58` to
`f16a644a1ead42edde2888ae9aea055be796dd70` through rustfmt; all subsequent gates
are bound to the latter. The formatting result above is the actual formatting
command, not a claim that a separate v3 `fmt --check` command ran.

Both PG suites ran actual PostgreSQL 17.6 and enforce
`server_version_num=170006`. Execution followed D1329: locked Cargo fetch first,
then the unchanged owner `in-pg-netns.sh` routing namespace5432 to the existing
shared PostgreSQL at host127.0.0.1:55432. The owner helper retains SHA256
`368fd9867c97ca1d59aa8b9f7123b77f0e23fa8b6a16fbb5ace4d18b6e1bfd99`.
No harness guard was relaxed and no private PG server/container was started for
this qualification. The normal-package result and the two actual PG results
remain separate evidence. Command logs and source maps are retained under
`/home/mole/work/pr4-20261017-1135/logs/`.

#### Retained failed attempts and fixture repairs

The failed v1 qualification remains a failed attempt. Its negative audit test
was repaired to use the registry's existing `SECURITY_SENSITIVE` literal 4
instead of accessing the auditor's private constant. Production visibility,
the privacy type and the closed decoder's rejection predicate were unchanged.

The earlier v2 PG result also remains failed: 9 passed, 2 failed, exit 101,
log SHA256
`9bce58f1ce3072ae98d6cb73e78c6a8805d05d955cea6fead012836a48fce1ba`.
One fixture attempted the schema-forbidden lifecycle 2. The other used one
DurabilityRoot for both concurrent attempts and exercised local root admission
instead of the intended database race. The v3 successor verifies the native
rejection of the forbidden lifecycle mutation and unchanged stored row, and
uses two genuine DurabilityRoots to prove database serialization and the losing
stale quote. It disables or relaxes no production constraint. The other nine
native cases retain their previous assertions. The earlier historical
registration/package failures above keep their original source and scope;
the v3 success does not rewrite them as passes.

#### Checkpoint and installation boundary

The native SQL remains an unnumbered append proposal mirrored in
`tests/support/forge_dust_limit_migration.sql`. The new PG cases apply it only
inside their own harness databases after the ordinary migrations. This
checkpoint does not rewrite migration 0059 or claim historical slot 0102.
The corresponding successor to the legacy dust CHECK-bound regression is
retained with the eventual real numbered append; the current eight legacy
cases correctly exercise their currently installed schema.

The closed operation's proposed tag 13 and resource/documentation additions
belong to this reviewed composed packet; they do not announce a new CP
reservation. The existing event type, schema/retention tuples and native
operations retain their meanings. The DUR-03 and CharacterRevision composition
addenda state the dust burn, exact state proof and unchanged CharacterRevision
model explicitly.

Forge object USE/window admission, FORGE_V1 pending-command handling, material
recipes, gold fees, conversions, fusion, transfer, tier effects and the combined
Forge history API remain outside this continuation. Those families must
compose their accepted current authority and common history budget. Final
numbered installation and deployment-data preflight require their actual
composed-tree binding. The v3 local evidence is bound to the native input tree
above; it is not a ready/merge declaration or exact-successor CI receipt.
Post-publication CI must be read on its actual published head. The task remains
in authoring, with the family acceptance checkboxes unchanged.

### 2026-10-17 — native DustLimit retained-history DATA reader, qualified v4 checkpoint

This successor adds `read_forge_dust_limit_history(connection, character_id,
before_ordinal, page_size)` to the existing native FORGE-1a owner. It returns
typed, retained successful DustLimit receipts and their history ordinals. Its
boundary is DATA reading under the caller's existing read admission, as for
`read_forge_dust`. A CharacterId or cursor does not establish current gameplay,
session, Content, custody, recovery or Forge-object authority.

#### Exact source and composition

The coordinator qualified the complete private composition
`32f55a8e98afcd9e6415b79c6ccaeaf7004e01ed` of published Forge predecessor
`723ad25188671494148490631633bdf0edf2328d` and the pinned Ground main
`ce7bccab973625d5d3d0d0c566426903229d2373`. Its private base tree is
`59b9d402fd1edff1e8905b2ba49a96be92bd8740`. The exact native GREEN tree is
`5e300afe8c14c0b69f4326f4cb77b2daae45d0fc`, captured by
`exports/feature2146-tree-history-v4-native.json`, SHA256
`647605f8a1e51581a6bb6dc5313a7134dd4ce9c0c522d493f08ed60f6d4e238d`. The private composition's parents describe qualification
provenance; they do not declare the parent list or SHA of the forthcoming
remote successor. A later main head requires its own composition and evidence.

The native input receipt `feature2146-source-v4.json` has SHA256
`bfddae3ccb2fa28ccb2431a06ab28740d9da0b53312936786b78206ad1cae146`. The exported source packet has SHA256
`273146b02a4e6fc16e3ee77840cc0d05d81cbb2b568a54c3778a96af9178fbcc`. The actual formatted-source receipt has SHA256
`6b397b76c7093e13ab3ef3024ab9b1201a08b3cd1d5915adc29f4860b7cab54b`.

| Native GREEN source | Git blob | SHA256 |
| --- | --- | --- |
| `crates/gameplay/durability/src/durability/character_forge_dust.rs` | `66b2a2af899a30a50dc5093642b0ac1e015f540a` | `e81776ff2570c34c2457ea11fa33367e872c36e8563393c1a8655a0b2d857870` |
| `apps/game-server/tests/support/forge_dust_limit_postgres_cases.rs` | `40241422f4091586793a0a02a3a42483aa3b1472` | `74eea614a134a8ae37048893acf15a5ec3f23ec966cfb30c0dee572efcc617cf` |

These are the two history-continuation sources on that complete base. The
separate task-card append preserves every byte of the prior 39,806-byte record.
Actual rustfmt changed only five layout sites and three optional trailing
commas relative to the frozen history proposal. All fourteen test names and
the complete earlier eleven-case/support prefix remain unchanged. The bounded
source-consistency review has SHA256
`0964c40ddd803f151e842d3340c2328f9debe78d78f0cefd53a350c6916ee7e8`. This review performed no additional native run.

#### Reader behavior and scope

One PostgreSQL statement reads history, reservation and receipt under one
statement snapshot. It binds the CharacterId and uses descending, exclusive
ordinal keyset pagination. The decoder checks the coupled stored evidence and
rejects missing, mismatched or malformed state. Prepared reservations and
terminal `RevisionChanged` receipts remain absent from successful history.
Existing reservation and receipt decoders are reused with their checks intact.

The page size is 1 through the existing 1,000-row retention ceiling. A smaller
page takes one lookahead row within the same ceiling; no offset, count scan or
per-row query is added. Cursor zero or a value beyond positive PostgreSQL
BIGINT is refused before SQL. A newer native commit between pages does not
repeat the retained cursor row. Each page follows the caller's transaction
isolation and existing retention; the reader does not promise a cross-page
snapshot. The current DustLimit algebra permits 225 successful raises, so the
1,000 page-size boundary test does not fabricate 1,000 successful operations.

Three added native cases cover real commit/replay pagination, exclusion of
prepared and terminal-refused occurrences, invalid page/cursor boundaries,
and actual same-World and cross-World Character DATA isolation. The actual
fixture successor uses distinct valid Character names. It does not fabricate
current authority for those other Character roots. All eleven earlier native
DustLimit cases and their assertions remain present.

#### Completed native v4 evidence

The coordinator executed all thirteen stages on Molehill-PC with Rust 1.95.0,
jobs 2, the dedicated feature target directory and the pinned complete Ground
composition above. The terminal summary is
`feature2146-history-v4-summary.json`, SHA256
`7634abf859b3aff919397095a271df47249608c84e214f18eab6cf19188b0e6e`,
with state `COMPLETE_NATIVE_QUALIFICATION`. The complete imported receipt
bundle has SHA256 `2ce3639ff40df7460d0c763e891ec0da2f6a8795f52a4bcd86a89b459585df29`; all 27 contained receipt hashes were
independently reproduced. Each post-format GREEN gate is bound to the two
exact source identities above. Fetch/metadata retain their real pre-format
input, and both deliberate RED executions retain their own mutated input.

| Executed stage | Terminal result | Actual log SHA256 |
| --- | --- | --- |
| Locked Cargo fetch, before offline PG routing | PASS, exit 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Actual Cargo metadata / Rust edition | PASS, exit 0 | `5028ae447f5e7a743142354754441fbe6c1eb123ec6bd4d51df6b6ec3115df33` |
| Rust 1.95.0 formatting of the two owned Rust files | PASS, exit 0; actual successor bytes captured | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Whole-workspace `cargo +1.95.0 fmt --all -- --check` | PASS, exit 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Native governance validator | PASS, exit 0 | `ed5de0b194f14550187fbb13e78364869d9be5848032f691f807fecb2abe0a1f` |
| Strict package Clippy, all targets, `-D warnings` | PASS, exit 0; 156.81 seconds | `0be416910e599da305236a9011ec6e342c94e71e87178ae04f833787a4cd9133` |
| Actual Cargo artifact/dep-info duplicate-source guard | PASS; existing source-registration policy retained | `db5b5f4338644ceb286e11783c17465770d3b88c1b71c9d8362e456c0414f2f2` |
| `--lib forge_dust_limit_audit_tests::` | 5 passed, 0 failed, 0 ignored | `75bf622855b8318f19be42f3a18ab8cc023fe1e830007d062497fb31631f6816` |
| RED: remove only the history Character row filter | Intended regression reproduced; 0 passed, 1 failed, exit 101 | `a83589b15dc32c7ea568842205ee898323b1d619cb11d991bbadb55311ae5bec` |
| RED: change only exclusive `<` cursor comparison to `<=` | Intended regression reproduced; 0 passed, 1 failed, exit 101 | `141215d558e05e09db1666c281ff671279e7dfe143f9a2713a70a2111e389e67` |
| Canonical `character_authority_postgres` / `forge_dust_limit_postgres_cases::` | 14 passed, 0 failed, 0 ignored; 132.41 seconds | `266feee663152c873ce848ddb21c39c9189a05c24e5608e509e1357b28f022a8` |
| Existing `character_forge_dust_postgres` target | 8 passed, 0 failed, 0 ignored; 36.66 seconds | `64c6ba9cb4ccacd2c731f26bafb631cbaebefb7a1939fd9708dbd879866c972b` |
| Full normal `oteryn-game-server` package, PostgreSQL URL absent | 10,300 passed, 0 failed, 37 ignored; 58 groups; 862.29 seconds | `e182cb44c31ff1cbca1e4fbd47010fa3bbdb5c8d80b17eebb7c28c26f55ddbed` |

Each RED run reached the real PostgreSQL 17.6 fixture and failed its exact
selected native assertion. They are successful regression witnesses, not
passing tests. Both runs restored the exact GREEN owner before the following
stage. The restored canonical GREEN suite passed all fourteen names with
fourteen `server_version_num=170006` markers. The eight legacy dust cases also
executed their actual source-bound 170006 checks on the same approved route;
that legacy harness emits no per-case version marker, so no eight-marker
claim is made. The normal-package result remains separate from actual PG
qualification; its 37 existing ignored cases are reported unchanged.

D1329 routing remained in force: `cargo fetch --locked` first, then the owner's
unchanged namespace helper mapping isolated loopback5432 to the existing
shared PG17.6 at host127.0.0.1:55432. No new PG server or container was started.
The cleanup receipt confirms the exact fourteen GREEN databases named in its
own case output are absent; RED cleanup likewise uses its own logged names.
The legacy harness retains its own database cleanup. These scoped receipts do
not assert ownership of unlogged databases or absence of unrelated databases.

The full normal-package receipt has SHA256
`30d479290ef498a074a162b19874858926241aee969fb5b23d4f82a95048bdd8`. Logs and JSON evidence
remain under `/home/mole/work/pr4-20261017-1135/logs/`.

#### Retained installation and gameplay boundaries

The history continuation adds no SQL migration, protocol field, operation tag,
grant, workflow or registration change. Existing Gain, Spend, DustLimit freeze,
commit and reconcile behavior, native fences, root locks, audit and retry
budgets remain in the qualified composition. Historical failed attempts keep
their original source and failed status; this v4 checkpoint does not rewrite
them or borrow their results.

The DustLimit schema is still an unnumbered append proposal installed only in
its native harness databases. This reader does not install it in production,
claim a migration lease or make Forge USE/window admission available. Runtime
`FORGE_V1`, material recipes, gold fees, conversions, fusion, transfer, tier
effects and the combined Forge history API retain their separate accepted
owner/integration requirements. The qualified reader covers only retained
DustLimit success DATA. Native tree qualification is complete for the scope
above; final card governance, remote successor identity and that exact head's
CI are separate publication evidence. This remains an authoring draft with
family acceptance checkboxes unchanged.

## Bounded checkpoint — 2026-10-17 (CP D1430)

MERGE_AUTHORITY: CP_ONLY. Draft retained; the builder never marks ready.

- Merged `main` into the branch (additive conflicts only; both sides kept).
- The branch carries `main` bytes for `.github/workflows/merge-group-gate.yml`,
  `tools/repository/validate_repository_policy_core.py` and
  `tools/repository/test_validate_merge_group_pg_sim.py`; the earlier
  `forge_postgres` gate registration was dropped (it failed the protected-base
  Merge authority audit).
- PG-COVERAGE-1 pattern: the four private tier cases moved unchanged to
  `apps/game-server/tests/support/forge_cases.rs` and are mounted by
  `character_authority_postgres.rs`. The standalone `forge_postgres.rs` target
  was removed. `domain::forge` and `durability::forge` are registered as
  modules (registration lines, re-applied by the slot commit) so the cases use
  the production types.
- Scope unchanged: native DustLimit and its DATA history reader; no numbered
  migration, no gameplay admission.
