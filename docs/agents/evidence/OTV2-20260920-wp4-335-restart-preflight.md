# WP4 / PR #335 restart preflight checkpoint — 2026-09-20

## Disposition

`WP4_335_RESTART_READY_AFTER_WP3_RELEASE`

This document is retained coordination evidence only. It does not release WP4 mutation, return shared custody, authorize Merge Queue, or supersede live Issue/PR/control-plane state.

## Live coordinates at preflight

- Repository: `Oteryn/Oteryn-Game`
- Protected baseline: `main@f4f1292544b1fffbb09e9df6ebefd5c8bb1f7879`
- WP4 issue: #329
- WP4 PR: #335
- WP4 branch: `agent/durable-fresh-admission-child-b-329`
- Preserved WP4 head: `834db1d7118d751e31287715d3eaac7780a0c7b9`
- WP4 merge-base: `53c6bdf06a2282d893035a995c46052c88f935b4`
- WP3 PR: #673
- WP3 head observed at final readback: `0eaf0a69c78a1d4521f86d87cd5307cd0e3963c0`

Authoritative dispatch/readback:
- #162 comment `5750148792` — releases read-only WP4 restart preflight only.
- #335 comment `5750224655` — records canonical worker alias and keeps mutation held.
- #162 comment `5750380522` — confirms WP3 remains frozen for discovery sweep and WP4 remains read-only preflight only.

## Hold

At this checkpoint:

```text
WP4_335_MUTATION = HELD
WP4_335_BRANCH_RECONCILIATION = READ_ONLY_ONLY
WP4_SECOND_WRITER = FORBIDDEN
WP4_MERGE_QUEUE = FORBIDDEN
MUTATION_RELEASE_TRIGGER = terminal protected WP3 #673 + explicit Work return of the five shared paths
```

No #335 source mutation or merge-up was performed during this preflight.

## Fourteen-path reconciliation

PR #335 changes fourteen effective paths.

Five paths remain intentionally serialized to WP3 #673:

```text
apps/game-server/src/durability/admission_journal.rs
apps/game-server/src/durability/db.rs
apps/game-server/src/durability/mod.rs
apps/game-server/src/durability/schema.rs
apps/game-server/tests/durability_postgres.rs
```

Nine paths remain B-only custody:

```text
apps/game-server/migrations/0002_fresh_admission_authority.sql
apps/game-server/src/bin/oteryn-game-migrate.rs
apps/game-server/src/durability/admission_authority_guards.rs
apps/game-server/src/durability/fresh_admission.rs
apps/game-server/tests/support/authority_matrix.rs
apps/game-server/tests/support/authority_recovery.rs
apps/game-server/tests/support/postgres.rs
docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md
docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md
```

Blob comparison from the #335 merge-base to protected `main@f4f12925...` showed:

- 13/14 #335 paths are byte-identical to their merge-base versions on protected main, or remain absent on both baseline and main.
- The only protected-main-changed #335 path is:
  `docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md`.
- That task record has a real overlapping edit near the beginning of the document and must be resolved semantically during the later merge-up.
- The current PR `mergeable_state=dirty` therefore has a known present-day documentation conflict; future protected WP3 changes will additionally require semantic composition on the five shared runtime/test paths.

## Open-PR collision census

A fresh census of all open PRs found:

- PR #673 is the only active writer overlapping the five serialized shared paths.
- No active competing writer was found on the nine B-only paths.
- PR #356 still contains historical overlap on twelve #335 paths, but current control-plane state classifies #356 as preserved historical/WIP evidence, not an active competing writer.

This is consistent with #162 comments `5747967503` and `5749202219`.

## Forward migration status

`apps/game-server/migrations/0002_fresh_admission_authority.sql` remains absent from protected `main@f4f12925...`.

Therefore the existing #335 `0002` remains the lawful forward migration surface if it is still absent immediately before WP4 mutation activation.

If `0002` becomes protected/released before activation:

```text
WP4_NEW_FORWARD_MIGRATION_REQUIRED
```

Released migration history must not be edited.

## Protected nonreuse contract

The protected file:

