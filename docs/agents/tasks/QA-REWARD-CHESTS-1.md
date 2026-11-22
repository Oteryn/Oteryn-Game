# QA-REWARD-CHESTS-1 — reward-claim chests match Tibia Global (qa lane)

```yaml
task_id: QA-REWARD-CHESTS-1
title: "QA-REWARD-CHESTS-1: reward-claim chests, exact Tibia Global rewards"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
base_sha: e2152d050038cecdb3728c222af2ac184e1cac5a
branch: claude/happy-davinci-ijz3wy-qa
pr: 3282
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Owner rule (2026-11-17): where a Canary/Crystal donor differs from Tibia Global,
Global wins. Corrections live in
`tools/content-schema/reward-claim-authoring/global_corrections.py`, each pinned
to its donor fact (fails closed on donor drift).

## Corrections (TibiaWiki revids)

| quest | change | evidence |
|---|---|---|
| Captain Iglues Treasure | 1 → 2 Salmon | Spoiler rev 1013321 (book + 12 salmon is the daily-respawn left chest) |
| Alawar's Vault, Key 4502 bag | 23 → 33 gold | Quest rev 683774, Spoiler rev 1029814 |
| Fire Axe, first chest | bag without the Fire Axe (Pile of Bones only) | Spoiler rev 855030 |
| Power Ring, first box | gold coin → Power Ring | Quest rev 683946 |

Checked against the wiki, no change: Mintwallin (small diamond only; rest is
daily respawn), Elephant Tusk (2x Tusk 3044, rev 683843/1114030), Blood Herb,
Studded Legs/Shield chests, Six Rubies, Throwing Star, Lone Medusa, Combat Knife
and Nightmare Teddy UIDs (Crystal 5025/6305 bind at runtime), The Way of the
Monk, Steel Helmet scroll text (rev 1043623), Ornamented Shield bags
(rev 1043470), Mad Mage potions (daily respawn, rev 1117012), and 45 batch-2
quests whose claims match the wiki totals.

## Routed to the server lane (generic mechanics)

- Authored written text and containers on reward claims: Purple Tome maps
  (rev 1022778/1022275), Crystal Wand bag + Twinkiller Rune (rev 815925/1022096),
  Isle of the Mists book ("The Plains of Havoc", Book (Black)).
- NPC exchanges: Willie banana → Studded Shield (rev 749200), Lee'Delle honey
  flower → Studded Legs (rev 749199), Wyda blood herb → witchesbroom (rev 855039),
  Seymour present → legion helmet, Amber notebook → short sword.
- Numbered door key on native_authored_v1 claims (Alawar Keys 4501/4503).
- Demon outfit addon grant (Demon Helmet), party-size Desert Dungeon golden bag.

## Open facts (owner browser / YouTube batch; donor values kept)

1. Iron Helmet: 3x Sudden Death (3155) vs wiki "Sudden Death Rune" without a count (rev 747234).
2. Iron Helmet: Porgol's Strange Behaviour is a Stamped Letter "Dear Muriel!" (rev 841458); item id 3506 unverified.
3. Fire Axe second chest (300 gp, steel shield, plate legs, double axe, dark helmet): once-only on Global? Not claimed.
4. Alawar Keys 4501 (chest 2472 at 32172,31602,10) and 4503 (one of the chests at 32030/32031,31686/31688,8): no donor UID, once-only unconfirmed, which 4503 chest unknown.
5. Power Bolts Pile of Bones with the Two Handed Sword: position unknown; no placement authored.
6. Isle of the Mists drawer (second emerald + book): no drawer on our map.

## Validation state

- `test_reward_claim_authoring.py::test_committed_content_carries_tibia_global_corrections`: red on the old content, green now.
- PostgreSQL: all 35 cases passed through the local shard (17 m35, 17 qready1 and 1 QA Power Ring). The qready1 run refreshed `reward-use-proofs/qready1.json`. The Fire Axe bag, Fire Axe two-floor and Power Ring USE cases reproduced the missing placements and passed with the generated overlay.
- The chest-binding overlay, Item spell registrations and World pins were regenerated. `pin-check` and the World Project v2 tree equivalence validator passed.
- Reward-claim authoring passed 121 tests, including `test_container_native.py`.

## Batches 3 and 4

- Batch 3: 22 single-chest quests match the wiki (Adorned UH .. Spike Sword); no change.
- Batch 4: 206 placements across ~45 multi-chest quests; 100 OK. Ghostsilver Lantern web: Global gives the lit lantern 23738 (rev 1148076), but 23738 has no native item profile (PG DefinitionNotFound, qa5 run), so the claim stays on the donor 23734 until the server lane admits the 23738 profile (qaitems).
- Book texts held on the donor: Global texts for Power Bolts 6112 (rev 841657), Dreamer's Challenge book 6196 (rev 1114576) and Pits of Inferno Tible 6202 (rev 1022683) were authored in eb5868346 and reverted. world_reward_claims.rs pins each written-text uid to its donor body sha256, so the claims stopped binding (PG panic: source namespace must match the native World binder, qa6). The Global texts go to the server lane (rcauth) to repin; then eb5868346 is re-applied.
- Correct item not yet materializable (server/items lane): Giant Smithhammer box 12510 → 3208
  (Explorer Society/Spoiler rev 1090719), Beregar firewalker boots 9019 → 9018 (rev 1148768),
  Secret Service poem scroll 5952 → 347 (rev 1115206), Secret Service lottery ticket 5956 → 5957 (rev 1114329).
- BLOCKED-ENGINE: Kingdom of Kormarak corpse is a daily respawn (Spoiler rev 884223); waits on the daily-reset mechanic.
- Platinum count of the u8_1 boss reward chest (100 vs 50): owned by qcx6 (record 78); not changed here.
- Owner batch: Magic Sword north wand (Yellow Spell Wand), Dark Trails notebook/Shargon chest positions,
  missing one-time items (Outlaw Camp bag, Shargon chest, Crystal of Focus, Explorer Society books), 53 placements without wiki contents.
