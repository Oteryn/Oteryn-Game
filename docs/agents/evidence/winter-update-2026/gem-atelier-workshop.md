# Winter Update 2026: Gem Atelier and Fragment Workshop rework

Units: WU26-08 to WU26-13. Ledger rows: P06 (Wheel), P07 (Gem Atelier, fragments, vessels,
resonance).

## 1. Official rules (thread 559, Product Manager posts #60906, #60907, #60965)

- **No domain affinity.** Any gem of a given quality fits the new gem slot of that quality.
  Existing gems lose their domain.
- **No Vessel Resonance.** Slots no longer scale with Wheel investment in a domain. Vessel
  Resonance is replaced by **Gem Vessel perks**: unlocking any Gem Vessel perk unlocks one
  new gem slot in the Atelier and gives +1 damage and healing. Gem slots come in a fixed
  order of quality. Client: "Each node grants +%1 Damage / Healing and unlocks a vessel in
  the Gem Atelier."
- **No mod dependency.** A mod is always at the grade unlocked in the Fragment Workshop,
  regardless of the other mods on the gem.
- **Mod Reshaping.** Randomly reroll one mod of an existing gem: slot 1 = 125,000 gold + 1
  lesser fragment; slot 2 = 1,000,000 gold + 2 lesser fragments; slot 3 = 5,000,000 gold +
  3 greater fragments. Client: must be inside a temple; account must be Premium; the gem
  must be unlocked first; "You need at least %1 %2 fragments" / "%1 gold ... at this
  position".
- **Atelier UI overhaul**: `Open Gem Atelier`, `Gem Vessel`, `Modify Gem`, `Replace Vessel`,
  `Select a Vessel to Modify Gems`; a gem must be removed from its vessel before it can be
  dismantled.
- **Fragment Workshop.** Each mod upgrade gives +5 hit points, mana and capacity (client:
  "+5 Health, Mana and Capacity for each mod grade unlocked"), in addition to the promotion
  point for a mod at Grade IV ("one promotion point for each fully enhanced mod"). The
  Workshop shows these bonuses.
- **New supreme mods with augments**, four per vocation, two for each of these spells:
  Knight Shield Slam, Intense Wound Cleansing; Paladin Divine Barrage, Ethereal Barrage;
  Sorcerer Death Echo, Lightning; Druid Forked Glacier, Forked Thorns; Monk Chained Penance,
  Thousand Fist Blows. (Mod kinds and values are not published.)
