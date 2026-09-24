# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: Historical journal-only durability candidate
mode: IMPLEMENT
status: superseded
integration_state: HISTORICAL_EVIDENCE_ONLY_NOT_FOR_INTEGRATION
repository: Oteryn/Oteryn-Game
issue: 167
pr: 212
recovery_issue: 240
coordinator_issue: 162
base_branch: main
branch: null
owner: null
original_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
historical_branch: impl/game-durability-journal
historical_head_sha: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
historical_recovery_pr: 243
accepted_successor_issue: 250
accepted_successor_pr: 252
write_authority: none
owned_paths: []
shared_paths: []
blocks: []
external_repositories: []
updated_at: 2026-09-07
```

## Terminal scope

This old #212 candidate is SUPERSEDED, not implementation-completed. It was
closed without merge; its branch/head and all incident evidence remain retained.
Do not resume its former merge-up/TDD instructions, qualify or integrate it,
rewrite its history or transfer one of its former paths to another worker.
The paused-provenance label was a historical hold, not an outstanding permission
to restart after the accepted clean successor completed.

## Incident preservation and recovery

The original P0 remains part of the immutable history: destructive cross-scope
commit `cd808d396018832b632be26911105a36f0cb7a20`, unallocated restoration
`73e17f418c63ec038f5aa7ef8f0888ac74b75aa2`, and retained #212 source head
`fb30fba2a888835dfc7cbde27f940b79d7bfe05d`. A clean later tree or successor
acceptance does not retroactively authorize those actions.

Reviewed recovery allocation #241 integrated as
`a171410de07c2dab718f52f780d4314bdcc53604`; its own task is already archived
at `docs/agents/tasks/archive/OTV2-20260828-durability-provenance-recovery.md`.
It permitted exact file-content input, not #212 commit/review/CI inheritance.
The intermediate #243 lineage was then explicitly superseded by #167 comments
5456369600, 5456381241 and 5462519470 through architecture #249/allocation #251.
Its source branch remains `impl/game-durability-journal-recovery-240` at
`eb28c42125c346e7f6f1c72e69d51af35af8fc1f`; #392 protected its superseded
archive at `b2876939087ef73761dea2af99c6660ca5a81b53`, then Work closed #243
without merge in comment5574113375. Neither historical branch is deleted here.

## Accepted bounded successor and parent closure

Current #250 is closed/completed and records the accepted journal/reconnect
successor #252 merge `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`, its own
independent qualification and isolated PostgreSQL evidence, followed by #290
archive `47faa84152ffdcac00d8e2173d582aa54be2cbcf` and ownership release.
Those are successor facts, not qualification of this old #212 candidate.

After this final legacy record is protected/read back, Work may close #167's
bounded journal-only substrate outcome through that accepted successor and
#240's provenance-recovery lifecycle. Original admission/history and the
successor's own review/CI remain distinct. No fresh implementation or repair
is claimed by this record-only change.

This does NOT complete or release current fresh-admission #329/#335, SQLx driver
#351/#356, Foundation #353/#361, Server Seam #247, WP1, G0/G1, value transactions
or production. Those require their own current contracts and qualification.
The obsolete clean-journal prerequisite is not a substitute for those gates.

## Source, scope and validation

Work's exact scope is #162 comment5574135019, preparation main
`b2876939087ef73761dea2af99c6660ca5a81b53`: only this old active-to-archive
pair. No current worker, runtime, SQL/released migration, Cargo, workflow/pin,
registry, architecture, credential or external-repository bytes are modified.

The complete original task, architecture/registry predecessors, release list,
recovery constraints and historical tests remain available at
[protected b287693](https://github.com/Oteryn/Oteryn-Game/blob/b2876939087ef73761dea2af99c6660ca5a81b53/docs/agents/tasks/active/OTV2-20260825-impl-durability.md),
blob `142b340b154786cc4b8006b725ca47525efd8ff2`.

Validate terminal metadata, exact identities and the complete two-path diff;
require author self-review, canonical exact-head checks, normal protected MQ
and main readback before parent closure. This low-risk archival enforces an
already accepted supersession and adds no authority or safety-policy change;
a new external AI review is not required solely for this lifecycle operation.
Runtime E2E is NOT_APPLICABLE to the archive; historical or skipped tests are
not a new runtime PASS. Record final head/CI/merge in GitHub rather than a
self-referential or no-op source commit. Retain both historical source branches.
