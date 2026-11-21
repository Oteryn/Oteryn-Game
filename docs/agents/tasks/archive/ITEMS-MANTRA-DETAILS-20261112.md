# ITEMS-MANTRA-DETAILS-20261112

```yaml
task_id: ITEMS-MANTRA-DETAILS-20261112
title: Preserve literal Mantra and known zero weight in Item Details
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-mantra
pr: 3368
base_sha: 4d36c23b2645f03e443a65d170dbc2a60c33d533
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/mantra_resume
created_at: 2026-11-12T22:57:37Z
updated_at: 2026-11-21T19:43:44Z
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

Item Details projects literal Known SignedPoints Mantra and retains Known zero
Weight. The takeover completed local qualification for both display changes.
Independent review and exact-head CI remain pending for any future integration.
The broader static Cyclopedia audit is outside this task.

## Architecture and source of truth

Owner allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6838262307.
The owner-authorized Cloud coordinator transferred this lane to one writer and
explicitly extended the task-record scope to this active location while validation
is pending. The immutable local handoff patch has SHA256
8837e3fae3029857e5148469e08af3e39035a764ca10b56fcba8f1977b652a29.
It was applied exactly. Its original six producer lines and138 regression
lines remain intact as the prior portion of this grouped repair.

The existing selected materializable profiles contain harmony amulet2, merudri
scale mail13, robe of enlightenment15 and coned hat of enlightenment4. These are
existing native profile references, not newly inferred appearance/server IDs.
The real details function and existing attribute codec/client consumer are the
qualified projection path. No deployed-server qualification is asserted.

## Authoring continuation: known zero Weight

The active control plane explicitly returned PR3154 and frozen head
9b9ac05eab879c082cbe0b9fd5ef8f7fd42118b2 to AUTHORING before this write, grouping
one newly verified defect in the same inspection module rather than opening an
overlapping PR. Owned source/task-record paths remain unchanged.

The own catalogue request22026 resolves the existing selected native key
`oteryn:item.tibia.i36809` (curl of hair), a materializable CharacterInventory/Ground
profile with physical.weight=Known(0). Request::Catalogue invokes project with
count1/metadataNone, then details previously omitted Weight because of its
nonzero guard. ReferenceItemField explicitly distinguishes Known(0) from Unknown,
NotApplicable and Conflict. The fix removes only that value-filtering guard;
Known-state checks, arithmetic and Weight/Total Weight label rules remain.

Independent retail nettle evidence in the control-plane packet reports
Weight0.00oz. It motivates preserving the state but does not bind any retail
appearance/Cyclopedia ID to the native key or own catalogue request. Source Item
values and IDs are unchanged.

New regressions use the real selected curl-of-hair profile for Known0 versus
Unknown/NotApplicable/Conflict and codec retention. Real wardrobe and sword
profiles exercise a zero container template, zero aggregate, nonzero contents
aggregate and positive stacked count, preserving Weight versus Total Weight.
The 2026-11-13 authoring run stopped before executing these tests because of the
16 GiB compiler OOM. The takeover results below supersede that blocker.
Historical authoring checks: cargo fmt --all --check pass; strict dispatch Clippy
--all-targets --all-features --quiet -- -D warnings pass (exit0, source/head
fence retained); governance validation and129 governance unit tests pass;
git diff --check pass. The new inspection source SHA256 is
d22439e9b3e366456bcb03b031e1a320322f4517bb0a9724959a5addd2444ddb.
These checks type-check the new regressions but do not execute them. Root read
and accepted the actual one-line guard removal and103-line/two-test delta before
publication. That is bounded TASK diff verification, not formal independent
review. The root authorized one grouped commit/push on the same PR and a new
freeze after these checks; no further unit retry is performed.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this changes a read-only display projection. The attribute codec,
source semantics, runtime eligibility, persistence and equipment effects retain
their existing owners. Independent review remains REVIEW_PENDING.

## Acceptance criteria

- [x] Reproduce the omission using real current profile records and independent2/13/15/4 expectations.
- [x] Add only the literal Mantra attribute for Known SignedPoints.
- [x] Preserve known zero and negative values; omit Unknown, NotApplicable and invalid parameter kinds.
- [x] Cover shared instance projection, codec round trip and unrelated attributes/source preservation.
- [x] Complete route-less inspection and full dispatch library qualification after the executor memory failure.
- [x] Original Mantra-only PARTIAL draft3154 published and frozen.
- [x] Format, type-check and verify the grouped Known-zero Weight delta with root.
- [x] Prepare the requested takeover branch with a normal merge of the fetched main.

Independent review and exact-head CI are future integration gates.

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

The takeover started from draft #3154 at
`a45faafc52e68783f2e4a9de5add4c7504b03f4d` and merged `origin/main` at
`4d36c23b2645f03e443a65d170dbc2a60c33d533` into `claude/happy-davinci-ijz3wy-mantra`.
The normal merge commit is `56918500aacb03f4bbfd3571a9aaa9b346a3f8c9`. The tested Git blob
for `inspection.rs` is `f301d9dea99110b44bd79df45e19e9e54de1d490`. The final authoring commit
records the fixture update and local verification evidence.

The decoration kit profile (`oteryn:item.tibia.i23398`) replaces the old curl
of hair fixture, whose current weight is 1.30 oz. The decoration kit is
materializable and has Known zero weight. The regression assertions are unchanged.

- `cargo fmt --all --check` pass.
- `cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets -- -D warnings` pass.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib inspection` RED (exit 101) on shard `1/3` with both producer hunks temporarily reversed to the fetched main behavior and all test code intact. FAILED. 10 passed, 4 failed, 0 ignored, 0 measured, 3023 filtered out.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib inspection` GREEN (exit 0) on shard `1/3` with both producer hunks restored. ok. 14 passed, 0 failed, 0 ignored, 0 measured, 3023 filtered out.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib` pass on all three route-less shards (each exit 0, with zero tests filtered out). 4542 unique tests passed, 0 failed, 16 already marked ignored.
- `python3 tools/agents/validate_governance.py` pass.
- `python -m unittest discover -s tools/agents/tests` pass.
- `git diff --check origin/main` pass.