- **Resistance cap.** Total gem resistance against one element is capped at 30% (corrected
  from 18% in #60965); different elements cap separately; the client warns when a setup
  exceeds it. Status 2026-11-10: "Resistances cap at 30%" listed as an official reply; the
  original post said it was not yet on the test server.

## 2. Client evidence

New strings listed above plus `Promotion Points`, `Revelation Mastery %1`, "Gain additional
promotion points towards unlocking the related Revelation perk", "Reshaping a gem's mods
lets you randomly re-roll each mod of the gem.", "Empty vessel slots are available in the
Gem Atelier.", "You have imported an older Wheel of Destiny configuration. Some values may
have changed ...". Removed: `Vessel Resonance %1`, `VR %1`, Sealed / Dormant / Awakened /
Radiant Vessel (VR 0-III), `Domain Affinity`, `All Affinities`, Top/Bottom Left/Right,
"You must be inside a temple to switch the gem's domain.", "You cannot dismantle the last
gem of this domain.", "Upgrade previous mods in this gem to unlock the full benefits ...",
the resonance-match damage bonus text. No new appearance ids.

## 3. What Oteryn has today

- Gem model with domain: `crates/gameplay/progression/src/atelier.rs` (`Gem { quality,
  domain, mods, locked, vessel, ... }`; operations Reveal, Socket, Remove, SwitchDomain at
  `:243`, Lock, Dismantle at `:274` with `LastDomainGem` at `:284`-`:292`, GradeUp).
- Production mutation path: `crates/gameplay/durability/src/durability/character_atelier/actions.rs`
  (Initialize, SetLock, Reveal, SwitchDomain, Dismantle, GradeUp, with item and fee custody);
  wire capability 148, commands 308 / 309 / 424 (`crates/protocol-oteryn/src/atelier.rs:6`-`:9`).
- Vessels are a 4-element array indexed by domain (`game_character_wheel_vessels`,
  `apps/game-server/migrations/0314_character_gem_atelier_paid.sql`), checked by
  `valid_vessels` (`crates/gameplay/durability/src/durability/character_wheel.rs:1163`:
  gem domain must equal vessel domain); sockets are set through the Wheel save request.
- Resonance: Wheel conviction slots of category `vessel_resonance`
  (`rulesets/progression/wheel-of-destiny/wheel.json:805`, `:881`, ...) give stages per
  domain; `contribute_gems` activates `min(stage, mods, quality positions)` mods
  (`crates/gameplay/progression/src/wheel_effects.rs:316`-`:340`).
- Mod grade is already the mod's own Workshop grade, no dependency on earlier positions
  (`wheel_effects.rs:348`-`:355`); `gems.json` still states the old
  `minimum_of_self_and_present_preceding_mod_grades` rule (stale data, unused).
- Grade IV already adds Wheel promotion points and Revelation domain points
  (`crates/gameplay/durability/src/durability/character_wheel/presentation.rs`,
  `wheel_effects.rs` Grade IV branch).
- Basic mod HP / mana / capacity come from per-grade catalogue values
  (`crates/gameplay/progression/src/wheel_effects/gems.rs`); there is **no flat +5 per
  unlocked grade**.
- Resistance: `after_wheel_resistance` (`crates/gameplay/spell/src/spell/cast/wheel.rs:260`-`:278`)
  subtracts the summed gem resistance with no cap. **No 30% cap.**
- Absent (verified by reading the atelier, protocol and client wheel feature): Mod
  Reshaping, Replace Vessel, Gem Vessel perks with +1 damage/healing, temple requirement on
  Atelier operations, supreme mods for the ten spells. All ten spells have bundle
  definitions in `content/abilities/definitions/player-spells.json` (for example Death
  Echo's native strike in `crates/gameplay/spell/src/spell/native_delayed.rs:444`).
  `content/abilities/source-imports/unavailable-spells.jsonl` also lists rows for them; those
  are source-import conflict records, and this read did not establish that any of the ten
  is uncastable at runtime (check per spell before WU26-13).
- Client: `crates/client-ui/src/features/0148-wheel-allocation/` (vessel assignment by
  domain in `state.rs`, "activation depends on current vessel resonance" text in
  `state/gems/view.rs:75`), native `apps/client/src/features/0148-wheel-allocation/gem_*.rs`.

## 4. Gap as implementation units

### WU26-08 Server: drop domain affinity and resonance, add Gem Vessel slots (class B, L)

- Owned files: `crates/gameplay/progression/src/atelier.rs`,
  `crates/gameplay/progression/src/wheel_effects.rs` (gem contribution),
  `crates/gameplay/durability/src/durability/character_wheel.rs` (`valid_vessels`, vessel
  read/write), `crates/gameplay/durability/src/durability/character_atelier.rs` (initial
  grant), `rulesets/progression/wheel-of-destiny/wheel.json` (resonance nodes become Gem
  Vessel nodes), `gems.json`, tests.
- Rules: the Wheel's former `vessel_resonance` nodes become Gem Vessel nodes; each unlocked
  node opens one slot in a fixed quality order and adds +1 flat damage and healing; a gem
  fits any open slot of its quality; every mod of a socketed gem is active at its own
  Workshop grade; SwitchDomain and the "last gem of the domain" refusal are removed;
  dismantle still refuses a socketed or locked gem.
- Persistence (hard): new migration converting `game_character_wheel_vessels` from four
  domain cells to an ordered slot array, dropping gem domains in `state_json`, and
  retiring receipt kind `SWITCH_DOMAIN` for new writes (CHECK in 0314 kept for history).
  Session-generation fencing unchanged. Claim the migration number.
- Acceptance: `gem_fits_any_open_slot_of_its_quality`,
  `gem_vessel_node_opens_slot_and_adds_one_damage_and_healing`,
  `all_mods_active_without_resonance`, `dismantle_last_gem_allowed_when_not_socketed`, PG
  `atelier_domain_migration_keeps_gems_and_sockets` (old domain sockets map to slots).
- Depends on the open question about slot order (section 5).

### WU26-09 Wire and client for the new Atelier (class A, M)

- Owned files: `crates/protocol-oteryn/src/atelier.rs`, `crates/protocol-oteryn/src/wheel.rs`
  (vessel extension), contract under `docs/contracts/protocol-oteryn/v1/`,
  `crates/client-ui/src/features/0148-wheel-allocation/`,
  `apps/client/src/features/0148-wheel-allocation/`.
- Protocol: new capability (claim the number) without the domain field and with a slot list
  (quality, open, gem id); `Replace Vessel` = move a gem into an occupied slot in one
  request; mutate tag for Reshape (WU26-10). Old capability 148 stays for old clients
  until retired.
- Acceptance: golden capability file, `atelier_v2_reply_has_no_domain_field`, client fixture
  "Select a Vessel to Modify Gems".
- Depends on WU26-08.

### WU26-10 Mod Reshaping (class C, M)

- Owned files: `crates/gameplay/progression/src/atelier.rs` (new `Reshape { id, position }`
  action), `crates/gameplay/durability/src/durability/character_atelier/actions.rs`
  (custody), dispatch atelier wire mapping, PG case under
  `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` (new file).
- Rules: positions 1 / 2 / 3 cost 125,000 gold + 1 lesser, 1,000,000 + 2 lesser, 5,000,000
  + 3 greater fragments (lesser = Oteryn basic fragment `i46625`, greater = supreme
  `i46626`); refused outside a temple, without Premium, for a gem that is not unlocked
  (revealed); the new mod is drawn from the same position pool as the reveal draw
  (`actions.rs` reveal order) for the gem's vocation; the gem keeps its id.
- Persistence: receipt kind `RESHAPE` (new migration extending the 0314 CHECK; claim the
  number); fee and fragment burns through the existing custody records.
- Acceptance: `reshape_position_costs_match_thread_559`,
  `reshape_refused_outside_temple_and_without_premium`,
  `reshape_draws_from_the_position_pool_and_keeps_gem_identity`, PG
  `atelier_reshape_burns_gold_and_fragments_once`.
- Depends on WU26-08 (no domain in the draw).

### WU26-11 Workshop: +5 HP / mana / capacity per unlocked grade (class B, S)

- Owned files: `crates/gameplay/progression/src/wheel_effects/gems.rs` or
  `wheel_effects.rs`, presentation in
  `crates/gameplay/durability/src/durability/character_wheel/presentation.rs`, tests.
- Rule: for every Workshop grade unlocked on any mod, +5 max HP, +5 max mana, +5 capacity
  (character-wide, independent of sockets); Grade IV promotion point unchanged.
- Acceptance: `workshop_grade_adds_five_hp_mana_capacity`,
  `workshop_bonus_applies_without_socketed_gem`.
- Open: whether this replaces the per-grade catalogue HP/mana/cap of basic mods or adds to
  it (section 5).

### WU26-12 30% gem resistance cap per element (class B, S)

- Owned files: `crates/gameplay/progression/src/wheel_effects/gems.rs` (cap at
  contribution), `crates/gameplay/spell/src/spell/cast/wheel.rs` (consumer test), Wheel
  presentation flag for the client warning.
- Rule: summed gem resistance per element <= 30%; elements independent.
- Acceptance: `gem_fire_resistance_caps_at_thirty_percent`,
  `gem_caps_apply_per_element_independently`.

### WU26-13 Supreme mods for the ten new spells (class C, M; blocked)

- Owned files: `rulesets/progression/wheel-of-destiny/gems.json`, supreme effect mapping in
  `crates/gameplay/progression/src/wheel_effects/gems.rs`, spell augment consumers.
- Blocked by: mod kinds and values are not published. Before starting, confirm each of the
  ten spells is castable at runtime (see section 3).
- Acceptance (when unblocked): `supreme_mod_targets_new_spell_and_reaches_its_consumer` per
  vocation.

## 5. Open questions

1. Number of Gem Vessel nodes and the fixed quality order of slots (for example lesser,
   regular, greater, repeated), and whether +1 damage/healing is per node or per slot.
2. Whether +5 HP / mana / capacity per grade replaces or adds to the existing per-grade
   basic-mod values.
3. Supreme mod kinds and values for the ten spells.
4. How a Gem Vessel perk is reached in the Wheel (same nodes as the former resonance nodes
   is the reading of the client text "Each node grants ...").
5. Import format of "older Wheel of Destiny configuration" (client-side; Oteryn presets are
   local `OW1:` codes, `crates/client-ui/src/features/0148-wheel-allocation/settings.rs`).
