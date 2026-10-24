# CLIENT-REG-1 — reduce shared client mechanic registration edits

```yaml
task_id: CLIENT-REG-1
mode: IMPLEMENT
status: review
pr: null
repository: Oteryn/Oteryn-Game
```

## Summary

Apply the three authorized STRUCTURE-TARGET-1 changes locally: derive the session's
generic routes from protocol capability ownership, wrap session errors in the dev
client, and ignore untracked events with a trailing wildcard in the synthetic model.
Bound META policy 3.1.0 was read at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## Reason

Owner-supplied audit evidence: every client mechanic edits 7–8 shared points in
`crates/session/src/lib.rs` and neighbours; four mechanics did so on 2026-10-23
alone. The dev client's per-session-error variant, conversion and display arms
were changed by 10 commits in 30 days. The synthetic model's exhaustive ignore
arm was changed by eight commits in 30 days. Removing these duplicate ownership
and forwarding lists reduces merge conflicts between parallel mechanic lanes.

## Scope

- `crates/session/src/lib.rs`, `gated_command.rs` and
  `session_tests_capabilities.rs`: derive the same tuple shape by filtering
  `capability_tables::GATED` with the existing generic-route capability IDs;
  assert equivalence with the frozen former table in one test.
- `tools/dev-client/src/lib.rs`, `tests.rs` and the existing
  `client_tests_*.rs` assertions: replace flattened session errors with
  `DevClientError::Session(SessionError)` and delegate their display.
- `tools/synthetic-client-harness/src/live/model.rs`: retain tracked event arms,
  then ignore all remaining events with `_ => {}`.
- This task card and ignored local validation artifacts under
  `target/CLIENT-REG-1/`.

The generic-route capability list includes opt-in capabilities (1, 8, 10, 14)
that are not advertised by default. It preserves the exact former route set;
filtering by the default advertisement alone would remove existing opt-in paths
and add typed routes. Default negotiation and typed routes remain unchanged.
Session diagnostics now use the session's own Display text as requested;
adapter I/O, TLS and timeout errors retain their adapter variants and text.

## Excluded scope

Session mechanics registry step 2; new mechanics or capabilities; protocol,
identity, authority, persistence or production-client exhaustiveness changes;
`#[non_exhaustive]`; dependency/manifest changes; PostgreSQL changes; protected
CI paths; Git mutation, publication, remote review and other worktrees.

## Validation

All commands passed; full logs are under ignored `target/CLIENT-REG-1/`.
Cargo checks/tests used the absolute queued wrapper with `CARGO_NET_OFFLINE=true`.
No dependencies were downloaded and no PostgreSQL code was touched.

```sh
/home/mole/.cargo/bin/cargo fmt -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

Formatting was applied to all three packages, and the six touched `include!`
files were also formatted/checked explicitly with `rustfmt --edition 2024`:
`crates/session/src/session_tests_capabilities.rs` and dev-client's
`client_tests_{admission,snapshots,commands,command_errors,liveness}.rs`.
Cargo fmt does not traverse those includes.

- Unit tests: dev-client **34**, session **73**, synthetic-client-harness **37**;
  **144 passed**, zero failures, ignored tests or filtered tests. All doctests
  passed (zero cases). The added golden comparison proves the derived set equals
  all eight tuples of the previous table, including the opt-in capabilities.
- Exact coverage filters included in the passing full runs:
  `oteryn-session --lib tests::derived_gated_routes_match_previous_table` and
  `oteryn-session --lib tests::` (generic admission, command and delta gates);
  `oteryn-dev-client --lib tests::` (all updated nested error assertions);
  `oteryn-synthetic-client-harness -- live::tests::` (tracked event application).
  These filters are coverage selectors; the recorded validation command ran
  the complete suites without a filter. No PostgreSQL test was needed or run.
- All-target Clippy `-D warnings` passed for all three packages. The log retains
  existing vendored Tokio dependency warnings; no touched-package diagnostics.
- Governance: **22** required documents / **9** lanes validated; **129** Python
  unit tests passed. Formatting and whitespace checks passed.
- Re-read the whole diff against
  `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository review checklist:
  no findings. `assertion-equivalence.txt` verifies the existing dev-client
  test tokens are unchanged after removing only the new error wrapper and
  normalizing rustfmt whitespace/trailing commas. No assertion was weakened.
- Workspace-wide `rg` over `apps/`, `crates/`, `tools/` covered every
  `DevClientError` consumer, constructor, match and conversion. All flattened
  session-error matches were internal tests and are updated. The external
  synthetic controller only propagates the error. The new `Session` payload is
  preserved by its conversion and delegated Display. No struct field, trait
  method or public function was added; the private route iterator is used by
  both production generic routing paths. `SessionEvent` and the production
  client's exhaustive match are unchanged.

## Unfinished

None within the authorized local implementation and validation scope.
Publication and repository CI remain outside this local-only task; no PR,
frozen candidate, external review or merge is claimed.
