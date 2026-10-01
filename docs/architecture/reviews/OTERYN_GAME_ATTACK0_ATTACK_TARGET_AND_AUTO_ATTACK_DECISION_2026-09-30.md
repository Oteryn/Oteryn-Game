# ATTACK-0 Attack target and auto-attack

- Decision: `ATTACK0-ATTACK-TARGET-AND-AUTO-ATTACK-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol and
  combat) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the architect programme plan (#162 5910870596, M1): a player cannot attack today
  (`spell/cast.rs`: "No attack-target owner exists yet")
- Builds on: GAME-ABILITY-01 (one ability and effect pipeline, sole damage authority; whole gate
  §4 occurrence identity and §8.1 catch-up), VSL-COMBAT-01 (death, loot, XP), MOVE-RL-11 (D84-D87 visibility, VIS-2), SPELL-D7 (the
  `ATTACK_TARGET` intent), CHARM-0 (D186 hooks), PROFICIENCY-0, A13 (vocation and magic
  level), GAME-CHAR-01 Stage B decision 10 (parity gates), owner rule 5905825574 (an official
  source governs; owner-trusted fan sources are allowed)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ATTACK-WIRE-1 | impl, protocol review | two command types (11 and 12 proposed, reserved at allocation), state domain 10 (§3), codecs, limits, client target selection and fight-mode buttons | VIS-2 (MOVE-RL-11 is still a candidate); the protocol numbers re-checked at allocation |
| ATTACK-1 | hard (combat), combat review | the attack-target owner, the auto-attack timer and the in-fight deadline (§4); player fist and melee attacks and creature melee attacks as GAME-ABILITY-01 abilities (§5) | ATTACK-WIRE-1; GAME-AI-01 runtime wiring (creatures that exist and choose a target) |
| SPELL-TARGET-1 | spell lane | the spell Target Resolver: `ATTACK_TARGET` resolves to the §4 target, and single-target damage spells stop being rejected (`spell/cast.rs`) | ATTACK-1 |
| ATTACK-PARITY-1 | impl | the parity fixtures of §6 against the TibiaPal damage calculator | ATTACK-1 |

Later, each with its own decision or amendment: distance and throwing weapons (ammunition use
needs a DUR-03 burn cause), wands and rods (mana cost), chase movement, PvP (PARTY-PVP-0).

## 1. Question

How does a player pick a target and hit it, how does a creature hit back, and how much damage
does a hit do?

## 2. Facts

**PROVEN**

- GAME-ABILITY-01 is the only damage authority; a second combat engine is not allowed.
  VSL-COMBAT-01 already turns a creature death into loot and XP. Its §5 and the whole gate §4
  require every hit to have a stable parent occurrence identity; charm rolls take a
  (`GameplayDecisionRoot`, `DecisionOccurrenceId`) (`charm_effects.rs`).
- Whole gate §8.1: every repeating gameplay timer declares a catch-up policy, and damage may not
  use `SKIP_TO_LATEST`.
- GA-XD-02 puts item consumption under DUR-03. DUR-03 §15 admits only the fee burn cause and
  `DECAY_RETIRE`, so using up ammunition has no DUR-03 shape today.
- No command sets an attack target, and the runtime holds none. The spell `ATTACK_TARGET` intent
  resolves to "no target", and the missing Target Resolver rejects damage and foreign-target
  effects (`spell/cast.rs`).
- GAME-AI-01 is `PROPOSED`; its targeting is evidence-gated, and no creature attack path exists in
  code.
- D85 gives creatures a visible identity (runtime actor id and generation, health percentage) in
  domain 1. It reaches the client only through VIS-2, which is not built; MOVE-RL-11 is a
  candidate.
- The spell formula engine (`spell/formula.rs`) already evaluates `player_expression` trees over
  `level`, `attack_skill`, `attack_value`, `attack_factor`, `shielding_skill` and
  `shield_defense`, with the official 13.05 level curve (equal to `level / 5` up to level 500).
- The charm hooks are five (`charm_effects.rs`): among them AttackDamageCalculation before the
  commit, AttackHit, CommittedHit (kills) and the incoming creature attack and hit hooks.
- The Tibia manual (`docs/reference/tibia-manual/combat.md` §5.3.1, §5.3.12.c): a fixed attack
  cadence, one target at a time, at most two blockable attackers per round, and a logout block of
  60 s after fighting. It gives no formula.
- A13 makes vocation and magic level durable; SKILLS-0 (PR #1346, a candidate) adds the weapon
  skills.
- Protocol numbers: `main` registers command types 1-3 and domains 1-3. #162 reserves capabilities
  1-2 (5907282001), command types 7-8, domains 7-8 and capability 3 (5909366267), and command
  type 9, domains 9 and 11 and capability 4 with domain 10 released (5910888594, PR #1344, not
  merged). The control plane confirmed domain 10 for this decision (5911720221), reserved command
  type 10 for `ACCOUNT_ACHIEVEMENTS_QUERY` (5912071646) and capability 5 for `DEPOT_V1`
  (5912405163), and names command type 11 and capability 7 (once 6 is confirmed) as the next free
  numbers.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, for the formulas of §5)

- Attack interval 2,000 ms for every vocation (`vocations.xml:3-123`; rate divisor
  `vocation.cpp:365-367`). A missed interval is never replayed (`player.cpp:3991`, `lastAttack`).
- Fight-mode attack factor: offensive 1.0, balanced 0.75, defensive 0.5 (`player.cpp:840-851`).
- Maximum melee damage `0.085 x factor x attack x skill + level / 5` (`weapons.cpp:94-100`); the
  hit is `normal_random(min, max)`, a truncated normal with mean 0.5 and deviation 0.25 of the span
  (`tools.cpp:466-475`), then multiplied by the melee damage multiplier and damage modifier, with an
  elemental split (`weapons.cpp:210-216`). Fists use attack value 7 (`:226`).
- Defence `(shielding / 4 + 2.23) x defence x modeFactor x scaling x vocationDefence`, scaling
  0.16 with a shield, 0.146 with weapon defence, 0.15 with fists; skill 0 gives 1 or 2; fist defence
  7 (`player.cpp:776-818`). The mode factor depends on the time since the last attack
  (`:853-872`). It is applied as `uniform_random(defence / 2, defence)` (`creature.cpp:966-967`).
- Blocking is time-based: one block per 1,000 ms, capped at 2, used up per blocked hit
  (`creature.cpp:141-145, 961-963`).
- Armor removes `uniform_random(armor / 2, armor - (armor % 2 + 1))` when armor > 3, and 1 when it
  is 1 to 3 (`creature.cpp:976-982`).
- Creature melee maximum damage `getMaxMeleeDamage` (`weapons.cpp:88-91`) from the monster's
  attack and skill.
- An attacker standing in a protection zone cannot attack (`combat.cpp:327-329`).
- The in-fight (logout block) condition lasts 60 s, refreshed by hits dealt or taken
  (`player.cpp:4487-4503, 6518-6525`; config `pzLocked`).

**Owner-trusted fan source**

- The TibiaPal damage calculator, which the owner tested and trusts (#162 5905825574 and
  5905851791), is the
  parity oracle for these formulas.

## 3. Wire (ATTACK-WIRE-1)

| Kind | Id | Name | Content |
|---|---|---|---|
| capability | at allocation | `ATTACK_V1` | gates everything below; its number is reserved on #162 when ATTACK-WIRE-1 is allocated (5 went to `DEPOT_V1`) |
| command type | at allocation (11 proposed) | `ATTACK_TARGET_INTENT` | `{target: {actor_id, generation} or none}`; none stops attacking |
| command type | at allocation (12 proposed) | `FIGHT_MODES_INTENT` | `{fight_mode: OFFENSIVE, BALANCED or DEFENSIVE; chase: STAND or CHASE; secure: bool}` |
| state domain | 10 | `ACTOR_COMBAT_STATE` | the own actor's current target (or none), fight mode, chase, secure and in-fight flag |

- Domain 10 is confirmed (5911720221). The two command types and the capability are reserved on
  #162 when ATTACK-WIRE-1 is allocated; 11 and 12 are proposed, the next free after command type
  10 went to `ACCOUNT_ACHIEVEMENTS_QUERY`.
- The target is the D85 identity of a creature visible to the session (VIS-2). A target that is
  not visible, not a creature, or in a protection zone is refused, and so is any target while the
  attacker stands in a protection zone.
- **First slice: creatures only.** Attacking a player is refused until PARTY-PVP-0 decides PvP
  rules; `secure` is carried now and has no effect until then.
- **Chase** is carried now and has no effect in the first slice: every actor acts as STAND. Chase
  needs server-driven player movement, which has no owner yet.
  **Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §8).** Once accepted
  and CHASE-1 lands, `CHASE` makes the player follow its target through the Movement owner. Until
  then the client ships the chase toggle disabled (always `STAND`).
- Fight modes are runtime-only, as in Tibia, where the client sends them at login: the defaults
  are balanced, stand and secure on.
- Domain 10 is owned by the channel runtime; its revision is monotonic per GameSession (FND-02
  §15), with a snapshot at every admission, reconnect and channel transfer.
- **Reconnect.** The in-fight deadline and its flag belong to the runtime actor and survive a
  reconnect to the same GameSession, as FND-ID-01 requires ("reconnect must not ... clear
  combat/PZ/logout locks"); a player cannot escape the logout block by reconnecting. Only the attack
  target is cleared on reconnect and transfer: it is a client selection, not a lock, and clearing
  it stops swings without shortening any block. The client sets it again.
- **Re-entry protection** (`DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`, owner-accepted):
  after a valid re-entry from an unexpected loss of control (not a graceful logout), the character
  has 4 s of PvE protection. During it, `ATTACK_TARGET_INTENT` with a target is refused
  (`REJECTED`, not buffered), the protected character makes no auto-attack swing, and no monster
  starts or makes a new attack on it (§4 validity). The in-fight deadline is not reset by it, and
  committed damage still resolves.
- Damage stays visible through the target's health percentage in domain 1 (D85). A dedicated
  combat-effects view (numbers, animations) is a later wire decision.
  **Amendment (pending on acceptance of SPELL-PRESENT-0; `OTERYN_GAME_SPELL_PRESENT0_SPELL_AND_COMBAT_PRESENTATION_DECISION_2026-09-30.md` §4,
  §5).** That view is the `WORLD_PRESENTATION` domain: hit, block and armour effects and damage
  numbers from each committed swing, emitted by COMBAT-PRESENT-1.
- Limits (ATTACK-WIRE-1 registers them): `ATTACK0-RL-01` target changes per second,
  `ATTACK0-RL-02` fight-mode changes per second.

**Amendment (pending on acceptance of PARTY-PVP-0;
`reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md` §7, §8 and §11).** Once
accepted, a character is a valid target where PARTY-PVP-0 §7 allows it: the World PvP type, level 8,
no protection zone, post-login immunity, party and guild immunity, secure mode and the black skull
rule, checked in the GAME-ABILITY-01 legality stage, with the refusal `PVP_REFUSED {reason}`;
`secure` takes effect; `FIGHT_MODES_INTENT` gains `expert_mode` behind `PVP_V1`. The §4 logout block
is joined by PARTY-PVP-0's PZ block and 15-minute kill block (§8.1); a PvP-sourced logout block is
also written ahead durably and restored at admission, so a node crash does not end it. Until PVP-1, PVP-RT-1,
PVP-DEATH-1 and PVP-WIRE-1 land, the first slice stays creatures only.

## 4. Runtime (ATTACK-1)

- **Owner.** The channel runtime holds one attack target and the fight modes per actor. Setting a
  target is a runtime action; it writes nothing durable.
- **Swing occurrence.** Each swing is a timer occurrence of the attack-target owner, keyed by
  (runtime scope, attacker actor id and generation, swing sequence). Its lineage is the CommandRef
  that set the target. The swing binds the formula content revision and the SIM profile. Its RNG
  purposes are closed: `hit_chance` (reserved for distance), `damage_draw`, `defence_draw` and
  `armor_draw`. Charm rolls take the swing as their `DecisionOccurrenceId`.
  **Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §4.1).** The closed
  purposes gain `miss_landing` and `break`; a ranged swing's DUR-03 key adds the World, Channel,
  scope ownership generation and CharacterId (RANGED-0 §6.2).
- **Timer.** Every attack interval (2,000 ms) while the target is valid, the actor swings once
  through the GAME-ABILITY-01 pipeline as one typed ability, `AutoAttack`. Catch-up policy
  `DEADLINE_STATE`: at most one swing per due deadline, never a backlog, as Canary never replays
  a missed interval. After a stall the next deadline counts from the time the swing actually ran,
  not from the missed schedule (Canary sets `lastAttack` to the execution time), so a stall never
  shortens the next interval. Nothing else deals auto-attack damage.
- **Cooldowns.** The attack interval is independent of spell and spell-group cooldowns.
- **Validity.** Visible, a creature, alive, on the same floor, adjacent (1 tile), neither actor in
  a protection zone, the attacker not under re-entry protection (§3). Out of range, the swing waits. A dead or vanished target clears the target.
- **Weapon.** First slice: fists, and melee weapons once ITEM-MOVE-WIRE-1 admits equipping. A
  distance weapon, throwing weapon, wand or rod in the hand is treated as no weapon for
  auto-attack until its own decision (see the brief).
  **Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §4).** Once
  accepted, distance, throwing and wand swings are variants of this `AutoAttack` with range-based
  validity, one weapon-use slot, PREPARE and PRIMARY COMMIT (RANGED-0 §4).
- **Skills.** Until CHAR-BUILD-1 and SKILLS-0 ship, every character fights with the starting
  skill 10. Once they ship, the attack reads the live skill and reports one try per swing to the
  build-state training of SKILLS-0.
- **Creatures hit back.** ATTACK-1 also gives a creature a melee `AutoAttack` against its target
  on the same timer rules, through the same pipeline, with the incoming creature attack and hit
  charm hooks. Target choice stays with GAME-AI-01; this decision only supplies the attack.
- **Charm hooks.** AttackDamageCalculation before the damage commits (critical hits),
  AttackHit after it, and CommittedHit for kills (PROFICIENCY and charms). The incoming hooks run
  for creature attacks.
- **In-fight deadline (logout block).** ATTACK-1 keeps one runtime-actor-local deadline: 60 s
  after the last hit dealt or taken (`ATTACK0-RL-03`, registered by ATTACK-1; manual §5.3.12.c). While it runs:
  - a logout command is refused, and FND-ID-01 sees the blocker;
  - a closed client does not remove the actor: the actor stays in the world until the deadline
    passes, and then ends as at logout;
  - domain 10 shows the flag.
  The 15-minute block after a player kill belongs to PARTY-PVP-0.
- **Durability.** Creature health and the actor's own health are runtime state. Only a creature
  death (VSL-COMBAT-01) and a character death (DEATH-1) become durable. Fists and melee consume
  nothing, so no DUR-03 shape is involved in this slice.

## 5. Formulas (ATTACK-1)

- The formulas of §2 (Canary) are authored as `player_expression` trees for the existing spell
  formula engine (`spell/formula.rs`) and one small content table of constants (fight factors,
  defence scaling, the fist values, the block cadence). No new formula engine is built.
- The level term is the engine's official 13.05 curve, which equals Canary's `level / 5` up to
  level 500.
- Divergences are resolved toward the TibiaPal calculator, and toward an official CipSoft source
  where one exists. Every value carries `PARITY_PENDING` until ATTACK-PARITY-1 matches it.
- Creature defence and armor use the same formulas with the creature's content values; creature
  attacks use `getMaxMeleeDamage`.
- Blocking: the time-based block budget of §2 (one per 1,000 ms, at most 2), which is the manual's
  "two attackers per round"; hits beyond the budget meet armor only.
- Critical hits, leech and elemental splits come from charms, imbuements and proficiency, not
  from this decision.

## 6. Parity (ATTACK-PARITY-1)

- Fixtures compare the formula table with the TibiaPal calculator for each vocation, level bands
  (8, 50, 100, 300, 600, 1000), skills (10, 50, 100, 120), fight modes, fists and one weapon per
  melee class.
- A mismatch above the calculator's rounding keeps `PARITY_PENDING` and is reported on #162.

## 7. Rejected options

- **A combat engine outside GAME-ABILITY-01.** The accepted gate forbids it.
- **Durable fight modes.** Tibia sends them from the client; storing them adds a Character write
  per click.
- **PvP in the first slice.** It needs skull and protection rules (PARTY-PVP-0).
- **Distance weapons in the first slice.** Ammunition use needs a DUR-03 burn cause first.
- **A new formula engine.** The spell engine already has the inputs and the level curve.
- **Replaying missed swings.** Neither Canary nor Tibia does it, and it would burst damage after a
  stall.

## 8. Decision test

- **Must decide now:** YES. Without it the M1 milestone (walk, see, fight, loot) has no fight in
  either direction.
- **Minimum sufficient:** two commands, one runtime domain, one ability for players and
  creatures, formulas on the existing engine.
- **Superseding evidence:** an official formula, or TibiaPal disagreeing with Canary.
- **Deliberately not decided:** PvP, chase movement, distance weapons and ammunition, wands and
  rods, combat effect animations and damage numbers, critical and leech values, exercise weapons,
  creature spells and distance attacks, and equipment itself (ITEM-MOVE-WIRE-1).

## 9. Before-freeze checklist

1. **Contract amendments:** none. SPELL-D7's `ATTACK_TARGET` resolves to the §4 target through
   SPELL-TARGET-1.
2. **Serialization:** runtime-only, inside the channel owner's tick; one occurrence per swing.
3. **Restart:** the target, fight modes and in-fight deadline are runtime-actor state. The
   deadline and flag survive a same-GameSession reconnect; the target is cleared; a new runtime
   actor starts without them.
4. **Typed references:** the target is the D85 identity; weapons are A12 keys.
5. **Wire:** §3, capability-gated.
6. **Split work:** one swing per due deadline, one ability invocation per swing.
