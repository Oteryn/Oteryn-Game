# Bestiary: client source inventory and initial UI gaps

The client source has 845 Bestiary creature records and 21 classes. The 833
records already imported from client 15.30 are byte-identical in the retained
15.33.dd0955 file; twelve records were added. Re-reading those 833 names and
looks from the UI would duplicate existing evidence. This packet records the
source comparison and the remaining UI work, without changing gameplay data.

Comparison commit: `3e168ce6afe374485da20ee0ac13d3749558e2f8`.
Client staticdata SHA-256:
`bd1f4c30c3e03268a315caf14050ea584aca8c241831422c41f4e7f264dcaa02`.
The lead verified that hash against the file belonging to the running Ubuntu
test client. Source observations are compatibility evidence, not Oteryn
identity, admission or runtime authority.

## Retained results

| Classification | Observation | Boundary |
| --- | --- | --- |
| PROVEN | 845 source records; 21 classes; 833 unchanged raw record hashes; 12 added; none removed | Strict old reader functions accepted current record shapes; the old exact-count publication function was not used to relabel this as a 15.30 import |
| PROVEN | 1,865 canonical Creature definitions; 824 have a Bestiary block | Source presence does not establish effective runtime admission or UI equality |
| DERIVED | 813 rows have a single name/outfit candidate with Bestiary; 32 need binding review | These are not established client-ID to ProductionKey mappings; same-name variants remain explicit |
| DERIVED | Outfit frame metadata found for 844 rows; Cake Golem uses another look form | This is metadata coverage, not a missing-graphics finding or pixel/animation parity |
| PROVEN | UI class totals sum to 845 | Known counts are a snapshot of one character's progress |
| PROVEN | Rat control values and thresholds match the retained profile | One card, not all 845 cards |
| CONFLICT | Rat UI has a detailed Location(s) list; canonical profile has a general description | Full literal transcription and resolution remain pending; no content correction is applied |

`staticdata-delta.json` retains the twelve added names: omega bulwark, omega
bastion, omega battler, omega moonbreaker, omega cinderflayer, omega rampart,
giant rat, greedcarrier rat, hoarder rat, greed elemental, greedweaver and
greater greed. Raw integers `f4` through `f7` remain raw. No inferred meaning is
silently promoted to a verified current schema field.

The dedicated `content/creatures/bestiary/index.json` is marked
`READY_UNPOPULATED`, but the 824 Bestiary blocks exist under canonical Creature
definitions. The directory marker alone must not be reported as missing data.

## Initial UI control

Rat shows Health 20, Experience 5, Speed 67, Armor 1, Mitigation 0.12%,
Charm Points 5, Trivial difficulty, Common occurrence and thresholds 10/100/250.
The retained tooltip screenshots support the threshold and label readings.
Earth sensitivity 80% and holy sensitivity 80% correspond to canonical
resistance 20%; physical and energy sensitivity 100% correspond to neutral.
Comparing received-damage percentages directly with resistance numbers would
create false differences.

Visible loot is a gold coin (`1+`) and cheese (`1`) in Common. The canonical
table references both items. The UI rarity bucket does not qualify exact
`probability_ppm`, and `1+` does not establish a different maximum than the
canonical gold range 1–4. No probability mismatch is asserted.

The Rat profile location text is general, while the UI lists named places.
Its separate Portuguese encyclopedia document is not that UI location list.
`ui-rat-control-observation.json` retains qualified values and partial location
reading; the full literal transcription is still pending. It is not a 1:1
complete card export.

## Private proofs and continuation

The final immutable checkpoint records 19 search attempts, including one
necessary repeat to return to butterfly results. Only nomad and butterfly
results were visually qualified (three result captures, two distinct queries).
All twelve added-name searches produced no new result labels. That does not
prove absence: a failed search can retain the previous grid, and animated
sprites can change the grid pixels without changing its result names. The
qualified checkpoint supersedes raw collector classifications.

Nomad has three visible variants; the first is complete and the other two
show 0/3. Their detail cards were not read. Wafer Paper Butterfly has a
fully unlocked detail card: Health 2, Experience 0, Speed 160, Armor 0,
Mitigation 0.56%, Charm Points 1, and dynamic total kills 37. Its visible
locations read `Candia, Carlin, Edron Surroundings, Thais`. Kill thresholds
were not read. No loot rows were visible; that is not no-loot authority.
Its client ID 2544 remains a candidate, and the missing initial canonical
name match is not proof of global entity absence.

Bestiary collection stopped when the owner switched to Charms. The remaining
queue is retained for a future explicit continuation, rather than presented
as completed. The continuation has 78 private image receipts in addition to
the initial 49; no raw image bytes are in the commit.

Original screenshots and DAT/appearance files are excluded from this PR.
`private-image-receipts.json` retains image sizes, hashes and an access-controlled
local directory locator. `provenance.json` pins the source snapshot and parser
input hashes; `source-receipts.json` identifies exact repository blobs. A public
receipt does not make the private image available to a reviewer.

`binding-candidates.json` and `gap-queue.json` route further work to ambiguous
or missing bindings, fields of the twelve added records, location text,
tooltip/stage meanings and visual evidence. A failed search is not proof that
a creature does not exist or that its stats are unavailable. Search result
grids may retain a previous result; raw collector status must be independently
qualified before publication as a card observation.

This is a partial audit. No all-card UI audit, verified server-ID mapping,
pixel/animation parity, missing-entity count from 845 minus 824, runtime fix,
gameplay experiment, deployment or merge is claimed.

## Validation

```sh
python docs/agents/evidence/bestiary-client-gap-audit-20261113/validate.py
python docs/agents/evidence/bestiary-client-gap-audit-20261113/test_validate.py
```

The validator checks population, source/candidate boundaries and proof links.
Optional `--repository .` rehashes the exact source Git blobs; optional
`--private-root <private directory>` rehashes screenshot bytes. The eight
negative/control tests reject missing or duplicate population entries,
authority promotion, incomplete-as-complete claims, inconsistent additions,
duplicate classes and missing proof receipts. They do not test the game.
The two additional negative controls reject counting an animated retained grid
as a qualified search result and promoting a blank loot panel to no-loot authority.
