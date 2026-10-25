# SEC-1

```yaml
task_id: SEC-1
title: "SEC-1: server-side integrity checks — bounded checkpoint of SEC-TRUST-1/SEC-REL-1 policy leaves"
mode: IMPLEMENT
status: waiting
checkpoint: BOUNDED_CHECKPOINT
merge_authority: CP_ONLY
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/sec-1
issue: 2054
pr: null
base_sha: f094e0320289975a220852ff60ca7baf6d234ff4
head_sha: pending
owner: "CP takeover worker for #2188 (D1379); earlier record writers: SEC-1 worker session_015PVevC8C3J6ZTkLXAYy6b9, /root/gm_compile_fix"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/SEC-1.md
  - crates/gameplay/social/src/security/mod.rs (new)
  - crates/gameplay/social-rules/src/trust_policy.rs (new)
  - crates/gameplay/social-rules/src/trust_policy/tests.rs (new)
  - crates/gameplay/social-rules/src/release_policy.rs (new)
  - crates/gameplay/social-rules/src/release_policy/tests.rs (new)
slot_only_paths:
  - "apps/game-server/src/lib.rs: the `security` module declaration only (D1427)"
public_contracts: []
depends_on:
  - "SEC_CLIENT01 ACCEPTED (D1034); acceptance is not child implementation authority"
  - "CLOCK-1 residue MERGED #2114; ITEM-LOOK-1 MERGED #2182"
  - "SEC-TRUST-1 / SEC-REL-1 contract owners: not allocated (D1326)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2188](https://github.com/Oteryn/Oteryn-Game/pull/2188). Five pure security policy leaves and their test-only registration are merged. Evidence: `crates/gameplay/social-rules/src/trust_policy.rs`.

Remaining: All runtime trust/release consumers, challenge/telemetry/bot-control integration and production qualification remain; the leaves grant no runtime protection. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Outcome

Bounded checkpoint (**MERGE_AUTHORITY: CP_ONLY**). PR #2188 adds five pure
SEC-TRUST-1/SEC-REL-1 policy leaves under `apps/game-server/src/security/` and
declares the module test-only in `lib.rs` (D1427), so CI compiles, lints and runs
their 17 unit tests. It ends no child task and changes no runtime behaviour:
nothing in the server build calls these predicates. Runtime wiring stays
**REPORT_ONLY**.

- `trust_policy.rs`: consecutive revision/floor rules, checkpoint age and skew,
  embedded/highest-seen constraints, checked raw-wall/monotonic arithmetic and
  fetch-after-step ordering.
- `release_policy.rs`: root/release signer-role separation and revocation-first
  key membership.

The predicates inspect supplied facts only. They do not authenticate signed bytes,
resolve current trust, bootstrap a restored store, grant admission or authorize a
challenge or outcome.

## Decisions

- **D1326:** no grant for SEC-TRUST-1 or SEC-REL-1. CP names those owners later.
- **D1379:** CP takeover of #2188 as a bounded checkpoint. `main` is merged in with
  a merge commit (no rebase).
- **D1427:** slot-only hunk in `apps/game-server/src/lib.rs`, which declares the
  `security` module and adds nothing else. The leaves have no dependencies outside
  the owned paths, but in a non-test build every `pub(crate)` item would be unused,
  so a plain `mod security;` fails strict Clippy with `dead_code`. The declaration is
  therefore `#[cfg(test)] mod security;`. See Validation.
- **D1431:** this file is the single SEC-1 packet. The checkpoint content moved here
  from the duplicate root-level `docs/agents/tasks/SEC-1.md`, which is deleted. The
  earlier bot-control gap record (scope (b): step integrity in `gameplay_transport/`)
  is superseded here and stays unallocated. The full dated history of the deleted
  record is kept in git at `af2acb12be608f46e845341e1fa7e9d21d0ce15a:docs/agents/tasks/SEC-1.md`.

## Excluded scope (needs separate CP allocation)

- Non-test registration and runtime wiring: `connection.rs`, `gameplay_transport/`,
  `capabilities.rs` and `crates/protocol-oteryn/src/lib.rs`.
- Cryptographic verification.
- The trust store, fetch and clock latch (SEC-TRUST-1 / D1031).
- Signed artifact schema, key-id rules and decoder limits (SEC-REL-1 / RELEASE-MANIFEST-3).
- PROD-COMPAT admission and ADMIT-CAP-1.
- The SEC-CHAL, SEC-TELEM and SEC-DETECT children, and telemetry collection
  (DATA-PRIVACY-01).
- The earlier bot-control step-integrity proposal.
- Protocol and registry changes, and migrations.

## Acceptance criteria

- [x] Only the allocated pure leaves, this packet and the D1427 declaration are changed.
- [x] Strict Clippy passes and CI runs the leaf tests (17).
- [x] No runtime activation, no SQL, no migration, no registry change.
- [ ] CP merge decision (CP_ONLY).

## Validation

These checks ran on Molehill-PC (slot-run, slot 3), on a tree that is `af2acb12` plus
the committed declaration. The `lib.rs` sha256 is `5c1789f7…` and matches the pushed blob.

- `cargo fmt --all --check`: PASS.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` with
  `#[cfg(test)] mod security;`: PASS. The only warning comes from vendored tokio
  and was already there.
