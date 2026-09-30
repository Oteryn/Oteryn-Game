# OTV2-20260929 world map handover

Historical continuation evidence for `OTV2-20260928-world-base-map` (PR #1170) and
`OTV2-20260928-world-metadata-authoring` (PR #1160). It grants no authority; re-resolve live
GitHub state and current `main` before continuing.

The owner asked for a handover to the next agent. State at handover:

- PR #1160 (`claude/blissful-turing-oca168`, task `OTV2-20260928-world-metadata-authoring`):
  step 1 metadata. CI green on `e22162c3` (`origin/main` merged after #1260).
- PR #1170 (`claude/blissful-turing-oca168-world-map`, this task): stacked on #1160; CI green
  on `11b7583c`. Keep the stack: merge `origin/main` into the #1160 branch first, then that
  branch into this one; never rebase or force-push.
- Both PRs are waiting on review and owner decisions. Independent review is not yet decided.

## Owner decisions (answered)

1. Blue Valley floor 7: answered **1a** (owner). The base water tiles are replaced by the land
   of `maps.7z:blue_valley.otbm` where the official 15.30 minimap shows land: 905 tiles (896
   inside the Blue Valley box), limited to base water, pinned, counted and tested like the
   fill; the island footprint grows from 9,522 to 10,427 tiles.
2. `maps.7z:summer-update-2025.otbm`: answered **2b** (owner). Imported as a partial fill:
   underground components that the base lacks (16,777 tiles on floors 8-15) plus the 151
   floor 2-6 tiles that the official minimap shows as land; the Edron underground (floors
   8-12, box x33274-33456, y31786-31884, 27,359 tiles) is excluded from that fill, so a
   fill-only mix does not combine two layouts; owner decision 1a below reworks it. The underground cannot be proven by the
   official client map images (`minimap-*` and `satellite-*` in `content/assets/files/` hold
   floors 00-07 only); floor numbering is the same as OTS (07 is ground level), proven by the
   city temples and the Thalassara floor-7 fit.

3. Edron underground: answered **1a** (owner). The floor-10 cave is imported from the summer
   file and repaired with the player-recorded real-Tibia minimap (tibiamaps/tibia-map-data;
   sha256-pinned in `source.edron`, files read from `--tibiamaps-root`, never committed;
   evidence class reference-derived). `edron_rework.py` and the README have the exact rules.
   **Edron z10 is imported and repaired; z8, z9 (except the rule 2 repair), z11 and z12 keep
   the base**, as tibiamaps agrees with the base there. Counts: rule 1 3,982 filled, 699
   replaced; rule 2 floor 9 761 added, 4 replaced, 379 rock, floor 10 262 added, 3
   replaced, 135 rock; 5,519 tiles added and 706 replaced in all.
   - The new floor-10 cave (3,534 tile main component) is **not reachable** from the surface
     or from other floors: no floor change leads into it in the merged map.
   - `unresolved_entrances` (tibiamaps markers without a floor-change item at the position or
     one floor apart; none was invented), `[x, y, z]`: (33295, 31819, 9), (33433, 31821, 9),
     (33341, 31880, 9), (33436, 31797, 10), (33441, 31797, 10), (33436, 31797, 11),
     (33440, 31797, 11), (33441, 31797, 11), (33439, 31807, 11), (33439, 31808, 11).
     The first is the summer file's floor change `1080` (33295, 31819, 9) down into the cave;
     floor 9 keeps the base, which has no tile there, so it is the link to decide.
   - Open for the owner or the item agent: place a floor-change item at (33295, 31819, 9)
     (then the cave joins the base), or accept the cave as a sealed area for now.
   - **Owner decision: keep the Edron z10 cave sealed for now.** There is no evidence-backed
     entrance, so no floor-change item is invented; the entrances stay in the list above.

4. Minimap draft: Temple of Light answered **2a**, the owner then approved drafts for every
   remaining gap. `minimap_draft.py` drafts named `AREAS` (committed bbox and floors) from the
   tibiamaps floor 2-7 images (pinned in `source.minimap_draft`, read from `--tibiamaps-root`;
   reference-derived). **It is a rough draft: correct shape and walkability, generic ground,
   no borders, decorations, doors or furniture.**
   - Rule: coloured tibiamaps pixel and no base tile (fill tiles count) or plain water; floors
     0-7 also need official minimap land, water keeps the base water. The colour table is
     learned per floor outside the areas (130 mapped, 18 unmapped rows, summary `draft.mapping`).
   - Tiles (added / plain water replaced): Temple of Light f6 469, f7 5,832; Great Expedition
     Island f3-6 317, 218, 356, 400, f7 5,454; Great Expedition Wharf f7 1,644; Blue Valley
     f4-6 763, 1,114, 2,712, f7 5,575; Marapur/Thalassara f2-6 922, 1,736, 2,817, 4,085,
     4,652; Nargor f4-6 273, 515, 728; Upper Roshamuul f6 901. Total 22,978 added, 18,520
     replaced. No area under the 5% skip limit.
   - 18 yellow markers (15 floor 7, 3 floor 6) are not items: `unresolved_entrances`.
   - Islands: Blue Valley footprint 15,612 tiles. Temple of Light is event-only because the
     wiki marks it an event place, not because of the map; the converter is correct.
   - Remove or replace a draft: drop the area from `minimap_draft.AREAS`, or swap in a real
     source.

## Research results (the session scratchpad is not kept)

- Missing islands, measured against the official minimap land mask:
  - Temple of Light, Great Expedition Island and Wharf: no source in any CrystalServer map at
    the pin; now minimap drafts (decision 4).
  - `winterlight_solstice/island.otbm` is the event island, not Great Expedition.
- `15.30/` fragments use absolute coordinates; the file name is only an entry point.
- CrystalServer loads only `world.otbm` at startup; `custom/` loads only with
  `toggleMapCustom=true`. 29 quest and world-change maps load on demand via `Game.loadMap`
  and are overlay variants of places the base already has:
  - Dream Courts: by weekday.
  - Soul War ebb/flow.
  - Fury Gates, Nightmare Isle, Oriental Trader, Full Moon: world changes and events.
  - Ferumbras habitats.
  - Cults misguided.

  They belong with a future world-change mechanism, not the base.
- Bosses: 57 BossLever configs (60 names), 110 world or raid spawns and 81 scripted spawns
  were checked. Every room is in the base except the General Murius raid spawn. 57
  bosstiary monsters have no placement anywhere at the pin.

## Next work (in order)

1. Done: decisions 1a and 2b are applied on this branch through `convert_world_base.py`,
   with the pins, `--check`, validators and tests.
2. The drawing list is drafted from the minimap (decision 4) and needs detail work (borders,
   decorations, doors, furniture, floor changes) or a sourced replacement: Blue Valley,
   Temple of Light, Great Expedition Island and Wharf, Marapur/Thalassara floors 2-6, Nargor
   floors 4-6, Upper Roshamuul floor 6, Great Expedition floors 3-6. Edron floors 8-12: done
   for z10 (decision 1a), the entrances above stay unresolved.
3. Outside this task:
   - item B1b: 4 provisional `items.xml` ids plus id 99, owned by the item agent;
   - ladders, ropes and sewer grates: floor-change use rules, owned by the item/interactions
     agent;
   - runtime loading of the B3 map;
   - world-change overlays;
   - TibiaWiki enrichment.

## Working notes

- Sources: `zimbadev/crystalserver` at `00ce02a5`. A blobless or sparse clone of
  `data-global/world/` is enough. `maps.7z` needs `py7zr` and the Edron rework Pillow, both
  from `requirements-regenerate.txt`; the tibiamaps files (`bounds.json`,
  `floor-06/07/09/10/11-map.png` and `-path.png` of `raw.githubusercontent.com/tibiamaps/tibia-map-data/main/data/`,
  sha256 in `edron_rework.py` and `minimap_draft.py`) are fetched by hand into a directory passed as `--tibiamaps-root`.
- Lint with the CI version: `python3 -m ruff` 0.16.1 from `requirements-dev.txt`, run from
  the repository root. A plain `ruff` on PATH may be older. New scripts with a shebang need
  `chmod +x` (EXE001).
- Before each push run the package `test_*.py` and `validate_*.py`,
  `validate_materialized_game_tree.py`, `tools/agents/validate_governance.py`, and, for
  placements, the Rust `content_world_project_repository` test.
