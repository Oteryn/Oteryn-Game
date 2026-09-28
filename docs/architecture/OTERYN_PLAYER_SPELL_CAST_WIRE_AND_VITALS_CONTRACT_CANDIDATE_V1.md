# Player Spell Cast Wire and Own-Actor Vitals — Contract Candidate V1 (spell plan P3b)

- Date: 2026-09-27
- DecisionStatus: `ACCEPTED WITH CHANGES` for SPELL-D1 to SPELL-D6 (owner, 2026-09-28; architect
  verdicts in #162 comment 5867161696; §8). SPELL-D7 (owner decision D89, #162 comment
  5875958040) amends the target intent (§3, §8.1). Each delivery child still needs its own allocation and
  independent review.
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

The protocol owner (VSL-COMBAT-01 child E) assigns the IDs. Command type 2 and state domain 2 are
already taken by USE-WIRE-V1 (#1066). On `main@0f80b8c` the next free ones are command type 3, and
state domain 3 with delta type 1 and snapshot type 1 (SPELL-D1, SPELL-D2). The names below use
those numbers only as proposals.

```proto
// Proposed file: docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto (not created).
enum SpellTargetIntent {
  SPELL_TARGET_INTENT_UNSPECIFIED = 0;
  SPELL_TARGET_INTENT_NONE = 1;          // self or area spells; the server derives the target
  SPELL_TARGET_INTENT_ATTACK_TARGET = 2; // the actor's current attack target, as the server holds it
  SPELL_TARGET_INTENT_POSITION = 3;      // SPELL-D7: a world position, for cast_at_position spells
}

// SPELL-D7: a position in the actor's own Channel, in the pinned frame's native coordinates.
message SpellTargetPositionV1 {
  sint32 x = 1;
  sint32 y = 2;
  // Limited to the int16 range.
  sint32 floor = 3;
}

// ClientCommand.payload of the proposed command type 3 WORLD_ACTOR_SPELL_CAST_INTENT. At most 8 bytes
// before SPELL-D7; with the SPELL-D7 fields at most 32 bytes.
message WorldActorSpellCastIntentV1 {
  // 1-based index into the spell book of the loaded content generation. The index, not the words:
  // no free text in the first child, and the server looks the spell up in O(1).
  uint32 spell = 1;
  SpellTargetIntent target = 2;
  // SPELL-D7: present only with SPELL_TARGET_INTENT_POSITION; otherwise absent.
  SpellTargetPositionV1 target_position = 3;
  // SPELL-D7: per cast, stateless. Honoured only for a spell with targeting.aim_at_target.
  bool aim_at_target = 4;
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

// CommandResult.payload of command type 3: outcome only. At most 4 bytes.
message WorldActorSpellCastResultV1 {
  SpellCastDisposition disposition = 1;
}

// StateDelta.payload of the proposed domain 3 ACTOR_VITALS, delta type 1, and
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
- **The index is revision-local and canonical (SPELL-D1).** It is the 1-based position of the spell
  in the spell book of one content generation, ordered by `ProductionKey` in ascending UTF-8 byte
  order. Both peers derive it from the same content-generation artifact, which the client
  identifies by the 32-byte `content_generation` carried in its `WORLD_SPATIAL_VISIBILITY` snapshot.
  A client that does not hold that exact generation sends no spell intent. The server resolves the
  index only against the admitted session's content generation; a stale or unknown index is
  `REJECTED`. It is never persisted or logged as
  spell identity: audit and logs use the spell's `ProductionKey`. Zero or unknown enum values and
  unknown fields fail closed.
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
| HP, mana, soul (current and max) | Current ChannelRuntime (runtime actor owner, as VSL-COMBAT-01 §6 for creatures) | Runtime-actor-local and non-durable, stated explicitly as the cooldown baseline allows (SPELL-D2): the values belong to the runtime actor, not to a GameSession, and are lost only when the actor ends. Vitals start at the current maximum only when a new runtime actor is created (the actor was absent). Any session that attaches to an existing present actor keeps its vitals exactly: a same-GameSession reconnect, and the post-grace new-GameSession recovery of FND-04B §21 ("no heal, refill"). A fresh admission after the actor is gone restarts at the maximum. Durable vitals through DUR-02 are required before any external or production evaluation. Max values per vocation and level follow SPELL-D5: the official tibia.com library, then the wikis decide where they state the per-vocation base and per-level gains (S15, S3, S11, S13, S14), Canary/Crystal fill only what they do not state (S4), every value keeps its provenance, and a remaining conflict goes to the owner. |
| Cooldowns | Current ChannelRuntime, keyed by (actor, spell) and (actor, group) | Runtime-actor-local and non-durable; kept across a same-GameSession reconnect and FND-04B §21 recovery (no cooldown reset); `SemanticTimeMicros` from the owner clock. |
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
| Party, summon, support with conditions | no | S6–S10 are decided (2026-09-28), but not implemented: each shared `native_behavior` key needs its owner and tests (S7), Wheel-gated spells need a Wheel owner (S6), summons need GAME-AI-01, party buffs need the party service, and conditions need their Effect operations |

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
| `SPELL-RL-04` | Spells in one content-generation spell book | a finite measured value, set by the resource owner at registration with max+1 tests (the census has 252) (SPELL-D6) | Admission fails closed |

## 8. Owner decisions (2026-09-28)

The owner accepted the architect verdicts in #162 comment 5867161696. The candidate's D1-D6 are recorded as
SPELL-D1 to SPELL-D6, because D1-D52 are already used in the owner-decision register.

| # | Decision | Verdict and change |
|---|---|---|
| SPELL-D1 | Typed spell-index intent with the target-intent enum, not a free-text talk command, for V1. | **Accepted with change.** The protocol owner assigns the command type (type 2 is `USE_INTENT`; next free is 3). The index is canonical per content generation: the 1-based position in `ProductionKey` ascending UTF-8 byte order, derived by both peers from the generation named by the snapshot's `content_generation`. It is `REJECTED` when stale or unknown, and never persisted or logged as identity (`ProductionKey` is). Unknown enum values and fields fail closed. |
| SPELL-D2 | `ACTOR_VITALS` own-actor state domain; runtime HP, mana and soul non-durable in V1 (proposed as session-local; recorded as runtime-actor-local, see verdict). | **Accepted with change.** The protocol owner assigns the domain (domain 2 is `WORLD_OBJECT_OVERLAY`; next free is 3). Vitals are runtime-actor-local and non-durable, not session-local. Vitals start at the maximum only when a new runtime actor is created. Any session attaching to an existing present actor keeps them exactly: a same-GameSession reconnect, and FND-04B §21 post-grace recovery with a new GameSessionId. A fresh admission after the actor is gone restarts at the maximum (declared V1 limitation). Durable vitals under DUR-02 are required before external or production evaluation. |
| SPELL-D3 | Costs and cooldowns paid at PRIMARY COMMIT in the same owner mutation as the Effect Plan; nothing paid on failure (§5). | **Accepted.** A retry with the same FND-02 CommandId never executes or charges the cast a second time: it returns the original result while FND-02 still retains it, and `COMMAND_OUTCOME_EXPIRED` with reconciliation once the bounded retention has evicted it (FND-02 §13.2). |
| SPELL-D4 | Self heal is the first connected spell child (§6). | **Accepted.** Composition (§9 step 2) stays behind the Character progression initialization/readiness gate (VSL-COMBAT-01 §24.1); registries and codecs (§9 step 1) do not. |
| SPELL-D5 | Product source for max HP, mana and soul per vocation and level. | **Accepted with change.** The same source rule as spells: the official tibia.com library, then the wikis decide where they state the values (S15, S3, S11, S13, S14); Canary/Crystal `vocations.xml` fills only what they do not state (S4); provenance is kept; a conflict goes to the owner. These become the V1 product input for vitals only (VSL-COMBAT-01 §24.5). Owner sub-decisions of 2026-09-28, applied in `tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json` (#1093): **D5a** the soul maximum follows the account type as the wiki states (free 100, premium 200; Platform owns the account type); **D5b** where Canary and Crystal differ on monk and exalted monk regeneration, the Canary 15.30 branch decides. The V1 soul maximum is 100 for every account. Accepting `PROD-ENTITLEMENTS-01` does not activate Premium: Premium/VIP activation stays unauthorized (architecture README) until an explicit, product-specific Premium activation or transition decision is in force. That decision must define when the maximum is re-evaluated for a running actor (PROD-ENTITLEMENTS-01 §6.4 makes expiry and revocation effective for running sessions) and whether current soul above a lowered maximum is clamped; composition must not assume either. |
| SPELL-D6 | Route the `SPELL-RL-*` rows to the resource owner (§7). | **Accepted with change.** RL-01 = 1, RL-02 = 2, RL-03 = 1. RL-04 must be a finite measured value at registration. The rows go into `RESOURCE_LIMITS_REGISTRY.json` under its single-writer lease, serialized with B4 (#513). |

### 8.1 SPELL-D7: cast at position and aim at target (owner decision D89)

Owner decision D89 (#162 comment 5875958040, "Tak, jak Global") accepts the review packet in #162
comment 5875913331. Source: Tibia 15.25 targeting modes (TibiaWiki BR `aimattarget`; Canary 15.30
`spell:optionalTarget`, `applyInstantSpellDirection`, `playerCastInstant`; Fandom `Divine Grenade`).

- **Position intent.** `SPELL_TARGET_INTENT_POSITION` with `target_position` is valid only for a
  spell whose data carries `targeting.cast_at_position`; for any other spell it is `REJECTED`.
  The server checks the position against the spell's `range_tiles`, line of sight
  (`block_walls`), floor (`check_floor`) and the protection zone; a failure is
  `SPELL_CAST_DISPOSITION_TARGET_ILLEGAL`. `target_position` present with another intent, or
  absent with `POSITION`, fails closed. The client maps crosshair and cursor modes to `POSITION`
  and "at target" to `ATTACK_TARGET`; `NONE` casts at the caster's own position.
- **"At target" for a position spell.** For a `cast_at_position` spell, `ATTACK_TARGET` resolves to
  the current position of the actor's attack target, as the server holds it at the cast, and then
  applies exactly the `POSITION` checks (range, walls, floor, protection zone). No attack target is
  `TARGET_REQUIRED`. It does not use the `needs_target` path, which S20 forbids for these spells.
- **Aim at target.** `aim_at_target` is a per-cast flag, **stateless**: no persisted character
  setting and no equivalent of Canary's client opcode `0xC8`. The server honours it only for a
  spell with `targeting.aim_at_target` and only when the actor holds an attack target; it then
  turns the actor to the primary direction of that target before it resolves the direction area.
  Otherwise the flag is ignored.
- **Dispositions.** Unchanged; no new value.
- **Bounds.** The intent payload grows to at most 32 bytes; the protocol owner registers it with
  the command type (§9 step 1).
- **Until delivered.** The Game core keeps rejecting `cast_at_position` spells (fail closed), as
  today.

**Protocol IDs.** They are assigned by the protocol owner lane (VSL-COMBAT-01 §24.3 child E, Server
Seam/protocol composition), which #162 allocates with the single-writer lease on
`PROTOCOL_OTERYN_V1_REGISTRY.json` (precedent: USE-WIRE-V1 M1, #1066).

**Independent review.** Required for step 1 (protocol and security; independent exact-byte fixtures)
and step 2 (authority and state, with the high-risk authority/recovery qualification).

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
