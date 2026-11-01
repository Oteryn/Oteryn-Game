# OTV2-20261101-gem-atelier

```yaml
task_id: OTV2-20261101-gem-atelier
title: Gem Atelier carried resources, managed destinations, balances and Workshop catalogue
type: mechanic
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/gem-atelier
pr: 2795
base_sha: 38641fecd4558a4a14e6b15b3874110ffa2dff19
head_sha: 075ce4486355d8a7d94d49ed80b8c01a84a986ee
final_head_sha: null
final_head_frozen_at: null
owner: gem-atelier-lead
created_at: 2026-11-01
updated_at: 2026-11-01
execution_policy: continuous_progress
owned_paths:
  - apps/client/src/features/0148-wheel-allocation/tests/play.rs
  - apps/game-server/migrations/0744_character_atelier_carried_stash.sql
  - apps/game-server/migrations/0745_character_atelier_carried_coins.sql
  - apps/game-server/migrations/0746_character_atelier_carried_outputs.sql
  - apps/game-server/migrations/0747_character_atelier_audit_bounds.sql
  - crates/client-runtime/src/features/0148-wheel-allocation/protocol.rs
  - crates/client-runtime/src/features/0148-wheel-allocation/tests/resources.rs
  - crates/client-ui/src/features/0148-wheel-allocation/protocol.rs
  - crates/client-ui/src/features/0148-wheel-allocation/state.rs
  - crates/client-ui/src/features/0148-wheel-allocation/tests/resources.rs
  - crates/client-ui/src/features/0148-wheel-allocation/tests/state.rs
  - crates/client-ui/src/features/0148-wheel-allocation/tests/workshop.rs
  - crates/client-ui/src/features/0148-wheel-allocation/view.rs
  - crates/client-ui/src/features/0272-atelier-resources/fixtures/registrations.json
  - crates/client-ui/src/features/0272-atelier-resources/protocol.rs
  - crates/client-ui/src/features/0272-atelier-resources/registrations/handlers.json
  - crates/gameplay/dispatch/mechanics/0624-atelier_resources.mechanic
  - crates/gameplay/dispatch/src/gameplay_transport/atelier.rs
  - crates/gameplay/dispatch/src/gameplay_transport/atelier/resources.rs
  - crates/gameplay/dispatch/src/gameplay_transport/atelier/workshop.rs
  - crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/wire_paid/resources.rs
  - crates/gameplay/durability/src/durability/character_atelier.rs
  - crates/gameplay/durability/src/durability/character_atelier/
  - crates/gameplay/durability/src/durability/character_atelier/resources.rs
  - crates/gameplay/durability/src/durability/item_fee_burn.rs
  - crates/gameplay/durability/src/durability/item_fee_burn/
  - crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit/
  - crates/gameplay/progression/src/atelier.rs
  - crates/gameplay/progression/src/atelier/
  - crates/protocol-oteryn/capabilities/0272-atelier-resources-v1.json
  - crates/protocol-oteryn/src/atelier.rs
  - crates/protocol-oteryn/src/atelier/resources.rs
  - crates/protocol-oteryn/src/atelier/resources/tests.rs
  - crates/protocol-oteryn/src/tests/capabilities_golden/0272-atelier-resources-v1.json
  - crates/session/src/capability_support.rs
  - crates/session/src/session_tests_capabilities.rs
  - crates/session/src/session_tests_chat.rs
  - docs/agents/evidence/OTV2-20261101-gem-atelier-parity.md
  - docs/agents/tasks/OTV2-20261101-gem-atelier.md
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/protocol-oteryn/v1/atelier-resources-v1.md
  - docs/contracts/protocol-oteryn/v1/atelier-v1.md
  - docs/contracts/protocol-oteryn/v1/registry/capabilities/0272-atelier-resources-v1.json
  - docs/contracts/protocol-oteryn/v1/registry/command_types/0484-atelier-resources-query.json
  - docs/contracts/resource-limits/entries/1684-ATELIER-RL-ASSET-LINES.json
  - docs/contracts/resource-limits/entries/1685-ATELIER-RL-EFFECT-WORK.json
  - docs/contracts/resource-limits/entries/1686-ATELIER-RL-VALUE-LINES.json
  - docs/contracts/resource-limits/entries/1687-ATELIER-RL-AUDIT-BYTES.json
public_contracts:
  - docs/contracts/protocol-oteryn/v1/atelier-resources-v1.md
  - docs/contracts/protocol-oteryn/v1/atelier-v1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Complete P07's carried-container reveal, fragment payments, native managed dismantle routing, live resource balances and full compatible Workshop modifier selection through the existing Atelier panel, custody owner, revision sequencer and transaction receipt. Resource balances are separately negotiated; the Workshop catalogue uses existing query 308 grade rows. Audit every donor rule and retain named unresolved decisions rather than silently changing accepted contracts.

## Architecture and source of truth

- PROVEN: admission main `38641fecd4558a4a14e6b15b3874110ffa2dff19`.
- INTEGRATION: the publisher completed the real local merge of current main `17f035e04c317803cfd1b3f410a4e1b53d3ad6f1` and regeneration with the World pin unchanged. Its Forge audit shape keeps its own 29,494/29,808-byte limits. Available local a8/a9 implementation and metadata qualification passed; Q9 separately governs published branch sync/readback.
- PROVEN: Crystal reference `201189f871d23597b82caf88fd611454a2ddefb8`; `player_wheel.cpp:1218-1250,1514-1597`, `player.cpp:5304-5356,5821-5844`.
- PROVEN: META policy 3.1.0 bound to `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; existing Atelier decision and protocol contract remain applicable.
- OWNERSHIP: the task lead is the sole publisher of `chatgpt/gem-atelier`, with branch-write ownership yielded only for the coordinator's Q9 merge-up. The conflict-resolved local candidate is prepared and available local gates passed; a compliant normal-Git publication route and ownership/outcome handback remain external. Isolated worker ownership covered donor/rules, durable custody, protocol tests and evidence.

