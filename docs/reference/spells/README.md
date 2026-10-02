# Spell import candidate and audit

This draft preserves the local spell implementation and normalized import data
including the complete source inventories and the available external-reference evidence.
It is not a release or an assertion that every spell works in a live world.

## Preserved candidate

`r21-local-candidate/` contains the 246 active player-spell definitions, six
source-proven removed entries, normalized dependencies, 162 Creature policies,
118 Item policies, provenance, and explicit runtime prerequisites. The source
changes build on the distributed r21 source snapshot, with CI source setup,
portable test paths, regenerated Item-reference outputs, candidate-evidence
placement and Rust formatting repaired afterward. Original
upstream Lua/XML and captured website HTML are not included in this snapshot.

The source candidate was validated locally with 60 passing Rust test-target
summaries and 251 passing Python tests. These are historical local results, not
CI results on this draft's eventual commit. The Rust launcher used a 16 MiB test
thread stack; the production 2 MiB WP3 stack was unchanged. Eight Rust test
invocations were ignored, with the full-manifest qualification run separately.
The final all-target process exit status was unavailable after environment
reattachment; all 60 terminal target results and a subsequent successful compile
are retained in the coordinator report.

Six source-exact completion test inputs stay external. The spell CI workflow
checks out the exact pinned sources and stages these inputs with
`retrieve_completion_fixtures.py`, verifying the pinned revisions, paths, URLs,
SHA-256, byte lengths and Git blob identities from
`r21-local-candidate/external-fixture-inputs.json`. Local runs must perform the
same staging before the completion tests. Do not substitute a newer
source revision. Their absence must fail the test setup; the tests are never skipped to make
CI pass.

## Audit evidence

`r22-audit/` records full-tree source inventories and per-entry external
comparisons. Every finding keeps its exact source revision and provenance.
`agree`, `no_source`, `sources_disagree`, and `ours_differs` describe comparison
evidence; they do not grant runtime admission. A missing inapplicable field is
distinguished from a missing external fact.

The earlier player report covered all 252 catalog entries, but omitted five
monster-only registered spells from the player import. Current active-player
comparison covers all 246 entries; 241 match at least one reference and five
(Blank Rune and the four house spells) have no infobox match. Full mechanics and complete independent monster verification remain open where
public sources do not provide the necessary facts.

Public Tibiopedia is accessible through ordinary HTTP. On 2026-10-02, ordinary
Fandom access returned 402, BR returned 403, Tavily reported its usage limit, and
all connected Remote Desktop devices were offline. Historical wiki captures are
dated evidence, not fresh live-page verification. Remote Desktop is authorized
only for public browser research.

## Complete source and external comparison evidence

The approved full-tree census has 2,351 enabled registration rows and 904 distinct
name/carrier keys across alternative datapacks. The approved player-only union
is 252 keys, matching 246 active and six removed catalog entries by name/carrier;
that identity match does not prove every parameter.

The full approved monster data import preserves 5,188 profiles and 20,640 actual
attack/defense slots. 20,383 slots have mapped typed authoring dependencies, 242
retain unresolved custom semantics, and 15 preserve source NOOP/omission behavior.
4,790 profiles are authoring/schema resolved, 391 remain blocked, and seven have
invalid donor fields. These categories never imply runtime execution. The complete
normalized data, per-file hashes and per-slot admission flags are preserved in
`r22-audit/monster-import/spells-r22-monster-authoring-import.tar.gz`. No original upstream assets are included.

The monster reference audit covers every approved slot and attempts 1,820 named
species: 1,692 public Tibiopedia attack tables were read and 128 pages were
unavailable. Exact intervals, chances and many script/effect facts are not published;
unknown fields and observed-damage differences remain explicit. Do not infer missing
facts or overwrite source damage with wiki measurements.


A separate current-head candidate is preserved for Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`
and Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`: 5,237 profiles and
20,742 attack/defense slots, including 20,485 mapped typed dependencies, 242
unresolved semantics and 15 source omissions. Its archive, proofs and per-slot
flags are in `r22-audit/monster-import-current/`. It does not silently promote
production source pins. The corresponding `monster-references-current/` joins
every slot with zero source-field/hash mismatches; 1,736 public attack tables
were read and 133 pages were unavailable across 1,869 species. Exact unpublished
chance/interval/effect fields remain unknown. Both archives contain normalized
authoring data, not an activated monster scheduler.

Fresh player Tibiopedia evidence has 203 pages and 237 fact rows, including five
historical conjure pages absent from the current index. The player formula audit
partitions every active entry: 44 neutral-bound comparisons, 21 nominal-component
comparisons, 39 without an independent formula, 11 without an independent damage or
regeneration schedule, nine without an independent field schedule, 120 where HP
arithmetic does not apply, and two source/calculator model conflicts for Shield
Bash/Slam. Nominal comparisons do not prove interval endpoints or RNG behavior.

The exact r22 CI source setup reproduces the census, readiness and starter outputs.
74 schema cases and 222 Python tests passed; the relocated candidate Item binding
passed 32 native-data Python tests and 11 Rust native-content tests (one full-manifest
test remains separately ignored in this focused run). These local observations are
not a complete production qualification.

## Remaining gameplay work

Full wild-monster attack/defense scheduling and dispatch are not composed.
Familiar self-heal is implemented, but does not establish general monster AI.
Soul War quest access, progression and rewards are not implemented. Client
consumers/rendering, accepted protocol/resource allocation, WorldInstance
admission, VIS2 negative-floor support, and real PvP/black-skull fact producers
remain explicit prerequisites. See `r21-local-candidate/remaining-prerequisites.md`
and the per-entry audit reports. Production activation remains false.

Task: `OTV2-20261002-spell-import-r22`.
