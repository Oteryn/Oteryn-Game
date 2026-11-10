# Gameplay audit — selected critical routes

Base: `e2e5ced306040113dc9c395251a802f50eb5776a` (current-main snapshot supplied by root). Checkout: `/workspace/scratch/9868b8ed6bb7/Oteryn-Game-audit`. Audit is read-only; no source fix, PR, merge, deployment, heavy build, or SQL execution was performed by this worker.

## Result and evidence level

**One confirmed P1 gameplay defect: GP-001.** Confirmation is from source-level execution-path tracing and an independent reread by `durability_economy_audit`; it is not a completed native integration reproduction. No P0 was established in this lane. No additional P2/P3 gameplay finding has sufficient evidence to report as a defect.

This is a **partial audit**, not a completed whole-gameplay review. The assigned eleven package directories contain **502 Rust files and 154,693 lines**, including tests and build scripts, counted with `Path.rglob('*.rs')` on the pinned checkout. Only the files/ranges in `coverage-ledger.json` were substantively analyzed. Looking up filenames, entry points, or references does not count as reviewed behavior. Remaining paths need follow-up before any package is called fully reviewed.

## GP-001 — Qualified magic bonuses reject legitimate self, native, and conjure casts

**Severity:** P1 under the repository's real-caller/wrong-behavior criterion.
**Status:** CONFIRMED, static executable-path evidence; independently confirmed.
**Affected baseline:** `e2e5ced306040113dc9c395251a802f50eb5776a`.
**Independent evidence:** `verification-gp-001.md`.

### Concrete trigger and wrong result

A current, otherwise eligible player whose final magic projection includes a nonzero loyalty or equipped weapon proficiency increase reaches a gate comparing the fully qualified magic value to a second projection that omits those bonuses. When the two differ, legitimate self-effect and several native/conjure casts return `SpellCastDisposition::Rejected` before effect resolution. This includes a straightforward self-effect case such as the loaded `Light` spell (`utevo lux`); the pinned catalog admits paladin and monk among its vocations, requires level 8, and has `self_target: true` and no explicit target.

The concrete numeric state can be raw paladin magic 30, progress 0, loyalty 3600, with no condition/Wheel adjustment. The committed loyalty fixture specifies projected magic 31. A separate equipped-weapon fixture supplies a +2 proficiency magic bonus. In either case, the qualified value differs from the raw-only comparator. These fixtures were read, not executed during this audit.

### Production path and exact locations

All paths below are relative to `Oteryn/Oteryn-Game` at the pinned SHA.

| Role | Location | Evidence |
|---|---|---|
| Current loyalty producer | `crates/gameplay/dispatch/src/gameplay_transport/spell_access_facts.rs:425-476` | Current build qualification copies `raw.loyalty_points` into cast access projections. |
| Current proficiency producer | `crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency_cast.rs:149-177` | Right-hand equipment and selected current proficiency track populate `owned.access.proficiency_skills`. |
| Raw binding guard | `crates/gameplay/spell/src/spell/owned_cast_facts.rs:245-269` | Raw build magic must match the cached actor facts before qualification. |
| Full magic projection | `crates/gameplay/spell/src/spell/owned_cast_facts.rs:394-397` | Applies `loyalty_level(0, self.build.magic())`, condition/Wheel adjustments, then adds `proficiency_skills[7]`. |
| Incomplete comparison projection | `crates/gameplay/spell/src/spell/actor_conditions.rs:1563-1622` | `owned_effective_magic_level` starts from raw `character_facts().magic_level`; its adjustments contain conditions/Wheel, not loyalty/proficiency. |
| Real dispatch qualification | `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs:2541-2554,2612-2617` | Loads owned facts, installs proficiency, selects the self-effect route. |
| Real self preparation caller | `crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs:1059-1072,1131-1158` | Builds the qualified caster and passes it to `prepare_source_self_cast_with_caster`. |
| Self rejection | `crates/gameplay/spell/src/spell/cast/gate.rs:156-166` | Rejects `caster.magic_level != state.owned_effective_magic_level(context.now.get())?`. |
| Native rejection | `crates/gameplay/spell/src/spell/cast/prepare.rs:530-542` | Repeats the unequal-projection comparison. |
| Callback rejection | `crates/gameplay/spell/src/spell/cast/prepare.rs:691-701` | Repeats the comparison. |
| Conjure rejection | `crates/gameplay/spell/src/spell/cast/prepare.rs:804-813` | Repeats the comparison. |
| Named-player failure sibling | `crates/gameplay/spell/src/spell/cast/prepare.rs:934-945` | Same equality check occurs in the failure-payment preparation. |
| World item callers | `crates/gameplay/dispatch/src/gameplay_transport/world_item_cast.rs:257-280,344-370,399-405` | Builds the bonus-inclusive caster and invokes native/food/conjure preparation. |

