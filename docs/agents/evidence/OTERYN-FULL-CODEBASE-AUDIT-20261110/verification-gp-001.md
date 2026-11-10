# Independent verification of GP-001

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`.
Verifier: durability_economy_audit, independent re-read of production call paths.
Verdict: **CONFIRMED by static executable-path evidence; P1 for affected legitimate casts**. No Rust/SQL execution was performed. Initial `command -v rustc` returned no PATH entry; root subsequently located Rust 1.99.1 under `/home/agent/.cargo/bin`, but the native dependency cache is absent and a native build was not run.

## Evidence

- `crates/gameplay/spell/src/spell/owned_cast_facts.rs:245-269,394-397`: independently read build must equal the raw cached magic level; caster then applies account loyalty projection and adds `proficiency_skills[7]`.
- `crates/gameplay/spell/src/spell/actor_conditions.rs:1619-1622,1672-1674`: comparison value applies only condition/Wheel adjustment to raw `character_facts.magic_level`; loyalty and proficiency are absent.
- `crates/gameplay/dispatch/src/gameplay_transport/spell_access_facts.rs:461-476`: current SQL build and current `raw.loyalty_points` enter the real cast producer.
- `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency_cast.rs:149-174`: equipped weapon and selected live track set `owned.access.proficiency_skills`; this does not update cached Character facts.
- `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs:2541-2554,2612-2617,1131-1158,1059-1072`: real dispatcher installs proficiency, selects the unaugmented self-effect path, builds the bonus-inclusive caster, passes it to `prepare_source_self_cast_with_caster`.
- `crates/gameplay/spell/src/spell/cast/gate.rs:156-166`: self path rejects the unequal magic levels before resolution/payment.
- `crates/gameplay/dispatch/src/gameplay_transport/world_item_cast.rs:257-280,344-370,399-405` and `crates/gameplay/spell/src/spell/cast/prepare.rs:530-541,804-813`: food/native and conjure preparation repeats the raw-versus-qualified comparison.
- Sibling gates in `prepare.rs:691-700,939` require the same correction in a future fix.

## Concrete valid trigger and counterguards

Choose no active condition/Wheel magic adjustment and a valid cast, valid actor/session/equipment/learning/mana/soul facts. With raw magic 30 and loyalty projection 31, caster equals 31 but comparison equals 30. The committed formula unit fixture `crates/gameplay/progression/src/loyalty.rs:displayed_loyalty_levels_follow_the_native_training_formula` supplies exactly paladin magic 30, progress 0, loyalty 3600 -> 31 (test inspected, not run). Independently, nonzero proficiency magic creates a mismatch even with loyalty zero; `postgres_tests/monk_virtues.rs:812-859` has a real equipped-weapon track whose +2 magic is asserted (test inspected, not run).

The producer explicitly requires cached raw facts to equal the build before calculating bonuses, so caching the bonus already cannot avoid the check. Applying current Premium refreshes/reloads owned facts and reinstalls proficiency; no step between production and the gate removes the bonus. Skill adjustments can incidentally erase a loyalty delta under a debuff, so the trigger requires that the final projected magic differs; not every loyalty balance causes a level increase.

## Scope and limits

Do not state that every combat spell fails. The traced self-effect, native helper, callback and conjure gates fail when reached with a differing bonus-inclusive magic level. Some ordinary combat routes consume the qualified caster through different preparation and need their own analysis. The observed wrong result is an early `SpellCastDisposition::Rejected`; no evidence here establishes item loss or spending on this rejection. Native integration test reproduction remains to be run in an isolated test environment.

## Minimal arithmetic demonstration

A separate Python check of the gate equivalence (not a Rust execution) uses `(raw, loyalty_effective, proficiency)=(30,31,0)` and `(30,30,2)`. In each case the qualified value differs from the raw-based comparator and the exact inequality is true.

## Direction for future repair

Keep raw build binding verification, but make the qualified-caster consistency comparison use the same authoritative loyalty/proficiency/condition/Wheel projection. Do not simply drop every consistency check. Add native self-effect and conjure regression cases with nonzero loyalty/proficiency, plus unchanged zero-bonus controls. No code was changed.
