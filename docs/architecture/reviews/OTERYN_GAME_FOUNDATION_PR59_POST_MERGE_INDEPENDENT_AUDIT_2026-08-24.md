# Oteryn Game Foundation PR #59 — Post-Merge Independent Audit

- Date: 2026-08-24
- Repository: `Oteryn/Oteryn-Game`
- Audit issue: #77
- Source implementation issue: #53
- Source implementation PR: #59
- Final PR head: `ce891601498729e4b7e8711e88340f62002a21ba`
- Squash merge commit: `a70318484b1ffdd328b53cdc70a4386a516d0109`
- Audited Git tree: `6d2d7839f9512a841e01b82424821e5e38e4c432`
- Integration base: `099e147031ce9320586602b98c62df1c4311bbe8`
- Main preflight for this audit: `1f69677b40851551953caf853c08b37ce7b29c68`
- Audit mode: post-merge independent technical/provenance review
- Product mutation: none

## 1. Executive verdict

```text
PRODUCT_IMPLEMENTATION: PASS
POST_MERGE_INDEPENDENT_AUDIT: PASS
MATERIAL_FINDINGS_P0: 0
MATERIAL_FINDINGS_P1: 0
MATERIAL_FINDINGS_P2: 0
HISTORICAL_PRE_MERGE_INDEPENDENT_EXACT_HEAD_GATE: NOT_PROVEN
RETROACTIVE_PRE_MERGE_COMPLIANCE: NO
```

The merged Foundation implementation satisfies the allocated technical acceptance of Issue #53 on the exact audited tree. No new reproducible material P0/P1/P2 implementation defect was found in this audit.

This audit does **not** rewrite the historical merge sequence. The retained PR/review evidence does not prove that the mandatory genuinely independent review was completed on the final PR head `ce891601...` before PR #59 was merged. Independent reviews recorded on earlier heads are superseded for that gate. The current post-merge PASS therefore closes the technical uncertainty, not the historical process fact.

## 2. Exact-tree provenance

GitHub records the final PR head `ce891601498729e4b7e8711e88340f62002a21ba` with Git tree:

```text
6d2d7839f9512a841e01b82424821e5e38e4c432
```

GitHub records squash merge `a70318484b1ffdd328b53cdc70a4386a516d0109` with the same Git tree:

```text
6d2d7839f9512a841e01b82424821e5e38e4c432
```

Therefore the complete repository contents of the final PR head and the squash-merged implementation commit are byte-for-byte tree-equivalent. This audit of `a7031848...` is an audit of the exact final PR #59 tree, not a reconstruction from a superseded candidate.

The squash commit message contains stale prose naming `ecc9bdb8...` as the "Exact final candidate". That statement is provenance-only and is superseded by the Git object evidence above. It did not alter the merged tree.

## 3. Independence and method

This review was performed after merge as a fresh audit of the immutable GitHub tree, contracts, registries, PR history, review threads and exact-head CI logs. Implementing/coordinating self-review statements were treated as claims requiring independent corroboration, not as proof.

The audit did not modify Foundation product code, contracts, registries, workflows, repository protection or production state.

The execution sandbox could not resolve `github.com` for a local clone, so no new local cargo execution is claimed. Verification of executable behavior uses the retained GitHub Actions logs that checked out exact head `ce891601...` and ran the repository-selected build/test/security gates. Static review uses GitHub content addressed by the exact audited commit/tree.

## 4. Normative inputs

The review checked the implementation against:

- Issue #53 — `OTV2-IMPL-FOUNDATION: native protocol/runtime/admission foundation`;
- `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`;
- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`;
- `docs/contracts/protocol-oteryn/v1/foundation.proto`;
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`;
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md`;
- `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`;
- root and `apps/game-server/` agent governance.

Deferred physical persistence, production listener activation, Platform JWS/KMS implementation, gameplay command/state registrations and unmeasured reconnect/liveness timing values were not silently promoted into Issue #53 acceptance.

## 5. Issue #53 acceptance matrix

| # | Acceptance criterion | Verdict | Independent evidence |
|---|---|---|---|
| 1 | Bounded FND-02 framing/envelope/message decode before peer-sized allocation | PASS | `FrameLength`, legal protobuf field-number checks, bounded length-delimited parsing, direction-specific nested ingress validation, registered limits and boundary regressions |
| 2 | Stable wire identifiers, errors, direction/phase/sequencing metadata | PASS | typed UUIDv7 wrappers; MessageType 1..14 matches registry; error codes/dispositions match registry |
| 3 | `CommandRef`/`CommandId` reservation, duplicate/outcome/gap handling and connection-generation fencing | PASS | `CommandRef = GameSessionId + CommandId`; exact-next reservation; 64 outstanding bound; lower IDs never re-execute; checked generation successor |
| 4 | Server sequence, state revision, resync and snapshot barriers/assembly | PASS | sequence commit-after-application state machine; revision/gap fail-closed behavior; monotonic snapshot IDs; chunk/body/domain limits and duplicate-domain rejection |
| 5 | FND-03 one-owner runtime generation/ordinal fencing | PASS | `ScopeRuntimeFence` is non-Clone/non-Copy, external constructor is inaccessible, stale generations/stamps fail, external grants must be strictly newer |
| 6 | FND-04 fresh admission, GameSession, CharacterLease and reconnect/recovery state machines without invented timing | PASS | typed fresh replay key; exact initial transport; atomic durable seam; exact PREPARE/COMMIT binding; terminal irreversibility; process rehydration from current fenced authority |
| 7 | Stable safe diagnostic/error dispositions | PASS | FND-02 numeric error/disposition mapping matches registry; admission failures are typed and do not expose bearer material |
| 8 | Negative/golden/property tests plus workspace build/fmt/Clippy/security gates | PASS | final exact-head Merge Gate #310 and related exact-head workflows succeeded; game-server 104/104 on Linux; Windows client lane, CodeQL, dependency and supply-chain checks succeeded |
| 9 | Full-diff self-review plus genuinely independent exact-head review before merge | PARTIAL / HISTORICAL GATE NOT PROVEN | self-review and many independent reviews exist, but retained evidence does not show a genuinely independent review bound to final `ce891601...` before merge |
| 10 | Freeze exact head, protected checks/review, squash merge, verify main, archive task/release ownership | PARTIAL | exact-head checks, squash merge, main verification and archival occurred; review prerequisite in item 9 is not proven |

Items 9–10 are process/provenance qualifications. They do not create a new product-code P0/P1/P2 finding.

## 6. FND-02 protocol and resource-limit audit

The merged implementation exposes:

- protocol major `1`;
- transport profile `1`;
- ALPN `oteryn-game/1`;
- 1 MiB frame maximum;
- 65,536-byte bootstrap/resume payload maximum;
- 16,384-byte admission/reconnect material limits;
- 128-byte UTF-8 build ID;
- 128 capability entries;
- 64 expected command revisions;
- 65,536-byte command/result payloads;
- 64 outstanding commands;
- 256 domains per reconciliation/snapshot;
- 262,144-byte state delta payload;
- 256 snapshot chunks;
- 524,288 bytes per chunk;
- 16,777,216 assembled snapshot bytes.

These values match the active Resource Limits Registry.

The envelope parser validates legal protobuf field numbers before narrowing, uses checked arithmetic for length-delimited fields, rejects malformed/truncated values, and dispatches typed nested validation by message direction before returning the payload.

Supported capabilities remain additive; selected capabilities are closed against the v1 registry, which currently defines no optional capability IDs. Semantic handshake checks preserve same-major additive schema compatibility rather than incorrectly requiring exact schema fingerprint equality.

No Canary/legacy opcode or fallback path was introduced.

## 7. Command, sequence, revision and snapshot audit

`CommandIngress` reserves only the exact next non-zero `CommandId`, advances the high-water mark exactly once, does not consume the 65th command when the 64-command window is full, never treats a lower already-reserved ID as new work, and permits terminal commit only from the earliest pending ID.

The server sequence implementation separates observation from commit, so a payload/revision failure cannot advance `last_applied`. Resync requires a contiguous retained boundary or falls back to snapshot.

The snapshot public facade validates both the private chunk/generation/byte assembler and the completed `SnapshotBody`. It enforces non-zero unique domain IDs and the 256-domain hard maximum before exposing assembled bytes. Generation changes discard partial assemblies and snapshot IDs remain monotonic.

No new material finding was identified in these paths.

## 8. FND-03 runtime-fence audit

`ScopeRuntimeFence` owns the next ordinal for one ownership generation and cannot be copied or cloned. Its constructor from an external grant is not public, so downstream callers cannot create duplicate ordinal issuers for the same generation through the public API.

`accept_input` requires the exact current ownership generation and uses checked ordinal increment. Exhaustion prevents further issuance. `apply_external_grant` accepts only a strictly newer generation, resetting ordinal issuance only after a proven ownership replacement. Old work stamps fail after replacement.

The final exact-head doctests independently prove that copy, clone and external duplicate-construction attempts do not compile.

No new material finding was identified in this path.

## 9. Fresh admission audit

The game-domain fresh-admission seam preserves the full 32-byte GrantNonce in `FreshAdmissionReplayKey` with stable tagged durable encoding. `FreshAdmissionCommit` binds:

- game-issued `GameSessionId`;
- Character/World/Channel identity;
- exact CharacterLease generation;
- RuntimeScope ownership generation;
- initial `connection_generation = 1`;
- exact authenticated initial transport.

The durable authority snapshot carries current lifecycle, current connection generation and current transport. The public facade reloads current durable authority after fresh commit and clears the process projection on ambiguity.

Fresh-grant replay cannot revive a terminal session, roll an already reconnected active session back to generation 1, or bind a second transport to the same generation-1 authority.

This is the allocated game-domain authority seam. Full Platform credential/JWS/KMS verification and physical persistence remain outside this PR's allocated implementation scope.

No new material finding was identified in this path.

## 10. Reconnect, lost-response and process-recovery audit

`ReconnectAttemptRef` has stable non-zero durable encoding and no numeric ordering semantics.

A prepared reconnect is bound to:

- exact `GameSessionId` through the journal key;
- exact reconnect attempt;
- exact predecessor generation;
- strict-successor candidate generation;
- exact authenticated candidate transport;
- exact CharacterLease;
- exact RuntimeScope ownership generation.

The public `ReconnectAttemptJournal` contract requires `reconcile_reconnect_attempt` to return the current GameSession authority, attempt disposition and retained binding from one transaction/lock/fenced linearization point. Split `load_session` + `lookup` composition is explicitly invalid.

For PREPARED reconciliation, the facade requires the session still be reconnectable with no controller and the predecessor/lease/runtime fence still match.

For COMMITTED replay, it requires the current session be active on the exact winning candidate transport and generation, with the exact same CharacterLease. RuntimeScope may advance after COMMIT, but a rollback below the COMMIT-bound generation fails `StaleRuntime`.

Process rehydration reconstructs only a current authoritative snapshot. It does not treat `GameSessionId` as bearer authority, cannot revive terminal state, cannot adopt a changed CharacterLease for the same GameSession, and cannot reconstruct an older runtime generation.

The final regression suite explicitly covers authority changes between earlier reads and attempt reconciliation, peer commits between initial read and claim, process replacement, stale controller, lease and runtime races, and COMMITTED RuntimeScope rollback.

No new material finding was identified in these paths.

## 11. Scope and production-side-effect audit

The merged composition root exports the Foundation module but still reports:

```text
GameplayAvailability::UnavailableBootstrap
```

and explicitly states that no gameplay listener, production admission path or persistence authority is activated.

PR #59 did not change protocol registries/contracts, introduce gameplay command/state IDs, create a new persistence schema, or invent deferred liveness/grace values.

Physical gameplay-wire Tier-1 E2E was therefore legitimately `NOT_EVALUATED` for this allocation; the exact-head synthetic harness and bootstrap smoke are correctly treated as boundary evidence rather than misreported physical gameplay E2E.

## 12. Exact-head CI evidence

For final PR head `ce891601498729e4b7e8711e88340f62002a21ba`:

- Merge Gate #310 — run `32701561959` — SUCCESS;
- Merge Authority Audit #235 — run `32701561954` — SUCCESS;
- Architecture Semantic Audit #259 — run `32701561965` — SUCCESS;
- Agent Governance #349 — run `32701561990` — SUCCESS.

The Linux workspace job checked out exactly `ce891601...` and passed:

- `cargo +1.94.0 build --locked --workspace --all-targets`;
- strict Clippy with `-D warnings`;
- `cargo +1.94.0 test --locked --workspace`;
- `oteryn-game-server`: 104 passed / 0 failed;
- three `ScopeRuntimeFence` compile-fail doctests;
- synthetic client harness;
- native game-server bootstrap smoke.

The same Merge Gate also passed Windows client build/strict Clippy/visible smoke/synthetic harness, CodeQL for actions/python, dependency review, Rust supply-chain checks, policy/metadata validation, final aggregate validation and `game-gate`.

The deterministic Merge Authority Audit validates the merge-control-plane mechanism; it is additional evidence and is **not** misclassified here as the missing semantic independent code review.

## 13. Historical review/provenance finding

### PROC-01 — mandatory pre-merge independent exact-head review is not proven

Severity: process/provenance; not a product-code P0/P1/P2.

Issue #53 and `apps/game-server/AGENTS.md` required a genuinely independent exact-head review for this high-risk delivery before merge.

PR #59 contains multiple independent Codex/Qwen review cycles, and those cycles found many real P1/P2 defects that were subsequently repaired. However, every independently attributable review verdict retained in the PR evidence is bound to a superseded SHA. The final retained task state before merge still described an independent exact-head review as mandatory/pending.

No retained review submission or timeline artifact was found that binds a genuinely independent semantic review to final head `ce891601...` before merge.

The archived task record later stated that independent exact-head reviews returned no reproducible findings. That wording is stronger than the retained evidence supports and is corrected by the audit closeout change accompanying this report.

### PROC-02 — squash message contains stale candidate prose

Severity: provenance note.

Squash merge `a7031848...` names `ecc9bdb8...` as the exact final candidate in its commit message. Git object evidence proves the actual merged tree equals final PR head `ce891601...` exactly, so this is not a source-content discrepancy. It should be treated as stale commit-message provenance only.

## 14. Final conclusion

The exact Foundation tree merged by PR #59 is technically acceptable for the scope allocated by Issue #53.

No new reproducible material implementation findings were found:

```text
P0 = 0
P1 = 0
P2 = 0
```

The post-merge independent audit result is `PASS`.

The historical pre-merge independent exact-head review gate remains `NOT_PROVEN` and must not be backdated or represented as satisfied. The correct durable record is:

```text
implementation: PASS
post-merge independent audit: PASS
historical pre-merge exact-head independent review: NOT_PROVEN
```

No Foundation product-code repair is recommended from this audit. The required repository action is limited to preserving this audit receipt and correcting the archived task provenance.