## Acceptance criteria

- [x] A complete sourced parity table records DONE, PARTIAL, MISSING and named native adaptations; unresolved decisions remain explicit.
- [x] Reveal consumes a carried nested gem, never a stashed gem.
- [x] Grade upgrades consume carried fragments first, then Supply Stash, in one fenced transaction with gold and the mutation.
- [x] Refusal, replay, stale-session and durable readback cases exercise the real dispatch path.
- [x] Required focused validation and number collision audit are recorded for the pre-extension implementation checkpoint.
- [x] Managed destination precedence reuses native preferences and passes real PostgreSQL refusal/fallback cases.
- [x] Separately negotiated seven-balance projection reaches the existing0148 panel and passes codec, real-session and PostgreSQL qualification.
- [x] Query 308 exposes all 69 compatible Workshop modifiers, including Grade I zeros, and the existing selector permits an upgrade without owning a gem carrying that modifier; sparse storage and immutable receipts remain unchanged.
- [ ] The successor candidate receives its own immutable-head readback and changed-path qualification.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Nested reveal | Crystal `player_wheel.cpp:1218-1250`; recursive inventory `player.cpp:5821-5844` | Existing Atelier custody owner with bounded recursive carried selection | Mounted Atelier PostgreSQL wire cases |
| Stash-paid grades | Crystal `player_wheel.cpp:1514-1597`; inventory-before-stash `player.cpp:5304-5356` | Existing stash ledger plus same-transaction Atelier proof | Mounted Atelier PostgreSQL grade/replay cases |
| Gold sources | Crystal `game.cpp:2862-2969`, recursive inventory including Store Inbox/Gold Pouch then bank | Existing item fee owner extends eligible carried sources only for Atelier; pouch debit remains PARTIAL Q8 under the accepted native exclusion | Nested coins, more than 20 sources and bank readback; no pouch-debit pass claimed |
| Dismantle destinations | Crystal `game.cpp:2782-2805`, `player.cpp:4932-5047` | Ordinary and native managed routing passed the complete34-case integration-a8 PostgreSQL suite; separate loot/obtain profiles remain Q5 | Managed precedence/fallback plus existing nested/split/refusal cases |
| Live panel balances | Crystal `player_wheel.cpp:1634-1643`, `player.cpp:1477-1503,5190-5210` | Q7 option A authorized by the task; capability272/command484 implemented and tested in the complete eight-package and34-case integration-a8 runs | Canonical codec, actual Session negotiation, current PostgreSQL balances and panel presentation |
| Workshop modifier catalogue | Crystal `player_wheel.cpp:1489-1507,1514-1568`; accepted Gem decision270-273 | Full 69 query-only grade rows; existing selector with legacy fallback; no owned-gem prerequisite and no change to Q1 draw pools | Projection/client tests and unseen-modifier upgrade inside the selected-resource PG case passed in the complete integration-a8 package/PostgreSQL runs |

