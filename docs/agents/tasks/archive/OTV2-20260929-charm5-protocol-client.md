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
  - "CHARM-3 Charm state (#1307, merged) implements the port's Charm side"
  - "Owner decisions D168, D169, D170 (docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D165_D173_2026-09-30.md)"
blocks:
  - "CHARM-5 composition: ID registration, session wiring, connection dispatch (off until CHARM-6 ships, D170)"
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
  - No unassign or reset command: CHARM-6 proposes its own (D170).
  - A test checks that the proposal names every implemented ID, message and byte bound.
- **Server adapter** `apps/game-server/src/gameplay_transport/charm.rs`:
  - the narrow, asynchronous `CharmProgressionPort` trait, which CHARM-2 and CHARM-3 implement; its commands take the
    CHARM-3 `CharmCommandOccurrence` derived from the FND-02 `(GameSessionId, CommandId)`;
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

Repair round 1 (2026-09-30, after merging `origin/main` at `54c9ca18`):

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-client -p oteryn-game-server
  --all-targets -- -D warnings`: pass.
- `cargo test --locked`:
  - `oteryn-protocol-oteryn`: 78 passed;
  - `oteryn-game-server` (all targets): 0 failed; `--lib` 1114 passed, 2 ignored, 8 of them CHARM-5 adapter tests;
  - `oteryn-client`: 27 passed;
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
- Round 1 on `a7ceccaa` (PR comment 5907133076): FIX. Resolved in one repair push:
  1. MATERIAL, D169 slots: added `CHARM_ASSIGN_DISPOSITION_ASSIGNMENT_SLOTS_FULL = 9` (codec, decoder, adapter,
     client precondition and feedback, tests) and `CharmViewV1.assignment_slot_limit` (0 = no limit, at most 32).
     The slots in use are derived from the assigned charms, not sent. View bound 488 -> 490 bytes.
  2. MATERIAL, CHARM-3 port: the port is asynchronous and its commands take `(occurrence, intent)`. The occurrence
     is derived from the FND-02 `(GameSessionId, CommandId)` as the spell-cast occurrence is, in UUIDv7 form, with an
     independent known-answer test. CHARM-3 does not check an expected stage under the root lock, so the proposal
     drops the "never pays for stage 3" claim; `expected_stage` stays as the port's pre-commit check.
  3. MATERIAL, D170: answer 3c is no longer called final. CHARM-6 adds its own unassign and reset types. §8 step 3
     keeps composition off until CHARM-6 ships.
  4. EVIDENCE_GAP, D168: §2 recommends capability-gated (reviewer and control plane), consistent with PROFICIENCY
     and doubling as the D170 release gate; the protocol owner decides.
  5. EVIDENCE_GAP, revisions: domains 4 and 5 use the CharacterRevision as `revision`, `base_revision` and
     `new_revision`, with the resume, transfer and rollback behaviour in §4.
  6. HARDENING: removed the registry check that soft-reserved types and domains 4 and 5; it moves into the
     acceptance PR (§8 step 1).
  7. OUT_OF_SCOPE: §2 states that the content generation is fixed per connection.
- Round 2 on `53d10a61` (PR comment 5910583237): FIX, proposal text only. The Sol ruling (#162 comment
  5907282001) already accepted the IDs, so the proposal adopts it:
  1. MEDIUM: the Status line and §2 record capability 1 `BESTIARY_CHARMS_V1`, command types 4 and 5, domains 4 and
     5, and capability gating. Without the capability, commands 4 and 5 are refused as unsupported.
  2. MEDIUM: §4 revisions follow the ruling:
     - every admission, reconnect, resume and transfer starts with a full snapshot;
     - `base_revision` is the last revision received on this connection;
     - a commit that does not change a domain sends no delta for it.
  3. LOW: §3 records the charms bound of 490 bytes. Sol must acknowledge it on #162 before the registry PR.

## Closeout

- Merge commit/result: squash merge of the CHARM-5 PR (resolve with `git log --grep`).
- Ownership release: on merge.
