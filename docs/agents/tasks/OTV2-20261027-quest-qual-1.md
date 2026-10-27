# OTV2-20261027-quest-qual-1

```yaml
task_id: QUEST-QUAL-1
title: Qualify the supported quest-claim inventory
type: qualification
mode: AUDIT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/quest-qual-1
pr: null
owned_paths:
  - docs/agents/evidence/quest-claim-qualification.json
  - docs/repository/QUEST_CALLBACK_COVERAGE.json
  - tools/content-schema/quest-authoring/quest_callback_coverage.py
  - tools/content-schema/quest-authoring/check_quest_claim_qualification.py
  - tools/content-schema/quest-authoring/tests/
depends_on: [QUEST-CALLBACKS-1]
blocks: [quest-completion-parity-claim]
```

## Goal

Turn every one of the **239 `supported_unqualified` claims** in
`docs/repository/QUEST_CALLBACK_COVERAGE.json` into an auditable qualified or
explicitly blocked record. Qualification means a named automated test or a
reproducible scenario proves the exact source placement, admitted item/reward,
once-only durable result, and real dispatch behavior. This card creates evidence
and a failing check; it does not add production gameplay code.

## Scope and owned outputs

1. Generate a deterministic qualification ledger keyed by canonical claim key;
   never copy the 239 rows into this card.
2. For each claim record: quest, placement selector (position plus unique/action
   and appearance where applicable), pinned Canary/Crystal citations, expected
   reward, current inventory status, qualification disposition, dependency, and
   evidence locator.
3. Evidence locator is exactly one of:
   - `test`: crate/target plus fully qualified test name and asserted scenario;
   - `scenario`: checked-in fixture/scenario ID, runner command, expected receipt,
     and immutable artifact/hash;
   - `blocked`: typed blocker and owning task, with no implied qualification.
4. Add `check_quest_claim_qualification.py --check` to the repository's existing
   quest-authoring/content CI invocation. It reports totals and claim keys, and
   exits nonzero for missing, duplicate, stale, malformed or falsely qualified
   records.

## Evidence rules

- A unit test of a parser/binder alone is insufficient. A qualifying test reaches
  the production `Use` route and proves exact World placement/binding, native
  Item admission, durable per-character claim plus mint, retry/reconnect
  idempotency, and a second new command refusing without another reward.
- A scenario may substitute only when it exercises the same observable route and
  retains machine-readable expected/actual receipts. Prose, screenshots, PR text
  and “covered by suite” are not evidence locators.
- One parameterized test may qualify many claims only when its reported case ID is
  the canonical claim key and each case asserts its own selector and reward.
- `qualified` is allowed only when the named evidence exists and the checker can
  resolve it. Source agreement establishes expectation, not runtime proof.
- Keep `not_playable` claims visible in source coverage but outside this card's
  239-row denominator; the checker must detect denominator drift.

## Dependency classification

| Disposition | Claims included | Required ledger value |
|---|---|---|
| Ready under QUEST-CALLBACKS-1 | Ordinary once-only single-item chest claims whose reward is natively admitted and whose exact World placement can execute through bundle-chest `Use` | named dispatch/PostgreSQL test or scenario |
| Blocked on QUEST-CALLBACKS-2 | A claim's completion proof requires a lever, step-in trigger or atomic multi-item offering before its chest is reachable/eligible | `blocked_on: QUEST-CALLBACKS-2`, callback/profile key and missing scenario step |
| Blocked on QUEST-CALLBACKS-3 | A claim requires a quest door, item-on-target action or storage-gated use/access callback before its chest is reachable/eligible | `blocked_on: QUEST-CALLBACKS-3`, callback class/source and missing scenario step |
| Blocked elsewhere | Reward or world semantics are outside both callback cards (for example native Item admission or exact World binding unavailable) | typed owner/task; never relabel as callback-blocked |

Dependency assignment must be derived per claim from the pinned quest scripts and
scenario path. Do not mark every claim in a quest blocked merely because that
quest contains an unrelated callback.

## Non-goals

- No Rust/runtime, SQL migration, content reward, World placement, item definition,
  callback profile, protocol or client changes.
- Do not implement or repair QUEST-CALLBACKS-2/3, lower unsupported rewards, or
  change a claim/reward merely to make qualification pass.
- Do not upgrade `supported_unqualified` to a deployed/full-quest/playable claim.
  This inventory qualifies the individual reward claim only.
- Do not edit pinned sources or anything under `docs/reference/`.

## Reference evidence

- **PROVEN:** `docs/repository/QUEST_CALLBACK_COVERAGE.json` reports 336 authored
  claims: 1 `dispatch_pg_qualified`, 239 `supported_unqualified`, and 96
  `not_playable`, from Canary `47dfd51f` and Crystal `ff7ede59`.
- **PROVEN:** QUEST-CALLBACKS-1 PR #2446 states that only the Steel Helmet Quest
  coin chest is dispatch/PostgreSQL-qualified and explicitly leaves the other 239
  for per-World qualification.
- **PROVEN:** QUEST-CALLBACKS-2 PR #2484 supplies the lever, step-in and offering
  scenario primitives; QUEST-CALLBACKS-3 owns the remaining three action classes.
- **DERIVED:** callback availability does not prove a claim. The ledger connects
  each source claim to an executable end-to-end witness or an explicit blocker.

## Required parity table

The generated ledger is the full parity table. Its review summary must group all
239 rows by `qualified`, `blocked_on: QUEST-CALLBACKS-2`, `blocked_on:
QUEST-CALLBACKS-3`, or other typed blocker, and include zero unexplained rows.
Every qualified row includes source expectation, Oteryn observed result, and test
or scenario evidence; every blocked row includes the missing prerequisite.

## Acceptance and validation

- [ ] The checker independently reads the coverage inventory and proves a 1:1
  ledger mapping for exactly the current 239 supported-unqualified claim keys.
- [ ] It resolves every test/scenario locator, validates blocker vocabulary and
  task ownership, rejects duplicate/stale keys and source/reward/placement drift,
  prints disposition totals plus offending claim keys, and returns nonzero.
- [ ] Test fixtures demonstrate failures for a missing row, invented test,
  denominator change, stale source witness and invalid callback dependency.
- [ ] Run `python tools/content-schema/quest-authoring/quest_callback_coverage.py --check`.
- [ ] Run `python tools/content-schema/quest-authoring/check_quest_claim_qualification.py --check`.
- [ ] Run its focused unit tests, `python tools/agents/validate_governance.py`, and
  `python -m unittest discover -s tools/agents/tests`.
- [ ] PR body reports the 239-row parity totals and states clearly that no
  production code or full-quest playability was claimed.
