# OTV2-20261110-wpkg-r2-repair

```yaml
task_id: OTV2-20261110-wpkg-r2-repair
title: WPKG-R2 collector repair and grouped follow-up
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-wpkg2
pr: 3042
base_sha: 4f0b6c62a789489fca6c37dd36d5917d096f328e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: PR3042 repair lead; publication and review-dispatch owner in comment 6767805546
created_at: 2026-11-10T15:59:04Z
updated_at: 2026-11-10T16:25:05Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_hydra.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_audit_busy.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_client_watch.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_persistent_bans.rs
  - crates/gameplay/durability/src/durability/character_authority.rs
  - crates/gameplay/social/src/cast/service.rs
  - crates/client-ui/tests/highscores.rs
  - tools/qualification/node_boot/run.sh
  - tools/qualification/node_boot/test_diagnostics.py
  - docs/contracts/logging/events/durability.json
  - docs/agents/tasks/active/OTV2-20261110-wpkg-r2-repair.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/cast/cast-v1.md
  - docs/contracts/S17_CAST_CONSENT_PERSISTENCE.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Repair the remaining reproducible failures of collector PR #3042, retain strict
authority and content assertions, and qualify one exact published candidate.
The owner requested frequent coherent pushes, subagents, light local checks,
database validation on GitHub, and one grouped P2/P3 follow-up at the end.
This record covers the repair continuation, not a new claim to implement or
re-audit every historical collector lane. The final SHA and current CI/review
state live in PR/check evidence; this file does not supersede them.

## Architecture and source of truth

- PROVEN: the bound META policy is Oteryn/Oteryn at
  `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- PROVEN: `protocol-oteryn`, distinct WorldId/ChannelId, current node and
  session-generation fences, and the accepted S17 contracts remain unchanged.
- PROVEN: the lead is the only publisher; workers return disjoint edits or
  read-only reviews. API publication creates a new one-parent candidate with
  an expected-head, non-force ref update, followed by exact readback and freeze.

## High-risk authority/recovery qualification

Applicable to cast observer admission/renewal. The repair retries only a
`DurabilityError::RootUnavailable` read; every attempt still proves the current
node incarnation and reads the durable ban table. SQL/authority failures and
persisted bans remain final. No cached ban result, weaker proof, new capability
offer, protocol field or durable write is introduced.

Current facts come independently from the recovered DurabilityRoot/node proof,
the persistent account-ban row, the existing opaque VerifiedAdmission, and the
monotonic clock. Elapsed waiting/reading advances `now` before existing Join/Renew
checks, including `tick` of an existing observer. Missing holder, ban, storage
failure and time expiry are considered separately. The expiry regression also
asserts that the old viewer is removed, not only that renewal returns an error.

The actual-root PostgreSQL regression polls admission while holding a semantic
pass, requires Pending, releases the pass, then checks Join and Renew success,
persisted-ban refusal, expiry and missing-table refusal. The existing persistent
ban regression retains restart/idempotency/current-node checks. On `455c0c243`, the actual-root regression passed in 6.777 s on GitHub
(job129944835052), and the persistent-ban case passed in 5.258 s
(job129944834914). Local extracted proofs do not replace these database runs.

The sibling sweep covered both Join and Renew and the other direct test caller
of `apply_durable`. Independent review caught the stale-time risk before push;
the published repair advances time for both paths. Character recovery changes
only add closed stage diagnostics and propagate the same errors in the same
order; no recovery acceptance or mutation predicate changes.

## Acceptance criteria

- [ ] Required `game-gate` and `heavy-gate`, including all 24 PostgreSQL
  partitions, pass on the unchanged final PR head.
- [ ] The approved final Codex review has an exact-head receipt and no open
  concrete P0/P1 finding; existing resolved findings stay resolved.
- [ ] The grouped P2/P3 follow-up is recorded with explicit dispositions.
- [ ] Landing, if eligible, satisfies main-health and post-merge requirements.

## Reference parity

NOT_APPLICABLE: this is a repair of existing code and qualification fixtures,
not a new mechanic or change to reference gameplay values.

## Excluded scope

No workflow/protection weakening, production deployment, external-repository
mutation, duplicate Windows fix or broad appearance dependency import.
Node boot remains informational under the current workflow and the owner
exception in PR comment 6745293631; diagnostics are not a claim of full Node
qualification. Windows repair is owned by the separately merged PR #3094.

## Implementation / findings