| Regression | RED | GREEN |
| --- | --- | --- |
| `inspection_current_profiles_project_literal_mantra_and_preserve_other_details` | FAILED | ok |
| `inspection_mantra_keeps_zero_and_signed_points_distinct_from_unknown` | FAILED | ok |
| `inspection_retains_real_known_zero_weight_and_omits_unknown_weight` | FAILED | ok |
| `inspection_weight_labels_preserve_zero_totals_contents_and_positive_counts` | FAILED | ok |

This run resolved the prior compiler OOM blocker for the requested route-less
scope. The unsharded builds were killed at the 16 GiB limit before running tests,
both with four jobs and after cleaning dispatch artifacts and retrying with two
jobs. The successful runs used the repository's `OTERYN_DISPATCH_PG_SHARD`
split, `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, and
`CARGO_PROFILE_TEST_DEBUG=0`. The generated shard plans together cover all
724 mounted PostgreSQL test modules. The full library count deduplicates common
tests by name. The PostgreSQL route variable was unset for all Rust test runs.

### Historical validation before takeover

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

Prior Mantra-only candidate evidence (before this zero-Weight authoring delta):

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
The prior final task-record commit changed metadata only after the retry
terminated; the later explicit AUTHORING continuation above now introduces
additional production/test changes, whose execution remains unqualified.

The prior FAST_UNIT_UNQUALIFIED_OOM result is superseded for the route-less
scope by the takeover run. The historical PostgreSQL baseline results are
separate evidence. Candidate PostgreSQL and CI qualification remain pending.

## Self-review

The bounded producer and regression diff was read; source/profile values and
codec ownership remain unchanged. The coordinator's earlier bounded inspection
covered the Mantra-only candidate. Root subsequently accepted the bounded
grouped zero-Weight TASK diff. Neither statement is an external independent
review result.

## Independent review

REVIEW_PENDING: the Codex implementation lane does not review its own code and
emits no paid review trigger. The coordinator receives the final exact-head packet
and owns the applicable review route. No ready or merge claim.

## PR and closeout

Source draft: https://github.com/Oteryn/Oteryn-Game/pull/3154.
The takeover completed on `claude/happy-davinci-ijz3wy-mantra` with this
archived record in its final authoring commit. PR #3154 identifies the source
draft. The takeover branch is delivered directly for the coordinator's use.
Exact-head CI and independent review remain pending for future integration.

## PR #3368 review follow-up

The native importer used the donor engine's missing-weight default of zero.
The Crystal reward capture inherited that default for the four Rotten Blood
essences and the three appearance-only rewards. Cursed wood and darklight
geode retained earlier zero-weight native profiles when the event overlay
preserved their existing owners. The Galthen loot overlay used Canary profiles
with no XML weight for the two Mitmah items. Crystal explicitly gives those
two 250 and 100 centi-ounces.

The final profile generator now applies the verified Global Cyclopedia weights
to these eleven exact Item keys. The decoration kit retains its existing
zero-weight native profile. The projection code stayed the same. Native reward
verification accepts a correction only when the generated weight evidence
matches the exact Item ID and the corrected record matches the pinned donor
record in every other field.

The generic Item importer left the equipment modifier vectors unknown. Its
native providers omitted the qualified Global vectors for sanguine trousers
and ethereal coned hat, and the canonical legs of enlightenment record had
no vector for the Monk reward overlay to copy. The generator now replays the
Global vectors for sanguine trousers and ethereal coned hat (10 and 12), and
the Crystal vectors for soulgarb and legs of enlightenment (19 and 3). Crystal
lists the hat at 10. For soulgarb, the pinned XML's `mantra=19` takes precedence
over the Global promotion's 17 because Crystal's parser assigns the XML value
directly to the server item. [Pinned items.xml](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml)

### Follow-up validation

- `cargo test --locked -p oteryn-gameplay-dispatch --lib inspection` RED on
  `5ed80187` plus the regression tests. 14 passed and the two added tests
  failed on the eleven false zero weights and four absent Mantra values.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib inspection` GREEN with
  16 passed. The weight regression also checks that the decoration kit still
  shows 0.00 oz.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib` passed across all
  three route-less shards with 4544 unique tests passed, zero failures and
  16 pre-existing ignored tests. The test runs used two build jobs.
- `cargo test --locked -p oteryn-gameplay-content --lib srx3_reward_binding_requires_complete_source_profile_at_real_decoder` passed.
- `cargo fmt --all --check` and `cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets -- -D warnings` passed.
- Profile registration and its `--check` passed. The equipment custody
  census and Grizzly room generators refreshed their profile-input digests.
- `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /tmp/mantra-review/world` passed.
- `python3 tools/agents/validate_governance.py`, `python3 -m unittest discover -s tools/agents/tests` and `git diff --check` passed.
