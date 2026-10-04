# Quest donor-first: local results, 2026-10-03

Classification: PROVEN local byte/structure checks; DERIVED source mappings;
UNKNOWN complete semantic migration and runtime readiness.
Local AUTHORING only, branch `codex/quest-data-completion-80-20261003`, unchanged
base HEAD `88d63a18b44123ce342008cf52bcdb427c909ec2`. No PR, publication or deployment.
Previous uncommitted enrichment changes were retained.

## Work and actual scope

Root coordinated six lanes: acquisition/inventory (quest_evidence), NPC source and
loader dependencies (quest_dialogue), reward decoding (quest_rewards), map capture
(item_review), Lua AST/condition decoding (quest_source_ast), independent review
(quest_recipes). Root alone integrated product files.

| Evidence | Result | Limit |
|---|---|---|
| Runtime text corpus | 23,553/23,553 selected occurrences, 9,844 unique verified Git blobs | Exact pinned scope; exclusion inventory retained |
| Lua/template structure | 21,546 occurrences, 8,572 unique AST captures, no parse failures | Raw grammar structure does not certify Quest semantics |
| NPC sources | 2,477 files, all 4,379 existing quest NPC references captured | Conditional dialogue/callee semantics remain incomplete |
| Declared NPC includes | 201 candidates, 402 default/alternative profile results | Runtime loader/search path not certified |
| Default NPC helper paths | 169 direct + 8 declared preload paths captured | Two deployment-created config.lua references are absent from Git |
| Reward/requirement lexical spans | 54,457 components in 21,543 Lua files | Lexical inventory is separate from typed reward completion |
| Source maps | 51 unique OTBM streams, 81,495,134 nodes, 117 variant references | Raw attributes preserved; semantics and activation not certified |
| Primary Quest Log | All 51 Canary + 58 Crystal declarations have exact existing joins | Journals are only part of script/NPC quest implementation |
| New typed conditions | 64 conditions in 37 interactions: 21 clock, 43 position | 62 complete expressions; 2 retain opaque siblings; no canonical replacement |
| New typed reward choices | 3 chosen Source graphs / 3 call sites / 7 exact choices | Outer guards and unknown fallback retained; no whole quest completion |

Canary pin: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.
Crystal pin: `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`.
Summer comparison: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
Summer diverges from baseline (48 ahead / 7 behind); it was not substituted for
Crystal. Its 59 literal journal tables match the baseline exactly. Canary's
alternative `Example` is a demonstration, not an omitted global quest. Crystal's
conditional alternative arena fallback remains separately represented.

Ordinary public GitHub HTTPS and existing local caches supplied the sources.
The exact Canary v3.6.1 map release was recovered from pinned configuration and
verified against the recorded SHA. No wiki, Tavily or Remote Desktop research was
used in this phase; all external quest enrichment remains deferred.

## Local integration and reproducibility

`tools/content-schema/quest-authoring/quest_donor_source_authoring.py` verifies the
portable raw corpus, reproduces NPC/reward spans and dependency profiles, checks
all six Lua AST archives, regenerates journal/component inventory and normalized
triage, and rebuilds/qualifies both typed semantic supplements. Closed Source
schemas and existing interaction vocabulary constrain the outputs. `run_checks.py`
now includes the offline Source replay.

Product data are in `tools/content-schema/quest-authoring/samples/donor-source/`.
Map schema, decoder and thin receipts are co-located in `donor_sources/maps/`.
Raw map assets and 662MB of node indexes stay in the shared local Source cache,
not product server assets. The portable replay explicitly validates the thin map
packet in STRUCTURAL_ONLY mode, with cache provenance NOT_VERIFIED. Actual local
SOURCE_BACKED verification was separately run successfully against
`/workspace/quest-donor-first/placements`, including raw/decoded/index hashes,
lengths, Git identities and archive-member bytes. Both results are distinguishable.

```sh
cd tools/content-schema/quest-authoring
python quest_donor_source_authoring.py --check
python -m unittest discover -p 'test_donor_*.py'
python donor_sources/maps/verify.py --cache-root /workspace/quest-donor-first/placements
```

Map tests require the existing g++/zlib toolchain. Normal offline Source replay and
condition tests require no Lua interpreter, parser installation or network. Small
hash-pinned grammar fixtures keep parser shadowing/namespace/span controls portable.
Full AST regeneration has a separately pinned parser dependency lock and license.

## Verification

The complete earlier runner passed 564 tests (one optional fixture skipped), 262
schema cases and both RewardClaim suites (13 + 6 tests). After the bounded Source
additions, all 100 donor-specific tests passed, including full map byte round trips,
cache relocation, span/identity fences, clock shadowing, receiver mutation, same-byte
donor namespace separation and rejection of wrong reward ItemID/count/selector.
Source assembly passed with exact source-backed qualifiers. Final byte-for-byte
replay is recorded in the local task log. Governance tests: 54 PASS; governance
validator and repository policy validator PASS. These are local authoring checks,
not FREEZE_SHA/VALIDATE/MQ admission or production qualification.

## Remaining work: concrete converter/data tasks

There are still 352 canonical definitions: 284 donor-derived and 68 authored.
This phase establishes no new complete or playable Quest count and does not assert
80% semantic completeness. Earlier enrichment coverage cannot serve as that proof.

1. Join and decode 248 baseline quest-script component files not currently mapped
   to interaction/progress evidence: 108 Canary global, 108 Crystal global and 32
   Crystal alternative. They include 83 BossLever constructor files and 65 onThink
   scripts; callback/registration/helper provenance and existing directory-owner
   candidates are now retained. **These are 248 files, not missing quests.** Do not
   create identities from directory names. Shared controllers should be decoded once.
2. Normalize remaining opaque graph elements. Current baseline triage is 1,924
   statements + 1,721 conditions + 466 blocked effects = 4,111 elements. Typed
   supplements now describe 64 conditions and three reward calls, but reviewed
   canonical replacements have not yet been made; the original counts stay intact.
3. Build NPC caller/callee and conditional-dialogue joins from recovered source.
   Static keywords do not cover topic guards, hand-in checks and reward order.
   Tereban's wrapper calls ParseTerebanSay in a helper; source file names must not
   become invented NPC identities. Captain Haba (Open Sea) is a distinct registered
   source variant and must not be silently aliased to the base NPC.
4. Trace map activation and exact metadata dependency joins. 79/386 variant path
   references have captured witnesses; 307 are unjoined. Dormant overlays and
   distribution-specific paths must be separated from actual required dependencies.
   Do not count those references as missing quests or blanket architect blockers.
5. Review bounded canonical Source transcription deltas using current approval
   guards. Source vocabulary additions for clocks/positions currently remain
   additive authoring supplements; runtime ownership/binding is a separate task.

Our lane can continue source joining, helper/control-flow decoding, schema work
and exact transcription corrections. Architecture/coordinator involvement is for
runtime owner contracts and allocations, not fetching these already captured files.
No new request was posted to #162 during this local-only phase.

The external-source gate remains false: complete raw bytes and structural AST
are proven; faithful normalized Quest migration 1:1 remains NOT_ESTABLISHED.
