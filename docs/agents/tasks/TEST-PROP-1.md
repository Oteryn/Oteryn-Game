# TEST-PROP-1 — property tests for pure gameplay rules

```yaml
task_id: TEST-PROP-1
title: "TEST-PROP-1: property tests for pure gameplay rules"
mode: IMPLEMENT
status: IN_PROGRESS
local_status: VALIDATED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-test-prop-1
pr: null
class: ordinary
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - Cargo.lock
  - workspace-boundaries.toml
  - crates/gameplay/economy/Cargo.toml
  - crates/gameplay/economy/src/lib.rs
  - crates/gameplay/economy/src/bank_property_tests.rs
  - crates/gameplay/items/Cargo.toml
  - crates/gameplay/items/src/lib.rs
  - crates/gameplay/items/src/stacking_property_tests.rs
  - crates/gameplay/items/src/item_capacity_property_tests.rs
  - crates/gameplay/items/src/equipment_effects_property_tests.rs
  - crates/gameplay/combat-rules/Cargo.toml
  - crates/gameplay/combat-rules/src/attack/mod.rs
  - crates/gameplay/combat-rules/src/attack/formulas_property_tests.rs
  - crates/gameplay/domain/Cargo.toml
  - crates/gameplay/domain/src/domain/mod.rs
  - crates/gameplay/domain/src/domain/character_build_property_tests.rs
  - crates/gameplay/domain/src/domain/timed_item_property_tests.rs
  - crates/gameplay/progression/Cargo.toml
  - crates/gameplay/progression/src/lib.rs
  - crates/gameplay/progression/src/charm_effects.rs
  - crates/gameplay/progression/src/experience_property_tests.rs
  - crates/gameplay/progression/src/mana_training_property_tests.rs
  - crates/gameplay/progression/src/charm_effects_property_tests.rs
  - crates/gameplay/spell-rules/Cargo.toml
  - crates/gameplay/spell-rules/src/lib.rs
  - crates/gameplay/spell-rules/src/cast_property_tests.rs
  - crates/gameplay/spell-rules/src/native_attack_property_tests.rs
  - crates/gameplay/spell/Cargo.toml
  - crates/gameplay/spell/src/spell/tests.rs
  - crates/gameplay/spell/src/spell/cast_property_tests.rs
  - docs/agents/tasks/TEST-PROP-1.md
```

## Goal

Exercise existing pure gameplay rules with the std-only deterministic helper in
`crates/test-support/src/property.rs`. No PostgreSQL, runtime IO, third-party test
dependencies, party XP split or house auctions. Test modules follow the actual
rule owners: skill arithmetic and timed values belong to `domain`; the pure
mana/soul cast resolver belongs to `spell`.

## Invariants

Paths in this table are relative to `crates/gameplay/`.

| Rule | Invariant | File |
|---|---|---|
| Bank credit/debit | Checked results match a u128 oracle; insufficient funds, cap excess and arithmetic overflow refuse without wrapping | `economy/src/bank_property_tests.rs` |
| Deposit/withdraw/transfer | Composed pure debit/credit plans conserve total gold and invert exactly when both sides succeed | `economy/src/bank_property_tests.rs` |
| Bank amount/coins | Amount is positive and at most its cap; canonical denominations conserve value; output quantity and ordinal overflow refuse | `economy/src/bank_property_tests.rs` |
| Stack split/merge | Moved plus remaining equals input; top-up clamps to room; full/overfull receivers receive nothing | `items/src/stacking_property_tests.rs` |
| Stack declarations | Proven limits outside 1..=100 and unknown classes refuse, rather than silently clamp | `items/src/stacking_property_tests.rs` |
| Inventory/container | Successful placement stays within declared slots and stack sizes, preserves quantity and selects the newest compatible receiver; failed top-up never becomes partial acceptance | `items/src/item_capacity_property_tests.rs` |
| Carry capacity | u128 oracle verifies checked weight totals; insufficient room rejects the whole incoming quantity | `items/src/item_capacity_property_tests.rs` |
| Incoming trees | 500 entries inclusive; empty/501-entry trees and out-of-range quantities refuse | `items/src/item_capacity_property_tests.rs` |
| Absorption/resistance | Canonical rational protection clamps to -100..=100%; nonnegative damage stays in 0..=2x input; invalid rationals and unrepresentable intermediates refuse | `items/src/equipment_effects_property_tests.rs` |
| Attack damage | Fist/melee/creature bounds are ordered, nonnegative and nondecreasing in level/skill on representable inputs; excessive magnitudes refuse | `combat-rules/src/attack/formulas_property_tests.rs` |
| Level contribution/armor | Full-range level scaling and armor bounds are nonnegative and monotone, including scaling transitions and integer extrema | `combat-rules/src/attack/formulas_property_tests.rs` |
| XP table | All 2000 admitted levels strictly increase and invert through the real progression calculator; generated XP lies in the expected interval; terminal/overflow awards refuse | `progression/src/experience_property_tests.rs` |
| Skill formulas | Requirements are monotone for multipliers >=1 until unreachable; invalid inputs refuse; relevel conserves progress with the declared saturation bound | `domain/src/domain/character_build_property_tests.rs` |
| Paid training | All admitted vocations/families have monotone requirements; paid progress never regresses; max-level progress saturates without overflow | `progression/src/mana_training_property_tests.rs` |
| Critical/element mapping | Draws stay below 100%; percent multiplication uses a saturating u128 oracle; critical chance/damage map to their respective outcomes; every elemental proc preserves mitigation flags and level cap | `progression/src/charm_effects_property_tests.rs` |
| Item charges/duration | Charges stop at zero; elapsed time clamps to zero and partitioning elapsed time gives the same live values; stored limits reject excess | `domain/src/domain/timed_item_property_tests.rs` |
| Spell mana costs | Fixed costs are exact; percentage costs match a wide oracle, are monotone in maximum mana and saturate to u32 | `spell-rules/src/cast_property_tests.rs` |
| Spell cooldowns | Individual/group keys remain distinct; shared groups have exact deadlines; overflow at any deadline leaves the whole cooldown state unchanged | `spell-rules/src/cast_property_tests.rs` |
| Spell formula bounds | Harmony matches exact i128 arithmetic with saturation; equipment spell bounds are ordered/nonnegative and every element bond has a total mapping | `spell-rules/src/native_attack_property_tests.rs` |
| Mana/soul spending | Pure production cast resolution spends exactly the unsigned costs; insufficient pools refuse before spending, including zero/u32::MAX boundaries | `spell/src/spell/cast_property_tests.rs` |

