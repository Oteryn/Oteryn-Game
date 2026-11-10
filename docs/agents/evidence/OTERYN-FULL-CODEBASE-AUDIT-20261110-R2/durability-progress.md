# Durability — partial source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**.

Frozen cumulative coverage is **92/721 complete files**, 131 files with substantive ranges and **44,204 deduplicated lines**. This combines attributable earlier lead/cross-reference work with the current full-scope lane; the current lane's report separately covers 49 complete files and 24,069 lines. These figures are different scopes, not additive estimates. The remaining exact paths are in the coverage ledger.

## Entrypoints and reviewed behavior

`DurabilityRootConfig::new` validates and retains explicit bounded connection material. The production root uses a process admission flag, a dedicated runtime, verified TLS/SCRAM settings and a max-one pool. Test-only connectors have a distinct boundary. `try_issue_semantic_pass` takes only an already-idle holder; maintenance owns connection establishment and read-only migration compatibility inspection. Foreground ready admission has a separate bounded recovery window.

`IssuedSemanticPass::run_with_context` bounds the operation and observes holder return. Transaction begin installs transaction, statement and lock timeouts. The COMMIT helper distinguishes an already-expired unsent COMMIT from a timeout after the COMMIT future starts. Database rejection is definitive; transport and other non-database failures at COMMIT are retained as outcome unknown. Unknown outcomes explicitly retire the holder and request recovery. Source error payloads are classified and destroyed without retaining or formatting uncontrolled payloads.

The V2 reconnect adapter in `core_modules.rs` implements prepare, exact receipt replay, typed reconciliation and terminal replacement. It compares immutable record bindings, requires the supplied current authorization/claims, locks admission relations and ordered advisory domains, checks actor-epoch attempt budget and exact predecessor generation/lease/scope fields, fences predecessor attempts and stores replacement receipt/candidate/pending commands within one transaction. World and Channel/Instance identities remain separate. The stored predecessor scope may lag the authorized fence only in the explicit forward synchronization path; it must not be ahead. Continuity rebinding requires an exact current row and expected protection state.

The full V1 journal and guard-store source were also reviewed. Current projections compare exact SQL mirrors against canonical records, transport reservation, pending-command order and retained protection state. Historical guard decoding does not confer authority. Guard publication invokes the sealed predicate under relation locks, checks permanent decision history and writes current/history rows atomically. Party target proofs separately bind the borrowed physical runtime and independently current SQL account/character/runtime guards. Spell reconnect evidence remains bound to its physical item transaction and exact old/current generations.

The full fresh-admission store, lifecycle codecs and owner composition were reviewed. Canonical bounded history retains the original grant, distinct current session identity and exact loss/recovery protection; it never grants a controller by decoding. Fresh commit recomputes node, Channel assignment, Character recovery fence, S2 floors and the mandatory capability binding under its transaction. Recovery verifies current registered S2 floors and exact account/character claims before reserving or consuming transport/grant identities. House loss and terminal paths require the real named custody fence; access is additionally required when granting recovery. Activity and House histories preserve original recovery protection and original login clocks. Terminal release prepares bed/training/position work, exact claims and the retained receipt in one transaction.

Schema compatibility compares the embedded migration ledger against at most N+1 rows. Checksum bytes are transferred only at the expected length for an expected version; version, success and checksum must all match. Runtime inspection does not execute DDL. This establishes ledger compatibility, not the correctness of every migration.

The bosstiary kill API checks current character/custody/gameplay fences, retains an occurrence receipt and updates the boss counter. It does not increment CharacterRevision. The separate slot mutation API can burn a fee and writes a retained command receipt.

The node assignment writer was reviewed end to end: launch-secret possession, exact SQL-role grants, checked assignment generations, durable pending slot, replay reconciliation and the atomic not-ready runtime fence. Character authority is issued only after the sealed recovery record and complete integrity checks match. The separate advisory recovery gate is acquired before the current admission read so an append-only successor cannot hide behind a stale SQL snapshot. Audit delivery is bounded and at-least-once, with exact acknowledgements and explicit retention expiry.

## Findings and limits

Unconfirmed cross-module integration candidates remain in private evidence packets and are excluded from confirmed totals. Full subsequent migration and caller review continues, including counterguards to prior independently confirmed findings. No new confirmed finding is asserted in this progress report.

The suspected CharacterRevision mismatch after bosstiary kill credit was refuted in the inspected owner chain: that operation does not change CharacterRevision. This does not generalize to fee-paying slot mutations. The vendored timeout's inner-future-first poll order is a material counterguard to a proposed COMMIT timeout concern; no lost-outcome claim has been established.

Included DB/foundation/reconnect tests were inspected. Their environment gates, multi-connection fixtures, source-string assertions and linkage checks have narrower scope than a production holder/replay test. No durability Cargo suite, SQL migration, PostgreSQL race, native client or live TLS connection was executed. Exact hashes/ranges were independently validated; remaining work stays IN PROGRESS.
