# CHARM-CONFIRMED-FIXES-20261113

```yaml
task_id: CHARM-CONFIRMED-FIXES-20261113
title: Correct the confirmed full charm reset fee
type: mechanic
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/confirmed-charm-fixes-20261113
pr: null
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
owner: /root/confirmed_charm_fixes
created_at: 2026-11-13T00:00:00Z
updated_at: 2026-11-13T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/domain/src/domain/charm.rs
  - docs/agents/tasks/CHARM-CONFIRMED-FIXES-20261113.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Correct the existing reset fee calculation. Owner authorization covers code,
offline tests and draft publication, without gameplay, deployment or merge.
Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6853236808.
The separate evidence PR #3172 remains owned by its evidence writer.

## Architecture and source of truth

PROVEN: official Tibia news #8140, published 2024-11-21, gives reset examples
100 → 100,000; 500 → 4,500,000; 1,000 → 10,000,000; 2,000 → 21,000,000 gold.
Source: https://www.tibia.com/news/?id=8140&subtopic=newsarchive.
DERIVED: these examples require `100000 + 11000 * max(level - 100, 0)`.
The previous implementation multiplied the entire level above the threshold.
The fee remains checked for overflow and retains the existing expansion discount.

Existing caller trace: dispatch `charm_progression.rs` → `charm_native.rs` →
durability `charm_state.rs` ResetAll arm → `reset_all_fee` → existing fee-burn
transaction. No transaction, fencing, protocol or schema changes.

## Reference parity

| Behaviour | Reference | Oteryn | Test |
|---|---|---|---|
| Flat fee through level 100 | Official #8140 | 100,000 gold | `reset_fee_matches_official_examples_and_observed_expansion_payment`, 0/99/100 |
| Increment only above 100 | Official #8140 examples | 101: 111,000; 500: 4,500,000; 1,000: 10,000,000; 2,000: 21,000,000 | Same table-driven test |
| Existing 25% expansion discount | Preserved rule plus paid client observation retained in PR #3172 | Level 757: 5,495,250 | Same table-driven test, all three entitlements |
| Arithmetic overflow | Existing checked arithmetic contract | Refuses unrepresentable fee | Same test, u64::MAX |

## Rejected candidate: Agony removal by Cleanse

PROVEN on base main: domain `condition.rs::ConditionType::is_negative` excludes
Agony. Foundation `prepare_cleanse` filters through that method; neither the
fixed-total nor ramp Agony variant is eligible. The existing test
`agony_is_never_selected_by_cleanse_or_dispel_and_immunity_does_not_refuse_it`
passes unchanged. Thus an absent explicit Agony comparison inside
`prepare_cleanse` is not a bug and requires no patch. This agrees with the
official manual: https://www.tibia.com/gameguides/?section=characters&subtopic=manual,
section 5.1.10. No claim of a reproduced Agony defect is retained here.

## Excluded scope

Historical leech/AoE and Cleanse resistance replacement recordings do not alone
establish current Global behaviour. Parry mana drain, Dodge/Minor ordering,
Low Blow AoE draws, Carnage chaining and Hex/Intense Hex representation remain
qualification work in PR #3172. No speculative combat patch is included.
First-free-reset behaviour and transaction semantics are unchanged.

## High-risk authority/recovery qualification

NOT_APPLICABLE: a pure fee formula changes; no authority, recovery, session,
transaction, persistence layout or production mutation operator changes.

## Acceptance criteria

- [x] Official examples and observed expanded level-757 payment are covered.
- [x] Regression observed red before the production change and green after it.
- [x] Agony candidate traced and rejected without redundant code.
- [ ] Exact-head CI and independent review complete before integration.

## Validation

- RED: focused new test failed at level 101: actual 1,211,000, expected 111,000.
- `cargo test --locked -p oteryn-gameplay-domain --quiet`: pass, 209 unit and one doc test.
- `cargo test --locked -p oteryn-gameplay-foundation agony_is_never_selected_by_cleanse_or_dispel_and_immunity_does_not_refuse_it --quiet`: pass, one existing test.
- `cargo fmt --all --check`: not completed; Windows process command-line length limit (OS error 206). Changed-package `cargo fmt -p oteryn-gameplay-domain --check`: pass.
- `cargo clippy --locked -p oteryn-gameplay-domain --all-targets --quiet -- -D warnings`: pass; existing vendored Tokio warnings remain outside the changed package.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check` for owned paths: pass.
- PostgreSQL and live-game execution: not run; pure formula regression changes no database implementation. Existing paid caller is source-traced above.
- Independent review and exact-head CI: pending; this document grants no merge authority.

## Self-review

Bounded code diff reviewed against the official examples, threshold continuity,
all entitlements, checked overflow and the existing durable caller. No schema,
content, generated pin, registry or dependency changes.
