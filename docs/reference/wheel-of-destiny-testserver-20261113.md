# Wheel of Destiny: Winter Update test-server audit, 2026-11-13

Status: **in progress; full Global parity is not established**.

## Scope and evidence

The owner requested an audit of the whole Wheel and related Gem Atelier against the
logged-in Tibia test client and the current [test board](https://www.test.tibia.com/forum/?action=board&boardid=90),
with repairs collected in one dedicated PR. The target is the Winter Update test version,
not an older production client. Starting Game revision:
`ce45063507c22a11e0db7e9c0ff86dacbe6ff834`.

Evidence labels used below: **FACT** means directly inspected source or observation;
**INFERENCE** means a conclusion still needing a named check; **UNKNOWN** identifies a
missing observation; **INTENTIONAL DIFFERENCE** preserves an existing owner decision.
Source code proves what Oteryn does; donor code alone does not prove what Global does.

The imported catalogues contain five vocations, 36 slots per vocation, 46 basic and
94 supreme gem modifiers. Counts were read from `rulesets/progression/wheel-of-destiny/`.
Their historical `runtime_admitted: false` fields and README are not evidence that the
current executable Wheel is absent. Runtime producers and consumers exist in the paths below.

## Coverage map

| Surface | Current implementation | Audit state / next evidence |
| --- | --- | --- |
| Eligibility, point budget, connectivity, slot caps | `crates/gameplay/progression/src/wheel.rs` | Source inspection; focused tests pending |
| Devotion, Conviction and Revelation effects for all five vocations | `crates/gameplay/progression/src/wheel_effects.rs`, `spell_wheel.rs` | Trace every consumer; Global numeric comparison incomplete |
| Gem values and modifier grades | `crates/gameplay/progression/src/wheel_effects/gems.rs` | Donor-specific overrides need independent Global evidence |
| Gem Workshop permanent grade bonuses | Same shared effect projection; UI `crates/client-ui/src/features/0148-wheel-allocation/view.rs` | UI/server discrepancy below |
| Winter Update slots and removal of resonance/domain | `crates/gameplay/progression/src/atelier/vessels.rs` | Implemented; exact quality order explicitly unverified |
| Reveal, dismantle, enhance, lock, reshape | `crates/gameplay/progression/src/atelier/`, `crates/gameplay/durability/src/durability/character_atelier/` | Costs, pools, protections and real write paths under audit |
| Durable allocation, reset, receipts, replay | `crates/gameplay/durability/src/durability/character_wheel.rs` | Existing fenced writer; current-path regression checks pending |
| Admission, current vitals, spell effects and carrying capacity | `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/wheel.rs`, `crates/gameplay/durability/src/durability/character_wheel/capacity.rs` | Shared projection callers traced; behavior tests pending |
| v1/v2 protocol and presentation | `crates/protocol-oteryn/src/{wheel,atelier,wheel_presentation}.rs`, `crates/gameplay/dispatch/src/gameplay_transport/{wheel_allocation,wheel_presentation,atelier}.rs` | Compatibility and stale-state paths under audit |
| Native/client interactions and presets | `crates/client-ui/src/features/0148-wheel-allocation/`, corresponding client-runtime and app feature | UI strings are not gameplay evidence; native acceptance incomplete |
| Latest test-server changes | Official board 90 and current installed client | Exact dated source matrix pending |

## Findings and verification targets

### W01 — Workshop stat bonus is displayed without a matching shared producer

**FACT:** `view.rs::workshop_lines` sums the zero-based unlocked modifier grades and
advertises `5 * sum(grades)` extra Health, Mana and Capacity.

**FACT:** the inspected `WheelRuntimeEffects::contribute_gems` counts Grade IV modifiers
for domain points and adds effects from socketed gems; it does not add the advertised
Workshop Health/Mana/Capacity bonus.

The shared projection is consumed by current vitals, spell snapshots, Wheel presentation
and durable carrying-capacity reads. A repair belongs in that projection, with the existing
current-ruleset, promotion, level and Premium guards preserved. **UNKNOWN:** fresh official
Workshop text or a controlled live upgrade is still needed to establish the exact Global
bonus and its eligibility, before treating the UI wording alone as the target rule.

### W02 — Revelation Mastery deliberately doubles donor values

**FACT:** `wheel_effects/gems.rs::supreme_effect` multiplies every
`revelation_mastery_points` value by two. Its comment attributes this to two applications
in Crystal's C++ implementation. The imported catalogue gives 150/165/180/225 points for
Grades I/II/III/IV; the Rust projection consequently produces 300/330/360/450.

**UNKNOWN:** current Global gameplay must decide the expected values. A client summary
or a donor implementation alone is not a complete controlled threshold test. Verify the
equipped gem, modifier grade, domain points and resulting Revelation stage together.

### W03 — Twelve slot qualities use an explicit assumption

**FACT:** `atelier/vessels.rs::GEM_VESSEL_SLOT_QUALITIES` repeats Lesser, Regular, Greater
four times. Its comment and `docs/contracts/protocol-oteryn/v1/gem-atelier-v2-v1.md`
explicitly leave the published order open. This table controls both acceptance of a gem
and the server's slot presentation, so an incorrect assumption is behaviorally material.

**UNKNOWN:** capture the complete ordered vessel panel in the current test client.

### W04 — Further donor-specific overrides need independent evidence

The effect projector also overrides Monk mixed-modifier Mana, Grade IV capacity,
Flurry damage/critical fields and Combat Mastery steps. These are verification targets,
not declared bugs. Do not automatically replace them with imported description values.

### W05 — Removal permission is an accepted Oteryn difference

**INTENTIONAL DIFFERENCE:** the 2026-11-05 owner decision allows point removal/reset where
logout's combat lock does not hold the actor, including a protection-zone tile. The
compatibility field `at_temple` carries that permission in `WheelChangeFacts`.
This audit preserves the decision; Global's temple requirement alone does not revoke it.

## Validation and limits

No full-parity percentage or complete-implementation claim is justified by the catalogue
counts. Focused checks, independent review and exact-head CI results are recorded in the
task record / PR as they actually run. Numeric fixes require reproducible regression cases.
Original proprietary client binaries/assets are reference inputs only and are not added to Git.

Outstanding live observations include all vessel qualities, Workshop bonuses, Mastery
threshold changes, each vocation's updated perk values, reshape outcomes, combat effects,
and save/relog/reset behavior. An expired test-client session blocks those observations,
but not repository inspection or focused regression tests.
