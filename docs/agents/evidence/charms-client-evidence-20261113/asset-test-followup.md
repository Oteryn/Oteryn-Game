# Asset-first creator-test follow-up

This bounded13November2026 pass follows actual test assets from both owner-
authorized Global community Discords. It corrects the earlier shallow discovery
method: clear the query first, verify its literal text, then select the visible
Contains Link filter. A zero result applies only to that query/filter. It says
nothing about all conversations, attachments, other terms or other channels.
The machine-readable query log, dates, source windows and remaining unknowns
are in `asset-test-followup.json`. No raw chat, captions or media are republished.

## Venice: historical Parry versus Freeze sample

The Kusnier `parry` plus Link search returned25 results. Among the non-trade
assets, tom's16October2023 post links [Venice's frame-by-frame Werelions
comparison](https://www.youtube.com/watch?v=x_EEmqNE1PE&t=460s). The7:40
scoreboard was actually viewed, with side attribution checked against narration:

| Creator sample | Minutes | Activations | Damage | Mean per activation (derived) |
|---|---:|---:|---:|---:|
| Freeze | 15 | 56 | 10,227 | 182.625 |
| Parry | 15 | 89 | 18,238 | 204.9213 |

Parry's sample total is78.3319% higher. The displayed60-minute rows are exactly
four times these counts and totals:224/40,908 and356/72,952. They are projections,
not a separate hour-long experiment. The spoken rough250-450 Parry range is
not the observed sample mean. At1:33 the creator is himself uncertain about
target HP; do not replace source creature data with this estimate.

This is useful primary creator evidence for sampled hunting effectiveness.
It does not isolate monster armor or mana drain, show the denominator of
eligible incoming attacks, or qualify current2026 charm-stage probabilities.
The exact upload date was not verified; the dated Discord link establishes
that this material existed by16October2023.

## Kusnier: supply and sustain comparison

Tibia Analysers `charm` plus Link returns22 results, including the actual
[Kusnier test](https://www.youtube.com/watch?v=rNSNdfQyadQ&t=129s), posted by
Bobek27February2026. At2:09 the visible labelled Hunt Analyser tables show:

| Sample | Elemental supplies | Crit supplies | Reduction (derived) | Exposure qualifier |
|---|---:|---:|---:|---|
| Grim Reapers,29min | 198,102 | 156,954 (Low Blow) | 20.7711% | 341 versus340 kills |
| Draken Walls,30min | 119,904 | 60,428 (Low Blow + Savage Blow) | 49.6030% | Raw XP1,207,766 versus1,202,340 |

Recorded damage totals are1,376,909 versus1,381,539 at Grims and2,226,410
versus2,197,846 at Draken Walls. Supply savings are not an independent damage
increase. The creator qualifies the character as about90-95 fist at4:32 and
attributes easier hunting to leech at5:15. This supports a practical sustain
comparison; it cannot determine exact leech rounding or correlated AoE RNG.
The later Parry/Dodge recommendation is explicitly opinion.

The same search exposes Gabriel's [own two-recording test and
replies](https://discord.com/channels/1431612466818977814/1431635129121505280/1500411983982497792)
from3May2026: [Low Blow/Savage Blow](https://www.youtube.com/watch?v=vkxKYG6x42I)
versus [Divine Wrath](https://www.youtube.com/watch?v=EqqDrky6xKM) on Falcons.
The full rendered conversation was read, including potion choice, imperfect
runs,60versus110ping and the bounty-talisman follow-up. Those recordings remain
an explicit next lead; this batch does not claim to have frame-audited them.

Tibia Analysers #charms has no pins. The targeted `parry`+Link query returns
zero; broader `charm`+Link nevertheless finds actual tests. This demonstrates
why a narrow failed search cannot establish missing community evidence.

## Comparison with exact Oteryn main

Compared against `3e168ce6afe374485da20ee0ac13d3749558e2f8`, freshly verified
before this successor. The videos' aggregate hunt totals cannot be reproduced
from source alone: exact builds, attack exposures, random draws and original
combat logs are not supplied. No Oteryn run or combat parity claim is made.

- `crates/gameplay/progression/src/charm_effects.rs:498` returns Parry's input
  base damage as reflection amount; the catalogue governs damage kind.
- `crates/gameplay/dispatch/src/gameplay_transport/attack/charm_combat.rs:228`
  (`damage_amount`) applies armor when the damage flag requests it, then target
  element response unless resistances are ignored. Its incoming-condition
  reflection arm at395 is guarded by `!mana`. Venice does not settle either.
- The same file199-225 adds charm bonuses to existing corresponding equipment
  skills, with zero corresponding skills left unchanged. This is a concrete
  source integration consistent with a sustain hypothesis, not proof that the
  observed supply difference arose only from charms.
- `crates/gameplay/combat-rules/src/resolution/equipped_offence.rs:93` accepts
  an explicit critical draw; its leech helper153-159 rounds the scaled
  real-damage product. These aggregate hunts cannot establish AoE draw sharing
  or half-tie behavior. Earlier Alytreta cases remain historical and separate.

All seven interaction questions listed in JSON remain open. This pass adds
usable numerical evidence and better discovery provenance; it does not label
the rest of Discord searched or the remaining questions undocumented.
