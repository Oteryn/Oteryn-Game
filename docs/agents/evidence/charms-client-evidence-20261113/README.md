# Charms: 25 visible descriptions and stage/cost observations

The first static UI pass covers all 25 catalogue names: 14 Major and 11 Minor.
All 25 visible current or locked-preview values and all 25 displayed unlock or
next-upgrade costs match the pinned source catalogue. Ten visible next-value
upgrade labels also match. The packet retains descriptions, source pointers
and private image receipts so later tests can target unqualified behavior.
The successor packet adds one authorized paid reset and restoration of the
original levels/assignments. No runtime or presentation fix is included.

Comparison source: `3e168ce6afe374485da20ee0ac13d3749558e2f8`.
The catalogue, mechanics, progression rules and description projection each
have an exact Git blob SHA-256 in `source-receipts.json`. The immutable reader
checkpoint is `checkpoint-20261113T104441Z`; its file hashes are in provenance.

## What the observations qualify

`observations-25.json` retains literal descriptions with visual line wraps
joined, the visible Unlock/Upgrade label and cost, private screenshot hash,
currency and source stage pointers. The owned/preview stage number is derived
from the unique source value and label; a locked stage-1 preview is not an
owned stage. These are observations of one state per charm, not a UI audit of
all 75 stages.

Freeze is sourced from `major-freeze-initial.png` and Vampiric Embrace from
`minor-catalogue-initial.png`. Those two initial views supply actual card
observations. Pagination images, contact sheets and the restored Scavenge
keepalive view do not add more charms. Current creature assignments, balances,
Clear charm gold fees and Reset all charms gold fees are excluded from the
static comparison.

## Findings and existing mechanics

| Classification | Finding | What already exists / limit |
| --- | --- | --- |
| PROVEN presentation gap | Cleanse projection omits random single-effect selection and 11-second immunity | Mechanics already retain selection/immunity candidates and `immunity_duration_ms=11000` as OTS hypotheses. UI corroborates the description, not the entire condition algorithm |
| PROVEN presentation gap | Adrenaline Burst projection omits the 150% speed magnitude | Catalogue duration 10000 ms and fuller mechanics already exist. UI does not prove the formula's base-speed offset, rounding or application |
| PROVEN presentation gap | Bless projection mentions experience loss but omits skill loss | Fuller mechanics already retain experience, skill tries and magic mana-spent loss domains. Relative/absolute arithmetic was not tested |
| PROVEN presentation gap | Scavenge projection mentions skinning but omits dusting | Fuller mechanics already retain skinning/dusting source candidates. The UI does not resolve success rounding or all eligibility |
| UNKNOWN | Gut says “yields 6% more creature products”; projection chooses drop chance | Existing mechanics retain `probability_or_quantity=UNKNOWN`. No chance-versus-quantity runtime difference is asserted |
| DERIVED projection string difference | The key-derived `Card.name` produces `voids call` rather than catalogue `Void's Call` | Traced source projection, not an Oteryn rendered-UI test |

The exact source path is
`crates/gameplay/dispatch/src/gameplay_transport/charm_progression/charm_details.rs`.
`findings.json` records the owning mechanics paths and the interpretation of
each finding. Simplified catalogue effects are not the complete mechanics
source; omissions in a short effect object do not establish missing runtime
implementation. No runtime defect is asserted by this packet.

The remaining unqualified work includes unexercised stages, combat/proc execution,
Carnage area geometry behind “small radius”, leech arithmetic, Gut behavior,
reset/clear fee rules, hidden prerequisites, icon pixel parity and animations.
Active results below are a distinct retained batch; they are not inferred from
the static descriptions.

## Bounded active reset and restoration

The lead performed one owner-authorized reset on the test client. Gold changed
from 72,262,029 to 66,766,779: a paid fee of 5,495,250. Major points changed
from 114/6914 to 6914/6914, and Minor echoes from 50/900 to 100/100.
Freeze stage1 then cost320 points and raised Minor capacity100→150 (+50);
stage2 cost480 and raised it150→250 (+100).

