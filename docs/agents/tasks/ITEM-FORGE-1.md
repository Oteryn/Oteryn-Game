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

- `apps/game-server/src/domain/forge.rs`
- `apps/game-server/src/durability/forge.rs`
- `apps/game-server/src/durability/forge_tests.rs`
- `apps/game-server/src/combat/tier_effects.rs`
- `apps/game-server/src/gameplay_transport/forge.rs`
- `apps/game-server/src/gameplay_transport/forge_tests.rs`
- `crates/protocol-oteryn/src/forge.rs`
- `crates/protocol-oteryn/src/forge_tests.rs`
- `apps/game-server/tests/forge_postgres.rs`
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
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/domain/forge_dust.rs, durability/character_forge_dust.rs, character_forge_dust_audit.rs (FORGE-1a; extend, do not fork)
- apps/game-server/src/content/item_forge289_promotion.rs, item_forge3332_promotion.rs
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
