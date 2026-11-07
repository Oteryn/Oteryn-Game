# OTV2-20261031-teamfinder

```yaml
task_id: OTV2-20261031-teamfinder
type: mechanic
title: Team Finder members and typing indicator
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/teamfinder
pr: 2767
base_sha: a52549e3b9230c7486a24df66f4e8ec59d1f96cb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: teamfinder-task-lead
created_at: 2026-10-31T11:14:14Z
updated_at: 2026-10-31T12:36:01Z
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/
  - crates/gameplay/dispatch/
  - crates/gameplay/durability/
  - crates/client-ui/src/features/0232-team-finder-members/
  - crates/client-ui/src/features/0233-player-typing/
  - apps/game-server/migrations/0644_team_finder_members.sql
  - docs/contracts/protocol-oteryn/v1/
  - tools/architecture-check/client-unsupported-baseline.d/
  - docs/agents/tasks/OTV2-20261031-teamfinder.md
  - docs/agents/tasks/archive/OTV2-20261031-teamfinder.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/team-finder-leader-v1.md
  - docs/contracts/protocol-oteryn/v1/team-finder-members-v1.md
  - docs/contracts/protocol-oteryn/v1/player-typing-v1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

S14b/S14c now has negotiated server commands for browsing advertisements, requesting or cancelling membership, reading the leader's applicants, and inviting, accepting or denying an existing applicant. Acceptance issues the existing ordinary Party invitation; the applicant joins through normal Party Accept. The durable S14a template remains compatible with capability 200 and commands 390/391, while its active advertisement and requests end with the leader's session occurrence.

S19 now has a strict typing on/off command and transient delivery to the donor's player spectators. It reuses the existing CHAT domain, command replay and aggregate ingress limiter. Typing never writes a durable gameplay row.

This is completion of the requested **draft implementation delivery** in [PR #2767](https://github.com/Oteryn/Oteryn-Game/pull/2767). It does not claim a merged, deployed or native-UI-playable release. The native Team Finder window, chat-input binding and indicator rendering remain explicitly unsupported in the client registrations. The shared codecs and negotiated server paths are implemented and exercised through admitted connections.

## Architecture and source of truth

**PROVEN:** the qualified base is `a52549e3b9230c7486a24df66f4e8ec59d1f96cb`. Existing S14a was introduced by PR #2756. The draft was opened after the first task-record commit, `c18a58d41922c1c2ab4d0ae718cf0362bfdf269d`.

The task-selected primary reference is [Crystal `201189f871d23597b82caf88fd611454a2ddefb8`](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8), evidence class `OTS_HYPOTHESIS_ONLY`. The secondary crosscheck is [Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`](https://github.com/opentibiabr/canary/tree/47dfd51f45280a59a1d3e50ba7edd573d7234446). Crystal takes precedence where their implementations differ. These donor repositories are references, not target runtimes or new authority sources.

**PROVEN:** decisions Q1-Q5 are the coordinator's [answer 6542716908](https://github.com/Oteryn/Oteryn-Game/pull/2767#issuecomment-6542716908). The publication/base exception is [Q6 answer 6543201281](https://github.com/Oteryn/Oteryn-Game/pull/2767#issuecomment-6543201281).

| Decision | Accepted rule |
|---|---|
| Q1 = B | Resolve the selected leader's advertisement through an opaque session-scoped handle and revalidate its live generation. Record the donor's applicant-owned lookup at `protocolgame.cpp:3140` as a donor-bug deviation. |
| Q2 = C | Leader acceptance records Team Finder status 3 and issues an ordinary invitation through the existing Party owner. The applicant joins only through existing Party Accept. No second Party writer or authority proof is introduced. Status 2 remains the donor's Team Finder status, without an ordinary Party invitation. |
| Q3 = B | Retain the durable leader template across relog. End the active advertisement and its requests at logout; require a new leader upsert after relog. Preserve and update the old S14a regression with the reason for its changed expectation. |
| Q4 = A | Permit one status-1 request per player in the World/Channel scope. Decisions 2/3/4 and cancellation release that pending slot. |
| Q5 = A | Preserve leader-supplied free slots as baseline. Occupancy is `max(slots - baseline_free, accepted_count)`; effective free capacity is `slots.saturating_sub(occupancy)`. Refuse a new acceptance when effective capacity is zero. Count the leader only through the baseline. |
| Q6 = B | Publish one complete successor of `c18a58d41922c1c2ab4d0ae718cf0362bfdf269d` on the original base. The coordinator waived merging latest main before this refresh and will package the draft onto fresh main, resolve conflicts and regenerate with repository tooling. No concurrent coordinator writes to this branch are authorized during delivery. Name the qualified base in the PR body; do not merge this draft. |

**DERIVED:** normal Party visibility, consent, leadership, membership and rate refusals remain applicable to Q2C. If the existing invitation owner refuses, the Team Finder acceptance does not change status. Cancelling a Team Finder entry is not a Party Leave operation. These follow the existing owner contracts and do not add gameplay policy.

## Reused owners and implementation boundaries

- `protocol-oteryn::team_finder::{LeaderListing, TeamTarget}` and the existing 390/391 codecs remain the authoring contract. The new member codec carries those fields unchanged, plus global occupancy and per-viewer status.
- `gameplay_transport/team_finder_leader.rs` delegates template and advertisement changes to the new `durability/team_finder` owner in the same fenced transaction. The prior leader receipt table remains authoritative for 390/391.
- `SpellItemAuthority`, the existing semantic pass and spell lane prove current caller authority. Current actor/lease facts and canonical SQL session/claim rows are revalidated independently of historical receipts and handles.
- `world_party::read_world_party_in_transaction`, `apply_world_party_command_in_transaction`, and the existing `QualifiedPartyVisibleTarget` proof own Party observation and invitation. No direct Party membership insert is added by Team Finder.
- The session owner's existing transaction fires migration 0644's terminal/generation/Channel trigger. Cascading active-state deletion preserves the durable template and command receipts; no background cleanup or second logout writer is added.
- Existing `ChatRuntime::push_party_spell_feedback` emits donor status messages after a newly accepted commit. Durable replay does not repeat messages or Party invitations.
- S19 extends the existing CHAT domain 12 with negotiated delta type 4, using the current Channel actor census, CHAT refresh cadence and bounded transient queues. Connection ingress owns replay and overload protection.
- Capability, registry, golden, mechanic, contract and client feature-registration fragments are present. Unsupported client registrations remain explicit; no production fallback offer is enabled.

## Numbers used

| Kind | Allocation |
|---|---|
| Migration | 0644, active advertisements, members, receipts and lifecycle trigger |
| Existing capability/commands | 200 and 390/391 retained |
| New capabilities | 232 `TEAM_FINDER_MEMBERS_V1`; 233 `PLAYER_TYPING_V1` |
| New commands | 444 query; 445 member action; 446 leader action; 447 typing |
| Mechanic ordinals/offer orders | 584 `team_finder_members` registers commands 444-446; 587 `player_typing` registers command 447 |
| Existing state domain | CHAT 12, typing delta type 4; no new state domain |
| Unused reserved numbers | Migrations 0645-0647; mechanic ordinals/offer orders 585-586 |

Both new capabilities are bundle-world only. Capability 232 requires 7, 42 and 200; Party retains its dependency on 6. Capability 233 requires 6 and 7. The selected-capability storage has room for the extended offer. Numeric reservations were checked again against fetched `origin/main` `1d292a25fc2e7db466e5893ff69c6716870d6d38` and open PRs; Q6B leaves qualification on the original base.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - independently current caller session, connection, Character lease and Channel generation
  - target identity, live session/generation, current Character claim and account presence
  - only the admitted leader owns its applicant map
  - historical receipts cannot recreate advertisements or reacquire authority
  - Party membership changes only through the existing Party owner
consumer_boundaries:
  - admitted command ingress and capability selection
  - SessionTeamFinderView opaque handles
  - SpellItemAuthority transaction boundary
  - canonical target SQL reads and ordinary Party visible-target proof
  - session terminal/generation trigger
  - generation-aware CHAT typing selection and terminal retirement
mutation_operators:
  applicable:
    - missing or wrong-kind handle
    - mismatched actor/session or target identity
    - stale caller connection generation
    - changed foreign Character claim generation
    - changed foreign session generation
    - same command id with changed payload binding
    - exact replay after local handle-view replacement
    - status transitions, cancellation, removal and terminal logout
    - actual successor Fresh admission and new advertisement upsert
    - delayed older typing snapshot after a newer selected or disabled generation
    - delayed typing admission after loss of control or terminal actor retirement
  considered_not_applicable:
    - no new gameplay clock, cooldown or expiry policy is introduced
    - no new cross-repository authority or production offer is introduced
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - ChannelRuntimeV1 actor/control/Character lease facts
  - existing recovery, session, node and scope authority checks
  - current admission Character/account guards and reconnect-session rows
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: leader authoring, browse, applicants, member actions and leader decisions reviewed
  protocol_versions: existing capability 200 retained; new 232/233 gates and strict codecs exercised
  direct_and_reconciled_paths: current writes and durable replay remain independently fenced
  fenced_durable_writes: template, active map, member receipt and ordinary invitation share a transaction
  restart_retry_replay_concurrency_pg_reload: exact receipt replay, recreated handle view and actual Fresh successor covered; status decisions serialize in the existing owner transaction
  evidence:
    - postgres_tests/team_finder_members.rs and its mounted cases, relog and fences helpers
    - durability/team_finder/tests.rs
    - team_finder_members/handles.rs tests
    - player_typing_tests.rs and player_typing_wire_tests.rs
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - delayed generic chat admission could erase typing generation high-water; only the generation-aware selector now resets admission queues
    - a disabled newer typing generation could lose its high-water; a non-delivering entry now retains it until terminal forget
    - delayed typing admission could recreate retired session state; current controlled actor/session validation and selection now share the runtime lock
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - fixed native PostgreSQL fixture qualification through existing progression and training admission owners
    - fixed successor fixture World-policy mismatch using the loaded Runtime publication and existing Fresh verification/claim flow
```

The foreign-target refusal helper changes only the fixture leader's current Character claim generation, proves that an already issued handle reaches the durable writer and produces a refused receipt, verifies no Team Finder or Party mutation, then restores that exact claim. Its second case changes the leader session generation, proving trigger cleanup, template retention and retained-handle refusal. It does not fabricate production authority or disable triggers.

Positive relog setup uses the existing Fresh source, verifier, authorization, claim transition and durable flow. Native progression and training admission initialize the real source context before restored-revision tracking. The successor grant preserves the currently published World policy; the fixture does not republish the runtime guard merely to match a stale default.

## Acceptance criteria

- [x] Browse and member/leader commands route through the current owned runtime and durable transaction.
- [x] Each durable transition has a mounted admitted-connection PostgreSQL case with committed-row readback.
- [x] Exact replay returns the retained result without another request, status change, invitation or notification.
- [x] Leader removal, terminal logout and generation retirement close active state; the actual Fresh successor needs a new upsert.
- [x] Typing implements the donor spectator geometry, live generation checks and existing transient ingress protection.
- [x] Required protocol, capability, golden, registry, mechanic, contract and client registration file sets are complete.
- [x] Native UI limitations and bundle-world-only availability are explicit.
- Final affected validation and exact-head status: **pass for the completed authoring tree; published-head evidence is recorded in PR #2767**.

## Reference parity

Donor line numbers below refer to the exact pinned revisions above. `C` is Crystal and `K` is Canary. Protocol paths are `src/server/network/protocol/protocolgame.cpp`; game paths are `src/game/game.cpp`; player paths are `src/creatures/players/player.cpp`.

| Behaviour | Reference value/source | Oteryn implementation | Test evidence |
|---|---|---|---|
| Browse fields; no invented eligibility filter | C protocol:2823-2890; K protocol:3675-3743. Levels, vocation mask and target are listing metadata; the request carries no server filter. | `durability/team_finder/query.rs`; shared `LeaderListing`; bounded keyset pages of all current World/Channel advertisements | `browse_round_trips_every_target_status_and_donor_field`; `page_bound_is_a_transport_limit_with_continuation`; `team_finder_members_request_single_pending_and_durable_replay` |
| Occupancy and capacity | C protocol:2852-2865; K protocol:3704-3717; Q5A adds explicit saturation/capacity refusal. | Authored free-slot baseline unchanged on the wire; occupancy is global, independent of the page; new acceptance checks effective free capacity | `team_finder_occupancy_preserves_leader_baseline_without_double_counting`; `team_finder_effective_capacity_floors_at_zero`; `applicant_pages_preserve_global_occupancy_and_baseline`; `team_finder_members_accept_uses_party_invite_then_normal_accept_and_capacity` |
| Request and cancel | C protocol:3125-3157; K protocol:3993-4025. Q1B fixes C:3140/K:4008 selecting the applicant's object; Q4A defines the global pending rule. | Command 445 resolves selected leader data and current authority, preserves an existing map entry, or removes only the caller's entry | `team_finder_members_request_single_pending_and_durable_replay`; `team_finder_members_cancel_deletes_only_the_applicant_entry` |
| Applicant view and vocation IDs | C protocol:2938-2967; K protocol:3790-3831; C `data/XML/vocations.xml`. Leader is status3; offline applicants are hidden. | Leader first on initial page, then current online applicants in stable Character order; native vocation keys map to donor client IDs including promotions | `team_finder_client_vocations_preserve_promoted_donor_ids`; `leader_decisions_use_donor_status_values`; the request/invite/accept/deny PostgreSQL cases |
| Status 2/3/4 and feedback | C protocol:3075-3112; K protocol:3944-3978; Q2C assigns ordinary invitation to existing Party owner. | Command 446 changes an existing applicant; status2 is Team Finder only, status3 invokes ordinary Party Invite, status4 denies; feedback after new commit only | `team_finder_members_invite_records_status_two_without_party_membership`; `team_finder_members_accept_uses_party_invite_then_normal_accept_and_capacity`; `team_finder_members_deny_preserves_status_until_cancellation` |
| Existing Party import | C protocol:3019-3029; K protocol:3886-3896. Insert current Party members as accepted without replacing prior statuses. | Existing party snapshot imported atomically with leader upsert; self excluded; existing applicant state preserved | `team_finder_members_accept_uses_party_invite_then_normal_accept_and_capacity` includes real Party import and preserved denied status |
| Remove, logout and relog | C protocol:2903-2908 and game:680-682,1422-1426; K protocol:3755-3760 and game:948-950,1612-1616; Q3B retains the S14a template. | Remove cascades template/active map; terminal/generation trigger deletes active advertisement/member map only; real successor must upsert anew | `team_finder_members_remove_cascades_requests_and_rejects_retained_handle`; `team_finder_members_real_logout_closes_advertisement_but_retains_template`; retained `team_finder_leader_listing_create_edit_remove_survives_relog_through_the_wire` |
| Strict typing input and output | C protocol:2818-2820,11314-11323; C player:11366-11372. Native protocol uses explicit Boolean states and negotiated delivery. | Command447, capability233 and CHAT domain12/delta4; spatial actor reference, no durable Character identity | `player_typing_intent_requires_one_boolean`; `player_typing_real_connection_broadcasts_on_off_and_replays_without_reexecution`; `player_typing_real_connection_requires_negotiated_capability` |
| Typing recipients and multifloor projection | C game:13074-13084; K game:12704-12714; C `src/map/spectators.cpp:103-116,157-160,180-184`, `src/map/map_const.hpp:20-27`, `src/game/movement/position.hpp:57-59` | Include speaker and selected player spectators; projected x/y within +/-11; surface ranges and underground +/-2 floors, including source floor8/recipient floor6 | `player_typing_geometry_uses_donor_multifloor_ranges_and_projected_coordinates`; `player_typing_broadcast_includes_speaker_and_selected_nearby_players_only`; `player_typing_geometry_rejects_invalid_floors_without_coordinate_overflow` |
| Transient lifecycle, replay and overload | Donor typing functions retain no database state; Oteryn `session_ingress.rs` and `connection/serve/{command,integrity}.rs` own native ingress semantics. | Coalesce latest unsent state; generation-aware snapshot reset, including disabled generations; current actor proof before admission; terminal cleanup; existing replay and aggregate limiter | `player_typing_coalesces_latest_state_and_reconnect_drops_pending_events`; `player_typing_delayed_chat_snapshot_cannot_downgrade_current_generation`; `player_typing_deselection_fences_delayed_older_selection_and_disables_delivery`; `player_typing_admission_refuses_lost_control_and_late_retired_snapshot`; `player_typing_chat_retirement_clears_recipients_and_departed_speaker_updates`; `player_typing_real_connection_reuses_transient_command_rate_limit` |

Canary has an additional leader-view count rewrite at protocol:3810-3835 that is absent from Crystal; the native schema uses the selected Crystal fields and explicit bounded member lists. Crystal browse resolves the outer leader GUID while counting accepted members at protocol:2852, so accepted map entries contribute to global occupancy even when the applicant is absent from the online applicant view. This is retained rather than adding an unrelated eligibility rule.

Protocol rule tests live in `crates/protocol-oteryn/src/tests/team_finder_members_tests.rs` and the typing/chat tests. Durability rule tests live in `crates/gameplay/durability/src/durability/team_finder/tests.rs`. Admitted-connection cases live in the existing mounted `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` tree. No standalone PostgreSQL test target is added. Each new PostgreSQL case prints `S14BC <case> POSTGRES_CASE_COMPLETED`; without `OTERYN_TEST_POSTGRES_ADMIN_URL`, it returns `Ok(())` as required by the existing harness.

## Excluded scope and remaining product work

- Native Team Finder controls and window, native chat-input typing events, and rendered typing indicators are not implemented. Registration status remains `unsupported`; the server wire is not presented as native client playability.
- Production fallback and Item-view offers remain unchanged. No promotion to production availability is claimed.
- No new Party membership, XP, combat, loot, profession, price, timer or cooldown rule is introduced. Party Accept/Leave and invitation refusals remain owned by existing code.
- Protected main, merge, deployment, review requests, the `heavy-tests` label, workflows, repository policy tools, vendor inputs, generated content sources and the read-only re-audit ledger are outside this task's writes.
- No gameplay question remains open after Q1-Q6. Packaging onto fresh main and its integration qualification belong to the coordinator under Q6B.

## Implementation / findings

Member receipts bind the exact command type/raw payload for recovery before connection-local handle resolution, and separately bind canonical action/target session/generation inside the writer. Receipts are immutable to the runtime role. Unknown handles without a receipt refuse; an exact retained receipt can still answer after the local page view has been replaced. Target data never becomes authority by being present in a handle.

The existing World Party owner is acquired before the Team Finder Channel lock. The active map, status change, receipt and any ordinary Party invitation commit together. A partial unique index enforces the single status-1 request. Query/resource bounds reuse existing Party and Guild transport budgets; they do not cap the number of gameplay advertisements or applicants.

Migration 0644 revokes PUBLIC privileges and grants only the runtime operations used by each table. The trigger uses `SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp`, and exposes no separately granted EXECUTE ingress. The existing terminal state is 3; active session reads require state2 and current independent claims.

The S19 lifecycle races found during self-review were repaired before publication. Generic chat admission/refresh no longer erase typing selection high-water. Generation-aware typing admission owns queue replacement and retains a disabled newer generation as an empty, non-delivering entry, so a delayed older selection cannot enable it again. Source, recipient and queue-drain paths all require a selected entry. Admission now proves the current controlled actor/session while holding the Channel runtime lock through selection. Terminal retirement removes the exact actor under that same runtime lock before chat cleanup; a delayed snapshot therefore cannot recreate the retired typing entry. Terminal forget removes both the entry and its queued speaker updates. Regressions cover both generic chat paths, disabled-generation fencing, loss of control and a late snapshot after actual actor retirement. The typing mutex never acquires the runtime lock, so the added admission path preserves runtime-before-typing lock order.

PostgreSQL fixture failures were corrected through existing owners. Synthetic progression revisions were replaced with native progression/training admission; the training formula uses the fixture's actual `content-1` revision. The successor Fresh grant now reflects the loaded Runtime World policy. These repairs do not weaken session checks, disable triggers, replace wire logout with manual row deletion, or change production authority.

## Validation

Qualification uses an isolated checkout and serial builds. The real database is PostgreSQL 18.6 (`server_version_num=180006`). The database URL is supplied only through the environment and is not retained in this record. Full package tests without that variable do not count as executed PostgreSQL qualification.

### Focused and structural

- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py` — **pass**, including the final reservation check against fetched main `1d292a25fc2e7db466e5893ff69c6716870d6d38` and open PRs.
- `python3 tools/contracts/registry_fragments.py` — **pass**.
- `cargo fmt --all -- --check` — **pass**; macro-mounted changed Rust modules are also included in explicit formatting checks.
- `cargo run --locked -p oteryn-architecture-check -- workspace .` — **pass**.
- `cargo test --locked -p oteryn-protocol-oteryn --all-features --quiet` — **pass**, 585 tests in the reported protocol run.
- `git diff --check` and `git diff --cached --check` — **pass**.
- `python tools/agents/validate_governance.py` — **pass**; rerun includes this final archived record.
- `python -m unittest discover -s tools/agents/tests` — **pass, 129 tests**; the earlier authoring run passed 129 tests.

### Component/integration

- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-client-ui -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings` — **pass**.
- `cargo test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-client-ui -p oteryn-game-server --all-features --no-fail-fast --quiet` — **pass in the full authoring run, 4,730 passed and 25 existing ignored tests; the final two typing regressions also pass in the focused final run**; `OTERYN_TEST_POSTGRES_ADMIN_URL` unset for this package-wide run.

### Admitted-connection and PostgreSQL qualification

- `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib team_finder --quiet -- --nocapture --test-threads=1` — **pass, 11 selected tests: eight new real-dispatch PostgreSQL scenarios, the retained S14a PostgreSQL scenario and two handle unit tests; all nine database markers observed**, with the PostgreSQL 18.6 admin URL supplied through the environment. The selected cases include eight new real-dispatch PostgreSQL scenarios, the retained S14a scenario and the handle unit tests.
- S19 real-connection on/off broadcast, duplicate replay, missing capability and aggregate limiter scenarios are mounted in `player_typing_wire_tests.rs` and included in the dispatch package test result above.
- The old S14a test retains its authoring/edit/remove assertions and now exercises real wire logout with durable-template retention. The new logout case performs a genuinely new Fresh admission, proves empty browse/applicants before upsert, then proves the new advertisement has no resurrected requests.

- `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib player_typing --quiet -- --nocapture --test-threads=1` — **pass**, 13 tests after the final lifecycle fixes. The three protocol typing tests also pass in the protocol suite.
- `cargo run --locked -p oteryn-game-server --all-features --quiet -- --smoke` — **pass**.

### Exact-head evidence

**The completed authoring tree passed the checks below. The API publication selects a distinct candidate; its full required checks are repeated after live head readback and before delivery. The final published SHA and those independent execution results are recorded in PR #2767 under FREEZE_SHA and Exact tested head.**

A commit cannot contain its own SHA. `head_sha`, `final_head_sha` and `final_head_frozen_at` therefore remain null in this authoring record; the final returned/live head, complete bounded-delta inspection, FREEZE_SHA and candidate-specific check evidence belong in immutable PR/check evidence after publication. No self-referential follow-up commit is authorized merely to fill these fields. Local authoring results are not represented as exact-head CI or post-merge gate results.

## Self-review

The implementing lead owns the mandatory complete-diff review. Disjoint worker reviews supplied donor parity, protocol bounds, durable receipt/foreign-target checks and the S19 generation-race review. The final read-only durability/dispatch sweep found no additional actionable P1 or P2 after the accepted repairs. Final validation status is recorded above; exact candidate identity is recorded externally after publication.

Reviewed risks include missing/expired local handles, stale foreign claims and target sessions, wrong applicant-map ownership, repeated commands after view recreation, capacity double counting, accepted-to-invited-to-accepted transitions, ordinary Party refusal, terminal cleanup, retained templates, real Fresh relog, delayed typing snapshots, disabled-generation high-water and retired-session recreation. No independent worker review replaces the lead's whole-diff review or grants merge authority.

## Independent review

This task delivers a draft for coordinator review and requests no review itself, as the user instructed. Subsequent integration review, fresh-main packaging, regeneration and qualification are the coordinator's responsibility under Q6B. No independent integration approval, protected-main result or merge verdict is claimed here.

## PR and closeout

- Canonical draft: [Oteryn/Oteryn-Game #2767](https://github.com/Oteryn/Oteryn-Game/pull/2767), branch `chatgpt/teamfinder`, base `main`.
- Task record created at `2026-10-31T11:14:14Z`; the initial record's later placeholder timestamp is corrected here.
- All Q1-Q6 answers were read and applied; no rule/value/scope decision remains open.
- Related prior work: merged S14a PR #2756. The read-only re-audit ledger is preserved.
- Publication is one complete successor of `c18a58d41922c1c2ab4d0ae718cf0362bfdf269d`, under the selected connector-compatible route and Q6B. The lead is the sole branch publisher.
- Merge commit/result: **not merged; draft-only delivery**. No review request, label, deployment or merge is authorized by this closeout.
- Ownership is retained through final publication/readback, then handed to the coordinator for later packaging; workers do not publish or merge.
- This archive move is part of the final implementation authoring delta. It reaches main only through a later authorized integration; it is not a separate archive-only change or evidence that the draft already merged.