- `cargo test --locked -p oteryn-game-server --lib security::`: 17 passed, 0 failed.
- Plain `mod security;` was checked for D1427: strict Clippy FAILS with 16 errors,
  all `dead_code` for items such as `MAX_CHECKPOINT_AGE`, `PolicyError`, `SigningRole`
  and `check_applied_head`. It is not used.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: 124 OK.
- PostgreSQL: not applicable (no SQL, durable state or migration).
- CI on the previous head `af2acb12`: 27 success, 3 skipped. One cancelled duplicate
  of Agent governance; its rerun succeeded.

## Independent review

- Codex review requested on the new head. Thread r4304709105 (duplicate packet) is
  fixed by D1431.
- Earlier heads: the Rust 1.94.0 standalone run of the leaf tests passed 17/17
  (history at the git path above).

## Context checkpoint

```yaml
last_progress: D1427 declaration and D1431 packet consolidation pushed
status: waiting
branch: skeleton/sec-1
pr: null
blocker: null
next_action: CP reviews CI and Codex on the new head, then decides on merge (CP_ONLY)
```

## Completion + wiring (post-checkpoint)

Imported from closed draft #2233 and rebased onto the ADR-0022 layout (`docs/architecture/ADR-0022-gameplay-crate-layering.md`); every existing path below was verified on origin/main `59785d76`. The checkpoint merged unfinished; nothing in the running game calls it. The completion PR must not merge until the mechanic is playable end to end through the real dispatch path, CI is green and Codex review is clean. Dispatch chain: `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (command `else if` chain) and the `GATED` table in `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` (highest capability id on main: 30). Wire modules live in `crates/protocol-oteryn/src/`. Migration numbers: next free number at allocation time (only one migration PR in the queue); 0130 MAP-PICKUP-1, 0131 BAGS-USE-1 and 0132 (provisional TIMED-RT-1c) are reserved.

Structure rules (binding acceptance criteria):
- [ ] One mechanic per module file in its ADR-0022 layer (pure rules L3, PostgreSQL in `durability`, runtime L4, dispatch L5).
- [ ] New tests in their own child files, not appended to existing files.
- [ ] Edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines (`connection.rs` is ~8,260 lines).

### Present on main (#2188)
- Five pure predicates in `crates/gameplay/social-rules/src/trust_policy.rs` (`check_record_advance`, `check_checkpoint_metadata`, `check_applied_head`, `backward_step_exceeds_skew`, `check_fetch_after_step`) and `crates/gameplay/social-rules/src/release_policy.rs` (`check_signer_role`, `check_release_key_membership`), 17 unit tests; `crates/gameplay/social/src/security/mod.rs` holds only the scope note. Nothing calls them.
- Excluded from this card: signature verification, trust store/fetch/clock latch (SEC-TRUST-1), signed artifact schema (SEC-REL-1), challenge/telemetry/detection children, all runtime wiring.

### Checklist
- [ ] **Caller first**: add a non-test caller before relying on the predicates in production (strict Clippy `dead_code`, D1427).
- [ ] **SEC-TRUST-1**: trust store (migration — next free number at allocation time, only one migration PR in the queue — + durable checkpoint/head record), signed-bytes verification (crypto crate decision), fetch and clock latch using `check_fetch_after_step` / `backward_step_exceeds_skew`, restore bootstrap using `check_applied_head`.
- [ ] **SEC-REL-1**: signed release manifest schema, key-id rules, decoder limits; enforce `check_signer_role` and `check_release_key_membership` on every verified object.
- [ ] **Admission hookup**: evaluate client release/trust in fresh admission and V2 recovery (`crates/gameplay/admission-evidence/src/lib.rs`, admission path in `apps/game-server/src/node/serve.rs`, ADMIT-CAP-1 merged #2187) and capability negotiation in `capabilities.rs`; refuse or flag untrusted builds with a typed error code.
- [ ] **Protocol**: client-integrity capability and challenge/response or attestation messages in `crates/protocol-oteryn/src/` (new module, registry entries); client side in `apps/client`.
- [ ] **Detection**: SEC-CHAL / SEC-TELEM / SEC-DETECT children (movement/step integrity in `gameplay_transport/actor_movement.rs` and `input_gate.rs`, telemetry per DATA-PRIVACY-01), GM-1 staff audit linkage; mode starts REPORT_ONLY.
- [ ] **Config**: node config switches (`apps/game-server/src/node/config.rs`) for report-only vs enforce, trust root provisioning, operator docs.
- [ ] **Tests**: keep the 17 unit tests; add durable trust-store restart tests; **end-to-end through the real session handshake/`connection.rs` dispatch**: valid build admitted, revoked/unsigned/wrong-role build refused or reported, clock step backward handled; no player-visible regression in report-only mode.

### Blockers and dependencies (re-verified on origin/main `59785d76`)
- ADMIT-CAP-1: MERGED (#2187).
- SEC-TRUST-1 and SEC-REL-1 owners not allocated (D1326); RELEASE-MANIFEST-3; DATA-PRIVACY-01. Each child needs CP allocation before code starts.
- GM-1: re-implementation pending (draft #2181 closing; card `docs/agents/tasks/GM-1.md`).
