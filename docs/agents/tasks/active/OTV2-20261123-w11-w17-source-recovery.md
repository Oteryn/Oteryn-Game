# OTV2-20261123-w11-w17-source-recovery

```yaml
task_id: OTV2-20261123-w11-w17-source-recovery
title: Recover and reconcile W11 W15b W16 W17 source facts
mode: AUDIT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: staging
branch: chatgpt/w11-w17-source-recovery-20261123
issue: 1622
pr: null
base_sha: 4f13511ca9ec3fb152f11299bb5cf7c71de16c41
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT source-recovery lead
created_at: 2026-11-23T17:01:21Z
updated_at: 2026-11-23T17:01:21Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/w11-w17-source-recovery-20261123/**
  - docs/agents/tasks/active/OTV2-20261123-w11-w17-source-recovery.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

The owner's direct 2026-11-23 request authorizes researching the four named
gaps, creating a PR and helping implement supported facts in Oteryn-Game.
The source-recovery lead is the only publisher of this newly created task
branch. Research workers have separate document paths and no Git-write,
review-trigger or merge authority. Existing quest/runtime ownership remains
unchanged. No production deployment or protected-branch integration is part
of this candidate.

The current candidate is a source-evidence and implementation packet. Its
new findings do not yet establish all inputs needed to activate Mimar,
Tibiadrome or the unresolved Phantom slot. Goblin milk is already implemented.

## Architecture and source of truth

- Bound META policy 3.1.0: `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Source roles: `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`.
- Existing shared boss activity/phase, native Creature, Map, Item and Quest
  owners remain authoritative; this task creates no alternative engine.
- Evidence classifications and exact sources are retained in
  [the packet](../../evidence/w11-w17-source-recovery-20261123/README.md).

## Publication route

Local Git supplies the inspected checkout; no material local commit is selected
for publication. CLI authentication was unavailable. The task lead selects the
bound policy's connector-compatible **new API candidate** route on this exact
single-writer branch: immediate predecessor read, one complete successor commit,
one non-force expected-head ref update, and immediate head readback. Candidate
checks are repeated for the returned remote head. No Remote Desktop publication,
sequential Contents commits, history rewrite or protected-ref mutation is used.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this candidate changes research documents only; it performs no
gameplay authority, session, persistence, inventory or encounter mutation.

## Acceptance criteria

- [x] Inspect current implementation before treating historical gaps as current.
- [x] Keep every recovered field linked to its evidence and limitations.
- [x] Separate official facts, author reports, accepted Oteryn choices and unknowns.
- [x] Verify the existing milk recipe without creating a duplicate implementation.
- [x] Identify production owners and concrete acceptance cases for unfinished work.
- [ ] Publish the bounded packet as a draft PR into `staging` and bind exact-head checks.
- [ ] Resolve the source facts listed in the packet before claiming mechanics completion.

## Reference parity

NOT_APPLICABLE as a new mechanic: this candidate admits no runtime behavior.
Field-by-field evidence and proposed production checks are in the packet.
No mechanics ledger row is marked DONE by this task.

## Validation

- `python3 -m unittest discover -s tools/content-schema/reward-claim-authoring -p test_container_native.py -v`: PASS, 23 tests on the unchanged inspected implementation.
- `python3 tools/agents/validate_governance.py`: PASS, 22 required documents and 9 lanes.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 135 tests.
- `git diff --cached --check`: required before publication; live PR records the final result.
- PostgreSQL and E2E: not rerun; documentation-only candidate. The packet does
  not use historical PG results as current execution evidence.
- Exact-head CI / independent review: live PR is the source after publication.

## Context checkpoint

```yaml
status: investigating
branch: chatgpt/w11-w17-source-recovery-20261123
head_sha: null
pr: null
last_progress: Source packet reconciled with current staging; milk authoring suite passed
blocker: Unqualified Phantom position, Mimar timing/admission, and Drome native profiles/room
owner_action_required: null
next_action: Validate the complete evidence packet and publish one new API candidate
```
