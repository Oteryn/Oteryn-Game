# OTV2-20260929-charm5-protocol-client

```yaml
task_id: OTV2-20260929-charm5-protocol-client
title: CHARM-5 Bestiary and Charm wire proposal, codecs, server adapter and client views
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/charm5-protocol-client
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 4ea220fa15f5a30535eb2b7eb3df7e7ada2b543b
head_sha: null   # a commit cannot hold its own SHA; the exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "hard worker (Claude Code)"
control_plane: session_012nzPTz29NThWJG45F2m5fP
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/bestiary.rs
  - crates/protocol-oteryn/src/charm.rs
  - crates/protocol-oteryn/src/charm_wire.rs
  - crates/protocol-oteryn/src/lib.rs   # module registration lines only
  - apps/game-server/src/gameplay_transport/charm.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # module registration line only
  - apps/client/src/cyclopedia.rs
  - apps/client/src/lib.rs   # module registration line only
  - crates/session/src/lib.rs   # one re-export statement (see Outcome); lead to confirm
  - docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md
  - docs/agents/tasks/archive/OTV2-20260929-charm5-protocol-client.md
public_contracts:
  - docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md   # PROPOSAL, not accepted
depends_on:
  - "#1295 CHARM-0 decision packet (slice CHARM-5, owner answers §7)"
  - "CHARM-2 Bestiary progress (claude/charm2-bestiary-progress) implements the port's Bestiary side"
  - "CHARM-3 Charm state (claude/charm3-charm-state) implements the port's Charm side"
blocks:
  - "CHARM-5 composition: ID registration, session wiring, connection dispatch"
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

- **Protocol proposal** `docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md`:
  - proposed command types 4 `CHARM_UNLOCK_STAGE_INTENT` and 5 `CHARM_ASSIGN_INTENT`;
  - proposed state domains 4 `CHARACTER_BESTIARY` and 5 `CHARACTER_CHARMS`;
  - proposed limits `CHARM5-RL-01` to `CHARM5-RL-05`;
  - the proto3 schema, and the owner answers 8a, 9a, 10a and 11a (§7, 2026-09-30).
  - No registry, proto file or resource registry was changed.
- **Codecs** `crates/protocol-oteryn/src/{bestiary,charm,charm_wire}.rs`: strict encode and decode for the Bestiary
  view, the Charm view, both intents and both results.
  - The Bestiary stage is derived, not sent.
  - No unassign command exists (CHARM-0 §7 answer 3c).
  - A test fails if another writer registers a proposed ID under another name.
- **Server adapter** `apps/game-server/src/gameplay_transport/charm.rs`:
  - the narrow `CharmProgressionPort` trait, which CHARM-2 and CHARM-3 implement;
  - decode, port, encode, with test doubles;
  - not dispatched from `connection.rs` until the IDs are registered.
- **Client** `apps/client/src/cyclopedia.rs`: pure Bestiary and Charm state, with:
  - snapshot and delta application;
  - rows with the derived stage, progress and what can be done;
  - local preconditions that return the disposition the server would return;
  - one feedback line per disposition.
- **Scope note:** the client may not import `protocol-oteryn`, so one `pub use` statement in
  `crates/session/src/lib.rs` re-exports the CHARM-5 types. It changes no behaviour. The same precedent is in
  `OTV2-20260929-spell-client-ui-w3b`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: no production mutation, fence, durable write or recovery path. The adapter only decodes and encodes
around a trait. The fenced Character transactions belong to CHARM-2 and CHARM-3.

## Excluded scope

- Content, durability, migrations and combat.
- CHARM-2 and CHARM-3 files.
- The registry, proto and resource-registry entries, which wait for protocol-owner assignment.
- `connection.rs` dispatch and session command and domain routing, which wait for registration.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-client -p oteryn-game-server
  --all-targets -- -D warnings`: pass.
- `cargo test --locked`:
  - `oteryn-protocol-oteryn`: 78 passed, 14 of them new;
  - `oteryn-game-server --lib`: 1028 passed, 2 ignored, 7 of them new;
  - `oteryn-client`: 26 passed, 5 of them new;
  - `oteryn-session`: 3 passed.
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py` and
  `git diff --check`: pass.
- The shared target dir `/root/.cache/oteryn-target` gave stale protocol artifacts from a concurrent worktree
  build, so the checks ran with a private `CARGO_TARGET_DIR`.

## Self-review

- Method: adversarial whole-diff self-review by the implementing agent.
- Every invariant has one negative case in each direction.
- The worst-case byte bounds are proven by hand-computed fixtures.
- Malformed intents never reach the port.
- Verdict: no open material finding.

## Independent review

- Required: YES. It is a `protocol-oteryn` wire proposal (AGENTS.md; CHARM-0 §3, slice CHARM-5).
- The owner or protocol owner must accept the proposal before registration.

## Closeout

- Merge commit/result: squash merge of the CHARM-5 PR (resolve with `git log --grep`).
- Ownership release: on merge.
