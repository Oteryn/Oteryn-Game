# OTV2-20261004-session-push-1

```yaml
task_id: OTV2-20261004-session-push-1
title: "SESSION-PUSH-1: server-initiated deltas, the domain store and a bounded event queue"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/session-push-1-20261004
issue: 1622
pr: 1752
head_sha: "exact frozen head in the FREEZE entry to the control plane"
final_head_sha: "exact frozen head in the FREEZE entry to the control plane"
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - crates/session/src/**
  - tools/dev-client/src/**
  - tools/synthetic-client-harness/src/live/**
  - apps/game-server/src/gameplay_transport/qualification.rs
  - docs/agents/tasks/archive/OTV2-20261004-session-push-1.md
public_contracts: []
depends_on: [CLIENT-NEG-1]
blocks: [ENTITY-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md` §2.1, §1.2.

- `oteryn-session` applies every `StateDelta` through one domain store (domains 1, 2, 3) by its own
  domain and revision, in `service_liveness` and around commands. A revision mismatch, an
  unselected domain, an unregistered delta type or a domain with no store poisons the session.
- Unclaimed deltas queue for `take_events()`, typed per domain. The queue is bounded:
  `MAX_QUEUED_EVENTS` = 256; the next delta fails closed with `EventQueueOverflow` (#1736 P2
  4177061733). Tested at max (256 queued) and max+1 (poisoned).
- A step or cast claims the first delta of its domain after the result, by revision alone; the
  fixed next-domain expectation is gone. A `COMMITTED` use returns at its `UseResult`, with
  `world_object_overlay_delta` always `None`. Acceptance covers domain 2 and the no-delta use only;
  domains 9, 11 and multi-domain uses belong to ITEM-CLIENT-1 (control plane, D497 7a).
- Harness: `LiveController::idle` and `dispatch` drain events into `RenderModel`; the vitals line
  redraws. `dev-client` re-exports `SessionEvent` and mirrors the two new errors.
- `qualification.rs`: each door USE expects `None`, and its domain 2 delta (base, new revision,
  entry) is asserted from `take_events()` before the next command; the final door state is
  unchanged.
- No capability added to `CLIENT_SUPPORTED_CAPABILITIES`; no protocol, registry, server or
  `apps/client` file touched.

## Server Seam qualification

`gameplay-server-seam.yml` dispatched on `claude/session-push-1-20261004` at code head
`40e8b6f4e4f670ed7b1eee809e52887ca41f34a6` (run 37197587416; the later commit adds only this record).
Result: `S3B_RESULT=SEAM_PASS` (workflow conclusion success).

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet`: pass
- `cargo check --locked -p oteryn-game-server --tests`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
