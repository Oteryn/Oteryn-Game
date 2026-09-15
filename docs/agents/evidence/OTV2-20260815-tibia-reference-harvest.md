# OTV2-20260815 Tibia reference harvest evidence

Coordination: `OTER-CLIENT-REFERENCE-HARVEST-20260815`.

## Ownership decision

### PROVEN — canonical destination

`blakinio/Oteryn-v2` is the current canonical source lineage for native game/client/protocol/world tooling. `blakinio/otclient` is historical migration/reference evidence and must not receive new Oteryn v2 client work.

Platform ADR 0041 assigns the native Client, authoritative Game Server, `protocol-oteryn`, native client/server/protocol E2E mechanics, canonical World/Content and bounded OTBM migration semantics to the Oteryn-Game lineage. Therefore client/world semantic tooling harvested from the stale Platform research branch belongs here.

### PROVEN — Platform remains a separate boundary

`blakinio/Oteryn-Platform` remains the portal/Identity/GameAuth/World Registry/Game Gateway and infrastructure/control-plane repository. Bounded host/isolation/reference execution harnesses may remain there when they are Platform runner/infrastructure concerns rather than game product code.

## Source PR #1006 disposition

Source: `blakinio/Oteryn-Platform#1006`.
Source branch: `ops/oteryn-tibia-client-analysis-20260811`.
Audited source head: `97f8df9e64e1e4f0520440073e497f24dad929ef`.

### Migrated here

- `tools/tibia-worldmap-reconstruction/README.md`
- `tools/tibia-worldmap-reconstruction/run.py`
- `tools/tibia-worldmap-reconstruction/examples/synthetic-capture.json`
- `tools/tibia-worldmap-reconstruction/tests/test_core.py`
- `tools/tibia-worldmap-reconstruction/tibia_worldmap_reconstruction/__init__.py`
- `tools/tibia-worldmap-reconstruction/tibia_worldmap_reconstruction/core.py`

The package contains no proprietary Tibia asset. It normalizes already-decoded evidence, preserves `observed` vs unknown state, keeps unproven mappings explicit, compares static coordinate/stack semantics deterministically and refuses OTBM export readiness when required mappings or ground identity are unproven.

### Evidence retained as facts, not copied as executable research scaffolding

The source task recorded these bounded research results:

- official Linux client identity at the checkpoint was version `15.32.df7b29` with executable SHA-256 `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`;
- an authenticated world entry had been achieved inside the Platform-owned research environment;
- decoded Worldmap capture was demonstrated with 83 records around coordinates `x=32536..32555`, `y=32508..32521`, floors `z=6/7`;
- static native rotate/action callgraph evidence was demonstrated;
- the old viewport-derived player-position estimate was not an exact player-position decoder.

These are **reference evidence only**. They do not establish Oteryn-v2 runtime conformance or production behavior.

### Intentionally NOT migrated

- all branch-only `.github/workflows/tibia-client-*` experimental/diagnostic workflows;
- `tibia-send-private-message*.yml`;
- `tibia-vnc-access.yml`;
- login/recovery orchestration and credential injection paths;
- gdb/ptrace/live-attach workflows;
- blind batched movement experiments;
- `docs/agents/reports/OTERYN-20260812-launcher-screen.b64`;
- raw/branch-specific session diagnostics and recovery prompts;
- any proprietary client binary, asset, credential, cookie, session material or private character data.

Those files are execution history, not canonical product tooling. Their source PR remains the provenance record until Platform closeout is terminal.

## Source PR #988 disposition

Source: `blakinio/Oteryn-Platform#988`.
Source branch: `research/OTERYN-20260811-official-linux-offline-launch`.
Audited source head: `f9ff34b37cf81c400a48f7ab9329393416ac304d`.

The task is an offline official-client identity/host-isolation research boundary. Its reusable host preparation, encrypted evidence, identity hashing and CI-rejection tooling is classified as **Platform infrastructure/reference execution tooling**, not native game/client implementation, and is therefore not duplicated into Oteryn-v2 by this migration.

Client/game-relevant durable conclusions retained here are:

- exact official archive/client identity was still unknown because automated requests to the approved CipSoft static package URL returned HTTP 403 from both GitHub-hosted and project Synology egress;
- no source-evasion workaround was authorized or attempted;
- the intended future acquisition path is the normal official browser download flow on a dedicated normal Linux host;
- official execution remained unperformed; BattlEye behavior remained unknown;
- account/login/live execution remained outside that task.

## Remaining unknowns

- exact live player XYZ structure/API independent of viewport-center inference;
- semantic mapping of captured raw Worldmap fields to stable client appearance/type/content identities;
- reliable tile passability/collision classification;
- exact outbound action writer/framing chain and higher-level action ABI;
- exact OTBM-relevant coverage recoverable from received/cached official-client state;
- exact current official Linux package/client identities under the blocked offline-host task.

These unknowns require separate owner-authorized research tasks. This harvest does not authorize live-client execution, credentials, anti-cheat-sensitive instrumentation or production/runtime implementation.
