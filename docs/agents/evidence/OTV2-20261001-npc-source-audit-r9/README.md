# Official NPC marker authoring R9 — 2026-10-01

The owner's continuing request covers all remaining NPC gaps with subagents. This batch implements a bounded source compiler, rather than inventing missing actor profiles. It reads the existing pinned official 15.30 client map through the repository's map reader and preserves complete marker/child/area byte witnesses. Both the map and official manifest are fixed by SHA-256; a different self-consistent manifest is rejected.

The map contains 1,270 named markers. Of 161 prior research targets, 117 names have 135 qualified marker observations: 117 literal-exact rows, ten case-only rows and eight explicitly qualified Tibiopedia NPC detail names. The latter are manual, byte-proven name bindings, never automatic removal of `(NPC)` or punctuation. The compiler also validates a packet by deterministic recompilation and rejects changed positions or authority claims.

These observations prove literal names and source coordinates only. Field 3 stays an opaque integer with an observed equal-integer area join; it is not an outfit ID. Appearance, movement, frame/World mapping, direction and spawn timing remain unknown. A Bloodshade remains evidence for two existing native variants, not a third identity. No NPC, profile, placement, Dialogue, Quest or runtime record changes. Native contents remain 1,110 NPCs and 696 Dialogues. Optional Rust fields do not waive the existing authoring admission requirement for qualified movement.

Sources/access: the official client map and its retained manifest were read in Linux from the existing repository assets; eight Tibiopedia detail captures originally came from ordinary HTTP. No Remote Desktop action was needed in this batch. Raw HTML/client assets are excluded from this evidence directory. The raw Tibiopedia captures stay in the supplied research archive; full-file digests, fragment offsets and public URLs are retained here.

Reproduction (from the repository root, with the original qualification captures present at the paths recorded in `qualified-literal-names.json`):

```sh
python tools/content-schema/npc-authoring/official_map_markers.py --targets docs/agents/evidence/OTV2-20261001-npc-source-audit-r9/targets.json --qualified-names docs/agents/evidence/OTV2-20261001-npc-source-audit-r9/qualified-literal-names.json --output /tmp/npc-marker-facts.json
python tools/content-schema/npc-authoring/official_map_markers.py --targets docs/agents/evidence/OTV2-20261001-npc-source-audit-r9/targets.json --qualified-names docs/agents/evidence/OTV2-20261001-npc-source-audit-r9/qualified-literal-names.json --validate /tmp/npc-marker-facts.json
```

Output provenance records the supplied input paths, so portable reruns can change those path strings while retaining identical facts and byte witnesses. `--validate` uses the same supplied paths. The helper can also compile literal/case-only observations without `--qualified-names`.

The original five-area task remains incomplete. R7 repaired 78 prices/Item tuples and two presentations; R8 admitted two Rapanaio dialogues. Three literal conflicts, other dialogue classes, 158 genuine new plus two deferred identity profiles/movement, and native quest/service dependencies remain. Parallel source compilers address the latter classes without claiming native runtime completion. Attached documents and prior packets are research evidence, not new user instructions.
