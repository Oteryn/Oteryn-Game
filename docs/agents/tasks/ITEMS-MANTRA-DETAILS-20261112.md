# ITEMS-MANTRA-DETAILS-20261112

```yaml
task_id: ITEMS-MANTRA-DETAILS-20261112
title: Preserve literal Mantra values in Item Details
mode: REPAIR
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-mantra-details-20261112
pr: 3154
base_sha: 5efd67da164ac9dc33c8c066434b228cebb86c2b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/mantra_resume
created_at: 2026-11-12T22:57:37Z
updated_at: 2026-11-13T02:26:01Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/inspection.rs
  - docs/agents/tasks/ITEMS-MANTRA-DETAILS-20261112.md
  - docs/agents/tasks/archive/ITEMS-MANTRA-DETAILS-20261112.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PARTIAL candidate: Item Details projects Known SignedPoints Mantra as its literal
signed decimal value. Independent review, exact-head CI and remaining validation
are pending. This record remains active; the overall static Cyclopedia audit is
not complete.

## Architecture and source of truth

Owner allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6838262307.
The owner-authorized Cloud coordinator transferred this lane to one writer and
explicitly extended the task-record scope to this active location while validation
is pending. The immutable local handoff patch has SHA256
8837e3fae3029857e5148469e08af3e39035a764ca10b56fcba8f1977b652a29.
It was applied exactly; reverse-apply verification confirms the retained delta.

The existing selected materializable profiles contain harmony amulet2, merudri
scale mail13, robe of enlightenment15 and coned hat of enlightenment4. These are
existing native profile references, not newly inferred appearance/server IDs.
The real details function and existing attribute codec/client consumer are the
qualified projection path. No deployed-server qualification is asserted.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this changes a read-only display projection. The attribute codec,
source semantics, runtime eligibility, persistence and equipment effects retain
their existing owners. Independent review remains REVIEW_PENDING.

## Acceptance criteria

- [x] Reproduce the omission using real current profile records and independent2/13/15/4 expectations.
- [x] Add only the literal Mantra attribute for Known SignedPoints.
- [x] Preserve known zero and negative values; omit Unknown, NotApplicable and invalid parameter kinds.
- [x] Cover shared instance projection, codec round trip and unrelated attributes/source preservation.
- [ ] Complete normal unit qualification after the executor memory failure.
- [x] Publish the bounded PARTIAL draft; final head is reported by remote readback.
- [ ] Obtain coordinator-routed independent review and exact-head CI.

## Reference parity

The decimal presentation retains the existing source value without a synthetic
plus sign. Unknown and N/A remain distinct source states; neither becomes zero.
This is one projection omission, not qualification of every Item or Cyclopedia
field. Death oyoroi canonical15/UI16 and goo shell canonical48/selected58 remain
separate reference conflicts and are not changed here.

## Excluded scope

No content, pins, assets, schemas, protocol fields, balance/equipment logic,
Cargo, policy, workflow, GUI, NAS, production, merge or deployment changes.
The separate spell and evidence branches remain untouched.

## Implementation / findings

The modifier projection previously fell through its match for Kind::Mantra.
Six production lines now append the existing Mantra label and SignedPoints
literal before continuing;138 regression lines exercise the actual projection.
The two new regressions failed red before the producer fix and passed afterward.

Fresh main5efd67da was incorporated by ordinary fast-forward merge after the
Cloud handoff base83be72ad. The exact checkpoint patch remains unchanged.

## Validation

Retained Cloud evidence on the allocated83be72ad tree plus the exact patch:

- RED: focused inspection filter,6 passed/2 failed, exit101; both new omission regressions failed.
- GREEN: inspection module,4 passed/0 failed, exit0; retained green.log.
- cargo fmt --all --check: pass on that retained tree.
- cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings: pass on that retained tree.
- Full PG-enabled dispatch attempt: INTERRUPTED, no terminal summary or reliable command-exit receipt; not a pass.

The interrupted PG log contains failure markers for annihilator route earned
reward/reentry and genuine party entry, plus condition potions transcendence and
bullseye expiry. No detailed terminal error output was retained. Their relation
to this change is not assumed. Unchanged83be72ad baseline results use the same
prepared PostgreSQL18.6 route (observed server_version_num180006):

- cargo test --locked -p oteryn-gameplay-dispatch --all-features --quiet annihilator_source_route_: exit101;1 passed/2 failed in133.48s. The earned-reward/reentry and genuine-party-entry cases both returned Error: "Unavailable" without the Mantra patch.
- condition_potions_postgres_transcendence_buff_expiry_receipt_and_refusals: exit0;1 passed in isolation.
- condition_potions_postgres_bullseye_buff_expiry_receipt_and_refusals: exit0;1 passed in isolation.

Only the same Annihilator test names failing independently on baseline are
proved; the interrupted candidate run did not retain its detailed error cause.
The potion failures were not reproduced by the individual baseline runs; their
original cause remains unknown. No test assertion, PG-route requirement, timeout
or budget is weakened.

A first5efd67da Clippy attempt was interrupted with no terminal receipt after a
code-mode host interruption. Its log is retained; it is not counted as a pass.
The unfinished checks resume in a detached, singleton, source/head-hash-fenced
runner with explicit command exit receipts. Completed baseline checks are not
repeated.

Fresh5efd67da candidate evidence:

- cargo fmt --all --check: pass, exit0.
- python tools/agents/validate_governance.py: pass.
- python -m unittest discover -s tools/agents/tests: pass;129 tests.
- cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings: pass; exit0, source-hash fenced.
- cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib --bins --quiet: blocked during compilation; exit101, rustc SIGKILL9 before test execution.

The fast-unit subprocess was explicitly route-less, matching the separate fast
unit scope; PostgreSQL execution was NOT claimed. Its test binary did not finish
compilation, so no unit-pass count exists. The cgroup memory limit was16 GiB,
peak reached that limit, and memory.events reported one OOM kill. No flags,
assertions, PG requirements, timeouts, incremental settings or caches were changed
to turn this attempt green. The detached runner retained explicit command exit
receipts and the unchanged inspection source hash
bb0e75881d3d468cc78e67f59ef1d1e1fb4c28e255230b6d6cc1ff017e0e7917.

One coordinator-authorized infrastructure retry ran the identical fast unit
command/profile/cache with only CARGO_BUILD_JOBS=1. It was bound to committed
head159412f4eced4c8430c6019629f59949f5ada2d0 and the unchanged inspection hash.
The retry also exited101 before tests, with rustc SIGKILL9; the cgroup OOM-kill
counter increased from1 to2. No further local retries are authorized or planned.
The final task-record commit changes metadata only, after the retry terminated.

FAST_UNIT_UNQUALIFIED_OOM: normal unit qualification remains pending CI or an
adequate executor. The completed PostgreSQL baseline checks above are separate
counterevidence, not a candidate PG pass. Fast route-less unit evidence, if run, must be identified separately
from actual PostgreSQL execution. No full PG, CI-green or mergeability claim.

## Self-review

The bounded producer and regression diff was read; source/profile values and
codec ownership remain unchanged. The coordinator's earlier bounded inspection
found no actionable defect in this projection scope. This is not an external
independent review result.

## Independent review

REVIEW_PENDING: the Codex implementation lane does not review its own code and
emits no paid review trigger. The coordinator receives the final exact-head packet
and owns the applicable review route. No ready or merge claim.

## PR and closeout

Draft PR: https://github.com/Oteryn/Oteryn-Game/pull/3154. Active record status
is waiting/PARTIAL; no completed/archive state is asserted. The source-only
repair is reviewable, while normal unit execution is unqualified after two
compilation OOMs. Exact-head CI and independent review remain pending. No merge
or deployment authority.
