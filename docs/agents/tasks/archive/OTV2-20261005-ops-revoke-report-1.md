# OTV2-20261005-ops-revoke-report-1

```yaml
task_id: OTV2-20261005-ops-revoke-report-1
title: "OPS-REVOKE-REPORT-1: report scope revocation to Platform"
mode: HIGH_RISK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/ops-revoke-report-1-20261005
issue: 1622
pr: 1856
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: claude-code-session_01LgLLAPRJyQ135ma5tWmqio (hard worker, control plane #1622)
created_at: 2026-10-05
updated_at: 2026-10-06
owned_paths:
  - apps/game-server/src/native_admission_source/scope_assignment.rs
  - apps/game-server/src/native_admission_source/descriptor.rs
  - apps/game-server/src/native_admission_source/mod.rs
  - apps/game-server/src/native_admission_source/http1_mtls.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/tests/native_scope_assignment.rs
  - apps/game-server/tests/native_admission_source_transport.rs
  - docs/agents/tasks/archive/OTV2-20261005-ops-revoke-report-1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md
depends_on:
  - 1822
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet §2.1 of `docs/architecture/reviews/OTERYN_GAME_ARCH_REVOKE_REPORT_2026-10-05.md`, contract §16.1 (RS-A1):

- `ReportScopeRevocationV1` adds the following:
  - an exact encoder equal to the §16.1 fixture, with no identity member;
  - a byte-exact success decoder;
  - the compiled path `/internal/v1/game-auth/native-scope-revocations`;
  - the 256-byte response bound for every status.
- A status outside §4's list (such as `404`) is a definite `unexpected_status`: no retry. Assignment status mapping is unchanged.
- `oteryn-game-ops`:
  - `assignment revoke` and `assignment reconcile` of a committed revoke report the revocation after commit;
  - `assignment report` on a revoked scope re-sends it from the durable row (`ownership_generation`, `decided_at`) and the declared epoch;
  - `--node-identity` stays refused;
  - every failure exits non-zero and names `assignment report`;
  - neither the generation nor the epoch is logged.
- The §13 negative and value fixtures are committed in `tests/native_scope_assignment.rs` beside the valid fixture, and a test shows the encoder cannot emit any of them.
- There is one response-decoder test per §13 shape.
- An ordering loopback Platform stub covers:
  - a revocation at G after an assignment at G-1;
  - `superseded` for a lower generation;
  - `409` for a same-key assignment or revocation, and for a different `revoked_at`;
  - an identical replay answering `accepted` with no state change;
  - a `404` stop.
- §16.2 (OPS-KEY-SEPARATION-2) is untouched. `mod.rs` needed no change.

## Validation

cargo fmt --all -- --check: pass
cargo clippy --locked --workspace --all-targets -- -D warnings: pass
cargo test --locked -p oteryn-game-server: pass
cargo run --locked -p oteryn-architecture-check -- workspace .: pass
python tools/repository/validate_repository_policy.py: pass
git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Review is decided by the control plane on the frozen head. Merge result: squash merge of #1856, pending CI and Merge Queue at authoring.
