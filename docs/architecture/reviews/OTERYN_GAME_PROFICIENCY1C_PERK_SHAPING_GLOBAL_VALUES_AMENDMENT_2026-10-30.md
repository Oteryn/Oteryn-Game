# PROFICIENCY-1C Perk shaping Global values (amendment of PROFICIENCY-1B)

- Decision: `PROFICIENCY1C-PERK-SHAPING-GLOBAL-VALUES-V1`
- Status: **CANDIDATE**. Signed by the work coordinator on 2026-10-30 under the owner's standing
  consents; the owner supplied the evidence source himself. Effective on owner review of the P08
  package PR, its exact-head CI and protected integration.
- Amends: PROFICIENCY-1B §3.1, §3.3 and §12 (`PROF1-RL-02`); PROFICIENCY-1 §4 (catalysts).
- Owning work: P08 (mechanic 41) on `claude/happy-davinci-ijz3wy-prof41`. Numbers: commands 379,
  380 and 392-395; migration 0575; burn cause ordinals 494 (catalyst) and 495 (orb).
- Runtime, migration and production authority: only through the P08 package PR.

## 1. Evidence source (amends 1B §3.3)

The owner supplied `https://www.tibiamonk.com/en/info/perk-shaping` and its vocation pages
`/en/info/perk-shaping/<vocation>` (read 2026-10-30) as owner-verified evidence. They join
1B §3.3's list as class `OWNER_VERIFIED`, after `CIPSOFT_OFFICIAL` and before English TibiaWiki.
OTS sources stay excluded.

## 2. Values (amends 1B §3.1)

Committed in `rulesets/progression/weapon-proficiency/shaping.json` (revision `shaping-r1`),
imported by `import_committed_proficiency_shaping`.

| Cell | Value | Evidence |
|---|---|---|
| MODIFY slot 1 | 250 dust, proficiency level ≥ 3 | tibiamonk: "The first exchange costs 250 Dust." |
| MODIFY slot 2 | 1,000 dust, weapon mastered | tibiamonk: "A second exchange costs 1,000 Dust and requires weapon mastery." |
| Rank steps 0→1 … 9→10 | 125, 200, 275, 350, 425, 500, 575, 650, 725, 800 (total 4,625) | tibiamonk refine table |
| RESHAPE_OFFER | 250 dust | tibiamonk: "Spend 250 Dust to roll new alternatives" |
| CLEAR | free; restores the original perk; refinement is lost | TibiaWiki Updates/15.30.a30dad; tibiamonk |
| ORB_RANK | 1 Lunar Ascension Orb (`i53695`); takes one modified perk to rank 10 | tibiamonk |
| Pool | 64 perks per vocation: 34 universal (5 offensive, 21 bestiary classes, 3 skill scaling, 3 sustain, 2 rune) and 5 augments for each of the vocation's 6 spells | tibiamonk vocation pages |
| Values | rank 0 and rank 10 of every entry; Greater Flurry of Blows base damage reaches 5 % at rank 10 | tibiamonk vocation pages |

Values keep the source's display unit (`percent` or `points`); the effect consumer converts them.

## 3. Declared assumptions and differences

- **Draw weights.** `ASSUMPTION: uniform; Global weights unpublished.` Every entry draws with weight
  1 (`PROFICIENCY_SHAPING_ASSUMED_WEIGHT`, the one place it is set). A published weight replaces
  it in a new shaping revision.
- **Ranks 1-9.** Their values are unpublished, so `RANK_UP` stays `NOT_ADMITTED` (1B §3.3).
  `MODIFY`, `RESHAPE_OFFER`, `RESHAPE_CHOOSE`, `CLEAR` and `ORB_RANK` read only evidenced cells.
- **Pool by weapon vocation (UNVERIFIED).** 1B keys shaping by proficiency definition, and the
  source publishes one pool per vocation. A definition takes the pool of its weapons' vocation:
  sword, axe and club → knight; distance and ammunition → paladin; wand → sorcerer; rod → druid;
  fist → monk. The seven definitions bound to both wands and rods (`p162`, `p177`, `p192`, `p217`,
  `p220`, `p221`, `p222`) have no shaping and answer `NOT_ADMITTED`.

## 4. Rank model (amends 1B §12 `PROF1-RL-02`)

`PROF1-RL-02` becomes **0..10**: MODIFY lands on rank 0 and ten steps reach rank 10 (tibiamonk:
"10 refinement ranks", refine table from rank 1 to rank 10). Migration 0575 widens the rank CHECKs
of 0055 forward (0055 is never edited), and the domain, the resource-limit entry
`docs/contracts/resource-limits/entries/0432-PROF1-RL-02.json` and the wire change with it in the
same batch. Until then the production shaping source exposes only cells the 1..10 model can carry,
so nothing is admitted on a value it cannot represent.

## 5. Orb burn (amends 1B §7.2)

`ORB_RANK` burns one Lunar Ascension Orb through the item-use record (cause ordinal 495, shape
Burn) in the receipt's transaction. Migration 0575 lifts 0060's `orb_cost = 0` pin forward.

## 6. Catalysts (amends PROFICIENCY-1 §4)

Admitted by the owner on 2026-10-30 as a progress grant, the composed burn-plus-receipt shape:
Lesser `i54266` 5,000, Proficiency `i51588` 25,000, Greater `i51589` 100,000 (English
TibiaWiki; Crystal `data/scripts/actions/items/proficiency_catalysts.lua:1-34`). The Test
Proficiency Catalyst `i52864` stays not admitted. Implemented by migration 0564 (cause 494).
