# QUEST-FINISH source decisions and limits

## Run8 current source decisions, 2026-11-06

These decisions supersede historical UNKNOWN/403 and pending-capacity notes below.
They select source facts and native projections; PostgreSQL qualification is a
separate result recorded by named tests and completion markers.

- Current Wiki access uses api.php and an honest User-Agent. The refreshed 93
  page bodies, page/revision metadata and hashes are retained in
  `.questpc/wiki-r8/`; `revisions.json` records the exact cuts. The complete
  infobox transclusion inventory has 788 pages: 765 main-namespace, 21 User,
  and two template/talk pages. Every pinned catalogue title occurs, but the
  inventory includes spoilers, lists and locations and is not 788 quests.
  Adventurers Guild, Friends and Traders and The Ultimate Challenges are
  collection headings; Access Quests is a list. They create no extra quests.
- The owner approved the native 30-slot projection of Corruptor's first three
  corpse stages; Crystal's source 48 remains recorded. The chain is
  52113→52114→52115→52116 with 600/600/10/600 seconds; the final stage is not a
  container. Migration1000 composes the existing corpse lifecycle owners.
  Ordinary source chances/counts remain intact. Named current items replace
  stale donor IDs 7618→266, 7620→268, 8820→7991 and omitted44773→51760.
  Bone Sword3338 and White Mushroom3723 also require exact native admission
  for real corpse custody; canonical source definitions are preserved.
- Simple Jo Staff Wiki51166 is an external Wiki item identifier. The named
  portable current client appearance is51119 (sprite239746); pinned Crystal
  independently names51119. Appearance51166 is an unrelated immovable object.
  Game identity i51119 and its binding remain unchanged. This is an explicit
  namespace reconciliation, not a silent substitution.
- Lost Wisdom uses Book of Wisdom51977→Scarf3572 plus800XP, retaining the
  donor mission/yes conversation. The return mission asks for confirmation;
  yes performs one atomic hand-in. The book chest is the actual mapped
 32614,32419,z7 object2472 at ordinal2, with no nonzero unique ID. The Wiki
  marker is one tile south. Its repeat is per character after300 seconds.
  Viola's actual source placement is32571,32507,z7; the nearby Wiki marker
  fits her source walk radius. Current Wiki retires her spell service.
- Key3620 is the Mintwallin Prison Key. Crystal's exact drawer source is
  UID5009 at32411,32155,z15, appearance2434, yielding key Item2969 with
  number3620. The Wiki marker is one tile south. The stale Black Knight
  section candidate does not override the named current item and real map
  source. This unlinked claim is assigned to the M lane; its source quest
  association remains unlinked. Rookgaard/Witch House unlinked claims are
  assigned to their R/W lanes.
- Runtime NPC activation here covers A Father's Burden, A Pirate's Tail,
  Bigfoot's Burden, Hero of Rathleton, Hot Cuisine and In Service of Yalahar,
  preserving the already active Stag. Exact historical hashes retain their
  old absent-track defaults without durable repinning. Fresh source planning
  uses its declared defaults; Hot Cuisine's zero is not changed to minus one.
  Father's acceptance is exactly one eleven-track transaction and receipt;
  unrelated operations retain the eight-track bound.
- Scope is canonical title A..M plus the explicit Corruptor/Viola/Stag fixes.
  Fresh Soul War/W02 source reads are retained for the R..S lane; this task
  does not add a competing room owner. No full quest or walk path is certified
  by a teleported source-selector reward fixture.

## Historical evidence (superseded where the run8 decisions differ)

Historical Cloud evidence snapshot. Current QUESTPC dispositions, including withdrawn
Newhaven/NPC/fee/book/callback drafts, are recorded in
`docs/architecture/reviews/QUEST-FINISH-2026-11-06.md`.

This records task-local evidence, not production deployment or current-Tibia verification.
The owner excludes Dawnport, OT-only features and Prey Hunting Tasks; Newhaven owns starting equipment.
Base: f52f71f7857b399dc8f6d70ff51c23f5b779f89d, with origin/main a9db6a76a merged without committing.
Integrated baseline tree: cf02deefb793053b9b2aa976cf11de1450b7317a.

## Before inventory

- Historical wiki catalogue: 373 titles; current inventory UNKNOWN (MediaWiki API HTTP 402, coordinator fetch requested).
- Canary: 324 implemented, 9 partial, 40 absent; 52 journal entries, 114 source groups, 978 scripts.
- Crystal: 328 implemented, 13 partial, 32 absent; 59 journal entries, 129 source groups, 1133 scripts.
- Canonical definitions: 352 (284 donor-derived, 68 authored).
- All 373 catalogue titles resolve: 278 direct, 27 family, 68 authored; missing canonical title references: 0.
- Rollout: 373 partial/needs-runtime, 0 runtime_enabled, 0 smoke_verified; these flags do not measure actual bespoke runtime support.
- Generic authoring preflight: 0 native bindings/deliveries, 1887 unbound stages; bespoke runtime adapters are outside that report.
- Historical normalized stage gaps: 4111 (3499 bespoke, 467 owner-pending, 145 shared); these are source projections, not executable failure counts.
- Chest claims: 336; 11 dispatch-PG qualified reward grants, 297 supported-unqualified, 28 not playable.
- Static callback records: 256 across 51 quests (167 step-in, 89 use), plus dynamic Rotten Blood altar.
- NPC programs: 13, 1123 rules, 78 NPCs; initially test-only.
- Full catalogue end-to-end quest proof: 0 recorded; reward-grant tests do not certify access paths or complete quests.
- Current-Tibia wiki verification: UNKNOWN; no successful current API read during this task.

