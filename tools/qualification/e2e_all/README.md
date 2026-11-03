# E2E-ALL-1 (incomplete)

Run from the Game worktree:

```sh
bash tools/qualification/e2e_all/run.sh
```

This reuses `node_boot/run.sh` unchanged, including its prerequisite pinned
Platform checkout, privileged disposable topology, PostgreSQL version and real
world compilation. It also retains node_boot's fixture pass. It is not a way
to run against a production server. See that runner for its host prerequisites.
The absolute cargo wrapper is used for the client tests; node_boot owns the
cargo commands and PostgreSQL environment inside its topology.

Evidence stays in a private `.e2e-all.*` directory inside this worktree. The
quest capture contains raw domain payloads and revision metadata, no admission
grant, database URL or private key. The client replay test must read that run's
captures; missing captures fail. Its ordinary unit mode uses an explicit
synthetic fixture and does not claim node qualification.

The summary has every DONE/PARTIAL mechanic from `SCENARIO_MATRIX.md`, plus
bounded scenario results. A full mechanic requires its own `<row>.full`
evidence. The current suite implements no full mechanic row, so the command
**exits nonzero**, even when its existing journeys and new slices succeed.
Legacy `skipped=` messages are not interpreted as missing content or a pass.
Unimplemented scenarios are FAIL, not MISSING-SERVER. SKIPPED and MISSING-SERVER
require explicit content/source evidence and also leave the suite incomplete.

Unfinished: full server caller tracing and concrete content routes for the
UNTRACED/UNRESOLVED rows; floor/tool/item/combat/NPC/social/progression/death
scenarios for the complete row requirements; captured client tests beyond the
quest journal; full node execution (deliberately not run during authoring).
