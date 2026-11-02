# PREY-DIFF authoring and model

`difficulty.json` is the owner-authored input. Five-tier boundaries, minimum levels,
list sizes, price multipliers, the defensive time policy and model presets are data.
The initial thresholds and benchmark presets are proposals awaiting the answers on
draft PR #2797. The default defensive policy is `dealt_or_attacked` (V2), as required
by OWNER-DEVIATION PREY-DIFF, 2026-11-01.

## Reproduce

From the repository root, with the repository Rust toolchain available:

```sh
cargo fmt --all
python3 tools/content-migration/prey_difficulty.py --simulate
python3 tools/content-migration/register_spell_families.py
python3 tools/content-migration/prey_difficulty.py --check
python3 tools/content-migration/register_spell_families.py --check
python3 tools/content-migration/test_prey_difficulty.py
cargo test --locked -p oteryn-gameplay-dispatch --example prey_time_sim
```

The normal `regenerate_content.py` pipeline rebuilds source shards first. The world
converter then prepares a fresh native Creature provider through the existing
registration producer with only the Prey injection disabled, runs the simulation
when its bound inputs changed, and registers the resulting rules. The Rust reward
qualification reads that same fresh temporary provider. It never bootstraps from a
stale generated Creature artifact. Run the repository's world pin refresh after staging generated content,
as required by its existing content workflow. Never edit a generated manifest, pin,
classification, value table or report by hand.

`--model-input <path>` exports the source facts without running Cargo. `--classify`
exports only the initial tier census, before the compiled reward-owner qualification;
the normal generation replaces that census with the runtime-qualified counts.

The Rust example writes `tools/content-schema/native-gameplay/prey-model-results.json`.
The Python owner writes `content/creatures/prey-tiers.json`,
`content/creatures/prey-slot-values.json` and both dated `docs/balance/PREY-*.md`
reports. Registration embeds the combined `PreyRules` in the native Creature provider
before its digest and the content manifest are computed. The runtime therefore reads
rules from its pinned active generation.

The model output is bound to the effective model inputs, native Creature source,
reward-admission input and the combat, clock and duration owners. A changed input
requires a new simulation; stale results fail generation. A tuning change which does
not affect combat (such as M, F, minimum levels or list sizes) can reuse the same
combat result, but still requires content regeneration and activation of the new pin.

## Values and units

The runtime table stores `loot_per_kill_micros` and `kills_per_hour_micros` as unsigned
integers with six fractional decimal places. `npc_sell_prices` stores whole gold per
item. Coin denominations are read from the existing currency owner. NPC prices are
read from `BuyFromPlayer` offers; `maximum_catalogue_offer` is the pending proposed
policy, and `minimum_catalogue_offer` is also supported. An item without an NPC offer
has zero NPC sale value. Market prices are never used.

The prompt defines estimated loot as source chance × average quantity × NPC price.
The generator evaluates that expression with exact fractions, including conditional
nested loot, and truncates only when storing micro-gold. It does not claim to reproduce
the separate runtime loot planner's dynamic factors, modifiers or admission failures.
The runtime price owner uses the existing paid-list quote and Prey duration, with M
and F represented in parts per million, and truncates the final nonnegative price to
whole gold like the list quote.

## Model limits

Each tier has a reproducible effective-stat knight benchmark from the repository's
existing auto-attack parity grid and known Item facts. Source equipment restrictions
are checked. Unknown extra defense is not credited. This is a proposal for the first
measurement, not evidence of empirical player equipment or complete Item admission.

The three model routes use source spawn identities, cells and respawn delays. They
compare single-race hunting, three-race mixed hunting and a hunt with a source boss
conditioned to be present once. They are synthetic routes assembled from real source
points; they do not prove that those points form a walkable live hunt. The reports give
the chosen-race kill share instead of assuming it is exactly one third.

Combat calls the production formula, defense, armor, mitigation, resistance,
deterministic draw and activity-clock owners. All four time policies see the same
unboosted physical auto-attack stream. The benchmark assumes sustained survival and
omits travel, healing cost, area spells, kiting, critical hits and non-melee monster
spells. Missing native melee profiles and physical targets not killed within the
configured duel horizon are reported explicitly; their facts are never invented.
Zero physical throughput is a result of this bounded benchmark, not a claim that no
other build could kill that creature.

The existing minute clock advances across activity gaps of at most a minute. It is
not a sum of attack animation lengths and is not scaled by kill share. Reports mark
an unexhausted Prey as a bounded observation instead of inventing a finite duration.
After release, the durable measured kill and committed-loot counters provide the
replacement evidence for these estimates.