## Charge defaults (DERIVED)

Canary 04b source items.xml lines 6946, 6966, 7255, 7272, 7283, 7294 and 8525
supply full charges for Items 3054=200, 3056=200, 3082=50, 3083=150,
3084=250, 3085=200 and 3284=1. The runtime already represents an absent
item_timed_state row as full definition charges. Admit only exact positive donor/default
matches and retain any other hold. Do not turn raw count into an Item definition.
The pinned source_charge_evidence.json validates each container placement separately.
Raw count=1 conflicting with a greater definition default remains held: Crystal
player_functions.cpp:2815-2843 and container_functions.cpp:177-183 forward subtype;
item.cpp:335-339 treats nonzero as charges. It is not evidence for a full-charge reward.

## Medusa donor difference (CONFLICT, explicit Crystal selection)

Crystal 201189f871d23597b82caf88fd611454a2ddefb8
`data-global/startup/tables/chest.lua:2017-2024` gives bag2853 with
3436,3567,3324, one each, at33049,32399,10 / UID6183.
Its quest_reward_common.lua:42-49 associates the fox text with UID6179/book2815.
Canary 47dfd51f quest_reward_common.lua:40-47 associates that text with6183;
its container helper:119-123 applies text to every non-key child.
Use the complete Crystal Medusa recipe, without inventing a book or annotating armor/weapons.
The native projection explicitly records CRYSTAL_MEDUSA_WITHOUT_CANARY_TEXT.
Keep the original Canary SOURCE packet and unresolved carrier hold unchanged.
Current-Tibia corroboration remains UNKNOWN pending the requested MediaWiki responses.

## Qualification limits

SOURCE receipt counters are immutable historical evidence. Native admission is separate.
A synthetic exact-source-selector USE proves reward granting and replay, not reaching the chest.
A Canary-only placement on a Crystal world must refuse without minting; positive sibling
placements do not qualify that unavailable selector. Whole-quest and deployed-world claims
require independent actual access-path execution.

## Coordinator API responses received 2026-11-05

The owner supplied MediaWiki JSON in this conversation; no local HTTP success is claimed.
Medusa base page3793/rev771196 and spoiler4919/rev748935 corroborate the
Medusa Shield, Blue Robe and Skull Staff reward; no fox-text carrier is specified.
The spoiler explicitly warns that its maps predate Drefia's revamp: it does not
verify the donor chest coordinates or the complete modern access path.
Goblin Temple base3574/rev683865 explicitly includes a vial of milk;
spoiler8013/rev748709 supplies the path but no numeric fluid subtype.
Dreamer's Challenge base12354/rev869211 and spoiler12355/rev891457 explicitly
say all four boxes contain a bag with wine, bread and certificate.
Present base3214/rev1044890 and spoiler5480/rev749066 list backpack, jug, plate,
cup and present; they do not specify liquid contents. Do not infer filled vessels.

Current gear page facts: Harmony Amulet108150/rev1194222=50195;
Magician's Robe25343/rev1186986=7991; Ranger's Cloak25353/rev1186981=3571;
Refined Bow109332/rev1148726=51760. These agree with reconciled donor identities.
Simple Jo Staff108393/rev1186982 instead says51166, conflicting with the pinned
Crystal XML and appearances namespace: named portable staff51119, but51166 is
nameless/unmovable and absent from canonical Item definitions/exact bindings.
No admission of either ID as the verified current staff was made. The draft
Newhaven capsule and Rust loot correction still select51119 and MUST be revised
or explicitly reconciled before integration. This is an open source/version question.

The embedded-in inventory response supplied500 pages and continuation0|64408;
this is not500 distinct quests (includes spoilers and other transcluding pages).
The remaining inventory and current base/spoiler bodies were requested together;
no response to that second batch was received before the owner stopped the task.
All new implementation stopped at the owner's handover instruction.

## PC resume source read, 2026-11-06

Fresh read-only GETs of Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
(`zimbadev/crystalserver`) reproduce chest.lua SHA256
`7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4`
and quest_reward_common.lua SHA256
`9b38b43e800e05bd0262753198ec3b9adcb64a523ff3b085bf2e79fba05b95ad`.
The fresh `src/utils/utils_definitions.hpp` SHA256 is
`03ebfc0da66b01cdef46dca4195c4f1a0132a0fce058d6c670cb401bcc35ae7d`:
line 441 defines water=1, line 442 wine=2, line 449 milk=9. These
are **PROVEN donor enum values**, not current-Tibia qualification.

This narrows the Goblin Temple and Dreamer's Challenge issue from an unknown
number to **CONFLICT**: their raw vial argument 1 denotes water, while their
named reward evidence says milk/wine. Present likewise has raw 1 for the
jug and cup; neither its inherited wiki list nor a bare vessel name establishes
filled contents. No recipe or canonical fluid admission was guessed.
The existing fluid MINT draft remains unqualified and no liquid recipe is admitted.

MediaWiki API read attempts to `tibia.fandom.com/api.php` and
`tibiawiki.com.br/api.php` both returned HTTP 403. No Wiki HTML, protection
bypass, authenticated route or current-wiki completeness claim was used.

The owner-excluded Dawnport quest catalogue, callbacks, reward claims and
Morris/Woblin quest offers remain excluded. This task does not change the
pre-existing vocation-choice service or its tests; it adds no Dawnport work.
Fresh validation and the still-open component inventory are in
`.questpc/RESUME.md`; no earlier Cloud or PC result qualifies this tree.
