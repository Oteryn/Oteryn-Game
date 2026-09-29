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

## Open owner decisions

1. Blue Valley floor 7: replace the 905 base water tiles by the land of
   `maps.7z:blue_valley.otbm`, only where the official 15.30 minimap shows land? Recommended
   yes. That needs a replace rule beside the fill-only rule, limited to base water with
   official land, and pinned, counted and tested like the fill.
2. `maps.7z:summer-update-2025.otbm`: answered **2b** (owner). Imported as a partial fill:
   underground components that the base lacks (16,777 tiles on floors 8-15) plus the 151
   floor 2-6 tiles that the official minimap shows as land; the Edron underground (floors
   8-12, box x33274-33456, y31786-31884, 27,359 tiles) is **deferred** as a cave rework, so a
   fill-only mix does not combine two layouts. The underground cannot be proven by the
   official client map images (`minimap-*` and `satellite-*` in `content/assets/files/` hold
   floors 00-07 only); floor numbering is the same as OTS (07 is ground level), proven by the
   city temples and the Thalassara floor-7 fit.

## Research results (the session scratchpad is not kept)

- Missing islands, measured against the official minimap land mask:
  - Temple of Light, Great Expedition Island and Great Expedition Wharf: no source in any
    CrystalServer map at the pin. These must be drawn.
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

1. Apply the owner answers to decisions 1 and 2. Both change only this branch, through
   `convert_world_base.py`, with the pins, `--check`, validators and tests.
2. Record the drawing list as the remaining map gap:
   - Blue Valley NE/E/S;
   - Temple of Light;
   - Great Expedition Island and Wharf;
   - Marapur/Thalassara floors 2-6;
   - Nargor floors 4-6;
   - Upper Roshamuul floor 6;
   - Great Expedition floors 3-6;
   - Edron floors 8-12 (deferred cave rework, decision 2b).
3. Outside this task:
   - item B1b: 4 provisional `items.xml` ids plus id 99, owned by the item agent;
   - ladders, ropes and sewer grates: floor-change use rules, owned by the item/interactions
     agent;
   - runtime loading of the B3 map;
   - world-change overlays;
   - TibiaWiki enrichment.

## Working notes

- Sources: `zimbadev/crystalserver` at `00ce02a5`. A blobless or sparse clone of
  `data-global/world/` is enough. `maps.7z` needs `py7zr` from `requirements-regenerate.txt`.
- Lint with the CI version: `python3 -m ruff` 0.16.1 from `requirements-dev.txt`, run from
  the repository root. A plain `ruff` on PATH may be older. New scripts with a shebang need
  `chmod +x` (EXE001).
- Before each push run the package `test_*.py` and `validate_*.py`,
  `validate_materialized_game_tree.py`, `tools/agents/validate_governance.py`, and, for
  placements, the Rust `content_world_project_repository` test.