Primary source link: [self-cast rejection at the exact baseline](https://github.com/Oteryn/Oteryn-Game/blob/e2e5ced306040113dc9c395251a802f50eb5776a/crates/gameplay/spell/src/spell/cast/gate.rs#L156-L166).

### Counterguards and scope limits

The raw binding guard prevents treating the actor cache as already loyalty/proficiency-adjusted. Premium refresh and owner fact reload reinstall current proficiency; neither removes the bonus. No stale client input is needed. A small loyalty balance that does not change the final integer magic level does not trigger the defect, and a modifier can incidentally erase a loyalty difference. The requirement is a differing **final** magic value.

Do not extend this to every combat spell. `prepare_ordinary_owner_cast_with_wheel` deliberately omits this equality and comments that the owned caster includes loyalty and proficiency that the condition-only state projection does not contain. The existing equipped amber test casts `devastating_knockout` through that ordinary path; its use of the light spell only inspects the projection and does not cast light with the bonus. No evidence here establishes resource loss on the rejected routes; the wrong result is an early refusal.

### Minimal native regression scenario to run

1. In the existing isolated dispatch fixture, admit a current player, valid spell book, valid mana/soul/learning/session/equipment facts, no magic conditions, and raw magic 30.
2. Set the account loyalty projection to the fixture value 3600 (or equip the existing proficiency fixture whose magic bonus is +2).
3. Invoke the real self-effect dispatch with `Light`; verify the owned caster differs from the raw-only comparator and observe the current rejection.
4. Repeat with no bonus as a control, then test a native helper and a conjure route with valid prerequisites.

**Not run:** full Rust/SQL integration regression. The Rust toolchain is available via `/home/agent/.cargo/bin`, but the native dependency cache is absent; the authorized audit did not install/build the workspace or provision PostgreSQL. Arithmetic substitution and source inspection do not constitute an integration test.

### Repair direction (no repair performed)

Make consistency checks use the same authoritative loyalty/proficiency/condition/Wheel projection while preserving the existing raw-build binding guard. Cover all sibling gates in one fix. Do not indiscriminately remove unrelated state consistency checks. Add native self-effect and conjure controls with nonzero and zero bonuses.

## Other critical routes examined

- **Creature death → loot/XP/Bestiary:** `combat/death_reward.rs` checks captured principal identity before descendant settlement, separately settles loot and XP, reuses memoized occurrence identity for XP replay, initializes missing progression before retrying once, and uses the same death occurrence for Bestiary. Reviewed selected dispatch `kill_reward.rs` settlement entry and ownership branches. The complete dispatch queue lifecycle, party XP transaction and all retry outcomes are not yet covered.
- **NPC → shared quest/reward → reconnect:** extended bounded flow in `gameplay-npc-flow.md` traces actual command-7 routing, source callback selection, lease/conversation and revision checks, retained unknown command, same-session reconciliation, and quest/XP reload after fresh admission and resume. Quest-specific transaction composition was independently inspected: receipt lookup precedes fresh authority, locked quest predicates are re-evaluated, participant refusal rolls back, and reward/quest receipt/state/XP obligation commit together. The generic MINT internals remain owned by the durability audit lane. No additional P1 was established. Existing rollback/post-commit interruption/replay fixtures were read, not run.
- **Quest content transitions:** bounded state transitions and callback guards were examined. The free-item callback cases inspected have storage changes leaving their eligible range. A suspected repeat-free-reward path was not established and is not reported as a defect.
- **Character and progression arithmetic:** selected vitals, soul, stamina, hunting XP, loyalty, mana training, hit training, daily reward and regeneration paths were inspected. A suspected PZ regeneration over-credit was refuted by the spell wrapper replacing legacy vocation periods with the current fed/unfed periods before holding clocks.
- **AI/encounter rules:** selected monster think timer/owner checks, summon registration compensation, threshold-heal occurrence/cooldown/current-actor guards, boss room admission arithmetic, and personal reward collection/contribution calculations were inspected. No whole-AI or whole-encounter claim follows from these selected routes.

## Independent verification

GP-001 was independently confirmed in verification-gp-001.md. Cross-lane security evidence is retained privately and not reproduced here.

## Explicit remaining gaps

Most package files have not received behavioral review. In particular: full ordinary combat/target legality/condition-tick/death integration, every native spell family and Wheel interaction, all NPC quest adapters and authored content combinations, durable quest XP/reward transactions, complete monster AI/perception/path planning, encounter-specific timers and lifecycle branches, full character progression persistence, and end-to-end tests remain open. Production deployment configuration, live account state and load behavior were not inspected. A complete audit requires continued source tracing and targeted native reproductions; an inventory or search match is not evidence of coverage.
