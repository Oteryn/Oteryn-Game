# OTV2-20261031-hireling

```yaml
task_id: OTV2-20261031-hireling
title: E12 Hireling identity, services, Store and native House lifecycle
type: mechanic
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/hireling
pr: 2783
base_sha: ae4188a9c8d461acb152044814ea23075de02ed4
head_sha: 4640be635826625265a45b9ae4f019a27f597972
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator /root
created_at: 2026-10-31T18:12:06Z
updated_at: 2026-11-01T13:45:44Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/
  - crates/protocol-oteryn/
  - apps/game-server/migrations/0719_hireling_identity.sql
  - apps/game-server/migrations/0720_store_hireling_delivery.sql
  - apps/game-server/migrations/0721_hireling_services.sql
  - apps/game-server/migrations/0722_hireling_lamp.sql
  - content/
  - imports/
  - rulesets/economy/store/hirelings.json
  - tools/content-schema/
  - tools/content-migration/
  - docs/contracts/
  - docs/agents/evidence/
  - docs/agents/tasks/OTV2-20261031-hireling.md
public_contracts:
  - docs/contracts/gameplay/hireling-house-position.md
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Active implementing packet. The recorded 4640be6 fields identify the historical authoring parent. Published implementation identity is authoritative in [PR2783 commit metadata](https://github.com/Oteryn/Oteryn-Game/pull/2783/commits); this file cannot embed its own commit SHA. The historical published implementation was locally qualified; current takeover qualification is recorded below. Integration remains with the coordinator.

## Outcome

Extend native E12 lamps, House placement/return/recovery, entitlements/appearance, all four services and Store; compose House movement/position/admission/release. Q17=A is accepted. The current merge candidate requires the takeover checks below.

## Architecture and source of truth

- PROVEN: existing native owners and pre-change RED evidence.
- PROVEN: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`; NPC SHA256 `d507c31db74a3558d27bfec6bd08a45076742e6f5ea6da5f5d2baccfc08faf5f`.
- PROVEN: independent current session/lease/House authority; immutable receipt provenance/replay.
- DERIVED: existing owners/codecs; no second currency/item authority or fake Channel placement.
- UNKNOWN: post-publication CI; node-boot has a separate Platform prerequisite. Accepted-code local qualification passed.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current Character/session/connection/lease and independent House/node generation
  - locked ACL/entitlement for services; custody proof for terminal saves
  - immutable input/item/command provenance and once-only truthful outcome
consumer_boundaries:
  - real USE/management/220/NPC/Store/stash/wrap ingress
  - native PREPARE/COMMIT, deferred proofs, audit/replay
  - transfer/loss/reconnect/terminal/fresh admission
mutation_operators:
  applicable:
    - missing/stale current authority or changed identity/binding/provenance
    - full custody, expired grace/cooldown and non-monotonic saved state
    - unknown COMMIT, contention, retry/replay and process/ACK loss
    - stale authority or clock reset during genuine handoff
  considered_not_applicable:
    - generic Instance commerce and synthetic session recovery are outside scope
one_invariant_per_negative_case: verified by recorded scoped unit and native PostgreSQL cases
independent_current_fact_sources:
  - native session/claims/node assignment
  - locked House property/ACL/entitlements
  - Item custody, content activation, runtime lease/map/collision
record_derived_matching_helper:
  allowed_for_positive_happy_path: expected immutable binding only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: reviewed; recorded local qualification passed
  protocol_versions: old Channel bytes retained; additive House fields
  direct_and_reconciled_paths: native commit/pending/replay
  fenced_durable_writes: independent current proof
  restart_retry_replay_concurrency_pg_reload: verified by recorded PG26 cases
  evidence:
    - docs/agents/evidence/chatgpt-hireling-tests.md
    - docs/agents/evidence/hireling-baseline.md
    - docs/agents/evidence/hireling-store-trade-projection.md
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - baseline Query/Dismiss/House routing/Bank
    - unknown native outcome and paid Store retry
    - transfer generations and destination loss/reconnect history
    - complete admission/release and handoff data/clock continuity
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Frozen-base summon/query/dismiss/Bank RED evidence predates production edits.
- [x] Real-wire/PostgreSQL cases prove current fences, once-only effects and uncertain/replayed outcomes.
- [x] Source regeneration/pins and bounded audits pass.
- [x] Accepted admission/release/recovery preserves native participants, clocks and started namespace.
- [x] Q17=A accepted; retain Familiar hosting in named Houses under Left out.
- [x] Local gates/review recorded; published identity belongs to PR commit metadata. Keep draft without integration.

