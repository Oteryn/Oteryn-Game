# OTV2-20261113-wheel-testserver-audit

```yaml
task_id: OTV2-20261113-wheel-testserver-audit
title: Wheel and Gem Atelier audit against the Winter Update test server
type: mechanic
mode: AUDIT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/wheel-testserver-audit-20261113
pr: null
base_sha: ce45063507c22a11e0db7e9c0ff86dacbe6ff834
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-wheel-audit-root
created_at: 2026-11-13T19:31:16Z
updated_at: 2026-11-13T19:31:16Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261113-wheel-testserver-audit.md
  - docs/reference/wheel-of-destiny-testserver-20261113.md
  - crates/gameplay/progression/src/wheel_effects.rs
  - crates/gameplay/progression/src/wheel_effects/
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Compare the current executed Wheel and Gem Atelier with the Winter Update test server,
preserve accepted Oteryn differences, repair proven defects, and retain exact unknowns.
The owner explicitly authorized native client interaction and repository branch/PR/push.
This work does not declare protected integration or full Global parity complete.

## Architecture and source of truth

- PROVEN: baseline `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`; bound META policy
  `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- PROVEN: existing Wheel/Gem contracts and producers remain the implementation owners.
- UNKNOWN: complete latest Global behavior; see the [audit](../../reference/wheel-of-destiny-testserver-20261113.md).
- One branch publisher: root. Workers return source evidence and owned-path patches;
  they do not commit, push, trigger paid review or merge.

## High-risk authority/recovery qualification

NOT_APPLICABLE to the initial read-only audit and documentation delta. Reclassify before
changing any durable mutation, session/lease gate, receipt or protocol boundary.

## Acceptance criteria

- [ ] Dated official/client evidence covers latest Wheel/Atelier changes.
- [ ] Complete source-to-consumer map and explicit uncovered behaviors retained.
- [ ] Every accepted defect has a failing regression and a verified minimal repair.
- [ ] Required changed-path validation and independent review are recorded accurately.
- [ ] Dedicated PR contains the complete coherent delta; no unverified full-parity claim.

## Reference parity

| Behavior | Reference | Oteryn | Check |
| --- | --- | --- | --- |
| Workshop stat bonus | Fresh Global observation pending | UI advertises +5 per grade; shared producer absent | Regression pending source confirmation |
| Revelation Mastery | Catalogue 150/165/180/225; fresh threshold observation pending | Donor-specific double application | Table-driven and composed projection checks pending |
| Vessel quality order | Current test-client panel pending | Lesser/Regular/Greater repeated four times | Existing slot tests establish implementation, not reference parity |
| Removal permission | Owner decision 2026-11-05 | Combat-lock based permission retained | No change requested to accepted difference |

## Implementation / findings

See the linked audit. Historical task/ledger entries were consulted only to locate existing
implementations. Current code and live GitHub state decide current implementation facts.
No implementation fix or complete test pass is claimed in this initial audit checkpoint.

## Validation

- `git diff --check`: pass for the initial audit documentation.
- `python3 tools/agents/validate_governance.py`: pass (22 documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: pass (129 tests).
- Rust focused/component checks: pending; pinned Rust 1.99.1 provisioned for this task.
- Live client: initial Wheel panel observed; subsequent disconnect prevents controlled changes.
- Exact-head CI: no candidate published yet.

## Self-review / independent review

Root self-review and required Codex review pending a published reviewable candidate.

## Context checkpoint

Next action: publish the scoped audit PR, then verify and repair source-confirmed findings.
Known blocker: test-client session expired; further live Wheel actions need owner login.
