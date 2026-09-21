# OTV2-20260907-wp1-required-gate-credibility

```yaml
task_id: OTV2-20260907-wp1-required-gate-credibility
title: Repair required PR/MQ gate credibility for F01/F02
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
parent_coordinator_issue: 162
ci_programme_issue: 308
authority_comment: 5574487313
admission_main_sha: 1b41d485cc4bf126d2a9e5fe9717cc8530ece3d5
red_head_sha: cea2511dfd6cbf6fbc89d7e6173e9db68f4a00fc
final_head_sha: 118188b2fe8c15d72f8878fa73b5c3d51d9ad219
merge_sha: f2de66afd44ed5b8fcb4933b6b3ed9f92506a32f
merge_group_candidate_sha: f2de66afd44ed5b8fcb4933b6b3ed9f92506a32f
protected_gate_blob: 539a726b7d39cabe785892f70ea30d1944189d91
owner: null
owned_paths: []
conditional_paths: []
blocks: []
external_repositories: []
updated_at: 2026-09-07
```

## Terminal outcome

WP1/F01+F02 is protected and complete. The required Merge Queue gate now executes
the real governance lifecycle regression from its candidate/governance path and
the existing Windows native build/Clippy/client-smoke/synthetic-harness sequence
is guarded by fail-closed PowerShell native-command semantics. The ordinary PR
required governance path already executed the shared queue credibility regression,
so no unnecessary `merge-gate.yml` source change was introduced.

Protected `main@f2de66afd44ed5b8fcb4933b6b3ed9f92506a32f` reads
`.github/workflows/merge-group-gate.yml` as exact Git blob
`539a726b7d39cabe785892f70ea30d1944189d91`. Ruleset 20991995, sole required
`game-gate`, FULL native Merge Queue, permissions and all pre-existing
qualification jobs remain unchanged.

## TDD and negative proof

Exact test-only RED `cea2511dfd6cbf6fbc89d7e6173e9db68f4a00fc` produced hosted Merge Gate
`34152966123`, governance job `101838881761`, and failed only at the intended
missing behavior after metadata/governance/policy passed:

`AssertionError: WP1 RED: required MQ candidate does not execute lifecycle discovery`

No workflow or protected pin was changed in that RED generation.

Exact GREEN `118188b2fe8c15d72f8878fa73b5c3d51d9ad219` passed Merge Gate
`34153385789`, Agent governance `34153385743` and Architecture semantic audit
`34153385754`. Hosted governance job `101840183521` reported:

`Queue credibility regressions PASS: approved blob, lifecycle discovery, 4 native failure positions, 14 workflow mutations, 28 fan-in failures and success controls`.

Those four native failure positions use real hosted `pwsh`; marker evidence proves
a later native success cannot mask an earlier failure. The real lifecycle suite
also executes its positive path plus disposable injected-assertion negative.

## Protected pin and independent review

The activation candidate never approved its own gate. Separate protected-audit
pin PR #398 was independently reviewed and protected as
`3328d329f222057594c758ea42ef600d175e07c4`, approving exactly gate blob
`539a726b7d39cabe785892f70ea30d1944189d91`. Its self-audit refusal remained
intact and its temporary audit-path lease was released by #409, protected as
`90f26733f0ec4ea7e2a4b0c215a9a1c593204a15`.

Fresh protected-base audit of unchanged #396 head then passed as run
`34155142643`; fresh required Merge Gate `34155142415` passed every layer.
Independent Codex exact-head review reported no major issues and no review thread
remained.

## Final native Merge Queue acceptance

Full final Merge Queue run `34155914624` qualified the actual merge-group candidate
that became protected `main@f2de66afd44ed5b8fcb4933b6b3ed9f92506a32f`.
All nine jobs succeeded: candidate/governance, dependency review, CodeQL
python/actions, Linux workspace, PostgreSQL17.6, Windows client, supply chain and
final `game-gate`.

Critically, merge-group candidate job `101847587676` itself executed the new
lifecycle/queue credibility controls and reported the same 4-position native
failure, 14 mutation and 28 fan-in negative proof. Windows/SIM, Linux and PG ran
on the same real integration candidate, not on a separate synthetic status.

## Released custody and remaining programme

This archive releases the special WP1 workflow/test/policy-core write authority.
No WP1 writer or protected-pin lease remains. The historical plan
`docs/superpowers/plans/2026-09-07-wp1-required-gate-credibility.md` remains
provenance only.

WP1 completion does not itself complete downstream product work. It unlocks Work
to apply the already protected WP2 allocation to existing #353/#361 and permits
WP3 acceptance sequencing. WP4 remains ordered behind protected WP2+WP3; WP5 real
sources and Server Seam/G0/G1 remain separate.

Runtime product E2E is NOT_APPLICABLE to this control-plane archive. This archive
itself still requires ordinary repository checks, normal protected Merge Queue
and main readback before the special task record is terminally absent from active
discovery.
