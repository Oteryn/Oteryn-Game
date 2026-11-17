# OTV2-20261112-pr3115-wu26-qualification

```yaml
task_id: OTV2-20261112-pr3115-wu26-qualification
title: Qualify the WU26 server package through batched review and fixture repairs
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3115-merge-20261112
pr: 3115
base_sha: f05944d8852342d30c933cc3a0f4338676f92571
head_sha: null
owner: Codex integration lead
owned_paths:
  - crates/gameplay/quests/src/quest/task_board_full.rs
  - crates/gameplay/quests/src/quest/task_board_full_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/fixture.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/paid.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/reshape.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/wire_paid.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/wire_paid/reshape.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/temple_scroll.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/task_board/token_case.rs
  - crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/atelier.rs
  - crates/gameplay/dispatch/src/gameplay_transport/atelier/v2.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/command.rs
  - crates/gameplay/dispatch/src/gameplay_transport/capability_offers_golden_tests.rs
  - crates/protocol-oteryn/capabilities/0483-gem-atelier-v2.json
  - crates/protocol-oteryn/src/tests/capabilities_golden/0483-gem-atelier-v2.json
  - docs/contracts/protocol-oteryn/v1/gem-atelier-v2-v1.md
  - docs/contracts/protocol-oteryn/v1/registry/capabilities/0483-gem-atelier-v2.json
  - docs/agents/tasks/archive/OTV2-20261112-pr3115-wu26-qualification.md
```

## Scope and dispositions

The owner assigned #3115 before client #3098, with independent Codex review and
repairs in coherent P0/P1/P2 batches until no findings remain. This task-specific
instruction overrides the previous review-count and P2-deferral limits for these
PRs. Normal forward merges brought accepted main into the server package; no
workflow or protection changes were authored by this repair.

- Fixed the reviewed P2: a capped bounty claim no longer refuses a historical
  maximum BIGINT token balance when its token credit is zero. The boundary
  regression failed with ArithmeticOverflow before the fix and passes after it.
- Preserved the current capped-credit and Task Board token guard around PREY's
  deliberate reapplication of migration 0547. Reapplication had overwritten that
  guard, so the eleventh daily claim failed at the ten-token cap.
- Made the Temple test's Inbox container a real stamped MAIL parcel. The native
  MAIL writer retains its complete source tree and delivers it with guards on.
  The invalid ordinary Inbox backpack had caused a deferred custody failure that
  rolled back every scroll retirement; all ten offline assertions remain.
- Made legacy Atelier fragment fixtures and references canonical. Its stash
  preparation loads and publishes a real native registry and invokes the actual
  stash writer with matching NativeItemWeights; it does not weaken the digest or
  custody checks or replace the legacy Atelier owner.
- Added an actual selected command-6214 test through serve_admitted, including a
  fresh non-Premium refusal, a paid redraw, modifier preservation, one fragment
  and fee, and retained-result replay without a second payment. The existing
  direct-writer carried/stash and negative scenarios remain.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo metadata --locked --no-deps --format-version 1`: pass.
- `cargo test --locked -p oteryn-gameplay-quests --lib`: pass, 424 tests; the
  focused token-boundary test has separate red and green evidence.
- `cargo clippy --locked -p oteryn-gameplay-quests --all-targets -- -D warnings`:
  pass. Dispatch/game-server builds and PostgreSQL Rust suites are deferred to
  GitHub runners under the owner's explicit local-test limit.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check`: pass; duplicate migration-number check: no duplicates.
- Isolated PostgreSQL 18.6 probes retained the real guards: capped-credit
  reapplication reproduced the failure and current-guard restoration committed
  eleven grants at balance ten. All-migration MAIL/custody probe reproduced the
  invalid Inbox failure, then real MAIL posting and all ten retirements committed.
  Every probe database was removed. This is SQL evidence, not a claim that the
  complete Rust scenarios or Node qualification have run locally.

## Review and integration

The first current-task whole-server independent Codex review found the bounty
P2 and recorded the command-6214 evidence gap and failing CI cases. All are
addressed together here. Independent final-delta review and exact-head GitHub
native, PostgreSQL and Node qualification follow the final authoring freeze.
No clean-review or green-CI claim is made in advance. The local Codex CLI failed
authentication before model execution; reviews use the native independent Codex
reviewer, which authored none of these changes.

Authoring is complete; merge and post-merge qualification remain pending. The
freeze comment binds the exact head. Merge result: squash merge of #3115, only
after qualification. #3098 takes accepted server main afterwards; no force push,
red-Node waiver or protection bypass is authorized by this record.

## Qualification successor after the first runner batch

The exact a6e312236 generation passed native qualification, but five PostgreSQL
cases failed. Their complete logs were collected before this next generation:

- The second small-fragment fixture reused immutable item identities from the
  earlier paid scenario. It now has its own 157/160/163 identities and ordinals;
  its reader observes that exact fresh batch. No retired row is reused or deleted.