The complete 42-row table is retained in `docs/agents/evidence/OTV2-20261101-gem-atelier-parity.md` and the draft PR. The balances and Workshop rows are DONE after integration-a8 source/component qualification; the table now has33 DONE and9 PARTIAL. These results do not establish complete readiness or published-head qualification. Pending source-priority/scope decisions stay visible rather than being counted as complete Crystal parity.

## High-risk authority/recovery qualification

Applicable: session-generation-fenced durable item, bank, stash and Atelier writes. Existing transaction guard and receipt own authority and replay. Negative cases independently change generation, source quantity, source ownership and command identity; refusal must preserve all original values. The mounted qualification covers direct dispatch, committed readback, same-byte replay after content removal, changed eligibility and locks, later actions and process restart. SQL probes reject historical or mismatched stash, coin, output and enlarged-audit evidence. Ordered migration reapplication compares 26 retained function definitions and committed split-output history. PUBLIC revocation and runtime grants are separately checked from the migration source.

## Excluded scope

No production activation, new gem store, protected-branch writes, PR merge into main, review requests, heavy-tests label, donor source redistribution, merged migration edits, audit-ledger edits or changes to unrelated mechanics.

## Implementation / findings

The existing native command424 and capability148 carry the resource-selection extension. No new client choice is needed for recursive selection or the automatic stash remainder. Baseline PostgreSQL cases first reproduced the missing nested reveal and stash debit; those same wire cases now pass in focused checkpoints. Four additive migrations bind the extended resources and output shapes to the existing immutable records. Fresh paid actions reload current equipment facts under the existing action lane before refreshing Wheel-derived vitals.

Atelier audit limits are separately registered: 540 assets, 1620 work units, 2 value lines, 196608 payload bytes and 262144 envelope bytes. Other fee causes retain their own owner-specific limits, including main17f's Forge payload/envelope limits of 29,494/29,808 bytes (`item_forge_live/audit.rs:13-20`). The legacy fee constants remain 25,398/25,712. Integration must distinguish the closed Atelier, Forge and legacy shapes rather than regress Forge to the legacy bound. Migration 0747 binds larger Atelier envelopes to current matching Atelier evidence.

The Workshop gap is settled by Crystal `player_wheel.cpp:1489-1507,1514-1568` and accepted `OTERYN_GAME_WHEEL_GEM0_GEM_ATELIER_DECISION_2026-09-30.md:270-273`: any compatible modifier can be graded without owning a gem that carries it. Existing query 308 gains the complete 46 Basic plus 23 vocation Supreme grade projection, using 0 when no stored grade exists; the accepted decision's sparse storage rule at142-143 remains unchanged. Mutation/lock/replay receipts keep their sparse immutable contents. The existing 0148 selector uses the query identities and selected-gem fallback for legacy sparse replies, preserving valid selection across gem changes. Both numeric kinds remain distinct. This requires no new wire tag, capability, store or Q10, and does not alter Q1's draw pool. The implementation passed the complete integration-a8 package and34-case PostgreSQL runs; publication/readiness gates remain separate.