`docs/agents/programs/OTV2_WP4_GAMESESSION_NONREUSE_PERSISTENCE_AMENDMENT_20260907.md`

remains the applicable WP4 persistence/reload amendment.

The post-WP3 continuation must preserve and qualify at least:

- one durable Game-owned `GameSessionUseLedgerV1`;
- exact permanent `GameSessionId -> CharacterId` membership;
- checked membership revision/floor without wrap or rollback;
- explicit `COMPLETE` / `INCOMPLETE` reload semantics;
- atomic fresh membership + session + claims + receipt + transport;
- atomic replacement/new-session membership with current-session transition;
- replay classification before used-candidate rejection;
- exact replay without membership/revision/capacity increase;
- hard capacity semantics: entry 65,536 may commit, a distinct 65,537th candidate must fail;
- no expiry/eviction of permanent membership;
- truthful current-session identity distinct from immutable admission identity;
- restart/reload proof for S0 -> S1 -> S2 -> reject retired S1.

No new production path or new architecture decision was proven necessary by this preflight.

## Required same-lineage merge-up after release

After WP3 #673 is terminally protected and Work explicitly returns the five shared paths:

1. Fresh-read protected main, #335, #673 terminal state, current path owners and migration release state.
2. Preserve the same #329/#335 branch and history.
3. Perform an ordinary non-force merge of then-current protected main into:
   `agent/durable-fresh-admission-child-b-329`.
4. Do not rebase, reset, force-push, squash-reconstruct, or create a replacement branch/PR.
5. Resolve the task-record conflict semantically, preserving both protected coordination state and the complete B execution history/counters/evidence.
6. For the five shared paths, treat final protected WP3 behavior as upstream authority.
7. Layer WP4 fresh-admission/nonreuse semantics on that authority; do not restore superseded historical #335/#356 driver behavior.
8. Preserve protected WP2 separation between immutable admission identity and later current replacement identity.
9. Implement/finish the accepted persistence/reload amendment inside existing WP4 surfaces unless a concrete composition need proves an additional path is required.

## PostgreSQL 17.6 qualification route

Protected workflows provide the configured PostgreSQL 17.6 route.

The relevant exact test command is:

```text
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres
```

Workspace qualification also includes Rust 1.94 locked build/test and strict Clippy.

Historical #335 PostgreSQL evidence, including the prior `366 passed / 0 failed / 0 ignored` generation, is regression evidence only. It does not qualify the future composed WP2 + final-WP3 + WP4 candidate.

The resumed candidate must rerun the real PostgreSQL matrix after composition, including:

- restart/reload/reconciliation;
- exact and conflicting replay;
- current-session identity reload;
- transaction rollback boundaries;
- concurrency and deterministic fencing;
- lost response / ambiguous completion recovery;
- registered-owner acknowledgement and result-owner retirement/clear before reuse;
- S0 -> S1 -> S2 -> reject retired S1;
- `COMPLETE` / `INCOMPLETE` behavior;
- 65,535 -> 65,536 boundary and exhausted distinct-candidate rejection;
- full-u64 and corruption/mirror negatives;
- existing reconnect V1/V2, claim publication, transport collision and lifecycle regressions.

## Remaining B acceptance work

The smallest current acceptance gap is:

1. consume final protected WP3 semantics on the five shared paths;
2. persist/reload the protected nonreuse ledger and replacement current-session identity;
3. finish registered-owner acknowledgement, actual result/backend owner retirement and durable clear-before-reuse behavior;
4. close the remaining contention / ambiguity / lifecycle qualification matrix;
5. run focused RED/GREEN followed by the full configured PostgreSQL 17.6 target;
6. run applicable full workspace, strict Clippy, formatting, governance and exact-head CI;
7. obtain a fresh independent HIGH whole-diff review on the stable final head;
8. only then return to control plane for integration qualification.

## Result

No new architecture escalation, shared-lease decision, or migration-number change is currently required.

The only current release blocker for WP4 mutation is sequential:

```text
WP3 #673 terminally protected
AND
explicit Work return of the five shared paths
```

Until both are true, this checkpoint remains read-only retained evidence.
