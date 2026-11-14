# OTV2-20261113-charms-leech-evidence

```yaml
task_id: OTV2-20261113-charms-leech-evidence
title: All-25 charm effect audit and verified critical/leech corrections
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/charms-leech-evidence-20261113
pr: 3183
base_sha: ce45063507c22a11e0db7e9c0ff86dacbe6ff834
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone task writer
created_at: 2026-11-13T20:38:46Z
updated_at: 2026-11-14T00:25:15Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/combat-rules/src/resolution/equipped_offence.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell_commit.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell_commit_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/charm_combat.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/equipped_offence.rs
  - crates/gameplay/dispatch/src/gameplay_transport/bounty_talisman.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_spell_augments.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast/prepare.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_companion_item_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_world_item_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat.rs
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/prepare.rs
  - crates/gameplay/dispatch/src/gameplay_transport/spell_leech_projection.rs
  - crates/gameplay/dispatch/src/lifted_tests/spell_magnitude_owner_tests.rs
  - crates/gameplay/progression/src/charm_effects.rs
  - crates/gameplay/progression/src/charm_effects/tests.rs
  - crates/gameplay/spell/src/spell/magnitude_owner.rs
  - crates/gameplay/spell/src/spell/native_combat.rs
  - docs/agents/evidence/OTV2-20261113-charms-leech-implementation.md
  - docs/agents/tasks/OTV2-20261113-charms-leech-evidence.md
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/parameter_spell.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_contracts.rs
  - docs/agents/evidence/OTV2-20261113-all-charms-audit.md
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/delayed_strikes.rs
  - crates/gameplay/combat/src/ability/player_lethal.rs
  - crates/gameplay/combat/src/creature_auto_attack.rs
  - crates/gameplay/spell/src/creature_damage_spell/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/charm_combat/source_vitals.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_incoming.rs
  - docs/agents/evidence/OTV2-20261114-charms-runtime-completion.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Correct direct-hit and direct-area Leech, preserving the actual physical/player commit and
replay boundaries; wire Low Blow and Savage Blow into qualified native/ordinary spell damage.
This is a bounded slice of the owner's six-mechanic request, not six completed mechanics.
Coordination: #1622, allocation `CHARMS-LEECH-EVIDENCE-20261113`.

## Architecture and source of truth

- `PROVEN`: existing Game runtime owns damage, player vitals, active creature policy and receipts.
- `DERIVED`: selected numerical Global model and its source-specific limits are listed in
  [implementation evidence](../evidence/OTV2-20261113-charms-leech-implementation.md).
- Reference research: #3177 at `97685d43a194010559a3a92821089067dbb147a3`.
- Existing external target date remains 2026-09-27. No migration/content/wire change.
- Hex/Cleanse condition changes already owned by #3176 are excluded from this branch.

## High-risk authority/recovery qualification

Applicable: numerical gains enter existing fenced player-vitals replacements.

- Authority: current controlled `ExactActorRef`, `GameSessionId`, attacker Character/lease,
  qualified creature incarnation and existing batch/player revisions; none is replaced by a
  string, history report, source filename, branch name or client-supplied damage.
- PREPARE/COMMIT: qualified numerical projection is retained by the prepared cast; receipt
  replay cannot select a fresh gain. The original joined physical/player install remains the
  only mutation boundary, under held owners. No added external side effect follows commit.
- Reconciliation: persisted binding format and deferred parent binding stay unchanged; source
  JSON is not rewritten. Existing stale-state rejection, receipt identity and replay rules apply.
- Negative tests: stale player revision aborts both damage and gain; duplicate/unapplied receipt
  grants no additional HP/MP; wrong-race charm does not alter the other target's rate.
- Live-control changes or ledger promotion: not authorised by this task. No deployment or merge.

## Validation and current gate

The original published candidate is `a997547565218b9204752a73915f52b8537c5aa7`.
AUTHORING is open for the expanded audit and three reproduced fixes. The new
all-25 contract/owner tests passed (5 tests; all 75 catalogue stages and 512
occurrences per stage), and the focused charm suites passed (104 tests).
Final affected-package tests passed: 4130 passed, 0 failed, 16 pre-existing ignored.
Strict Clippy, workspace/mounted-file formatting and diff checks passed.
Exact successor SHA/results are recorded in the PR, not self-guessed here.

Independent Codex round one: concurrency/persistence/protocol CLEAN; two P1
findings reproduced and repaired (innate critical gating and Enrage Leech).
The live-assignment regression also addresses the associated projection-coverage
blind spot. A material successor still requires its applicable delta review.
No duplicate review of unchanged `a997547565218b9204752a73915f52b8537c5aa7` is requested.

Real PostgreSQL run `43508976394` on that predecessor: 22 partitions passed,
2 failed. Relevant executed charm-consumer results and failure details are in the
[all-charms audit](../evidence/OTV2-20261113-all-charms-audit.md).
The exact Outburst expectation is aligned with the accepted ceiling rule; the
501-item death-reset `Unavailable(CommitRejected)` cause remains unverified.
Route-less local PostgreSQL returns are never counted as database qualification.

## Remaining work

Complete successor publication/qualification without declaring full charm parity.
The audit identifies outgoing player-spell hook gaps for 12 charms, an incomplete
Dodge native-source attack boundary, unresolved blocked-contact/Parry edge cases,
and unqualified delayed/secondary-target cases. Cleanse Hex stays with #3176.
Required before merge: own-head game-gate, all selected real PostgreSQL partitions,
independent delta review and no open material finding. No merge/deployment here.

## Owner expansion: all charms

AUTHORING resumed from a997547565218b9204752a73915f52b8537c5aa7 on the explicit request to check all 25 charms and their effects. Exact source rules, catalogue stages, current character/race assignments, real consumer reachability and the review P1 findings are checked separately. No full-parity or PostgreSQL claim follows from pure evaluation tests.

## Local checks on final authoring tree

- `cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib all25_`: pass (5 tests).
- Affected four-library test command retained in the audit: pass (4130 / 0 failed).
- Affected four-library strict Clippy command retained in the audit: pass.
- `cargo fmt --all --check`: pass.
- `rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_contracts.rs`: pass.
- `git diff --check`: pass.
- `python3 tools/agents/validate_governance.py`: pass.
- `python3 -m unittest discover -s tools/agents/tests`: OK (129 tests).

## Complete-runtime continuation: native Dodge increment

Owner expanded the task to finish all charm repairs and publish useful checked increments.
AUTHORING resumed in #3183 comment 6867162021. The accepted incoming runtime contract is
`CHARM4-INCOMING-EFFECT-ORDER-V1`; its historical live-Global evidence caveats are not
permission to leave the accepted runtime cancellation path unwired.

This increment wires Dodge into the native creature spell/condition/Mana Drain owner and
the native auto-attack owner, before reflection, block consumption and attached effects.
The outer source owner retains the actual cancellation in its ordinary occurrence result.
Native auto damage also preserves its source for existing defensive charm consumers.
Actual normal monster melee via `PlayerBiteVitals` is a separate existing route: it is not
mislabelled as newly qualified by the AutoAttackOwner regression.

The all-targets compilation check passed on the VPS. The actual Dodge failure was reproduced
on the owner's PC before that device went offline. Full VPS dispatch-test code generation
exceeded the safe process/memory budget before any tests ran; it is neither a test PASS nor
a test failure. New runtime tests are mounted in the normal registered dispatch target and
remain mandatory in CI. Exact commands/results and any resource limitation are retained in
`docs/agents/evidence/OTV2-20261114-charms-runtime-completion.md`.

The outgoing G1 helper's offline PC workspace is not recovered or declared successful.
G1, ordered Parry/source ticks, secondary/delayed boundaries and the separately owned
Cleanse/Hex #3176 composition remain active work. No completed global charm claim.

Canonical publication remains single-writer. The VPS lacks an authenticated normal Git
publisher; the explicitly selected fallback in #3183 comment 6867932865 is a new API-native
candidate over the freshly read canonical predecessor, with one complete bounded tree,
one sole-parent successor commit, one non-force task-branch update and immediate readback.
The offline unpushed PC merge is not relabelled as that candidate. No inherited candidate
review/CI proof, protected-main write, per-file Contents commits, force/reset or deployment.

### Native-Dodge increment local gate

All-targets check, strict Clippy, both format checks and diff validation passed.
Combat/progression/spell library tests passed: 551 / 0 failures / 0 ignored.
Governance validator and 129 governance tests passed. New full-dispatch native
Dodge tests were compiled, not executed on the memory-constrained VPS; their
actual CI execution is a required open gate. No final completion/merge claim.
