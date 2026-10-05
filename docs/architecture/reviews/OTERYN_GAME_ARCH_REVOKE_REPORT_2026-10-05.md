# ARCH-REVOKE-REPORT-0: the scope revocation report and producer key separation

- Decision: `ARCH-REVOKE-REPORT-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the Game-side rulings and the packet. The
  contract amendment RS-A1 (`docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §16)
  takes effect only on owner acceptance and Platform acceptance (§1.4). OPS-REVOKE-REPORT-1 is
  not allocated before both.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D745 (REVOKE-REPORT-CONTRACT-1). OPS-ASSIGN-REPORT-1 (#1822) reports
  only assignments and replacements and refuses `--node-identity` on revoke. Two Codex P1
  findings on #1822 at `1ce1363e` remain open:
  - The revocation report reused the revoked holder's identity at generation N+1. That holder's
    credential then matched the new generation.
  - The key-separation check compared the authority key only with a caller-supplied list.
- Runtime, Platform and production authority: NONE. Platform acceptance goes through
  Oteryn/Oteryn-Platform#1419.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR** adds Amendment RS-A1 to the owning runtime-status producer contract (§16 there)
   and this packet. No code.
2. **Revocation report (§16.1).** A new operation, `ReportScopeRevocationV1`, at its own compiled
   path `POST /internal/v1/game-auth/native-scope-revocations`. Its body carries the epoch, the
   scope, the revocation's generation and `revoked_at`, and no node identity. Platform orders
   revocations and assignments in one per-scope sequence by `(epoch, generation)`. A latest
   revocation matches no node report, so the scope routes nowhere until a later assignment.
   `ReportScopeAssignmentV1` is unchanged.
3. **Key separation (§16.2).**
   - Platform's identity registry refuses a key shared across purposes. This is the complete
     check.
   - `oteryn-game-ops` checks the authority key against every certificate of every node
     configuration it reports for. It reads them from `node_config_files` instead of a free
     list. Each scope identity it reports must be the runtime-status subject of one of those
     configurations.
4. **OPS-REVOKE-REPORT-1 (hard)** implements both on the Game side after #1822 merges and after
   the owner and Platform accept RS-A1 (§2).

Owner question (§1.4): 1. accept RS-A1.

## 1. Rulings

### 1.1 A revocation carries no identity

Contract §5 said a revocation is "reported the same way with the new generation", but a revoked
row has no holder. #1822 first filled `node_identity` with the revoked holder's identity. Platform
matches a node report by `(epoch, generation)` and TLS identity (§7). Generations rise by one per
decision, so the revoked holder's credential could report at N+1 and match. That turned the
revocation into a re-assignment of the same holder.

The revocation is therefore its own operation with no identity member. A latest revocation
matches nothing. No identity can be chosen that the revoked holder cannot authenticate as, so
"no identity" is the only shape that removes the holder.

A separate operation, instead of a state member on `ReportScopeAssignmentV1`, leaves the
assignment wire and its fixtures exactly as #1822 ships them. Each endpoint's schema stays
closed, and a Platform without the endpoint answers loudly (`404`, §16.1) instead of silently
treating a revocation as an assignment.

### 1.2 Key separation has one complete check, and it is Platform's

A single ops host cannot see every producer key in the fleet, so no Game-side list can be
complete. The complete check belongs where every identity is registered: Platform's identity
registry (§16.2). The Game check is defence in depth over a closed set the tool derives itself:

- the node configurations it reports for;
- every certificate each configuration names, enumerated by one `NodeConfig` method so that a
  new purpose cannot be forgotten;
- a refusal when a scope identity has no backing configuration.

An omitted file can no longer pass the check, because each reported identity must be backed by a
configuration. The listener certificate is included because a node host holds that key too.

### 1.3 Durable and restart behaviour

The revocation body comes only from the durable revoked row (`ownership_generation`,
`decided_at`) and the declared epoch. Nothing is retained in the state directory. The retained
holder-identity file of #1822's repair is not needed and is not written. A re-send after any
restart is byte-identical.

### 1.4 Owner and Platform acceptance

1. Accept Amendment RS-A1 (contract §16).
   - a) Accept as written (recommended): no identity on revocation, a separate operation, and
     Platform as the complete key-separation check.
   - b) Accept §16.1 only, and keep the Game check as the configured certificate list. The list
     can still be bypassed by an incomplete configuration, so Platform's check carries all of it.
   - c) Do not report revocations in v1. The operator removes the node identity from the scope on
     Platform by hand.

   Platform acceptance: the revocations endpoint and its ordering rule (§16.1), and the
   per-purpose key refusal of the identity registry (§16.2), under #1419.