Eighteen recorded level purchases (the initial Freeze pilot plus17 restore
purchases) spent6800 Major points and850 Minor echoes, rebuilding the original
six Major and four Minor levels. Nine named assignments were restored; Fatal
Hold remained unassigned. Final balances114/6914 and50/900 and final gold
66,766,779 match the baseline configuration with only the paid reset fee spent.
These are UI state/balance readbacks, not combat or full75-stage tests.
`active-pass-qualified.json`, baseline/reset records, source stage values and
75 additional private image receipts retain the evidence.

## Reset fee source discrepancy and calculator boundary

The official [21 November2024 update article](https://www.tibia.com/news/?subtopic=newsarchive&id=8140)
provides undiscounted examples100→100000,500→4500000,1000→10000000 and
2000→21000000. They support the derived base expression
`100000 +11000*max(level-100,0)`. Its level757 base7,327,000, multiplied by75%,
matches the observed5,495,250 fee. The source was available as indexed
official-domain text; direct retrieval returned403.

Current Game `charm.rs:846` instead uses full `level*11000` when level>100,
giving6,320,250 with the same discount:825,000 more. The fee is traced from
dispatch through the native port to durability calculation and fee burn.
`reset-fee-comparison.json` retains exact paths/head; no deployed Oteryn
execution or runtime correction is claimed. First-free-reset state and every
discount/entitlement branch remain untested. The observation shows an effective
25% reduction for this event; actual entitlement identity was not captured.

The inspected Platform calculator router at its recorded default head supports
experience, stamina, training and loot, with no charm-reset arm. Targeted Atlas
search did not find a reset calculator. This is bounded source inspection,
not a claim about every historical/external calculator or a fabricated reset
calculator output.

Further test priorities are unresolved Parry/armor/mana-drain behavior, Cleanse
edge cases and Dodge/minor ordering. Existing geometry footage should be
qualified before repeating Carnage basics; Gut/Scavenge require statistical
design rather than a few kills. Community reports are leads, not live Global
runtime proof. These priorities are not new runtime findings.

## Private evidence and validation

The [primary-video comparison](primary-video-comparison.md) adds actual
Itexo/Alytreta frame observations and six bounded cases, including four
two-target arithmetic samples and a30HP-loss/258HP-heal historical example.
`validate_primary_video.py --repository .` reproduces the source arithmetic and
rehashes five blobs; `--test` rejects numeric substitutions, tie-rounding and
timer-trace overclaims. This does not qualify current Global combat behavior.

The subsequent [both-Discord pass](discord-research-comparison.md) adds actual
UI research from Tibia Analysers and Kusnier's server: ten bounded result
windows, six dated message contexts, and five invalid queries explicitly
excluded. It identifies the historical Cleanse immunity-slot conflict without
promoting it to a current Global measurement. Run `validate_discord_research.py`
and its `--test` mode as well; no original chat/media is redistributed.

The successor [public research comparison](public-research-comparison.md) and
`public-research-source-matrix.json` add17 public/owner-supplied source locators,
five bounded mechanic comparisons and nine current-main source receipts.
Official Agony/Hex constraints, historical creator evidence and unresolved
ordering/rounding/chain claims remain separated. No runtime status is promoted.
Run `validate_public_research.py --repository .` and its `--test` mode in
addition to the original packet checks below.

Original screenshots/assets remain private. The original25 card proofs total
5,724,422 bytes; the complete private manifest has 39 PNGs including navigation
and supporting views. `qualified-card-index.json` identifies the 25 card
proofs, while `private-image-receipts.json` retains all 39 hashes and locators.
The active batch adds75 private image receipts (114 total across both batches).
The original private archive hash is recorded in `provenance.json`. No PNG, DAT,
APNG or archive bytes are committed.

```sh
python docs/agents/evidence/charms-client-evidence-20261113/validate.py
python docs/agents/evidence/charms-client-evidence-20261113/test_validate.py
```

Optional `--repository .` rehashes nine source blobs and re-derives observed
source stage values. Optional `--private-root <directory>` rehashes all114
private proofs. Sixteen control/negative tests reject missing Vampiric evidence,
duplicate keepalive entries, owned-stage promotion of locked previews,
all-stage/runtime overclaims, reset-fee substitution, currency swaps and proof
substitution, inconsistent reset/restore accounting, lost purchase actions,
wrong restored assignments and unsupported first-free-reset claims. These
checks do not execute the game.
