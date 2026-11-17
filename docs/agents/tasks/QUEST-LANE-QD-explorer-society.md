# qd-explorer-society

Status: BLOCKED before implementation; no playable-content claim.

## Allocation

- Base: `e2152d050038cecdb3728c222af2ac184e1cac5a`.
- Branch: `claude/happy-davinci-ijz3wy-qd-explorer-society`.
- Packet identity: `oteryn:quest.the_explorer_society`.
- Existing canonical identity: `oteryn:quest.the_explorer_society_quest`.
- One writer, single agent. No runtime, generated content or tests changed.

## Hard blocker

The supplied canonical quest key has no catalogue definition. The existing
`oteryn:quest.the_explorer_society_quest` also has three
`claim_native_lowering_missing` holds (Hengis book and both Ratha skull bags).
`RecipeEvidence::verify` in
`crates/gameplay/quests/src/quest/authored_bindings/embedded.rs` rejects completion
and dialogue/world bindings while these holds remain: its claim-only exception
requires native-observation triggers and forbids completion. The packet allows
per-quest bindings and embedded test counts, but does not allocate the canonical
source/claim admission inputs or generic admission implementation. No hold was
removed and no validator was weakened.

The retained canonical recipe explicitly substitutes six field missions for all
seventeen missions and removes specimen-decay timers. It cannot be cited as
proof of the requested complete global route.

## Source evidence

On 2026-11-17 both required honest-UA MediaWiki requests returned verbatim:
`HTTP Error 402: Payment Required`.

- `https://tibia.fandom.com/api.php?action=parse&page=The_Explorer_Society_Quest&prop=wikitext%7Crevid&format=json`
- `https://tibia.fandom.com/api.php?action=parse&page=The_Explorer_Society_Quest/Spoiler&prop=wikitext%7Crevid&format=json`

No protection workaround or repeated request was attempted. The retained
canonical recipe cites the base pageid **4954**, revid **725344**, and spoiler
pageid **34099**, revid **1090719**. These are historical retained references,
not revisions returned by this session. The payload also cites older donors;
those were not used in place of the packet-approved pinned donors.

The permitted transcript/library index routes returned HTTP 200:
`talesoftibia.com/libsearch`, `tibiasecrets.com/transcripts`, and
`elkolorado.github.io/tibialibraries/`. An index response alone does not prove a
mission mechanic or resolve canonical admission holds.

## Open facts and mechanics

- Current global full mission ordering, timed specimens, exact exchanges,
  rewards, shop predicates and teleporter predicates remain unverified here.
- No missing generic server mechanic is established by this investigation.
  Quest-gated teleporters and every item-on-target step still need a real
  dispatch proof; no unsupported mechanic is represented as completed.
- Resume requires a corrected canonical identity/allocation and an admitted
  resolution of the three claim holds, plus accessible global source evidence.

## Validation

Startup completed, including the retained PostgreSQL connection check.
Catalogue identities, recipe adaptations, retained source revisions and admission
branches were inspected directly. `git diff --check` was run for this record.
No Cargo, PG quest, count, clippy, content regeneration or pin checks were run:
there is no implementation or content delta. In particular,
`qd_the_explorer_society` has not been created or run. No test was deleted,
ignored, weakened or committed away. This is a blocker record, not completion.
