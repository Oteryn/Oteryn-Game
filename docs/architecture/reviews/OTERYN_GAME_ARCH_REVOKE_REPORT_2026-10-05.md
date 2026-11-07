# ARCH-REVOKE-REPORT-0: the scope revocation report and producer key separation

- Decision: `ARCH-REVOKE-REPORT-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the Game-side rulings and the packet. The
  contract amendment RS-A1 (`docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §16)
  takes effect only on owner acceptance and Platform acceptance (§1.4). The owner accepted RS-A1
  as written (option 1a) on 2026-10-05 (CP D747). Platform accepted it on 2026-10-05 (CP D758,
  https://github.com/Oteryn/Oteryn-Platform/issues/1419#issuecomment-5995072150), so §16.1 and §16.3
  of RS-A1 are in effect when this decision merges. Contract §16.2 (key separation) is
  **pending and not in effect** (owner ruling 2a, §1.4): it changed after D758 and needs
  Platform's confirmation under #1419. Its Game part is packet OPS-KEY-SEPARATION-2 (§2.2),
  which starts only after that confirmation.
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
   - Platform's identity registry refuses a key shared across purposes, and it also registers
     each routed node's listener key and refuses a client identity that reuses one. This is the
     complete check.
   - `oteryn-game-ops` checks the authority key against every certificate of every node
     configuration it reports for. It reads them from `node_config_files` instead of a free
     list. Each scope identity it reports, and each identity holding a non-revoked assignment
     in the durable table, must be the runtime-status subject of one of those configurations.
4. **OPS-REVOKE-REPORT-1 (hard)** implements §16.1 on the Game side after #1822 merges (§2.1).
   The owner and Platform accepted §16.1 (§1.4).
5. **OPS-KEY-SEPARATION-2 (hard)** implements the Game part of §16.2 (§2.2). §16.2 is pending
   and not in effect, so the packet starts only after Platform confirms it under #1419.

Owner acceptance (§1.4): accepted as written, option 1a, on 2026-10-05 (CP D747). Platform
acceptance under Oteryn/Oteryn-Platform#1419: recorded on 2026-10-05 (CP D758) for §16.1 and for
§16.2 as it stood then. The current §16.2 is pending Platform confirmation and not in effect.

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

An omitted file can no longer pass the check, because each reported identity, and each identity
holding a non-revoked assignment in the table this tool alone writes, must be backed by a
configuration. The listener certificate is included because a node host holds that key too.

A listener key is not a client identity, so Platform's client registry alone would miss a
listener on a host outside the invocation (Codex round 2 on #1832). Platform therefore registers
the listener key of each routed node with its route descriptor and refuses any reuse between
listener and client keys. This extends the §16.2 text Platform accepted under D758, so §16.2 is
pending and not in effect until Platform confirms it under #1419 (§1.4).

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

   **Owner ruling (2026-10-05, CP D747): a) accepted as written.** The owner confirmed it directly
   to the architect.

   Platform acceptance: the revocations endpoint and its ordering rule (§16.1), and the
   per-purpose key refusal of the identity registry (§16.2), under #1419.

   **Pending Platform confirmation (Codex round 2):** the listener-key registration and the
   listener/client key refusal added to §16.2 after D758.

   **Owner ruling 2a (2026-10-05):** this decision merges with §16.2 pending. §16.2 as a whole
   is not in effect, and "accepted" in this decision and in the contract does not cover it.
   Until Platform confirms it under #1419, the ops tool keeps the #1822 check
   (`other_producer_certificate_files`), and OPS-KEY-SEPARATION-2 is not allocated. If Platform
   declines, §16.2 returns to the architect.

   **Platform ruling (2026-10-05, CP D758): accepted** for §16.1 and §16.2 at frozen head
   `1c6ffc68`. It was recorded on #1419 on the owner's behalf, with the owner's explicit
   authorization: https://github.com/Oteryn/Oteryn-Platform/issues/1419#issuecomment-5995072150. The architect verified the record first-hand, and the owner confirmed it
   directly.

### 1.5 Checklist

1. Amendments are made in the owning contract (§16, with the §3, §5, §12 and §13 pointers).
2. Concurrency: `oteryn-game-ops` is the single writer of #415 decisions. Platform orders all
   reports of a scope in one sequence and keeps the highest.
3. Restart: the body is derived from the durable row. Nothing new is stored.
4. References are typed: scope ids follow FND-ID-01, and the generation and epoch are decimal
   uint64 values.
5. Older peers: neither the operation nor the endpoint has shipped, and the assignment wire is
   unchanged. A missing endpoint fails loudly.
6. Split work: one Game PR for §16.1 (OPS-REVOKE-REPORT-1) and one Platform PR. Either order
   is safe, because a revocation that is not delivered fails loudly and changes nothing in Game.
   §16.2 is a separate Game PR (OPS-KEY-SEPARATION-2) after Platform's confirmation; until then
   the #1822 check stays, so no state is left half changed.

## 2. Packet

### 2.1 OPS-REVOKE-REPORT-1 (hard worker)

```yaml
task_id: OTV2-20261005-ops-revoke-report-1
decision: ARCH-REVOKE-REPORT-V1; runtime-status producer contract §16 (RS-A1)
depends_on: [OTV2-20261005-ops-assign-report-1]  # §16.1 accepted by the owner (D747) and Platform (D758) 2026-10-05
worker: oteryn-hard-worker
review: hard and security review (Codex, final frozen head)
branch: agent/ops-revoke-report-1-20261005
base: main after #1822 merges
owned_paths:
  - apps/game-server/src/native_admission_source/scope_assignment.rs
  - apps/game-server/src/native_admission_source/descriptor.rs     # the new operation only
  - apps/game-server/src/native_admission_source/mod.rs            # the new operation only
  - apps/game-server/src/native_admission_source/http1_mtls.rs     # the 256-byte bound for the new operation
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
  - Key separation stays the #1822 check. §16.2 is OPS-KEY-SEPARATION-2 (§2.2).
- **Acceptance:**
  - The revocation bytes equal the contract §16.1 fixture.
  - The body has no identity member and is byte-identical after a restart.
  - The loopback Platform stub sees:
    - a revocation at G after an assignment at G-1;
    - `superseded` for a lower generation;
    - `409` for an assignment and a revocation at the same key;
    - `accepted` with no state change for a byte-identical replay of the latest revocation;
    - a `404` stop with a non-zero exit naming `assignment report`.
  - The negative fixtures of contract §13 (unknown, duplicate, `null` and missing members, nesting,
    over-long body) and its value fixtures (a wrong `operation` or `contract_version`, a
    non-canonical UUID, a zero, non-canonical or overflowing epoch or generation, a negative or
    non-canonical `revoked_at`) are committed beside the valid fixture for the Platform consumer.
    The Game encoder test asserts that the encoder cannot emit any of them. The
    response decoder treats every response other than the exact §4 success body as "not
    delivered", with one test per contract §13 shape: unknown, duplicate or `null` members;
    a missing `contract_version` or `result`; a wrong `contract_version` value or type; a wrong
    `result` value or type; a non-object; and trailing bytes.
  - The assignment wire and its fixtures are unchanged.
- **Not in scope:**
  - Platform ingestion and its identity registry (Platform, #1419).
  - Epoch storage or epoch raises (U-RS5).
  - Node-side changes.
  - Character Authority host configurations, which do not exist yet.
  - Contract §16.2, which is pending (§2.2).

### 2.2 OPS-KEY-SEPARATION-2 (hard worker, gated)

```yaml
task_id: OTV2-20261005-ops-key-separation-2
decision: ARCH-REVOKE-REPORT-V1; runtime-status producer contract §16.2 (RS-A1, pending)
depends_on: [OTV2-20261005-ops-revoke-report-1]
gate: Platform confirms the current §16.2 under Oteryn/Oteryn-Platform#1419, recorded by the CP
worker: oteryn-hard-worker
review: hard and security review (Codex, final frozen head)
branch: agent/ops-key-separation-2-20261005
base: main after OPS-REVOKE-REPORT-1 merges and the gate is recorded
owned_paths:
  - apps/game-server/src/native_admission_source/scope_assignment.rs  # ReportConfig only
  - apps/game-server/src/node/config.rs                               # the certificate-enumeration method only
  - apps/game-server/src/bin/oteryn-game-ops.rs                       # the key-separation check only
  - apps/game-server/tests/native_scope_assignment.rs
  - docs/agents/tasks/archive/OTV2-20261005-ops-key-separation-2.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - cargo run --locked -p oteryn-architecture-check
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

- **Not allocated** until the gate is recorded. If Platform declines, §16.2 returns to the
  architect and this packet is withdrawn.
- **Scope:** `ReportConfig.node_config_files` replaces `other_producer_certificate_files`,
  together with the backing rule and the comparison set of contract §16.2.
- **Acceptance:**
  - An authority certificate that shares a key with any listener, evidence or runtime-status
    certificate of a named node configuration is refused.
  - A report is refused while a node identity holding a non-revoked assignment in the durable
    table has no named node configuration.
  - A configuration whose scope identity has no backing node configuration is refused, and so is
    one with an unreadable named file.
- **Not in scope:** Platform's identity registry and listener-key registration (Platform, #1419).

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
- once §16.2 is confirmed and in effect: an authority key shared with any certificate of a
  reported node host is refused by `oteryn-game-ops`, and a key shared across purposes anywhere
  is refused by Platform.

**Must decide now? YES.**

- #1822 ships assignment reporting with revoke reporting refused. Until a revocation wire exists,
  a revoked scope stays routable to the revoked holder's credential at Platform until F expires
  or a later assignment is reported. OPS-REVOKE-REPORT-1 cannot be written without this wire.
- Platform's #1419 ingestion is being built now. Fixing the sequence rule before it ships is
  cheap; changing it after both sides ship is a coordinated wire change.

**Coupling and migration cost created now.**

- A new operation, `ReportScopeRevocationV1`, that Game and Platform must both implement, and one
  shared ordering rule over assignments and revocations per scope.
- A configuration change on the Game side: `node_config_files` replaces
  `other_producer_certificate_files`. Nothing has shipped with the old field, so no configuration
  migrates.
- No change to `ReportScopeAssignmentV1`, so no existing peer migrates.

**Evidence that would justify superseding it.**

- Platform holder-identity state that makes an identity-free revocation insufficient, for example
  per-node revocation lists.
- A Platform-side registry that proves key separation for every host. The Game closed-set check
  would then be redundant and could be retired.
- Measured heartbeat freshness (U-RS1) short enough that staleness alone closes the window
  before delivery, which would make the report an optimisation rather than a requirement.

**Deliberately not decided.** Epoch storage and raises (U-RS5), PKI issuance (U-RS2), Platform's
identity registry internals, and Character Authority host configurations.
