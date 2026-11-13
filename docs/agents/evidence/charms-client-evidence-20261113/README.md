# Charms: 25 visible descriptions and stage/cost observations

The first static UI pass covers all 25 catalogue names: 14 Major and 11 Minor.
All 25 visible current or locked-preview values and all 25 displayed unlock or
next-upgrade costs match the pinned source catalogue. Ten visible next-value
upgrade labels also match. The packet retains descriptions, source pointers
and private image receipts so later tests can target unqualified behavior.
No runtime or presentation fix is included.

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

The remaining unqualified work includes other stages, combat/proc execution,
Carnage area geometry behind “small radius”, leech arithmetic, Gut behavior,
reset/clear fee rules, hidden prerequisites, icon pixel parity and animations.
Later owner-authorized active tests are a separate batch; their results must
not be inferred from these static descriptions.

## Private evidence and validation

Original screenshots/assets remain private. The 25 card proofs total
5,724,422 bytes; the complete private manifest has 39 PNGs including navigation
and supporting views. `qualified-card-index.json` identifies the 25 card
proofs, while `private-image-receipts.json` retains all 39 hashes and locators.
The private archive hash is recorded in `provenance.json`. No PNG, DAT,
APNG or archive bytes are committed.

```sh
python docs/agents/evidence/charms-client-evidence-20261113/validate.py
python docs/agents/evidence/charms-client-evidence-20261113/test_validate.py
```

Optional `--repository .` rehashes the five source blobs and re-derives observed
source stage values. Optional `--private-root <directory>` rehashes all 39
private proofs. Nine control/negative tests reject missing Vampiric evidence,
duplicate keepalive entries, owned-stage promotion of locked previews,
all-stage/runtime overclaims, reset-fee substitution, currency swaps and proof
substitution. These checks do not execute the game.