### 1.5 Checklist

1. Amendments are made in the owning contract (§16, with the §3, §5, §12 and §13 pointers).
2. Concurrency: `oteryn-game-ops` is the single writer of #415 decisions. Platform orders all
   reports of a scope in one sequence and keeps the highest.
3. Restart: the body is derived from the durable row. Nothing new is stored.
4. References are typed: scope ids follow FND-ID-01, and the generation and epoch are decimal
   uint64 values.
5. Older peers: neither the operation nor the endpoint has shipped, and the assignment wire is
   unchanged. A missing endpoint fails loudly.
6. Split work: one Game PR (OPS-REVOKE-REPORT-1) and one Platform PR. Either order is safe,
   because a revocation that is not delivered fails loudly and changes nothing in Game.

## 2. Packet

### 2.1 OPS-REVOKE-REPORT-1 (hard worker)

```yaml
task_id: OTV2-20261005-ops-revoke-report-1
decision: ARCH-REVOKE-REPORT-V1; runtime-status producer contract §16 (RS-A1)
depends_on: [OTV2-20261005-ops-assign-report-1, owner acceptance of RS-A1, Platform acceptance of RS-A1]
worker: oteryn-hard-worker
review: hard and security review (Codex, final frozen head)
branch: agent/ops-revoke-report-1-20261005
base: main after #1822 merges
owned_paths:
  - apps/game-server/src/native_admission_source/scope_assignment.rs
  - apps/game-server/src/native_admission_source/descriptor.rs     # the new operation only
  - apps/game-server/src/native_admission_source/mod.rs            # the new operation only
  - apps/game-server/src/native_admission_source/http1_mtls.rs     # the 256-byte bound for the new operation
  - apps/game-server/src/node/config.rs                            # the certificate-enumeration method only
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/tests/native_scope_assignment.rs
  - apps/game-server/tests/native_admission_source_transport.rs    # exhaustive match arm
  - docs/agents/tasks/archive/OTV2-20261005-ops-revoke-report-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - cargo run --locked -p oteryn-architecture-check
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

- **Scope:**
  - `ReportScopeRevocationV1`: the exact encoder, the success decoder, the compiled path and the
    256-byte response bound for every status.
  - `assignment revoke`, and `reconcile` of a committed revoke, report the revocation after the
    commit. `assignment report` on a revoked scope re-sends it.
  - `--node-identity` stays refused on revoke. No holder identity is retained.
  - `ReportConfig.node_config_files` replaces `other_producer_certificate_files`, together with
    the backing rule and the comparison set of §16.2.
- **Acceptance:**
  - The revocation bytes equal the contract §16.1 fixture.
  - The body has no identity member and is byte-identical after a restart.
  - The loopback Platform stub sees:
    - a revocation at G after an assignment at G-1;
    - `superseded` for a lower generation;
    - `409` for an assignment and a revocation at the same key;
    - a `404` stop with a non-zero exit naming `assignment report`.
  - An authority certificate that shares a key with any listener, evidence or runtime-status
    certificate of a named node configuration is refused.
  - A configuration whose scope identity has no backing node configuration is refused, and so is
    one with an unreadable named file.
  - The assignment wire and its fixtures are unchanged.
- **Not in scope:**
  - Platform ingestion and its identity registry (Platform, #1419).
  - Epoch storage or epoch raises (U-RS5).
  - Node-side changes.
  - Character Authority host configurations, which do not exist yet.

## 3. Rejected options

- The revoked holder's identity at the new generation (#1822 `828db053`). The holder's credential
  matches N+1, so the revocation re-assigns it.
- An `assignment_state` member on `ReportScopeAssignmentV1`. It changes the wire #1822 ships, and
  it would let a consumer that does not know the member read a revocation as an assignment.
- A sentinel or tombstone `node_identity` value. It is an identity some certificate could one day
  carry, and it mixes two meanings in one field.
- Only the caller-supplied `other_producer_certificate_files` list (#1822 `1ce1363e`). An
  incomplete list passes.
- Game alone as the complete key-separation check. No single ops host sees every producer key.

## 4. Decision test

The decision holds if:

- after a delivered revocation, Platform accepts no node report for the scope, including one from
  the revoked holder's credential at any generation, until a later assignment and a matching
  node report;
- the revocation body is derived from durable state alone;
- the assignment wire is unchanged;
- an authority key shared with any certificate of a reported node host is refused by
  `oteryn-game-ops`, and a key shared across purposes anywhere is refused by Platform.
