# OTV2-20261015-bundle-overlay-admission

```yaml
task_id: OTV2-20261015-bundle-overlay-admission
title: Preserve one object domain in bundle World admission snapshots
mode: REPAIR
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 2219
pr: null
base_sha: fc03ef48f9e0604bba0c1e9b484e3ffa0be50a46
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex Oracle migration task
created_at: 2026-10-15T22:29:00Z
updated_at: 2026-10-15T22:29:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - docs/agents/tasks/active/OTV2-20261015-bundle-overlay-admission.md
public_contracts:
  - accepted MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1 section 3
depends_on: []
blocks: [Oracle native entry acceptance]
cross_repository_coordination_id: Oracle migration 20261015
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome

Governing Issue: #2219. The owner explicitly extended this task to repair
client/server compatibility and prove real native entry. The protected-main
server currently emits legacy domain2 with capability18, contrary to the accepted
one-object-domain contract. The advanced client correctly rejects it. The main
prototype client lacks required capability18 and is not a bundle World client.

## Architecture and source of truth

PROVEN: accepted MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1 section3 requires bundle
World object state only in domain17; fixture World retains domain2. Runtime source
is protected main77af046ce14c81df6ed6cab03ae693a68acff32c. Real Windows diagnostics
passed OAuth/ticket/gateway/TLS before UnselectedDomain domain2 in the advanced
client. Coordination comment on PR1942: 6203593500. That writer owns client/session
files; this task changes only server snapshot composition and focused owning tests.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this wire-domain repair changes no authority consumer, grant,
lease/generation, persistence mutation, identity, schema, fence or recovery
controller. Admission and current authority remain unchanged. Production rollout
is separately gated by #2217 and Platform #1639's durable Oracle controller.
Protocol correctness is high risk and requires independent exact-head review.

## Acceptance criteria

- [ ] The actual shared fresh/resync wire snapshot contains domain17 and no domain2
  when map capability18 is selected; its map payload decodes normally.
- [ ] Fixture snapshots retain existing complete domain2 and revision semantics.
- [ ] Exact-candidate Game checks and independent review pass.
- [ ] Oracle rollout advances the durable incarnation and real client entry passes.

## Excluded scope

No client PR1942 edits, no weaker client validation, no new wire/schema contract,
no new account/Character, no World/content replacement, no trust/floor reset.

## Validation

- cargo fmt --all --check: pass using the pinned Rust1.94 toolchain in local Docker.
- Focused RED: bundle_join_and_resync_send_only_the_map_object_domain fails on the unchanged 77af/fc03 snapshot path, panicking when the bundle snapshot consults the legacy object owner; 3207 other tests filtered.
- Local GREEN/package test qualification: blocked by local compiler memory (SIGKILL), including reduced-debug single-job compilation. This is infrastructure failure, not a failed or passing behavioral result. Exact CI must prove GREEN and the full owning package before integration.
- cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings: pass (exit0; pinned local Rust1.94).
- python tools/agents/validate_governance.py: pass.
- python -m unittest discover -s tools/agents/tests: pass (124 tests).
- git diff --check: pass.

The initial unused allocation codex/fix-bundle-overlay-admission-20261015 has no
published candidate or PR; preserve it pending intentional branch cleanup. The
sole active source writer moved coherently to the current-main allocation above,
preserving owned uncommitted changes without reset/rebase. Main's intervening
changes are runner workflow/operational records only; server inputs are unchanged.
Workflow parser repair is separately owned by #2217 / PR2222, not this source diff.
Molehill-PC is offline, so real native hardware acceptance remains UNKNOWN until
its agent returns. No runtime or entry success is claimed from local compilation.

## Operational continuation and source candidate

Source authoring is complete on the sole-writer branch
`codex/fix-oracle-native-entry-20261015`, delivery PR #2223. Its exact final
remote head is frozen and qualified through the live PR/STATE evidence; no other
writer may mutate it. This active packet tracks the still-open main-only rollout
and hardware acceptance under Issue2219, so canonical branch/pr dispatch fields
remain null instead of becoming a terminal source PR after integration.
The source PR is currently unmerged and must pass exact CI plus independent
review before Merge Queue. Material findings return the same writer to AUTHORING,
then freeze and qualify a successor; this record grants no authority to bypass
those gates. Runtime and real entry acceptance remain UNKNOWN. Archive this
operational record only after they pass under the accepted post-deployment
closeout exception. The existing #2217 packet owns runner/workflow acceptance.
