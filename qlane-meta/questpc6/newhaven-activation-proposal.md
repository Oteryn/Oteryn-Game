# Newhaven activation proposal — not applied

This packet is review material for the lead's pending owner choice. It grants no
activation authority. Product helper/test files have been restored to their
generated gear-only state. Do not apply either proposal patch automatically.

## Capacity choice and smallest proposed change

Pinned Crystal201189f871d23597b82caf88fd611454a2ddefb8 XML gives corpse52113
capacity48. The accepted Game corpse Item admission requires capacity1..30
(`docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md:108`,
with the later accepted SUMMER30 limit; executable check at
`crates/gameplay/content/src/content/creature_reward.rs:615`). Current row failure
is `corpse_item_inadmissible`; no source uncertainty remains for this failure.

The bounded proposal is an explicit **Game30-slot projection** of that source
corpse. Preserve the source48 fact separately, retain the global30 placement limit,
and retain the existing strict admission gate. Change neither the source fact nor
the global capacity policy. The unapplied
`newhaven-native30-proposal.patch` shows the proposed native-only helper and one
new projection test: source constant48, proposed native constant30, immovable,
nonpickupable, Ground-only, bound to proven current appearance52113. The patch
adds only the initial corpse profile; it is not a complete lifecycle implementation.

The alternative is to leave Corruptor reward admission inactive until a separately
accepted source-capacity contract change. The current product checkpoint uses that
inactive behavior while the owner decision is pending.

## Exact source corpse lifecycle and current Game difference

Source: `data/items/items.xml` at the pinned Crystal revision, lines87366-84.

| Source Item | Duration | Source target | Source container size | Elapsed transform/retirement |
|---|---:|---:|---:|---:|
| 52113 | 600s | 52114 | 48 | 600s |
| 52114 | 600s | 52115 | 48 | 1200s |
| 52115 | 10s | 52116 | 48 | 1210s |
| 52116 | 600s | 0/remove | absent | 1810s |

The owner-pinned current15.30 appearance blob
`2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2`
contains these exact corpse objects:52113 sprite277071 with container/usable/
forceuse/unmove/corpse flags;52114 sprite277071 container/usable/corpse;52115
sprite277072 container/usable/corpse;52116 sprite202575 corpse. Those prove
presentation identity, not Item acceptance, source capacity, or scheduler support.

Current lifecycle ownership is concrete and bounded:

- `game_corpse_retirement_deadline`, migration0266 lines94-98, retires corpses
  without a source schedule after60s. Its existing scheduled SUMMER policy retires
  after670s; neither policy implements this source's1810s.
- `game_summer_corpse_chain` currently admits only four listed SUMMER chains.
  The original same-transaction corpse schedule proof, stage durations and edges
  are enforced by the database, not by appearance membership or arbitrary Item
  temporal fields (`0266_summer_corpse_lifecycle.sql:4-12,24-40,70-90`).
- The existing source deadline drain and fenced decay-retire owners already
  recover durable schedules. They should remain the sole timer/custody owners.
  No new timer, item store, wire command or best-effort disappearance is proposed.

A source-faithful lifecycle extension would therefore need a reviewed forward
migration, not edits to0266: admit the closed52113→52114→52115→52116 chain with
600000/600000/10000ms transforms and1810000ms final retirement; extend the original
corpse-mint schedule proof for precisely that policy; add the three later native
profiles and their proven bindings; retain exact current-scope fences, original
MINT provenance and unchanged descendant retirement/custody behavior. If Game30
projection is approved, the first three source48 containers need that same explicit
projection; final52116 has no source container size. Prove restart, no early stage,
same-corpse identity, child custody and terminal retirement through the existing
owners. Until that extension is accepted, do not describe a default60s corpse as
the source's complete lifecycle.

## Ordinary loot correction — exact source rows

Source: pinned Crystal
`data-global/monster/newhaven_update_2025/bosses/the_corruptor.lua:19,66-78`.
Line19 selects corpse52113. The chance denominator is100000; ppm is chance×10.
Counts below distinguish explicit Lua fields from omitted source fields.

