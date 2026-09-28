---
task_id: OTV2-20260928-char-progression-init
title: Initialize Character progression at the D84 start through the owning fenced route
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 8e2e474a
branch: agent/char-progression-init-20260928
issue: 162
jira: KAN-32
allocation_comment: 5875188437
owned_paths:
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/tests/support/character_progression_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260928-char-progression-init.md
---

# Initialize Character progression at the D84 start through the owning fenced route

This is the Character readiness child of the Character lane (#162 5870293835). It clears the last Combat D prerequisite named in VSL-COMBAT-01 §24.1 and in the Combat D readiness note 5873043638. The claim is #162 5875317401.

## Defect

Migration `0009` lets a bootstrap-only Character exist at revision 1 without typed progression, and the R7 P03 XP writer fails closed with `MissingProgressionState` for such a Character. The only code that ever created progression was test support, so no admitted Character could receive XP.

## Outcome

- **Initializer.** `DurabilityRoot::initialize_character_progression` creates the owner-decided D84 state for a Character that has none: level 1, `total_experience` 0 (`INITIAL_LEVEL`, `INITIAL_TOTAL_EXPERIENCE`).
- **Revision identity.** The state is created under a caller-supplied revision identity, `ProgressionInitialization`: the six context revisions plus the policy and reward revisions. This is the identity its later XP awards must carry. The profile, ruleset and content revisions must equal the Character root's.
- **Fence.** It runs through the same FND-04 gameplay fence as the XP writer: recovery fence, admission relations, current session, Character lease, scope assignment, node incarnation, admission guards, and the root locked at the expected revision under the current interpretation. That fence is now one shared helper, `fence_character_root`, used by both writers with unchanged behaviour.
- **Idempotent.** An existing state under the same identity is returned as `AlreadyInitialized`, exactly as it is now, never reset or regressed. A different identity is refused with `ProgressionContextMismatch`. Absence is never read as zero, and a revision above 1 without state is treated as corrupt.
- **Not a Character revision.** Initialization does not advance `CharacterRevision`. `0009` permits typed state at the bootstrap revision without a receipt.

## Excluded

- No XP formula or level curve, and no client, protocol, schema or migration change.
- The production call site, and the policy identity it binds, are not in this task. The identity is the one the first XP-awarding path (Combat D) will carry, so that allocation, which owns the policy binding, calls the initializer before its first settlement. Wiring it into admission would also need a SHARED_LEASE on the admission composition.

## Validation

- `tests/support/character_progression_postgres_cases.rs` covers both properties on PostgreSQL 17.6:
  - creation once, exact replay, a first award from level 1 to level 2 after initialization, no regression after progress, and refusal of a different identity;
  - refusal on a stale connection generation, lease or scope, a stale revision, foreign content, malformed input and an ended session, with no row written.
- A unit test pins D84 and the input validation.
- `cargo test -p oteryn-game-server --all-features` against PostgreSQL passes. Clippy `-D warnings` and `cargo fmt --check` are clean.
