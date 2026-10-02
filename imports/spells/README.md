# Spell import candidate and audit

This draft preserves the local spell implementation and normalized import data
while the complete Canary/Crystal source and external-reference audit continues.
It is not a release or an assertion that every spell works in a live world.

## Preserved candidate

`r21-local-candidate/` contains the 246 active player-spell definitions, six
source-proven removed entries, normalized dependencies, 162 Creature policies,
118 Item policies, provenance, and explicit runtime prerequisites. The source
changes in this PR reproduce the distributed r21 source snapshot. Original
upstream Lua/XML and captured website HTML are not included in this snapshot.

The source candidate was validated locally with 60 passing Rust test-target
summaries and 251 passing Python tests. These are historical local results, not
CI results on this draft's eventual commit. The Rust launcher used a 16 MiB test
thread stack; the production 2 MiB WP3 stack was unchanged. Eight Rust test
invocations were ignored, with the full-manifest qualification run separately.
The final all-target process exit status was unavailable after environment
reattachment; all 60 terminal target results and a subsequent successful compile
are retained in the coordinator report.

Six source-exact completion test inputs stay external. Before running those
tests, retrieve and SHA-256 verify the pinned URLs and repository-relative paths
in `r21-local-candidate/external-fixture-inputs.json`. Do not substitute a newer
source revision. Their absence is a test setup requirement, not a reason to skip
the tests or claim that CI passed.

## Audit in progress

`r22-audit/` records full-tree source inventories and per-entry external
comparisons. Every finding keeps its exact source revision and provenance.
`agree`, `no_source`, `sources_disagree`, and `ours_differs` describe comparison
evidence; they do not grant runtime admission. A missing inapplicable field is
distinguished from a missing external fact.

The earlier player report covered all 252 catalog entries, but omitted five
monster-only registered spells from the player import. Current active-player
comparison covers all 246 entries; 241 match at least one reference and five
(Blank Rune and the four house spells) have no infobox match. Full mechanics and
complete monster spell verification are still being audited.

Public Tibiopedia is accessible through ordinary HTTP. On 2026-10-02, ordinary
Fandom access returned 402, BR returned 403, Tavily reported its usage limit, and
all connected Remote Desktop devices were offline. Historical wiki captures are
dated evidence, not fresh live-page verification. Remote Desktop is authorized
only for public browser research.

## Remaining gameplay work

Full wild-monster attack/defense scheduling and dispatch are not composed.
Familiar self-heal is implemented, but does not establish general monster AI.
Soul War quest access, progression and rewards are not implemented. Client
consumers/rendering, accepted protocol/resource allocation, WorldInstance
admission, VIS2 negative-floor support, and real PvP/black-skull fact producers
remain explicit prerequisites. See `r21-local-candidate/remaining-prerequisites.md`
and the per-entry audit reports. Production activation remains false.

Task: `OTV2-20261002-spell-import-r22`.