## Case budget and replay

The helper defaults to **256 cases per property**, with bounded per-case work.
Small finite domains (the XP table, admitted skill rows, elemental mappings) also
receive exhaustive checks. Set `OTERYN_PROPERTY_CASES=4096` for heavier runs;
`OTERYN_PROPERTY_SEED=<decimal u64>` selects a reproducible seed. The helper reports
the failed property, case index, individual seed and replay environment.

## Findings

No gameplay rule defects found at the default budget or at 4,096 cases per
property with seed 1466. No production rule changes were necessary.

The architecture check required the seven new internal dev-dependency edges in
`workspace-boundaries.toml`; its normal dependency and layer restrictions remain
unchanged. Cargo.lock changes contain only those seven internal dependency edges.

## Reference evidence

- BANK-0 rules and BANK0-RL-01 are cited by `economy/src/bank.rs`.
- D80–D83, GAMEITEM01 limits and BAGS0-RL-02 are cited by
  `items/src/item_transfer.rs`, `stacking.rs` and `item_capacity.rs`.
- EQUIP-0 §3.3 specifies canonical rational protection summation and final clamping
  (`items/src/equipment_effects.rs`).
- ATTACK-0 §5 and the pinned TibiaTools formula (`damage.ts:13–25`) are cited by
  `combat-rules/src/attack/formulas.rs`; the official 13.05 level curve is in
  `content/src/content/spell_data/formula.rs`.
- XP formula provenance and its declared evidence limits:
  `docs/reference/experience-table-20261005/README.md`; runtime input is the
  compiled `rulesets/character/experience/native-section.json`, embedded at build time.
- SKILLS-0 and the pinned Canary training profile:
  `tools/content-schema/native-gameplay/build-training.json`; TIMEDITEM0B §7–§8
  is cited by `domain/src/domain/timed_item.rs`.
- CHARM-0 §4.3 and the qualified catalogue semantics are cited by
  `progression/src/charm_effects.rs` and `domain/src/domain/charm_data.rs`.
- Spell::getManaCost and playerSpellCheck are cited by `spell-rules/src/cast.rs`
  and `spell/src/spell/mod.rs`; Harmony uses the existing news-8944 arithmetic
  and the qualified `native-spell-profiles.json` parameters.

## Validation

All **577 library tests** pass across the seven touched crates. The exact filter
for all 23 new tests is `property_`; all also pass with
`OTERYN_PROPERTY_CASES=4096 OTERYN_PROPERTY_SEED=1466`. Default property times per
crate were below 0.25 seconds; heavier runs were below one second per crate.
New files range from 55 to 217 lines, each well below the 600-line limit.

| Package | Full `--lib` result | Exact property reproduction |
|---|---|---|
| economy | pass, 14 tests | `cargo test --locked -p oteryn-gameplay-economy --lib property_ --quiet` |
| items | pass, 41 tests | `cargo test --locked -p oteryn-gameplay-items --lib property_ --quiet` |
| combat-rules | pass, 53 tests | `cargo test --locked -p oteryn-gameplay-combat-rules --lib property_ --quiet` |
| domain | pass, 121 tests | `cargo test --locked -p oteryn-gameplay-domain --lib property_ --quiet` |
| progression | pass, 64 tests | `cargo test --locked -p oteryn-gameplay-progression --lib property_ --quiet` |
| spell-rules | pass, 61 tests | `cargo test --locked -p oteryn-gameplay-spell-rules --lib property_ --quiet` |
| spell | pass, 223 tests | `cargo test --locked -p oteryn-gameplay-spell --lib property_ --quiet` |

Packages were passed together with repeated `-p` arguments for execution;
economy/items/domain/progression were rerun after the final test refinements.
No PostgreSQL suite was run: these mechanics tests are pure and need no database.

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-gameplay-economy -p oteryn-gameplay-items -p oteryn-gameplay-combat-rules -p oteryn-gameplay-domain -p oteryn-gameplay-progression -p oteryn-gameplay-spell-rules -p oteryn-gameplay-spell --all-targets --quiet -- -D warnings`: pass.
- `cargo run --locked -p oteryn-architecture-check --quiet -- workspace .`: pass.
- `python3 tools/agents/validate_governance.py`: pass.
- `python3 -m unittest discover -s tools/agents/tests`: pass, 124 tests.
- `git diff --check`: pass.

Completed full-diff self-review against the owner-provided
`~/oteryn-cache/CODEX-CHECKLIST.md` and the repository review checklist, including
the new untracked files. No new public functions, enum variants, struct fields or
trait methods were added, so there are no new production constructor/match/impl
sites or dispatch surfaces to update. Existing tests remain intact. The card's
owned paths exactly match all 31 changed files.

## Unfinished

No implementation or local validation work remains. Publication, review and
lifecycle closeout belong to the control plane; the record stays active until
that closeout. This worker makes local edits only and does not commit or push.