| Line | Source numeric ID | Actual source comment/name | Current selected Item | Source chance | Explicit Lua counts |
|---|---:|---|---:|---:|---|
| 68 | 3031 | gold coin | 3031 | 100000 | min13,max46 |
| 69 | 7618 | health potion | 266 | 23810 | min1,max3 |
| 70 | 7620 | mana potion | 268 | 23810 | min1,max3 |
| 71 | 3338 | bone sword | 3338 | 14290 | omitted |
| 72 | 3359 | brass armor | 3359 | 4760 | omitted |
| 73 | 8820 | magician's robe | 7991 | 4760 | omitted |
| 74 | 3375 | soldier helmet | 3375 | 4760 | omitted |
| 75 | 3074 | wand of vortex | 3074 | 4760 | omitted |
| 76 | 3723 | white mushroom | 3723 | 4760 | max2 only |
| 77 | 44773 | refined bow | 51760 | 4760 | omitted |

These are explicit reconciliation choices using intended source comments and
current Wiki named loot. Exact rows for7618,7620,44773 are absent in the pinned
XML; do not cite that XML as defining their intended names. XML8820 and the current
client instead name Morik's Helmet; this conflicts with the monster comment and
current Wiki Corruptor's Magician's Robe7991. Current Wiki Mage Hat7992 is **not**
listed as a Corruptor drop and must not be substituted.

Current Wiki The Corruptor p109336/r1200696 separates ordinary gold/health potion/
mana potion/white mushroom/bone sword from very-rare vocation gear. This proposal
keeps both classes and preserves the selected donor numeric chances rather than
inventing numeric probabilities from Wiki rarity words. It also keeps the one-time
guaranteed armed branch alongside those independent ordinary drops.

Verified current Wiki IDs/revisions:
gold3031 p2119/r1175624; health266 p20308/r1183252; mana268 p20228/r1183192;
white mushroom3723 p1335/r1113687; bone sword3338 p2400/r1147274;
brass3359 p1615/r1186985; robe7991 p25343/r1186986;
helmet3375 p2304/r1147311; vortex3074 p4939/r1186984;
refined bow51760 p109332/r1148726. Current served appearances use those named IDs.
Omitted source counts are not asserted to be explicit1; existing native importer
lowering uses1..1 when maxCount is absent
(`complete_remaining_loot_fields.py:94-98`). The source-default proof belongs with
the generator change if its authoring path requires it.

## Proposed producer and qualification steps after authority resolves

1. Apply only the owner-selected capacity proposal. Keep canonical source Item
   shards unchanged and the global admission/placement limit unchanged.
2. In the root-owned loot-section producer, correct precisely the Corruptor's
   four namespace mappings:7618→266,7620→268,8820→7991,44773→51760. The current
   generated table has nine entries and omits refined bow: add the tenth source
   row at47600ppm with the same established omitted-count lowering. Preserve all
   other chance/count values and independent ordinary matching-item drops.
3. Produce loot-section Item admission facts from the accepted native overlays for
   the selected rows and projected corpse. Merely creating a native Item profile
   leaves the current section's materializablefalse/containerNone corpse facts
   unchanged. Do not bulk-admit unrelated inactive Items.
4. Regenerate through the root's existing producer, update manifest digests and
   pins through their existing generators, and validate data consistency. No
   generated JSON has been hand edited for this proposal.
5. Run the two real all-five/modifier dispatch PG scenarios with markers, including
   simulated batch failure,841→840 stamina retry, fixed guarantee placements,
   ordinary duplicates, retained-table recapture, Quick Loot and real hotkey equip.
   Rerun the preparation authority PG case, applicable lifecycle cases if that
   extension is included, and the root's combined final checks/review.

The current checkpoint has not completed those activation steps. Full Viola still
needs the placed NPC/book source and a real atomic book burn + Scarf mint +800XP
owner; no substitute starter kit or fabricated grant is proposed.
