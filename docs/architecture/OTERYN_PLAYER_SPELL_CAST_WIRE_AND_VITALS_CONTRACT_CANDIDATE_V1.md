# Player Spell Cast Wire and Own-Actor Vitals — Contract Candidate V1 (spell plan P3b)

- Date: 2026-09-27
- DecisionStatus: `CANDIDATE` (needs owner acceptance; nothing here is accepted)
- Scope: the first connected player spell cast. It covers the client cast intent, the cast
  result, observation of own-actor vitals, and where cast inputs come from on the server.
- Authority: none. This document changes no protocol or resource registry, proto file, schema, DDL,
  client or production state.
- Parent documents:
  - [`OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`](OTERYN_SPELL_AUTHORING_SCHEMA_V1.md) §9, step P3b;
  - accepted GAME-ABILITY-01 baselines;
  - VSL-COMBAT-01 §4, §14 and §24;
  - FIRST-CONTROL-WIRE-V1 (#642, owner acceptance 5853424280).

## 1. What exists

The server-side spell core (P3a, `apps/game-server/src/spell/`) is isolated and not composed:

- `authoring.rs` loads a candidate spell bundle into a `SpellDefinition`.
- `SpellBook` finds a spell by its spoken words (`"param` form included) or by its rune item.
- `resolve_cast` runs the Canary `Spell::playerSpellCheck` order: group cooldown, spell cooldown,
  secondary group, level, magic level, mana, soul, learned or vocation, premium. It then
  evaluates the `player_expression` formulas and draws each magnitude. It returns a
  `CastResolution`: mana spent, soul spent, resolved effects and the new cooldowns. **Nothing is
  applied yet.**
- `plan.rs` hands damage and heal to the one GAME-ABILITY pipeline as an `EffectPlan`, with an
  atomic commit group keyed by the occurrence. Condition removal and conjure are returned beside
  it, unchanged.

The only live client command is `WORLD_ACTOR_STEP_INTENT`. No player HP, mana, soul, vocation or
magic level exists at runtime. Visibility carries the own actor only (`MOVE-RL-11`, max 1).

## 2. Binding constraints this candidate keeps

- **One pipeline** (GAME-ABILITY-01, VSL-COMBAT-01 §4). The client proposes a typed intent. It
  never writes HP or mana, never decides range or legality, never starts or clears a cooldown,
  and never declares a result. The server resolves the target through the Target Resolver and
  checks legality before the Effect Plan.
- **Explicit commit anchors** (cast/channel/commit baseline). Every cost and cooldown names its
  anchor.
- **Cooldowns are keyed typed state** with one owner (cooldown baseline). The ready time comes
  from simulation time, never from a client or wall clock.
- **Protocol IDs** belong to the VSL-COMBAT-01 child E protocol owner, serialized with the other
  protocol writers. This document proposes IDs; only that owner assigns them.
- **FIRST-CONTROL-WIRE-V1 pattern:**
  - typed proto3 payloads inside FND-02 envelopes;
  - zero or unknown enum values and unknown fields fail closed;
  - a small byte bound per payload;
  - the result reports an outcome, never state;
  - state travels only through a state domain with its snapshot and delta.
- **Product values.** Spell values come from the authoring schema: S3 (wiki decides), S4
  (Canary = Crystal) and S11 (official news, then the newer wiki). They are *candidate values
  with provenance*. VSL-COMBAT-01 §24.5 still requires owner acceptance before any of them
  becomes an Oteryn product value.

## 3. Proposed wire (for the protocol owner)

Proposed IDs are the next free ones on the current registry: command type 2, and state domain 2
with delta type 1 and snapshot type 1. The owner may pick others.

```proto
// Proposed file: docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto (not created).
enum SpellTargetIntent {
  SPELL_TARGET_INTENT_UNSPECIFIED = 0;
  SPELL_TARGET_INTENT_NONE = 1;          // self or area spells; the server derives the target
  SPELL_TARGET_INTENT_ATTACK_TARGET = 2; // the actor's current attack target, as the server holds it
}

// ClientCommand.payload of the proposed command type 2 WORLD_ACTOR_SPELL_CAST_INTENT. At most 8 bytes.
message WorldActorSpellCastIntentV1 {
  // 1-based index into the spell book of the loaded content generation. The index, not the words:
  // no free text in the first child, and the server looks the spell up in O(1).
  uint32 spell = 1;
  SpellTargetIntent target = 2;
}

enum SpellCastDisposition {
  SPELL_CAST_DISPOSITION_UNSPECIFIED = 0;
  SPELL_CAST_DISPOSITION_CAST = 1;
  SPELL_CAST_DISPOSITION_COOLING_DOWN = 2;    // spell, group or secondary group
  SPELL_CAST_DISPOSITION_LEVEL_TOO_LOW = 3;
  SPELL_CAST_DISPOSITION_MAGIC_LEVEL_TOO_LOW = 4;
  SPELL_CAST_DISPOSITION_NOT_ENOUGH_MANA = 5;
  SPELL_CAST_DISPOSITION_NOT_ENOUGH_SOUL = 6;
  SPELL_CAST_DISPOSITION_NOT_AVAILABLE = 7;   // not learned, wrong vocation or premium
  SPELL_CAST_DISPOSITION_TARGET_REQUIRED = 8;
  SPELL_CAST_DISPOSITION_TARGET_ILLEGAL = 9;  // range, line of sight, floor, protection zone
  // Unknown spell index, stale binding or ineligible actor.
  SPELL_CAST_DISPOSITION_REJECTED = 10;
}

// CommandResult.payload of command type 2: outcome only. At most 4 bytes.
message WorldActorSpellCastResultV1 {
  SpellCastDisposition disposition = 1;
}

// StateDelta.payload of the proposed domain 2 ACTOR_VITALS, delta type 1, and
// StateDomainSnapshot.payload of snapshot type 1. Own actor only. At most 32 bytes.
message ActorVitalsV1 {
  uint32 health = 1;
  uint32 max_health = 2;
  uint32 mana = 3;
  uint32 max_mana = 4;
  uint32 soul = 5;
}
```

How the proposed result maps from the core's `CastRejection`:

| `CastRejection` | Proposed disposition |
|---|---|
| `GroupCooling`, `SpellCooling` | `COOLING_DOWN` |
| `LevelTooLow` | `LEVEL_TOO_LOW` |
| `MagicLevelTooLow` | `MAGIC_LEVEL_TOO_LOW` |
| `NotEnoughMana` | `NOT_ENOUGH_MANA` |
| `NotEnoughSoul` | `NOT_ENOUGH_SOUL` |
| `NotLearned`, `VocationCannotUse`, `PremiumRequired` | `NOT_AVAILABLE` |
| `TargetRequired` | `TARGET_REQUIRED` |
| Target Resolver or legality failure | `TARGET_ILLEGAL` |
| `TimeOverflow`, `Formula` | `REJECTED` (server fault, logged; no state change) |

Why these choices:

- **An index, not spoken words.** The reference servers cast through talk, which Oteryn has no
  command for. The spoken-word lookup stays on the server for a later talk command. A typed index
  keeps the payload bounded and parse-free.
- **No parameter field in V1.** Parameter spells (`exura sio "name"`, `utevo res "creature"`)
  need a player-name or creature-name resolver. A later `optional` field can add it.
- **Cooldowns are not in V1 state.** `COOLING_DOWN` is enough to play. A cooldown state domain
  is a later additive domain, not a V1 requirement.

## 4. Server side: cast inputs and owners

| Input | Owner | First child |
|---|---|---|
| Vocation, level | GAME-CHAR (R7 P03 progression) | Read from the admitted Character; never from the client. |
| Magic level, skills | GAME-CHAR progression | Needs the same Character-owned initialization/readiness gate as VSL-COMBAT-01 §24.1. Until it passes, casting stays gated. |
| Learned spells, premium | GAME-CHAR and Platform entitlements | V1 serves spells without `learning_required`; premium follows PROD-ENTITLEMENTS-01. |
| HP, mana, soul (current and max) | Current ChannelRuntime (runtime actor owner, as VSL-COMBAT-01 §6 for creatures) | Session-local and non-surviving, stated explicitly as the cooldown baseline allows. Durable vitals come later through DUR-02. Max values per vocation and level are **PRODUCT inputs**; candidate values come from Canary/Crystal `vocations.xml` with provenance. |
| Cooldowns | Current ChannelRuntime, keyed by (actor, spell) and (actor, group) | Session-local; `SemanticTimeMicros` from the owner clock. |
| Damage and heal draw | SIM determinism: RNG stream bound to the occurrence | `uniform_draw` over the owner stream; retry never redraws. |

## 5. Commit anchor (Reference behaviour)

Canary `47dfd51f` and Crystal `ff7ede593` run the same sequence in
`InstantSpell::playerCastInstant` and `RuneSpell::executeUse`:

1. `executeCastSpell` for an instant spell, `internalCastSpell` for a rune.
2. Only when it succeeds, `postCastSpell`, which removes mana and soul, adds `CONDITION_SPELLCOOLDOWN`
   and `CONDITION_SPELLGROUPCOOLDOWN`, and awards mana-spent progress.

Proposed policy: mana, soul, spell cooldown and group cooldowns commit **at PRIMARY COMMIT, in
the same owner mutation as the Effect Plan**. A cast that fails before commit consumes nothing.
No reservation is needed, because a single Channel owner lane serializes the actor's casts.
Mana-spent progress (magic-level training) is a separate GAME-CHAR descendant and is out of V1.

## 6. What V1 can and cannot cast

| Spell shape | V1 | Blocker |
|---|---|---|
| Self heal and condition removal (`exura`, `exura ico`, `exana pox`) | **yes** | none beyond this contract, vitals and the Character gate |
| Targeted damage (`exori frigo`, `exori vis`) | no | Visibility of other actors (`MOVE-RL-11` = 1), an attack-target command, and the VSL-COMBAT-01 creature slice |
| Area damage | no | Same as above, plus area targeting |
| Rune use (`sudden death`, `great fireball`) | no | A use-with command and GAME-ITEM inventory, plus a DUR-03 charge decrement |
| Conjure (`adori gran mort`) | no | DUR-03 MINT, plus TRANSFER or consumption of the blank rune |
| Party, summon, support with conditions | no | S6–S10 of the authoring schema are still proposed |

The first child is therefore **self heal**. It is the smallest real cast: it exercises the
intent, the core check order, a formula draw, the Ability commit, the anchor, and vitals
observation. It touches no item or durable value. Targeted damage follows as soon as
VSL-COMBAT-01 child E provides other-actor visibility and an attack target.

## 7. Proposed resource limits (for the resource owner)

Same row format as `MOVE-RL-02` and `MOVE-RL-11`:

| Proposed ID | Resource | Hard maximum | Failure |
|---|---|---|---|
| `SPELL-RL-01` | Spell cast inputs applied per actor per Channel owner work cycle | 1 | Further inputs stay outstanding FND-02 commands (`FND02-OUTSTANDING-COMMANDS`) |
| `SPELL-RL-02` | Ability effects in one cast Effect Plan | 2 (the current `EffectPlan` bound) | `CAPACITY_EXCEEDED` before commit |
| `SPELL-RL-03` | Actors in one `ACTOR_VITALS` snapshot or delta | 1 (own actor) | Not encodable |
| `SPELL-RL-04` | Spells in one content-generation spell book | measured when proposed; the census has 252 | Admission fails closed |

## 8. Owner decisions requested

1. **D1.** Accept a typed spell-index intent (command type 2) with the target-intent enum, instead
   of a free-text talk command, for V1.
2. **D2.** Accept the `ACTOR_VITALS` own-actor state domain, with runtime HP, mana and soul as
   session-local and non-surviving in V1.
3. **D3.** Accept the commit anchor in §5: costs and cooldowns are paid at PRIMARY COMMIT, and
   nothing is paid on failure.
4. **D4.** Accept self heal as the first connected spell child. Targeted, rune and conjure spells
   wait on the blockers named in §6.
5. **D5.** Select the product source for max HP, mana and soul per vocation and level. The
   candidate is Canary/Crystal `vocations.xml` with provenance, as VSL-COMBAT-01 §24.5 requires.
6. **D6.** Route the proposed `SPELL-RL-*` rows to the resource owner.

## 9. Delivery after acceptance (each child with its own allocation)

1. **Registries and codecs.**
   - Protocol and resource registry entries, and the proto file.
   - Strict codecs with independent exact-byte fixtures.
   - Pattern: first-control-wire M1.
2. **Composition.**
   - Channel owner vitals and cooldown state.
   - The Character-sourced `CasterState`.
   - `resolve_cast` → `effect_plan` → Ability commit, with anchor payment in the same mutation.
   - Result and vitals delta publication.
   - Pattern: first-control-step M2.
3. **Native client.** Spell bar or hotkey intent, result feedback and a vitals bar.
4. **Qualification.**
   - In the existing native entry room.
   - Cast, cooldown rejection, mana rejection, and retry with no second payment.
   - Keep the start/east step-and-return proof cells.

## 10. Not decided here

- Exact rejection texts.
- Cast animations and effects.
- Exhaustion between different groups beyond the authored cooldowns.
- Durable persistence of vitals and cooldowns.
- Mana and health regeneration (conditions S8).
- Magic-level training.
- PvP rules.
- Any spell value as an Oteryn product value.
