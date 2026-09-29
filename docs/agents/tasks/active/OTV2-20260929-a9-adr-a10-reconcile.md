# OTV2-20260929-a9-adr-a10-reconcile

```yaml
task_id: OTV2-20260929-a9-adr-a10-reconcile
title: "ADR-0020 native client gameplay entry (A9) and A10/D3 reconciliation"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1214
base_sha: f5db55e39fce41e7435eb467a990e89c07bfce13
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/ADR-0020-native-client-gameplay-entry.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md   # pointer only
  - docs/architecture/reviews/OTERYN_GAME_A10_CORPSE_CONTAINER_DUR03_AMENDMENT_DECISION_2026-09-29.md   # supersession banner
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # A10 pointer defers to D3
  - docs/agents/tasks/active/OTV2-20260929-a9-adr-a10-reconcile.md
  - docs/agents/tasks/active/OTV2-20260929-a10-d39-contract-amendments.md   # archive move after #1210
  - docs/agents/tasks/archive/OTV2-20260929-a10-d39-contract-amendments.md
public_contracts: [DUR-03]
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0020 answers architect item A9 (#162 5879348805) with owner decisions D98, D129 and D138: the
production `oteryn-client` enters gameplay through a session crate extracted from
`oteryn-dev-client` and `oteryn-protocol-oteryn`, with a real Platform client grant, a minimal
renderer, mouse input, entity rendering after VIS-2 and chat after a chat lane. ADR-0011 gets a
pointer. The A10 record and its DUR-03 pointer defer to the D3 decision (#1198) where they differ
(#162 5886162546).

No code, CI, Platform or registry change is made.

## Architecture and source of truth

- `PROVEN`: ADR-0011; `apps/client`, `tools/dev-client`, `crates/renderer`, input crates,
  `crates/platform-client`; FND-04 grant profiles; `workspace-boundaries.toml`; the merge gate;
  the MOVE-RL-11 visibility decision; #1198 D3; #1210 A10.
- `UNKNOWN`: session crate name; Platform endpoint shape and timeline; chat lane design.

## High-risk authority/recovery qualification

Not applicable. Contract and ADR text only.

## Acceptance criteria

- [ ] The documents are on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, CI workflow edits, Platform changes and protocol registration.

## Finding dispositions

Codex review of `df5047f`: five P1, all ACCEPTED in repair generation 1 of 1. No owner decision
changed.

- 4131093113 (Gateway ticket chain): D138 is the unchanged ADR-0003 ticket, redemption and route
  chain; N4-P is the Gateway operation (ADR-0020 §3, delivery).
- 4131093123 (FND-04 error classes): fail closed while keeping each FND-04 code's progression,
  retry authority and public class (§2).
- 4131093134 (synthetic assets in production closure): `oteryn-synthetic-assets` stays dev/test;
  shipped placeholders move to a production-owned member (§4, N2).
- 4131093157 (A10 still actionable): the remainder is historical and non-normative; status and
  handback no longer allow allocation (A10 banner, status, handback).
- 4131093146 (three closure gates): N5 changes `merge-gate.yml`, `merge-group-gate.yml` and
  `rust.yml` together or one shared check (§1, delivery, revalidation).
- 4131548989 (first entry gate omits N6/N7): the gate now includes N6 and N7 per D129; a reduced
  entry needs a new owner decision (§7). 4131549000 (decision test): added "Decision timing" with
  the mandatory now/blocked/harder-later/supersession answers.

Third review of `547a17f` (after the owner-requested re-review): two P1 and one P2, accepted.

- 4131859464 (no channel for classified admission failures): child N8 adds a bounded refusal and
  reconciliation message; until then every refusal fails closed without retry (§2, delivery).
- 4131859472 (session boundary is TCP-specific): transport-neutral session crate plus a separate
  TCP adapter per ADR-0014 (§1).
- 4131859479 (N4 allocated before the Platform contract): N4 waits for the accepted N4-P contract
  (handback).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1214 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1214
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1214
```
