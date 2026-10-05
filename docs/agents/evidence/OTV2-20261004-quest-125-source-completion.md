# Local completion of 125 Source guard specifications

Local AUTHORING on `codex/quest-data-completion-80-20261003`, HEAD
`88d63a18b44123ce342008cf52bcdb427c909ec2`. Root is the sole product writer;
six subagents prepared source candidates and independent review. No PR, commit,
push, merge, deployment or Remote Desktop operation was performed.

## Delivered data

The 125 positions are guard expressions belonging to 40 canonical Quest keys,
not 125 distinct Quests. All 125 now have separately typed operational Source
specifications with no unresolved operands. Their status is
`SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN`. Original Source cores, the 125 historical
`PARTIAL_SOURCE_GUARD` records, execution holds and readiness are preserved.

The reproducible local authoring pipeline includes entity predicates, getters,
dynamic storage reads, Tile construction, field/index/configuration reads, helper
calls, Lua short circuit and comparison rules. It retains member lookup before
argument evaluation, nil/false truthiness, selected operand results, errors,
metamethods, mutation and unproven dispatch. It does not infer an actor class from
a variable name or map donor storage numbers to Native slots.

Source associations increase from 37 to 89 component files across 14 Quest keys
(52 new associations). Another 82 explicit shared-provider dependencies are
captured. The remaining 159 files are 82 without a proven Quest association,
45 shared factory instances and 32 world events. These counts are components,
not missing Quests; ownership is nonexclusive and activation remains unproven.

Supplementary progress packets restore 16 exact AST writes across three tracks:
Threatened Dreams Mission02[1] and Mission03[1], and Crystal's Shadows of Yalahar
Mission13. There are 18 canonical mission joins because Mission13 is also declared
by Isle of Evil. A separate finite configuration specialization adds Cults of
Tibia's Orc Idol mission write through the named boss configuration and the
damage-player callback. It is fenced by the exact full script and helper body
hashes. Original mission declarations are retained.

Packets are under
`tools/content-schema/quest-authoring/samples/donor-source/refinements/`.
Canonical recipes carry qualified references with exact packet/record hashes,
Source graph variants, baseline records and mission-owner witnesses. Reproduction
is part of `quest_donor_source_authoring.py`; the canonical data import is part of
`quest_tree_authoring.py content --source-packet`.

## Source access and retained discrepancies

Quest research reused the existing Canary corpus at
`04b83b512114bfd888000d6e1433ed8ecaec7c5b` and Crystal corpus at
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, including portable Lua AST captures.
No Quest corpus was downloaded again.

Only actual missing dependency implementations were fetched by normal public
HTTPS: the donor-pinned vcpkg LuaJIT ports and their exact upstream source
revisions. `entity_values/dependency/` retains source URLs, revisions, byte counts,
hashes, source files and upstream copyright. Captured implementations cover
`lower`, `type` and `os.time`, including the latter's possible nil result. The
installed runtime binary and upstream archive SHA512 were not verified; these
are Source specifications, not deployed-library attestations.

Existing cached Fandom and TibiaWiki BR specifications corroborate Spike Tasks
Quest and Tibia Tales/To Outfox a Fox. No fresh wiki or Remote Desktop access was
needed. The following remain explicitly unresolved Source discrepancies:

- Spike journal tracks 44302/44303/44308 have no matching direct or symbolic
  writes in the cached donor Lua corpus. NPC task/Fame tracks differ. Treat the
  donor journal as stale; do not invent a match.
- Tibia Tales' Fox journal points to a parent storage table, while Budrik writes
  its child Questline. A chosen correction is possible, but is not exact donor
  track equivalence.
- Grave Danger's Cobra journal scalar and the Scarlett monster-stage/access
  tracks differ by actor, storage path and donor pin. No matching player writer
  for that journal scalar was proven. Do not equate numeric storage IDs across
  donors or turn a monster-stage write into player journal completion.

Of the nine initially unmatched mission declarations in the affected Quest set,
four now have exact supplementary Source associations. Five remain flagged:
the three Spike declarations, Fox's parent-table declaration and Cobra's journal
scalar. The progress supplements contain 17 write/config records and 19 canonical
mission links. These are extraction results, not complete mission execution.

## Import boundary and owner follow-up

Canonical repository data and the server v2 parser have different formats.
Server `ProjectV2QuestAuthoring` accepts eight authoring fields and its generic
candidate values are scalar Text/Integer/Boolean/SourceId variants. The nested
Source specifications and `donor_source_data` references are not that parser's
direct input. No accepted Source-to-v2 import adapter was found. This is a format
integration task, not evidence that the donor Quest scripts are absent.

Coordinator/architect follow-up: assign the accepted conversion/load seam for
these canonical Quest records; choose the Fox child-track correction and Spike
task-track mapping when implementing their target runtime behavior. No decision
request was posted remotely in this local-only batch. The earlier coordination
request remains at
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5973711588.

## Verification

Independent review verified operational expression semantics, lookup order,
escaped/raw C++ strings, dynamic table keys, Source body/registration witnesses,
helper and configuration boundaries. Closed-schema negative controls reject
unknown operation fields, hidden unresolved nodes and Native/runtime promotion.
Mission review verified 353 raw Source witnesses and all 18 canonical owner
hashes. Final canonical readback and required checks pass.

Independent canonical readback passes: all 352 Source/authored cores, every other
recipe field, the 25 pinned compiler inputs, old 125 partial records and readiness
42/242/68 are exact against the preceding local snapshot. The 105 attached
definitions now have 5,325 qualified references: 743 conditions, 19 progress,
4,379 NPC, 4 reward and 180 component references. All packet and record hashes
match. Exact Source graph gates admit 115 references from the new operational
packet; the complete 125-record packet remains retained independently.

Retained evidence:
`OTV2-20261004-quest-125-canonical-readback.json`,
`OTV2-20261004-quest-125-source-flow-coverage.json` and
`OTV2-20261004-quest-125-stale-source-wiki-crosscheck.json`.

The final `run_checks.py` exits 0: 889 unit tests pass (3 unchanged optional
fixtures skipped), 262 schema cases pass, both RewardClaim suites (13 and 6
tests) and content checks pass. Source corpus/AST reproduction, qualified packet
replay, binding, rollout, bundle and canonical tree checks pass. The focused
aggregate passes 181 tests. The first broad run exposed an isolated test-loader
import defect; it was repaired and the complete broad run repeated successfully.
Governance, repository policy and `git diff --check` also pass. HEAD is unchanged.

The refreshed import assessment validates all 352 definitions and every attached
packet/record reference; it confirms both new operational and mission references
are present. Its server-v2 format limitation remains explicit in
`OTV2-20261004-quest-125-import-assessment.json`. No server execution was tested.
