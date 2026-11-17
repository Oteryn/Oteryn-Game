# W05b source decisions (2026-11-05)

Evidence class: donor observations below are `OTS_HYPOTHESIS_ONLY`. Exact revision:
`zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
Numbered excerpts retain donor line numbers and normalize trailing whitespace.
Their SHA-256 headers identify the exact retrieved upstream bytes.

| Fact | Source |
| --- | --- |
| Fury chooses one uniformly from ten named city maps at startup; gate entrance AID9710 requires premium, promotion, level60; dungeon destination33290,31786,13; AID9715 exits to the selected city | `data-global/scripts/world_changes/fury_gates.lua:5-16,20-29,52-65` |
| Nightmare chooses one uniformly from three entrance maps at startup; exits UID35020 to selected region; UIDs64001..64003 enter33497,32616,8 | `data-global/scripts/world_changes/nightmare_isles.lua:5-9,14-24,31-33,45-49,56,68-76` |
| Entrance layers contain no creature spawn definitions; scripts load map tiles, not spawned substitutes | same scripts `fury_gates.lua:28`, `nightmare_isles.lua:20`; exact extracted geometry and source Git blob ids in `content/encounters/world-changes/layers.json`, extractor `tools/content-schema/world-authoring/build_world_change_layers.py:18-34,38-86` |
| Flower Month starts in June, places Rosemarie, then cycles four areas indefinitely every 900000 ms; 25 dryads requested with a shared 250-attempt cap per wave; walkable-tile checks precede monster creation | `data-global/scripts/raids/world_changes/flower_month.lua:1-25,28-55,60-66` |

Native layer decision: follow accepted EVENTS-WORLD-1 W4: palette-resolved named optional
bundle layers replace base tiles at epoch boot, below the ordinary Channel overlay. Refuse
house intersections and unadmitted native references at compilation. Do not feed donor OTBM
files to the production loader or masquerade volatile overlay items as base map geometry.
W10 deliberately overrides donor's unconditional Fury availability with 343 permille; retain
that accepted decision, ten equal weights, and W11's three equal-weight daily Nightmare draws.
Spawn admission: entrance layers do not create health-only actors or fabricate spawn sources.
Existing native creature policies and ordinary base dungeon spawn sources remain authoritative;
Flower waves must go through the existing raid/physical-map admission owner.

## Flower Month donor precedence decision

The current owner's instruction explicitly asks for donor-following spawn admission.
It authorizes the bounded Crystal compatibility choice: June, four named areas in source
order, 25 requested Dryads, one shared 250-attempt budget per wave, 15-minute intervals,
with advance announcements immediately before each wave. The retained Wiki Dryad page
1191744/Raid page1168721 conflict describes a distinct Global observation and is preserved
in the existing source archive; no approximate Wiki population becomes an exact count.
A normal-User-Agent MediaWiki API query on 2026-11-05 returned HTTP402; no HTML or protection
bypass was used. This does not prevent implementing the explicitly requested donor behaviour.

The native implementation uses repeated four-wave durable raid firings (one-hour cycles),
with the existing World reset calendar and the physical Item/Map/creature admission owner.
A new cycle starts from the scheduler's current admission time, never catches up missed
historical cycles, and uses ordinary firing/Channel receipts to prevent retry duplicates.
A restart preserves the existing epoch schedule rather than granting an extra startup wave.
Completed cycles retain ordinary creatures until reset, as the donor does.

## Existing dungeon spawn admission

`dungeon-spawn-bindings.json` records representative exact native source keys and
coordinate points near the donor entrance destinations, plus canonical definition references.
Nightmare samples bind all four W11 creature keys (Silencer, Retching Horror, Choking Fear,
Terrorsleep); the Fury destination sample binds Hellfire Fighter. These are evidence of
existing native sources, not an invented count or a complete dungeon census.
The native creature records' exact Canary source bindings are retained with those examples.
The compiler's `spawns_with` admits actual source points against native creature facts;
`BundleWorld::activation_spawns` refuses a creature without an admitted native profile;
`node/serve/channel_boot.rs` calls `realize_bundle_spawns` after composition and native policy
installation, which calls `begin_activation_spawn_sources` and paced owner realization.
This change preserves that source family and admission order; no alias or health-only actor
replaces any missing canonical source. The composed map is the same map used for collision
and source creature admission.

Absent OTBM tile flags are retained as `null` by
`tools/content-schema/world-authoring/otbm_reader.py:236`; the native compiler interprets
absence as the empty flag mask0, while preserving every explicit source flag bit.

Calendar compatibility: new raid waves use the existing authoritative World raid calendar
(`raid_schedule/calendar.rs`, `TIBIA_SERVER_SAVE_BERLIN_06`) and the normal scheduler's
one-minute eligibility checks. The first wave has zero delay from its admitted firing;
source-cycle repeat intervals are measured from that firing. Existing Rosemarie placement
uses the older UTC month flag, so the two existing calendar owners can differ at a June
boundary; this wave-admission change does not silently rewrite that established NPC rule.
The corresponding donor default is explicit: Crystal201189f
`src/items/tile.hpp:287` initializes `uint32_t flags = 0`; optional OTBM flags are only
applied when present (`src/io/iomap.cpp:255-267`).

Shared attempts are retained in the existing Channel raid ledger, keyed by firing and wave,
and charged before the current physical map read. A partial retry cannot refund attempts.
Counters use the existing ledger capacity and reserve its terminal marker before the first
read, with the existing50-wave bound per firing. No new durable table is introduced: a
restarted Channel's pending old-generation run becomes LOST through the existing raid
runner, rather than gaining a fresh placement budget or replaying its monsters.

Repeating Flower eligibility uses the active reset epoch's independently joined committed
`activated_at` date in Europe/Berlin, rather than the wall calendar. Thus a delayed May31
reset cannot start June waves just because the wall clock crossed midnight or06:00.
Once-only calendar raids retain their existing wall-date behaviour. The PostgreSQL case
covers both a still-committed May31epoch and a committed June1epoch. Terminal raid markers
remain after close, while their no-longer-needed shared attempt counters are released.

Fury off-day return decision: W10's343-permille availability controls entrance geometry only.
The same World/reset draw chooses one of the donor's ten cities every epoch, including an
inactive entrance day. A separately compiler-admitted return choice lets a character who
persisted inside the base dungeon leave through AID9715 after reset. It never marks the
inactive entrance layer active or admits AID9710. Native movement still refuses unloaded or
blocked return targets. The donor's always-active gate provides all ten source exits
(`fury_gates.lua:5-16,63-65`); the independent return choice is an explicit reversible native
compatibility decision for the accepted343-permille policy, not a claim about Global Tibia.

Furyosa binding: the existing canonical raid `raid:furyfate.furyosa` retains its donor
schedule (Crystal201189f `data-global/raids/raids.xml:80`, mean86400s, margin21600000ms)
and waves (`data-global/raids/fury_gate/furyosa.xml:1`). Accepted EVENTS-WORLD-1 W6/W10
requires `fury_gates`; the generated raid record now carries that typed dependency.
The actual composed Movement map supplies active entrance-layer names and its boot epoch,
not return-only metadata. A dependency is satisfied only by a matching dated composition.
The durable scheduler independently re-proves the current active epoch and target digest;
`game_world_reset_map_epoch_current` locks the active row FOR SHARE
(`0148_world_reset.sql:267-274`, historical migration read only). Yesterday's scheduler
therefore cannot reuse yesterday's active Fury against a newly activated epoch.
The focused PostgreSQL case asserts the canonical interval/margin, then explicitly amplifies
only its test clock to60seconds/zero margin to make the eligibility outcome deterministic.
It drives production node candidate selection and the durable scheduler for stale epoch,
undated geometry, exit-only metadata, active matching geometry and duplicate requests.

Epoch zero qualification: a fresh bundle World without an active reset boots its ordinary
base, with no daily layer, return choice or Rosemarie grant. The strict composition API still
rejects epoch0; no epoch1 is fabricated. The production binding helper owns this distinction.

Admission boundary: this package implements the three named mechanics through typed daily
selection, checked bundle composition, native movement/spawn owners and the durable raid
scheduler. The generic candidate-selection API in
`crates/gameplay/world-change/src/content.rs:117` remains source-only and continues refusing
unresolved W1 candidates. No success from that generic API is used to authorize these calls,
and this package does not claim that every generic world-change candidate is executable.
