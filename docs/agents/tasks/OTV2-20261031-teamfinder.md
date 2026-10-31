# OTV2-20261031-teamfinder

```yaml
task_id: OTV2-20261031-teamfinder
type: mechanic
title: Team Finder members and typing indicator
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/teamfinder
pr: null
base_sha: a52549e3b9230c7486a24df66f4e8ec59d1f96cb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: teamfinder-task-lead
created_at: 2026-10-31T12:00:00Z
updated_at: 2026-10-31T12:00:00Z
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
public_contracts:
  - docs/contracts/protocol-oteryn/v1/team-finder-leader-v1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implement S14b/S14c member browsing, requests, cancellation and leader decisions through the existing Team Finder owner; implement S19 transient typing through existing session and chat ownership. Keep capability 200 and commands 390/391. Deliver a draft PR only; no review request, label, deployment or merge.

## Architecture and source of truth

PROVEN: current main is `a52549e3b9230c7486a24df66f4e8ec59d1f96cb`; leader slice merged in PR #2756 (`b8be29a08c12e402c13f03ca762ac916f98ade65`). The primary donor is Crystal `201189f871d23597b82caf88fd611454a2ddefb8` (OTS_HYPOTHESIS_ONLY evidence class). Existing protocol identity, party, session-generation and durable receipt contracts remain authoritative.

CONFLICT: the donor member handler looks up the applicant's listing after reading a leader id; leader status changes do not call the party owner; donor logout removes listings while the merged leader contract retains them. These choices require explicit questions on this PR before their final implementation is claimed.

## High-risk authority/recovery qualification

Applicable: session-generation-fenced member transitions and durable command receipts. Current session/actor evidence must be resolved independently of receipts. Qualification must cover stale generation, wrong actor, cross-session handles, replay, cancellation, leader removal, relog and concurrent decisions. Evidence is pending implementation.

## Acceptance criteria

- [ ] Browse and member/leader commands reach the owned runtime and durable writer.
- [ ] Every durable transition has a mounted real-dispatch PostgreSQL case.
- [ ] Replay returns the recorded outcome without reapplying state.
- [ ] Typing uses donor spectator selection and existing transient ingress limits.
- [ ] Required capability, registry, golden, contract and client registration file sets are complete.
- [ ] Focused checks, affected-crate Clippy/tests and exact-head inspection are recorded.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Browse fields and occupancy | Crystal protocolgame.cpp:2823-2889 | Existing LeaderListing will be reused | Pending |
| Request and cancel | Crystal protocolgame.cpp:3125-3158 | Selected-listing lookup decision pending | Pending |
| Leader status 2/3/4 | Crystal protocolgame.cpp:3075-3112 | Existing party owner integration decision pending | Pending |
| Leader logout | Crystal game.cpp:1422-1426 | Existing durable-template compatibility decision pending | Pending |
| Typing spectators | Crystal game.cpp:13074-13084 | Existing chat/session owner | Pending |

## Excluded scope

Protected main, merge, review requests, heavy-tests label, production offer, workflows, repository policy tools, vendor, generated content inputs and the read-only re-audit ledger.

## Implementation / findings

AUTHORING. Reserved block checked against main and open PRs: migration 0644-0647, capability 232-233, command 444-447, mechanic/offer 584-587; only numbers actually needed will be used. Bundle-world capability count 75 fits SELECTED_CAPACITY 112 before this change. Subtasks own disjoint files; the lead alone publishes the canonical branch. Remote computer is used for isolated authoring and tests; GitHub connector publication selects a new API candidate, with one predecessor, complete delta, non-force update and live readback.

## Validation

- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py`: pass, exit 0 on main a52549e3b9, no reserved-block clash.
- Initial documentation checks: pending.
- Rust and PostgreSQL checks: pending implementation.

## Self-review

Pending complete implementation and exact-head diff review.

## Independent review

Deferred to coordinator; owner explicitly requests a draft PR and no review request from this task.

## PR and closeout

Initial draft channel pending the first commit. Questions use one QUESTION comment each; ANSWER comments are re-read before every new step and before final reporting. No merge authorized.