Nine historical QUESTION comments remain on [draft PR2795](https://github.com/Oteryn/Oteryn-Game/pull/2795), with no ANSWER at the latest read. Q7 is implemented and tested and is not an open question; its scope is resolved from the original full-parity/new-capability authorization in [authoring clarification 6560586308](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6560586308), not by an invented coordinator answer. Capability 272/command 484 requires 148 and is bundle-world only; it automatically refreshes the existing 0148 panel through a separate strict read-only reply. The minimal 272 handler feature is a registration adapter; 0148 remains the panel owner. The reader `crates/gameplay/durability/src/durability/character_atelier/resources.rs:59-213` includes native Gold Pouch money at130-173,194-211, with checked addition at19-41. The pouch-inclusive implementation passed the complete34-case integration-a8 PostgreSQL suite.

Q1 and Q4 have concrete accepted authority: `OTERYN_GAME_WHEEL_GEM0A_REFERENCE_VALUES_AMENDMENT_DECISION_2026-10-01.md:4,20-41,65-69`, accepted through `OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md:98-119`. Q2/Q3 are accepted Gem decision guards (`:4-7,79-80,392,407-408`). Q6 is Gold Fee D175 (`OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md:33-40,149-156,321-322`). Q8 additionally asks whether Atelier should supersede GoldFeeDecision:104-110 and Bank Fee decision:168-173's Gold Pouch debit boundary; its independent live carried-gold display is implemented and tested in the full integration-a8 PostgreSQL run. These conflicts remain explicit pending answers. Q5 is narrowed: managed precedence using the existing native category map is implemented, with five new managed-routing PG cases passing in q7-a2 and the complete34-case integration-a8 run; distinct loot/obtain profiles require changes to the separate mechanic's wire/schema and remain outside the current other-mechanics-file scope. Existing equipment profiles also prohibit equipping these resources; that observed boundary is documented without forging a positive fixture or inventing an owner ruling.

Publication sync is temporarily pending [Q9](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6562728444). The user already authorized main-to-task merge-up; the missing piece is a compliant execution surface, not new permission. Bound META `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, `docs/agents/contracts/PUBLICATION_INTEGRITY_POLICY.md:15,27,54,66,82` and `docs/agents/operations/RESTRICTED_PUBLISHING.md:9,23` allows that normal merge-up but limits connector Git Data authoring to a sole-parent successor. The lead yields branch-write ownership only for the coordinator's one normal-Git main-to-task merge-up, expected task predecessor `075ce4486355d8a7d94d49ed80b8c01a84a986ee`, bringing current main `17f035e04c317803cfd1b3f410a4e1b53d3ad6f1` into `chatgpt/gem-atelier`. No publisher branch mutation occurs until the outcome is read back and ownership returns. Local preparation and available implementation/metadata checks are complete. Q9 is not a parity row and does not authorize merging this PR into main.

## Validation

Tests run on an isolated checkout on the owner-authorized computer because scratch has no Rust toolchain. The pre-extension core4 worktree passed all 27 mounted Atelier PostgreSQL cases. On immutable published checkpoint `075ce4486355d8a7d94d49ed80b8c01a84a986ee`, formatter, strict Clippy, progression (132), durability (604 plus 4 doctests) and dispatch (1907) passed. Its PostgreSQL runner was externally terminated after nine completion markers, with no error or final test result; that incomplete run is not a 27-case pass. For resumed work, q7-a1 architecture, eight-package all-features strict Clippy and tests passed (3,516 tests). q7-a2 passed 32 cases and failed the balances case; all five managed-routing cases passed. Diagnosis found an Entry fixture whose intersection-based selection dropped unoffered 272, leaving only 148. The direct durable diagnostic returned the expected balances, so this is not a proven production-reader defect or a successful live query. Both resource cases now use the real Bundle adapter with matching Content/frame and Channel pin: selected 148+272 and deliberately unselected 272 despite its real offer. The full suite is 34 cases (27+5+2); the Workshop unseen-modifier proof extends the selected case. The current integration-a8 component results are recorded below. Remaining readiness gates and immutable published-successor readback are separate.

The integration-a6 candidate stopped at two strict-Clippy `unused_unit` warnings, both fixed in the next synchronized source snapshot. Integration-a7 passed formatter and architecture checks, and eight-package strict Clippy in75.93s. Its standalone registry module invocation was exit0/no-op, not a generation/validation check. The focused real PostgreSQL resource run passed both negotiated/unselected cases, with zero failures or ignored tests, in26.61s test runtime (160.54s whole cargo invocation); both completion markers were observed. It exercises the Bundle offer, all69 Workshop rows and the unseen-modifier upgrade, pouch display while retaining the debit exclusion, read-only footprints, replay, stale Character revision and ExAction. The full package run then stopped at dispatch:1975 passed,1 failed,16 pre-existing ignored,230.13s. The sole failure at `bundle_gates_tests.rs:782` was the closed SeamRooms fixture inventory missing the new Atelier Bundle fixture: the only additional observed entry was (`gameplay_transport/postgres_tests/atelier/wire_paid.rs`,1). This is a FAIL for that package run, not an eight-package PASS. The test-owner repair adds exactly that inventory entry and its explanatory comment while retaining all existing guards and assertions; no production behavior changes. Its owned file expands the successor manifest to62 paths. The successor gate plan includes the focused inventory test, eight-package strict Clippy and complete tests, all34 real PostgreSQL cases, smoke, governance, formatter and diff/scope checks. Q7 and Workshop are promoted only on the complete integration-a8 results below.

Integration-a8 passed the focused inventory test:1 passed/0 failed,1.68s test runtime and103.52s whole cargo invocation. Eight-package strict Clippy passed in58.12s, and the complete eight-package tests passed **3622 tests,0 failed**, in448.42s whole cargo time. The complete PostgreSQL run passed **34 tests,0 failed,0 ignored**, in527.10s test runtime and527.74s whole cargo time, with all34 completion markers. Smoke passed (exit0,75.38s), governance passed (exit0,0.52s;22 required policy documents and9 lanes), governance unit tests passed (129 tests,0.339s test time and0.48s runner time), final formatting passed (exit0,9.13s), diff passed (exit0,0.19s) and scope passed (exit0,0.15s), with no conflicts, untracked files or unexpected changes. The manifest owns62 paths and60 files are net changed versus main17f. The two owned paths with no net diff against main17f are `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/fixture.rs` and `crates/gameplay/durability/src/durability/character_atelier/actions.rs`. Smoke reported `075.local`, not an immutable published successor. Q7 and Workshop are DONE for source/component parity. The later metadata/a9 checks passed; Node boot and Q9 remain external readiness dependencies.

All earlier bare `python3 tools/contracts/registry_fragments.py` exits are classified as exit0/no-op: that module has no main entry point. Actual protocol registry generation/validation runs through Cargo `build.rs`. The subsequent explicit import and `r.validate(r.PROTOCOL) + r.validate(r.RESOURCE_LIMITS)` check passed in a9 (exit0,0.06s). After synchronizing the two metadata-only P07/RL1687 descriptions, protocol aggregate consumers passed1 test (9.13s), fee audit consumers passed30 (57.71s), registered foundation consumers passed21 (0.34s), governance passed22 required documents/9 lanes (0.18s), governance unit tests passed129 (0.250s test time,0.33s runner), formatting passed (6.28s), diff passed (0.04s) and scope passed (0.11s;60 net changed files/62 manifest paths,zero unexpected paths,conflicts or untracked files). These description changes preserve numeric values and runtime behavior. All available local implementation and metadata gates passed; they remain local worktree results, not an immutable published-successor claim.

The local-check router at `.claude/skills/oteryn-dev/references/local-checks.md:40-59` also requires `bash tools/qualification/node_boot/run.sh` for the changed dispatch runtime, using the workflow-pinned Platform checkout in `PLATFORM_SOURCE`, and independently `cargo run --locked -p oteryn-game-server -- --smoke`. Node boot is BLOCKED on prerequisites: WSL Docker/Compose exited1 without working Docker/Compose and recommended enabling Docker Desktop integration, and `sudo -n true` exited1 requiring interactive authentication. This is not a Node boot FAIL or PASS, and no sudo bypass was attempted. Smoke passed independently on a8; it does not replace Node boot.

## Self-review

Reviewed the existing owner chain, bounded carried graph, source ordering, immutable receipt replay, complete resource rollback, per-cause audit limits and SQL runtime privileges. A full package run caught a legacy decoder error-category regression, which was fixed without weakening the existing assertion and gained a focused regression case. The pre-extension full rerun passed604 durability tests plus4 doctests; dispatch passed1907 tests with16 pre-existing ignored tests, and all27 mounted Atelier PostgreSQL cases passed. These remain earlier checkpoint results; the new local integration-a8 package and34-case PostgreSQL results are recorded above.

## Independent review

User requests a draft PR without review requests. Three peer lanes reviewed donor parity, durable source/SQL constraints, and protocol/PostgreSQL coverage. The decoder correction received an independent read-only peer check. A coordinator-dispatched frozen-head review has not been requested or claimed; integration review remains with the coordinator.

## PR and closeout

Draft PR [2795](https://github.com/Oteryn/Oteryn-Game/pull/2795) was opened immediately after the first task-record commit. The lead is the only publisher, with branch-write ownership temporarily yielded solely for Q9's coordinator merge-up. Used numbers: migrations0744-0747 and resource-limit1684-1687. Q7 uses capability 272/command 484 and mechanic 0624; its existing static bundle offer needs no separate hook. Capability 273 and commands 485-487 remain unused. Existing148 and commands308/309/424 with Wheel161 remain the original mutation/query owners. The final incremental audit completed at2026-11-01 19:53:56 UTC, pinned to main17f035e04c317803cfd1b3f410a4e1b53d3ad6f1 and all13 current open PR heads. Only2792 advanced, from f66bfb39b0b9cd92baafaabcb6fa9b186682ba7b to c5cecf6f50fc5adf63060cb850b55519145f036d; its complete142-file delta and21 relevant allocation paths were inspected. No reserved block collided, and final readback found no further head drift. The104 registered capabilities have offered/dependency closures35/40/87, with no missing prerequisite; the maximum87 selected capabilities fits112 available and the projection requires148. Local integration-a8 code and integration-a9 metadata qualification passed. Node boot is BLOCKED and Q9 published-successor readback remains external. Pending questions remain under Left out. No PR merge into main, readiness transition, review request or heavy-tests label is authorized by this task.

## Context checkpoint

The local Q7, native managed-destination and Workshop implementation is complete, with all available a8/a9 implementation and metadata gates passed. Published checkpoint `075ce4486355d8a7d94d49ed80b8c01a84a986ee` remains unchanged; the prepared local successor is not published. The42-row table has **33 DONE /9 PARTIAL /0 MISSING**. Q7 is resolved, implemented and tested. Q1–Q6,Q8 andQ9 remain open; the first seven retain concrete accepted/scope conflicts, and Q9 is the external publication dependency. Node boot remains BLOCKED. The task lifecycle is `waiting`, with the separate `WAITING_EXTERNAL` disposition below; this is not a ready/completed claim. No new local commit SHA is embedded here; exact publication/recovery identity belongs in the PR closeout readback.

```text
STATUS: WAITING_EXTERNAL
RESULT: Available local implementation and metadata gates passed; the prepared successor remains unpublished.
BLOCKER: Q9 compliant normal-Git publication route and branch-write ownership/outcome handback; Node boot prerequisites remain unavailable.
OWNER_ACTION_REQUIRED: Coordinator supplies the publication route and returns its outcome and branch-write ownership.
NEXT_ACTION: The coordinator provides a compliant normal-Git execution route for the prepared, conflict-resolved main-to-task merge and returns the operation outcome and branch-write ownership.
```
