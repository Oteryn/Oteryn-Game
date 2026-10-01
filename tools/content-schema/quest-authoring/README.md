# Quest authoring tools

Offline tooling for the CANDIDATE quest format
(`docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`). Evidence only: the server does not read
these files, and every source-derived output is `OTS_HYPOTHESIS_ONLY`.

| File | Purpose |
|---|---|
| `lua_tables.py` | Reads Lua table constructors (and their comments) without a Lua interpreter. |
| `ots_doors.py` | Transcribes the quest, key and level doors of both servers (`door_quest.lua`, `door_key.lua`, `door_level.lua`), joined by map position, into gates linked to the chest claims and keys, plus the `QuestDoorUnique` doors each opened by their own dedicated script (the Katana Quest lever door). |
| `ots_questlog.py` | Transcribes the quest logs of both servers into storyline quests with staged missions (D34) and their transitions (D35), joined by quest and mission name; writes the whole quest catalogue and the progress tracks with per-server transition sources. |
| `lua_writers.py` | Finds every storage write in the Lua sources and reads it as a candidate transition (D35): owner, callback, effect, `from` stage through its if-block, script registrations. |
| `lua_blocks.py` | Splits a Lua callback body into if/elseif/else branches, loops and statements (early `return` makes the rest an implicit `else`). |
| `ots_interactions.py` | Transcribes every quest script of both servers into interaction definitions (D36): edge, read-only conditions and children for the Quest, Ability, Item, Achievement and Presentation owners; Movement children are D37 relocations (to a named anchor or the previous tile; a computed target stays blocked) and WorldObject children are D38 overlay operations (`TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`, optionally `revert_after_ms`; a call the converter cannot yet classify by kind stays blocked); joined by quest and script path. |
| `ots_chests.py` | Transcribes the reward chests of Canary and CrystalServer (`startup/tables/chest.lua` plus the text and achievement tables of `quest_reward_common.lua`), joined by map position, into reward claims, reward-only quests, a catalog and a manifest. |
| `quest_content.schema.json` | JSON Schema of reward claims, door gates, reward-only and storyline quests. |
| `conflict_decisions.json` | D25 decisions for every Canary/CrystalServer conflict of the chest, door, quest-log and interaction transcriptions: chosen server, basis, the difference in our own words and the wiki revision when it decides; the converters apply it and fail on a stale decision. |
| `interaction_overrides.json` | Curated replacements for an interaction condition `ots_interactions.py` cannot read statically because it lives in a sibling `lib/quests/*.lua` table indexed by a role field or a world state: interaction key and source line to the resolved condition (existing D36 vocabulary only) plus a basis citing the table and its registration; `ots_interactions.py` applies it and fails on a stale entry (a line no longer unresolved). |
| `track_owners.json` | Owning quest of the progress tracks quest scripts write outside missions, where no mission-track prefix or script directory names it; the quest-log converter fails on a missing or stale record. |
| `script_quests.json` | Wiki quests confidently matched to a script directory and/or the `wiki_quest` of an auxiliary progress track, added to the catalogue as `kind: script_only` (a quest the servers implement in scripts with no quest-log entry); each entry gives its basis. |
| `chest_quest_links.json` | Curated chest-to-quest links for claims `ots_chests.py`'s own kv_quest_name/storage_key/label/section match could only find a section-header candidate for, or no link at all: each cites the Fandom wiki (an item, key or quest page) or the quest-coverage sample's own recorded source evidence; applied with `quest_link_basis: curated`, and the converter fails on a stale link. |
| `interaction.schema.json` | JSON Schema of interaction definitions (D36), including the D37 relocation and D38 world-object overlay child shapes. |
| `ots_map_check.py` | Checks chest, door and interaction positions against Canary's `otservbr.otbm` and CrystalServer's `world.otbm` (not committed; sha256 pinned), reading Canary's startup id tables; writes `samples/map-check/report.json`. |
| `ots_readiness.py` | Per quest, the engine features it needs and its data gaps, and the unlock order; reads only the committed samples. |
| `ots_gap_triage.py` | Classifies every unresolved interaction line and unresolved condition into `owner_pending` (an already-named or same-shape missing owner), `shared_mechanism` (a recurring >=3-quest pattern with an explicit rule) or `bespoke`; per quest and a ranked, unlock-order summary; reads the committed samples plus the pinned checkouts for line text; writes `samples/gap-triage/triage.json`. |
| `validate_quest_content.py` | Schema plus semantic checks: unique keys and positions, non-empty rewards, text on a handed-out item, claim/quest links in both directions, gate conditions against the claims (progress marker, key source), one identity per quest, mission ranges and stages against the progress tracks, catalog and manifest coverage; for interactions: anchors, blocked reasons, named transitions against the missions, undeclared progress tracks, manifest status. |
| `verify_quest_schema.py` | Focused positive/negative cases on synthetic fixtures (`--verbose` prints each case's first error). |
| `refresh_quest_source_checks.py` | Refreshes exact inventories of missing gate/condition references after conversion; unresolved and blocked definitions never become `mapped` merely because their JSON shape is valid. `--check` verifies reproducibility. |
| `quest_tree_authoring.py` | Populates `content/quests/definitions/` with the accepted first reward-only batch, resolving existing canonical claims through exact source identity/revision. Also emits the full source catalogue packet. |
| `quest_tree.schema.json` | Strict definition/shard schema; `definition_ready` describes known definition fields, while overall quest completeness and runtime readiness remain unassessed. |
| `quest_source_packet.schema.json` | Typed packet for every source quest kind, reusing the existing source Quest schema through an offline registry. |
| `quest_catalogue_authoring.py` | Builds the 373-title source inventory with candidate/family coverage and independently scoped wiki facts. Fresh revision fingerprints and source snapshot digests are checked offline. |
| `wiki_quest_facts.json` | Structured facts from 373 exact-revision Fandom pages, 370 spoilers and 283 BR pages, read through the owner-authorized browser fallback. Contains named entities/counts and provenance, without original narrative. |
| `run_checks.py` | Runs all offline schemas, regressions, semantic validators and deterministic content/catalogue/packet checks; the Quest Authoring Schema workflow invokes it. |
| `test_quest_completeness.py` | Regression cases for quest ownership, blocked children, missing reads/gates and stale diagnostic inventories. |
| `samples/quest-coverage-2026-09-27.json` | The 373 wiki quests (facts only) with their status in each server. |
| `samples/chests/` | `claims.json`, `quests.json`, `catalog.json`, `manifest.json`, `empty_containers.json`. |
| `samples/doors/` | `gates.json`, `manifest.json`. |
| `samples/questlog/` | `quests.json` (the whole quest catalogue), `progress.json`, `manifest.json`. |
| `samples/interactions/` | `interactions.json`, `manifest.json`. |
| `samples/gap-triage/` | `triage.json`. |

```sh
pip install -r ../monster-authoring/requirements.txt
python verify_quest_schema.py
python ots_chests.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
python ots_doors.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_questlog.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_interactions.py --canary <canary checkout> --crystal <crystalserver checkout>
python refresh_quest_source_checks.py
python ots_readiness.py
python ots_gap_triage.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_map_check.py <otservbr.otbm> --crystalserver <decompressed world.otbm> --canary <canary checkout>
python validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json \
  --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json \
  --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json \
  --progress samples/questlog/progress.json \
  --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json
python test_quest_completeness.py
python test_converter_enrichment.py
python test_progress_enrichment.py
python test_quest_identity.py
python test_quest_tree_authoring.py
python quest_tree_authoring.py content --source-packet samples/migration/quest-source-packet.json
python quest_tree_authoring.py content --check --source-packet samples/migration/quest-source-packet.json
python quest_catalogue_authoring.py
python quest_catalogue_authoring.py --check
python run_checks.py
python refresh_quest_source_checks.py --check
```

The coverage sample was built from the Fandom API (Template:Infobox Quest, retrieved 2026-09-27)
and a search of both servers' Lua sources; its `method` field records how, including the verdicts
corrected by hand.

The canonical tree currently includes the 110 distinct reward-only quests after
Desert Dungeon deduplication. The migration packet preserves all 250 source quests,
including 58 storylines and 82 script-only quests. It hashes the supporting progress,
interaction, gate, chest and manifest inputs. These are separate scopes: a populated
tree or complete source inventory does not establish complete gameplay definitions.

Fresh Fandom reads retain the historical 2026-09-27 revision cut; BR crosschecks
retain their distinct 2026-10-01 cut. Level/premium and quest-log differences stay
explicit, with no automatic promotion. A `conflict` row can also lack an authored
candidate: inspect `authored_candidates` and `family_representation` separately.
Reward/requirement entity lists are partial source facts, not full prerequisite
expressions or admitted canonical Item references. Tibiopedia was unavailable
(browser redirected to an empty setup page); three requested spoiler URLs were unavailable, with layouts resolved from
the base pages or aggregate listing. The original unavailable-page audit remains.

Source interaction conversion also resolves branch-local immutable constants,
bounded exact arithmetic and static membership lists. Engine value-copy calls
are admitted only with an unshadowed Game binding. An unresolved source conflict
retains both full typed graphs in `manifest.json` under `conflict_alternatives`;
readiness counts that conflict as a data gap for every linked quest. Explicit
NPC/script coverage holds likewise prevent presence from implying completeness.

The pinned-source completion packet records all 129 historical OTS candidates.
Only 40 executable curations and nine partial mission-family links are newly
authored; declaration/reference-only evidence is not promoted to a quest.
Progress preserves each source occurrence, its blob/line digest and full write
requester context, even when multiple writes share the same effect.

Wiki enrichment preserves 1026 exact source revisions and their distinct cuts.
Multiline rewards, quantities, aliases and scoped requirements are parsed without
media/fragment links. Source-only specs retain unknown IDs and prerequisites;
scoped level facts never become a guessed global minimum. Illuminator title
overlap remains an explicit identity conflict.

The standalone SOURCE migration bundle embeds all quest/progress/interaction/gate/claim
and wiki-catalogue payloads with strict offline schemas and exact input provenance.
It preserves full typed source-conflict alternatives and all 5468 progress write
occurrences; computed expressions and unresolved ownership remain source gaps.

```sh
python bundle_authoring.py
python bundle_authoring.py --check
python bundle_authoring.py --validate samples/source_migration/bundle.json
python bundle_authoring.py --validate samples/source_migration/bundle.json --source-backed
```

Standalone validation reports STRUCTURAL_ONLY / NOT_VERIFIED for input provenance.
Source-backed validation deterministically reconstructs the entire packet from
local samples and reports VERIFIED_AGAINST_LOCAL_INPUTS. Neither mode establishes
native admission or runtime readiness. The offline runner checks regeneration.

SOURCE prose is preserved in a separate strict text registry, keyed by exact
UTF-8 digest, Unicode length and placeholders already used by the source data.
Pinned Lua literal witnesses include raw tokens, source blob hashes and explicit
decoding/normalization evidence. Quest graph admission remains independent.

```sh
# Acquisition only, requiring the exact donor commits documented above:
python source_text_authoring.py --capture-source --canary <checkout> --crystal <checkout>
# Portable offline regeneration and checks from the committed capture:
python source_text_authoring.py
python source_text_authoring.py --check
python bundle_authoring.py
python run_checks.py
```

Offline checks verify recorded literal decoding and exact local packet inputs.
Original donor-file membership remains captured acquisition provenance rather
than a claim of independently verified remote contents during offline checks.
Missing literal evidence is UNKNOWN and contributes an owned source gap; a
known text body does not resolve reward-carrier or quest-identity conflicts.

The pinned wiki SOURCE inventory includes all105 titles that lacked authored
bindings at the503bba9e snapshot, including80 historical donor-PRESENT titles
previously omitted by an ABSENT filter. Seven later partial bindings do not
remove their specifications. Original25 facts and UNKNOWN holds are preserved.
The SOURCE schema does not establish native quest completeness.

```sh
python build_wiki_source_schema.py --check
python wiki_source_inventory.py --check
```

All105 selected SOURCE specifications include4656 factual rows from249 exact
revisions,197 curated groups,1066 source heading references and98 journal field
references. Document order remains SOURCE order; execution and canonical bindings
remain unknown. Provider disagreements, baseline scalars/item identities and
unparsed requirements remain scoped and preserved.

```sh
python wiki_source_supplements.py --check
```

The offline check reconstructs authored facts from digest-bound inputs and their
recorded acquisition receipt. It does not fetch or recheck public bodies or
line/spans; those were verified during the recorded acquisition. Output records
raw_body_rechecked:false, line_spans_rechecked:false, source_revision_verified:false
and historical_acquisition_verified:true. This recorded provenance is not a
signature, a fresh online proof or native/gameplay admission. Raw wiki bodies,
acquisition browser payloads and the donor C++ witness remain unpublished.

The next bulk SOURCE batch retains single-NPC auxiliary writes under exact,
uniquely owned mission families. Broad release prefixes, mixed NPC callers,
symbolic collisions and stale curated writer guards never infer an owner.
It adds262 progress tracks,333 transitions and754 pinned write occurrences
across35 existing quest owners; all old progress records are preserved. Eight
new literal hashes are captured, with1617/1617 referenced SOURCE texts known.
Eleven further partial quest components and one existing three-mission family
link reduce unbound wiki titles98 to86. Their coverage holds remain explicit.
The resulting donor packet contains268 quests,1425 tracks,3462 non-alias
transitions and6545 write occurrences; it still contains185 quests with gaps
and74 missing reference occurrences. Native Quest field readiness remains43/67.
These counts describe the prepared data candidate, not merged or playable content.

SOURCE getter/declaration closure adds65 records and91 indexed writes while
preserving all1425 old progress records. The reader packet has1490 tracks,
3516 non-alias transitions and6636 writes; missing references fall74 to1.
Sixty-three added records retain unknown semantics. Consumer readiness propagates
these holds, including through source-only aliases. Four numeric getter records
have no proven named declaration or complete writer inventory; they are never
joined to named Storage paths by number. The unresolved FastWay getter names
Storage while the declaration names GlobalStorage. Native field readiness43/67
and185 quests with source gaps are preserved; graph closure is a separate scope.

### Bounded SOURCE conditions and Tile item removal

Complete bounded multiline `if`/`elseif` headers retain their physical source
lines; incomplete or unproven predicates remain UNKNOWN. The exact unshadowed
`Tile(...):getItemById(literal):remove()` chain retains SOURCE REMOVE only when
its coordinates and finite loop bounds are proven. These rules do not establish
Native placement identity or playable quest completeness.
