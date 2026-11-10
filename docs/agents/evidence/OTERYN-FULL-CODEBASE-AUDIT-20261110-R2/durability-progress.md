# Durability audit — checkpoint 1

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**.
Scope: `crates/gameplay/durability`, 721 tracked files. This checkpoint manually reviews 19 full files / 5,994 text lines. The exact 702 remaining files are in `coverage-ledger.json`; prior selective cross-references are not treated as completed files.

This lane checkpoint is a subset of cumulative coverage: the main ledger also retains independently reviewed files and identical-blob historical evidence. Exact cumulative coverage and remaining paths are in `coverage-ledger.json`.

## Entrypoints and reviewed behavior

`DurabilityRootConfig::new` validates and retains explicit bounded connection material. The production root uses a process admission flag, a dedicated runtime, verified TLS/SCRAM settings and a max-one pool. Test-only connectors have a distinct boundary. `try_issue_semantic_pass` takes only an already-idle holder; maintenance owns connection establishment and read-only migration compatibility inspection. Foreground ready admission has a separate bounded recovery window.

`IssuedSemanticPass::run_with_context` bounds the operation and observes holder return. Transaction begin installs transaction, statement and lock timeouts. The COMMIT helper distinguishes an already-expired unsent COMMIT from a timeout after the COMMIT future starts. Database rejection is definitive; transport and other non-database failures at COMMIT are retained as outcome unknown. Unknown outcomes explicitly retire the holder and request recovery. Source error payloads are classified and destroyed without retaining or formatting uncontrolled payloads.

The V2 reconnect adapter in `core_modules.rs` implements prepare, exact receipt replay, typed reconciliation and terminal replacement. It compares immutable record bindings, requires the supplied current authorization/claims, locks admission relations and ordered advisory domains, checks actor-epoch attempt budget and exact predecessor generation/lease/scope fields, fences predecessor attempts and stores replacement receipt/candidate/pending commands within one transaction. World and Channel/Instance identities remain separate. The stored predecessor scope may lag the authorized fence only in the explicit forward synchronization path; it must not be ahead. Continuity rebinding requires an exact current row and expected protection state.

Schema compatibility compares the embedded migration ledger against at most N+1 rows. Checksum bytes are transferred only at the expected length for an expected version; version, success and checksum must all match. Runtime inspection does not execute DDL. This establishes ledger compatibility, not the correctness of every migration.

The bosstiary kill API checks current character/custody/gameplay fences, retains an occurrence receipt and updates the boss counter. It does not increment CharacterRevision. The separate slot mutation API can burn a fee and writes a retained command receipt.

## Cross-dependencies and findings

Integration candidates remain in restricted evidence pending independent verification and complete downstream review. They are excluded from confirmed counts. The existing root-confirmed economy finding remains separately recorded; this partial scope neither strengthens nor replaces that evidence.

## Hypotheses and limitations

The suspected stale CharacterRevision after bosstiary kill credit is **refuted in the reviewed owner chain**: the kill operation does not change the character revision, and the inspected owning table/receipt migrations add no such trigger. This conclusion does not apply to fee-paying slot operations.

A simple assertion that the outer semantic timeout necessarily overrides the COMMIT timeout is not supported. The actual vendored Tokio timeout polls its inner future before its timer, which is a material counterguard. A residual scheduling question about differing constructed deadlines is unproven and is not a finding. No outcome-loss or duplicate-operation claim is made from this question.

Reviewed schema-string tests and linkage tests have deliberately limited evidentiary value: they check text, symbols and selected pure bounds; their names do not establish atomicity, contention safety or SQL replay behavior. Include-only adapter files receive credit only for the adapter, not their included topic files.

## Validation and exact remaining work

All 19 entries in `ledger.json` have exact full ranges and SHA-256 matching both the immutable Git blob and worktree; `validation.json` contains the result. No source files were edited. No durability package tests, PostgreSQL fixtures, real TLS connection, Windows client or full-workspace build was run. Source review of tests is distinguished from execution.

Remaining: 702 exact paths in `coverage-ledger.json`, including the admission journal and current authority owners, included reconnect test topics, most gameplay persistence APIs and their tests. The package is not REVIEWED and is not runtime VERIFIED.