- The PostgreSQL MAX-token case still expected the old overflow refusal. Both
  MAX-1 and MAX now prove a successful zero-credit claim, unchanged token
  balance/ledger/source count, XP/points and exact receipt replay. Genuine
  arithmetic-overflow cases retain their strict refusal assertions.
- The shared Summer Templar fixture used gem resistance as total Life Drain
  immunity. The accepted production cap is thirty percent. Existing direct
  equipment protection now isolates that independent damage source, retaining
  the energy/holy assertions and the production cap.
- The first non-Premium command-6214 step returned outer Rejected instead of its
  required Accepted refusal payload in two independent existing compositions.
  Its exact rejecting branch was not yet known at that freeze. Temporary cfg(test)-only
  DBG-PR3115-6214 markers attributed outer permit/selection, decoder/fence, runtime
  predicate and commit-error branches without changing their return conditions.
  They were removed after the matched diagnosis below. That generation was a
  diagnostic qualification candidate, not a claim that this case was
  repaired or that a failing candidate may merge.
- The Activity cancellation case failed at its initial entry COMMIT, before the
  cancellation barrier, with an indeterminate outcome after 2001 milliseconds.
  The identical case passed in the existing composed client run. Its earlier
  underlying cause remains UNKNOWN; no write retry, deadline increase, waiver or
  speculative source change is included. The legitimate new-head qualification
  required by the confirmed repairs must execute it again.

A real-schema PostgreSQL identity probe reproduced reuse of retired seed148;
the fresh second batch committed while retaining three retired and three live
rows with quantity sum six, then removed its own database. This is fixture SQL
evidence, not execution of the complete Rust scenarios. Formatting, locked
metadata, governance and whitespace checks are repeated on the successor. The
unchanged Quest source is not rebuilt locally. Final native/PostgreSQL/Node
checks remain with GitHub runners, and the unresolved cases remain explicit.

## Matched capability-offer repair and diagnostic removal

Composed client head `5f117b87`, PostgreSQL 12 job `131123799450`, reported the exact
outer predicate: input_permit=true, command_selected=false. Its 42 E2E tests
passed; 204 of 205 library tests passed and the sole failure was the 6214 case.
The decoder, runtime and durable Atelier owner had not been reached.

The real registry 483 had every offer profile disabled and no dynamic offer
hook. Fresh negotiation therefore silently discarded 483 despite support for
148 and 483. The paired native client implements both the capability allowlist
and Vessel/Reshape callers. All 17 changed capability fragments were checked:
the only other all-false fragment, 182, already has its independent dynamic
Daily Reward offer hook and is unchanged.

Capability 483 is now offered only by bundle Worlds, retaining requirement 148,
the false production/item-view profiles and the contract's legacy offered=false
field. Existing 148 clients do not select 483 or gain its commands. The owning
contract now describes the implemented optional bundle/client path. The one
runtime golden was regenerated with `tools/contracts/update_capability_golden.py`;
no generated table or aggregate was hand-edited.

A new focused regression drives the actual SelectedCapabilities methods and all
five 6210..6214 command gates, legacy/missing-parent refusals, both non-bundle
profiles and the complete 137-capability bundle selection against capacity 160.
The lightweight probe compiles and runs the actual current protocol build.rs and
build_capabilities.rs against cached serde_json, then compiles the generated
tables, actual selection code/constants and that regression. It reproduced
RED ([148] instead of [148,483]) before the flag change and passed GREEN afterward.
This proves negotiation and command gating, not the complete native wire/SQL
scenario. The real 6214 scenario additionally asserts negotiated 483 before its
first request; its strict refusal, burn, fee and replay assertions are unchanged.

All temporary DBG-PR3115-6214 code was removed. The three diagnostic source files
are byte-identical to their pre-diagnostic `f5d051b8` content. Earlier batch evidence
now includes server PG11/23 passes and client PG14/23 passes for the confirmed
identity, capped-token and Templar fixtures. Those results belong to their exact
earlier heads, not this repair. Activity's earlier indeterminate COMMIT remains
unexplained unless the new exact-head results classify it; no retry or waiver is
introduced. Independent repair-delta review and final native/PostgreSQL/Node CI
remain required before integration.

## Existing offer-description guard

The composed client native job `131145818845` passed 3,842 of 3,843 tests; the
remaining failure was the existing bundle-profile description guard at
`capability_selection_tests.rs:371`. That test derives the correct bundle-only
set and requires each corresponding offer_gate description to name
`BUNDLE_WORLD_OFFERED_CAPABILITIES`. The new 483 text described bundle Worlds
but omitted that literal. The initial log-only diagnosis called this an allowlist
failure; reading the actual assertion corrected that classification.

Only the contract description is corrected. The existing test, runtime offer
flags, parent requirements, selection code and regenerated golden remain
unchanged. A whole bundle-only description check using the existing assertion's
marker reproduced exactly the missing 483 description, then passed after this
correction. The actual current protocol generator and selection regression,
formatting, governance and whitespace checks pass. This is metadata/negotiation
evidence, not final native or paid-wire qualification. Independent bounded-delta
review and the final candidate's own CI remain required.