- PROVEN: `c5c16989f` heap-pins the remaining native quest route futures without
  changing their predicates, calls or assertions. Trader passed in 70.506 s on
  predecessor `2aa8b76b8`; all 194 library cases in its partition passed.
- PROVEN: `2aa8b76b8` rotates six periodic refresh classes while preserving
  input/probe/server-message/chase precedence. The exact production poll proof
  is baseline-red/fixed-green; the real slow-vitals/chat regression passed in CI.
- PROVEN: `455c0c243` repairs the one-shot cast-ban holder path, uses monotonic
  fixture clocks, preserves failed observer server results and clears the Hydra
  fixture's retained ground cache across its simulated reset epoch.
- DERIVED: the cast holder gap can explain the original PG5 refusal, but that
  log hid the inner server error. On `455c0c243`, the same live chat/moderation
  scenario passed in 31.327 s (job129944834799); the original inner cause remains
  an inference rather than a directly logged fact.
- PROVEN: closed inbound/recovery diagnostic labels preserve behavior and are
  sanitized. The missing event registration found by the Rust policy job is
  repaired using the exact PR3076 registry entry from `96b4cd55`.
- PROVEN: main `4f0b6c62` added category 16 to the highscore map but left old
  selection assertions. Updating indices 13, 14 and 15 retains reserved-ID coverage.
- PROVEN: the matching Windows repair landed separately in PR #3094 as
  `b028bef4ddcc6d48bdd6cf61479c94e42435ed88`; it changes the layered-child-window
  compatibility manifest and HUD diagnostics. UNKNOWN: its new post-merge
  run42850705885 had not completed at this checkpoint. The Node exception does
  not stand in for Windows qualification.

## Validation

Focused evidence before the next freeze:

- Rust1.99.1 formatting/parser checks for the changed Rust files: pass.
- `git diff --check`: pass.
- `python3 tools/qualification/node_boot/test_diagnostics.py`: pass, 2/2.
- `bash -n tools/qualification/node_boot/run.sh`: pass.
- Exact cast retry helper: baseline held-resource failure, fixed 3/3 pass.
- Existing highscore selection test with exact production tables/functions:
  baseline reproduces Some(16)/Some(17), corrected test 1/1 pass.
- Unmodified production logging validator in an isolated small runner:
  baseline unregistered event, registered event pass.
- `python tools/agents/validate_governance.py`: pass on this authoring batch.
- `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.

Component/integration/E2E: GitHub owns full crate compilation and database/Node
execution. On predecessor `2aa8b76b8`, all 10,903 unit tests and 22/24 PG partitions
passed. On `455c0c243`, all 10,904 unit tests, Clippy and both PG archive builds passed.
Twenty-one of 24 PG partitions completed successfully before the successor was
prepared, including Hydra in 40.606 s (job129944834985), Trader in 88.520 s
(job129944835072), and the cast regressions above. Three PG partitions and
informational Node boot were still running. Rust policy exposed the missing
event registration; this authoring change repairs it.
These predecessor results are not acceptance of the successor candidate.

## Self-review

The lead inspected the complete bounded repair diffs, preserved assertion and
failure semantics, checked the exact remote tree after publication, and reviewed
the accepted invariants. The lead also reviewed the complete record/registry/highscore delta; the
highscore assertions match the existing production map and the registration
exactly matches the closed emitted event. No unresolved material finding was
identified in this bounded self-review.

## Independent review

Two Codex rounds completed; the latest clean receipt is comment 6762452825 on
`2d7b455b9c8757a69964a8b69049bd27c0db0218`. The owner explicitly approved one
additional final round on 2026-11-10. It remains unused at this authoring point.
The lead alone dispatches it after live same-head de-duplication. Worker reviews
of the cast and diagnostics found no concrete P0/P1; they are not Codex review.

## PR and closeout

PR: https://github.com/Oteryn/Oteryn-Game/pull/3042.
The one Rashid P2 thread is fixed and resolved. Remaining P2/P3 are consolidated
in comment 6766071958 for the requested grouped follow-up after primary work.
No merge or ownership release is claimed. Keep this record active while exact
qualification, the external main-health blocker or the grouped follow-up remain;
archive it in the final authoring change of the task's last PR. Do not move a
frozen head merely to copy CI/review status into this file.

## Context checkpoint

The exact published head, run/job IDs, review authorization, next action and
ownership are maintained in PR comment 6767805546. Current next action: publish
the coherent repair of the observed gate failures, freeze its exact SHA and
qualify/review it. No owner permission question is pending.
