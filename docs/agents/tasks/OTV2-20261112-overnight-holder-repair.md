# OTV2-20261112-overnight-holder-repair

```yaml
task_id: OTV2-20261112-overnight-holder-repair
title: Repair shared PostgreSQL holder contention blocking PR integration
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/overnight-main-repair-20261112
pr: null
base_sha: 3a50a9e6f6e3e817d28dd3bd5bf17449d0c18e4a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex overnight integration lead
created_at: 2026-11-12T00:00:00Z
updated_at: 2026-11-12T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/durability/src/durability/cast_privacy.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/autowalk.rs
  - apps/game-server/tests/support/authority_cases/s17_cast_privacy_holder_retry.rs
  - docs/agents/tasks/OTV2-20261112-overnight-holder-repair.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Retain existing privacy leases and command budgets while retrying only a busy
durability holder. Qualify the shared failures before integrating waiting PRs.

## Architecture and source of truth

- Accepted contract: `docs/contracts/S17_CAST_CONSENT_PERSISTENCE.md`.
- PROVEN: PR #3121 head `376051cb`, run `43110659171`, job `130832404681`
  fails independent autowalk pickup with `Unavailable(RootUnavailable)`.
- PROVEN: PR #3112 head `3e64083b`, run `43050986251`, job `130632381714`
  fails S17b with `CLCAST speech: session is unusable`. The coordinator also
  reproduced S17b on main `eddafe59` three times.
- DERIVED: refreshing or acknowledging consent without retaining a holder
  ticket can miss the one-second media lease or terminate the cast worker on
  a transient ACK refusal. The new deterministic regression requires refresh
  and ACK to remain pending while another owner holds the pass.
- Allocation: #3124 comments `6810070668` and `6810079940`. No writes to
  the coordinator's #3121/#3112 branches or their diagnostics paths.

## High-risk authority/recovery qualification

Existing worker instance, cursor, current node and registration proofs remain
checked inside each transaction. The retry changes acquisition only. It never
retries a transaction error, unknown commit, invalid batch or authority refusal.

Temporal invariant: retry acquisition ends at the original local worker
deadline; both one-second local and two-second database leases are unchanged.
The pre-existing immediate recovery attempt for an already expired worker is
preserved. The service still purges media on expiry and gates final writes on
the current local deadline.

Consumer boundaries: refresh and ACK. Operators covered by the new case:
busy holder with eventual release; busy holder through original expiry.
Existing cross-worker withdrawal tests cover independent worker cursors and
the invalidation barrier; they remain required and unchanged.

Finding-family sweep: refresh and ACK share the same bounded acquisition;
autowalk freeze and commit share one existing command budget and retain the
same request/candidate. Concurrent fixture futures have independent owner keys.
No schema, protocol, persisted identity, registration, consent semantics or
withdrawal proof is replaced.

## Acceptance criteria

- [ ] New deterministic privacy holder case and existing cross-worker privacy cases pass.
- [ ] S17b and autowalk real-session cases pass on the candidate.
- [ ] Required native/Clippy and all PostgreSQL partitions pass on the exact head.
- [ ] Independent Codex review has no unresolved confirmed P0/P1.
- [ ] Protected squash merge and post-merge main qualification succeed.

## Reference parity

NOT_APPLICABLE: repair of holder scheduling in existing privacy and qualification
paths; no new mechanic or game rule.

## Excluded scope

No workflow, ruleset/protection, timeout, test assertion, migration or Node
qualification changes. Node trade-open failure remains separately investigated;
no cause or fix for it is claimed by this batch.

## Implementation / findings

- Refresh/ACK acquisition retains a FIFO waiter until the existing lease deadline.
- Autowalk retries only exact RootUnavailable with unchanged CommandId 901.
- New privacy regression polls the actual durable methods while holding a pass,
  releases that pass explicitly, and verifies expiry does not renew authority.
- Local PostgreSQL 18.6 startup/connection and source formatting verified.
- Focused offline durability test could not resolve missing cached `schemars`;
  compilation and database execution are not claimed. Heavy validation belongs
  to GitHub runners under the owner's task instruction.
