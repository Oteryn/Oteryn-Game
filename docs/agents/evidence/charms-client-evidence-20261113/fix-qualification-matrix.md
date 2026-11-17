# Runtime-fix qualification and bounded Cleanse follow-up

This evidence worker remains scoped to PR3172 documentation. A separate worker
owns any authorized implementation. Source comparison remains pinned to main
`3e168ce6afe374485da20ee0ac13d3749558e2f8`; no runtime edits or game actions
occur in this batch. The JSON matrix lists12 bounded qualification decisions.

## Strongest implementation candidate

The [official21November2024 article8140](https://www.tibia.com/news/?subtopic=newsarchive&id=8140)
was **read directly in the browser**, improving on the earlier indexed route
whose direct web fetch returned403. Its examples are100→100,000;500→4,500,000;
1000→10,000,000;2000→21,000,000gold. These establish the excess-over100 offset:
`100000+11000*max(level-100,0)`. The per-level prose alone is less explicit than
these examples, so retain both route and derivation qualification.

At757 the derived base is7,327,000; a75% payable fraction gives5,495,250,
matching the recorded client transaction. Current `reset_all_fee` in
`crates/gameplay/domain/src/domain/charm.rs:846` instead uses the full level
above100 and gives6,320,250 under the same fraction. The825,000 discrepancy is
a narrow source-backed fix candidate. Preserve free-reset and actual discount
handling; test100/101 and all published examples. This does not qualify every
entitlement or future economy policy.

## Cleanse: official constraint versus trace candidate

The [official manual](https://www.tibia.com/gameguides/?section=characters&subtopic=manual)
Agony constraint is stronger than a forum opinion: Agony is not curable,
blockable or resistible. The complete source trace now **closes the earlier
candidate as already correct**. `is_negative` in
`crates/gameplay/domain/src/domain/condition.rs:137` excludes Agony before
`prepare_cleanse` selection. The existing
`agony_is_never_selected_by_cleanse_or_dispel_and_immunity_does_not_refuse_it`
test in `crates/gameplay/foundation/src/foundation/condition_gate_tests.rs:295`
asserts both nonselection and immunity bypass. This worker independently read
the selector and assertions; the implementation worker reports the existing
test passed. No Agony patch is needed. The earlier packet's incomplete
prepare_cleanse-only trace is superseded by this result.

The [25August2026 update8935](https://www.tibia.com/news/?subtopic=newsarchive&id=8935)
adds Hex removal and immunity. Newly discovered [8September2026 ticker8960](https://www.tibia.com/news/?subtopic=newsarchive&id=8960)
explicitly confirms Hex immunity after a later correction. **8960 was read
directly on tibia.com**, following the actual news link discovered on Kusnier's
server. Do not infer an exact Hex/IntenseHex representation or reproc algorithm
from these announcements. Trace native condition kinds and current support
before inventing an absent implementation.

## Both-Discord follow-up actually performed

Tibia Analysers server-wide literal `cleanse` returned one result. The
[20August17:03 question](https://discord.com/channels/1431612466818977814/1431633212995670078/1540013303617687602)
asks about Root. Its [17:53 image reply](https://discord.com/channels/1431612466818977814/1431633212995670078/1540025997083152406)
was expanded and visually read: Gabriel supplies a mobile screenshot of the
TibiaWiki Cleanse article, listing Rooted/Feared and excluding skill debuffs.
This is an actual secondary-source answer, not an unanswered question and not
Gabriel's own combat test. The screenshot predates later official Hex updates;
do not use its list to exclude Hex.

Kusnier server-wide literal `cleanse` with the visible Contains Link filter
returned6 results. All were read: news8935, news8960, tom's8935 link, and three
unrelated wheel-planner posts mentioning Wound Cleansing. The8960 link was
followed through NabBot to the official page. No hidden Discord API, messages
or token access were used. These bounded queries do not exhaust either server.

## Remaining evidence is not patch authority

Itexo2020 replacement/reproc and Alytreta2022 AoE/overkill observations remain
historical. Venice2023 measures hunt effectiveness, not armor/mana isolation.
Current Global community LowBlow assertions, Carnage analogies, and Wound
CryptStrike versus weapon-pierce distinctions remain classified by context.
Kusnier/Gabriel2026 hunt counters are useful performance evidence but cannot
establish isolated RNG, exact leech rounding or same-hit processing order.

The matrix therefore promotes no runtime rule from those cases. It supplies
one narrow numerical fix candidate, one closed already-correct Agony result,
one remaining Hex representation trace and explicit
remaining questions, while preserving evidence that further material may exist.