## Reference parity

The pinned Crystal revision and primary NPC hash above remain authoritative. The complete donor file:line, economic values, native owner and test mappings are in the [PR2783 parity table](https://github.com/Oteryn/Oteryn-Game/pull/2783) and `docs/agents/evidence/hireling-store-trade-projection.md`. Source spans include Store `gamestore/init.lua:1475–2062`, NPC packages `1821–2033,2304–2324`, unpriced item 5807 at line 1691 and the lamp/food action files. The referenced test names do not imply final-head qualification.

## Accepted decisions and excluded scope

Q1–Q16 are settled by the [binding PR answers](https://github.com/Oteryn/Oteryn-Game/pull/2783); the evidence file above retains each answer link and exact deviations. In particular: character-wide entitlements; free `various`; all packages/paid Store types; bounded paging/physical payout; valid legacy migration; dish 29409; closed Item compatibility pairs; native House STEP/ordered position and unchanged command continuity.

Left out / unresolved:

- Q6B: production Store activation remains separate. Tests use existing `StoreClient`/`StoreTransport` with a deterministic producer, not production mTLS connectivity/credentials.
- Q13B: general LootPouch producer/custody and bounded all-loot sweep; virtual 23721 gets typed refusal.
- Q9: malformed historical placements need authentic receipts/manual repair; no fabricated provenance.
- Native client management UI/activation is separate; capability 250 is bundle-world only.
- Q17=A ([owner answer 6555913851](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6555913851)): preserve durable Familiar/cooldown state and run FamiliarLogin on the next genuine Channel entry. Familiar hosting in named Houses is a separate prerequisite; no parity is claimed.

Q18 access is restored; interrupted PG21 remains UNKNOWN. No merge/rebase, review request, heavy-tests label, deployment or DONE.

## Implementation / findings

Migrations 0719–0722 (merge repair: draft 0700–0703 moved above main 0718); capability 250 (requires 165), command 470/order 610. Capability 251, commands 471–473 and orders 611–613 are unused. Audit causes 470/471/472 are separate; no new resource IDs.

Closure: 1675 rows/1409 IDs, 23 packages/69 ingredients, 1425 captured/2104 active profiles; 4,712,466-byte JSON < 8MiB. Preserve old Channel encodings.

Authored repairs cover admission/release, clocks, command 0 ACK and exact handoff capability retention. Store154 remains fixture-only. Default validation cleared nine stale-artifact E0433 errors by timestamp-only invalidation with unchanged source SHA, then fixed two real test-only aliases by importing native `spell_character_facts`; no feature activation. Results qualify their recorded checkpoints.

## Validation

### Focused

- Frozen baseline: 1 passed / 7 expected failures / 0 ignored; RED evidence.
- Prior passes: source/profile/binding, Foundation 346, Bank/pending-lamp PG18, ingress 4.
- PG24: PositionCapture PASS 16.46s; LogoutFresh PASS 18.18s.
- PG25 House Store: 1 passed / 0 failed / 0 ignored, 15.72s, native completion marker; its earlier capability failure is repaired.
- Entities corrected UUID fixture: 200/200 PASS.

### Component/integration

- Pre-repair 11-package units: 4532 passed / 1 failed / 16 existing ignored. Latest per-package `--lib` sum: 4533 passed / 16 existing ignored, not one fresh combined run.
- PG23: 35 passed / 3 failed. PG25 full Hireling: 38 passed / 0 failed / 0 ignored, 179.34s; 20 lifecycle + 10 Store markers prove 30 real PG + 8 units. Full Dispatch: 1760 passed / 0 failed / 14 existing ignored, 157.79s. PG25 script exit 0. PG26: 38 passed / 0 failed / 0 ignored, 283.40s tests / 501s overall, identical 38 names and 20+10 native completion markers.
- Clippy: PASS exit 0, 136s, all 11 packages `--all-targets --all-features -- -D warnings`.
- Architecture: PASS 13s; client registrations: 2/0/0, 0.03s tests; doctests: 48/0/0 across 11 packages, 93s overall.
- Content REST: 123/0/0, 2.19s tests/110s overall; `content_world_project_repository`: 5/0/0, 57.89s tests/74s overall.
- Current release `pin-check`: PASS exit 0, 29s. Data/generators are unchanged since successful Python checks; digests are in PR/evidence.
- Governance: PASS (22 policy docs, 9 lanes), governance tests 129 PASS. Final fmt PASS 10s and final diffcheck PASS 0s.
- Fresh numbers against main 03d1a55d06df29f4f110c768364e0e397266d667: exit 1 for unrelated PR2789/2780 overlaps migrations 716–718/capabilities 258–259. Our 0700–0703/250–251/470–473/610–613 block is clear; no global PASS. Main maxima: migration 662 / capability 240 / command 447 / mechanic 587 / offer 560.

### E2E and exact-head CI

- Default `final-smoke-qualified`: PASS exit 0, 99s, actual server `process_start` recorded.
- Separate node-boot: BLOCKED exit 2 `exact_platform_checkout_missing`; later preconditions were not reached.
- Whole qualification script: PASS exit 0, 936.61s. Published head: authoritative PR commit metadata. No CI-green claim; CI trigger/run/job/runner/classification/result remain unreported.

## Self-review

- Coordinator accepted-scope self-review: PASS; no known unaddressed P0/P1.

## Independent review

Required YES: authority/value/recovery. Internal review is complete, including the final three deltas; PASS with no known unaddressed P0/P1 in accepted scope. No CI-green claim.

## PR and closeout

- PR2783 stays DRAFT/implementing because integration is pending. Accepted code and local gates are qualified; no archive/DONE.
- No integration/ownership release. PR commit metadata records the published SHA after commit.

## Context checkpoint

```yaml
last_progress: takeover conflicts resolved in files; compile/clippy and focused units passed; PostgreSQL wrapper configuration missing
status: implementing
branch: chatgpt/hireling
head_sha: 4640be635826625265a45b9ae4f019a27f597972
pr: 2783
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: restore PostgreSQL configuration in the authorized cargo wrapper
blocker: PostgreSQL wrapper omits admin URL; historical Item fixture writes outside worktree; CI status not claimed
next_action: qualify the 30 native Hireling PostgreSQL cases once the wrapper is configured; owner stages/commits after review
```

## Takeover qualification (tk-hire, 2026-11-01)

This section supersedes historical qualification claims for the takeover. Local authoring
starts at `fe53fff490e0d9c55bd5fb006fa77d5dc3dc8b46`, with incoming main
`38641fecd4558a4a14e6b15b3874110ffa2dff19` already merged without a commit.
No Git state-changing command was run. All 11 conflicted Rust files have resolved
contents and no conflict markers. They remain unmerged in the Git index because
staging is reserved for the owner.

Resolution preserves main's activity refusal cleanup, expiry/routing/recovery,
activity death equipment and Instance PvP reads, together with native House
custody, lifecycle, equipment and loss history. The loss codec writes/reads the
current transferred identity exactly once. Existing main history tag 3 remains
ActivityOrigin; the new House ScopeTransfer history uses tag 4. Ordinary bytes
remain unchanged. Shared recovery readers independently validate either the
committed activity lineage or the committed House predecessor; a failed House
custody/ACL check cannot fall through to a generic Instance guard. Native House
recovery retains its already-restored views and clocks. Four compile errors from
main's optional `channel_id()` API were repaired by refusing a missing Channel.

Q17=A is accepted by owner comment 6555913851. Familiar state/cooldowns remain
intact; FamiliarLogin waits for the next genuine Channel entry. Named-House
Familiar hosting remains under Left out.

The migration renumbering was already present at entry: 0700 → 0719, 0701 → 0720,
0702 → 0721, 0703 → 0722. No further migration rename or SQL/grant change was
needed. References in the current contract, fixture comment and narrative evidence
now use the delivered numbers; historical number-allocation snapshots remain
historical. Both `bash .claude/skills/oteryn-dev/scripts/dup-migrations.sh` and the
same command with `MERGE_HEAD` returned no duplicate numbers; a filesystem scan
also found zero duplicates. Incoming main contains no migration numbered 0719–0722.

### Changed files during this takeover

- `crates/gameplay/dispatch/src/gameplay_transport/activity_recovery_admission.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/actor_spell/house_transfer.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/control_loss.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/grace_expiry.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/resume.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/control_loss.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_use_gourmet.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/hireling_lifecycle/legacy.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/session_ingress.rs`
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/activity_loss.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/control_loss_codec.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/house_lifecycle.rs`
- `crates/gameplay/durability/src/durability/pvp_state.rs`
- `crates/gameplay/foundation/src/foundation/admission_authority_publication.rs`
- `crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs`
- `docs/agents/evidence/hireling-store-trade-projection.md`
- `docs/agents/tasks/OTV2-20261031-hireling.md`
- `docs/contracts/protocol-oteryn/v1/hireling-trade-and-audit-v1.md`

### Generators and static content

- `bash tools/merge-driver/regen.sh --skip-checks`: blocked (exit 2), since its
  implicit `--resolve` requires marking the 11 index entries resolved.
- `PATH=/home/mole/oteryn-cache/codex-bin:$PATH python3 tools/content-migration/regenerate_content.py --skip-checks`: PASS;
  same materializer, Rust inventory pins, content tree and six authoring generators,
  without staging. Generated outputs were already current.
- `python3 tools/content-schema/native-gameplay/build_full_spell_artifact.py --refresh-loot-tables`: PASS.
- `python3 tools/content-migration/register_spell_families.py`: PASS (38 registered files).
- `python3 tools/contracts/registry_fragments.py`: PASS; no generated drift.
- `python3 tools/content-schema/hireling/generate_wrapped_catalogue.py --check`: PASS.
- `python3 -m unittest discover -s tools/content-migration -p test_hireling_item_profiles.py`: 10 passed, 0 failed.
- `python3 tools/content-schema/native-gameplay/test_spell_item_identities.py`: 13 passed, 0 failed.
- Item comparison: all 2,210 incoming-main profile identities remain; 909 additional
  Hireling profiles; 3,119 total active profiles, 6,869,182 bytes. Existing richer
  facts remain preserved; source-qualified admission enrichment is retained.

### Cargo validation commands

All cargo commands below use the absolute queued wrapper. Test summaries are
invocation counts; overlapping filters are not a unique-test total. A zero-match
UI library probe is shown and followed by the correct integration target.

| Exact command | Result |
| --- | --- |
| `/home/mole/oteryn-cache/codex-bin/cargo fmt --all` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo check --locked --workspace --all-targets --all-features` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-dispatch -p oteryn-gameplay-domain -p oteryn-gameplay-durability -p oteryn-gameplay-entities -p oteryn-gameplay-foundation -p oteryn-gameplay-items -p oteryn-gameplay-spell -p oteryn-gameplay-world -p oteryn-protocol-oteryn -p oteryn-game-server -p oteryn-client-ui --all-targets --all-features -- -D warnings` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib hireling -- --nocapture --test-threads=1` | NOT QUALIFIED: 38 returned OK in 0.01s, but 30 PG cases returned early; 0/30 native completion markers. Only 8 unit cases executed. |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --all-features --lib hireling` | 1 passed; 0 failed; 0 ignored; 0 measured; 292 filtered out; finished in 10.52s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-domain --all-features --lib hireling` | 12 passed; 0 failed; 0 ignored; 0 measured; 185 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --all-features --lib hireling` | 18 passed; 0 failed; 0 ignored; 0 measured; 601 filtered out; finished in 0.18s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib hireling` | 7 passed; 0 failed; 0 ignored; 0 measured; 220 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib house_custody` | 7 passed; 0 failed; 0 ignored; 0 measured; 220 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-foundation --all-features --lib control_loss_durability_tests` | 15 passed; 0 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-items --all-features --lib native_weight_normalization` | 1 passed; 0 failed; 0 ignored; 0 measured; 118 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-spell --all-features --lib gourmet_food` | 3 passed; 0 failed; 0 ignored; 0 measured; 309 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world --all-features --lib source_house_membership` | 1 passed; 0 failed; 0 ignored; 0 measured; 205 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib hireling` | 5 passed; 0 failed; 0 ignored; 0 measured; 636 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib session_ingress::` | 11 passed; 0 failed; 0 ignored; 0 measured; 1947 filtered out; finished in 0.03s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client-ui --lib registrations` | 0 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-foundation --all-features --lib transferred_claim_release` | 1 passed; 0 failed; 0 ignored; 0 measured; 356 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world --all-features --lib house` | 4 passed; 0 failed; 0 ignored; 0 measured; 202 filtered out; finished in 0.00s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-client-ui --test registrations` | 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-architecture-check -- workspace .` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-refresh .` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . target/hireling-validation/pin-check` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test content_world_project_repository` | 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.03s |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest content_` | 123 passed; 0 failed; 0 ignored; 0 measured; 152 filtered out; finished in 1.56s |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-game-server -- --smoke` | PASS (exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-game-server -- --smoke` | PASS (exit 0; actual server process_start observed) |

World pin refresh retained digest `6edcee04dc8dacbb34f735570e32545ff20793f8da37bd1168f94e941fca8622`; release `pin-check` passed.

### Registered generated-file checks

These are the Python checks registered by `regenerate_content.py --check`; the
registered Cargo content-world target is listed above. 15 passed; 1 is blocked
by the sandbox. No assertion or harness was weakened.

| Exact command | Result |
| --- | --- |
| `python3 tools/content-migration/imbuement_catalogue.py --check` | PASS (exit 0) |
| `python3 tools/content-migration/register_spell_families.py --check` | PASS (exit 0) |
| `python3 tools/content-migration/test_import_spell_families.py` | PASS (exit 0) |
| `python3 tools/content-migration/validate_world_project_v2_to_tree.py` | PASS (exit 0) |
| `python3 tools/content-migration/test_world_project_v2_to_tree.py` | PASS (exit 0) |
| `python3 tools/content-schema/validate_materialized_game_tree.py` | PASS (exit 0) |
| `python3 tools/content-census/item_key_references.py` | PASS (exit 0) |
| `python3 tools/content-schema/craft-authoring/craft_authoring.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/imbuement-authoring/imbuement_content.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/charm-authoring/charm_authoring.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/proficiency-authoring/proficiency_authoring.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/reward-claim-authoring/reward_claim_authoring.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/starter-kit-authoring/starter_kit_authoring.py content --check` | PASS (exit 0) |
| `python3 tools/content-schema/item-authoring/test_engine_items.py` | BLOCKED: historical fixture tries to create a directory beside the worktree (read-only filesystem) |
| `python3 tools/content-schema/item-authoring/item_weapon_proficiency.py --check` | PASS (exit 0) |
| `python3 tools/content-census/g4_item_crystal_binding_generator.py --check` | PASS (exit 0) |

### Review and unfinished qualification

The merge-resolution diff and additional repairs were re-read against all 19
items of `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Production callers for the
combined equipment, loss, recovery and PvP readers were traced. Workspace matches
for both ControlLossHistory variants, PendingDelivery/SessionIngress construction
and the changed helper signatures were searched; workspace/all-target/all-feature
compilation confirms the consumers compile. No new public API was introduced by
the takeover. The existing native Hireling PostgreSQL cases are the required real
dispatch evidence for these boundaries and remain unqualified here.

- PostgreSQL: the absolute required wrapper does not supply
  `OTERYN_TEST_POSTGRES_ADMIN_URL`. The Hireling filter selected 38 tests and
  returned in 0.01s with zero native completion markers. This is **not** a PG pass.
  Restore wrapper configuration and rerun that exact filter; require 20 lifecycle
  and 10 Store completion markers. No admin URL was read or printed. An owner
  environment question is pending; no credentials or wrapper outside this worktree
  were edited.
- `tools/content-schema/item-authoring/test_engine_items.py`: the unchanged main
  historical-context helper creates its fixture in `root.parent`, outside the
  authorized writable worktree. It fails before assertions with a read-only
  filesystem error. CI or an authorized fixture location is required; no test was
  skipped or altered.
- Full node-boot qualification is not run here: it requires the pinned Platform
  checkout, Docker and system mutation outside this worktree. The prior Platform
  prerequisite remains separate from default server smoke.
- No remote CI/`game-gate`, exact committed-head review or merge success is claimed.
  The owner must stage the resolved contents and commit after review; the index
  still reports the 11 unmerged paths.

### Final governance and formatting

- `python3 tools/agents/validate_governance.py`: PASS, 22 required policy documents / 9 lanes.
- `python3 -m unittest discover -s tools/agents/tests`: 129 passed / 0 failed.
- `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check`: PASS after the final code edits.
- `git diff --check`: PASS.

The first formatting probe reported formatting differences and the first
workspace compile reported four source errors at the optional Channel call sites.
They were repaired before the complete passing workspace and clippy runs above.

Validation logs and machine-readable command summaries are retained locally under
`target/hireling-validation/` (ignored). The current candidate is not declared
mergeable: native PostgreSQL qualification and committed-head CI remain outstanding.
