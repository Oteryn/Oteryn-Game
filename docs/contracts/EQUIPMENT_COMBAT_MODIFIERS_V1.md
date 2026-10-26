# Equipment combat modifiers (EQMOD2)

Task authority: EQUIP-MODS-2, server combat pipeline. This extends the existing
critical/leech equipment projection and auto-attack owner; it adds no transport command,
state domain, capability or persistence schema.

The current pinned content supplies `CLEAVE_PERCENT`, `REFLECT_DAMAGE`,
`PERFECT_SHOT_DAMAGE` and `PERFECT_SHOT_RANGE`. `REFLECT_PERCENT_ALL` is the additive
content modifier discriminator 38, with a required rational percent parameter. It
corresponds to the reference item parser's `reflectpercentall`, rather than inventing
separate item attributes for each element. Flat `reflectdamage` is physical only.
Content publication must use a reader that knows the new discriminator. Existing
content bytes and protocol-oteryn messages retain their meaning.

Critical and leech retain their skill units and clamps. Cleave and reflection percent
are summed in hundredths of a percent without the critical skill's u16 clamp. Perfect
shot damage is summed separately at each exact range. The existing own-slot, untimed
contribution policy remains in force. Both hands recognize weapon/shield patterns.
Equipment custody reads include the stored forge tier under the same Character/item
locks; tier zero means no stored tier. Nonzero tiers must also pass the existing `ForgeItemShape` validation against the
current content profile and nonstackable item facts. The existing range CHECK and tier
writer remain unchanged.

Combat order and eligibility:

- A melee weapon produces independently drawn attacks at the two directional flank
  tiles before its primary attack. Each cleave is scaled before that target's own
  defence, armor and resistance. Cleaves are extension damage: no critical, fatal,
  leech, recursive cleave or charm damage callbacks. Fists, ranged weapons, protected
  tiles and secure-mode player bystanders are excluded.
- Primary weapon damage receives critical, then fatal, before target blocking. Pinned
  default onslaught chance is `0.05*tier^2 + 0.4*tier + 0.05` percent for a nonzero
  weapon tier. Boots multiply that chance by `1 + amplification / 100`, where nonzero
  boots amplification is `0.4*tier^2 + 1.7*tier + 0.4` percent. A fatal adds rounded
  60% damage. It does not amplify critical chance or flat perfect shot.
- Perfect shot applies to distance/throwing attacks at the held target. Either exact
  axis range match forces a hit. The matching flat amounts are added after blocking;
  equal axis distances count once. Wands/rods and extension attacks do not qualify.
- Incoming monster damage reflects after blocking and before HP/mana shield. The
  reflected amount is `flat + floor(blocked_damage * percent / 100)`, capped at
  `ceil(attacker_max_health / 100)`. Physical flat without percent requires adjacent
  range; percent works at any distance. Elementless/healing/mana-only operations do
  not enter this damage-reflection path. The reflected hit is neutral leaf damage,
  with no defence, leech, crit or recursive reflection. A zero physical hit can still
  reflect a flat amount.
- Leech remains based on primary HP actually removed. A reflected lethal hit credits
  the defender's bound Character lease/session and enters the existing kill queue.

Reflection replay ownership is distinct from player commands. Each creature retains a
bounded map keyed by the reflector's exact placement and the source stream, with the
source owner's strictly increasing semantic clock. At most 1024 such keys are admitted
per creature generation (`EQMOD2-REFLECTION-STREAMS`, resource allocation 1200). The
existing 16-receipt retention evicts covered reflected receipts; the stream mark still
rejects an evicted old clock. Conflicting retained replays fail closed. HP, receipt,
stream mark and removed-HP attribution install as one staged creature-slot replacement.
The player's command high-water mark never advances because of reflection. Existing
streams remain usable at capacity. New streams fail before HP mutation.

The native source owners already reject repeated or superseded source casts before
calling the reflection hook. Source stream clocks must remain strictly increasing;
callers must not reuse a stream for unrelated operations at the same clock.

Equipped PvP targets still fail the existing `unarmoured_pvp_target_in_transaction`
admission gate. This task does not broaden that authority. Spell magnitude's strict
advanced-equipment qualification, timed item charge consumption, quivers and client
presentation remain with their existing owners; parity is not claimed for those paths.

Qualification: `eqmod2` unit tests in content, combat-rules and entities, plus
`eqmod2_postgres_content_cleave_reflection_perfect_shot_and_amplification` in the
auto-mounted dispatch library PostgreSQL tree. The PG case sends a framed command
through `serve_admitted`, reads actual fenced equipment/content, runs both combat
owners, checks primary-only leech, then exercises the ranged cost/damage path.

Pinned parity sources:
[Canary combat.cpp:2652-2756](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/combat/combat.cpp#L2652),
[weapons.cpp:255-315,536-585,706-731](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/weapons/weapons.cpp#L255),
[game.cpp:7940-7980,8494-8526](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/game/game.cpp#L7940),
[item_parse.cpp:953-980](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/functions/item/item_parse.cpp#L953),
[config.lua.dist:195-209](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/config.lua.dist#L195).
Crystal confirms the same primary behaviors in
[weapons.cpp:608-656,782-806](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/weapons/weapons.cpp#L608),
[game.cpp:7296-7320,7964-7998](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L7296),
[combat.cpp:2704-2713](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/combat/combat.cpp#L2704),
[item_parse.cpp:943-970](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/functions/item/item_parse.cpp#L943).
